//! WebSocket connection to the game server.
//!
//! Manages a persistent WebSocket link to the centralized game server,
//! handling request/response correlation and push event forwarding.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

/// Timeout for waiting on a server response to a command.
///
/// Based on upstream server timeout policies; large enough
/// for the game server to process any command.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors arising from game server communication.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    /// The WebSocket connection could not be established.
    #[error("failed to connect to game server at {url}: {source}")]
    Connect {
        url: String,
        source: tokio_tungstenite::tungstenite::Error,
    },

    /// Sending a message over the WebSocket failed.
    #[error("failed to send message: {0}")]
    Send(#[from] tokio_tungstenite::tungstenite::Error),

    /// The response timed out after the configured duration.
    #[error("server response timed out after {0:?}")]
    Timeout(Duration),

    /// The server sent a response that could not be parsed as JSON.
    #[error("invalid JSON from server: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// The connection is not established.
    #[error("not connected to game server — call `connect` first")]
    NotConnected,

    /// The response channel was dropped before a response arrived.
    #[error("response channel closed unexpectedly")]
    ChannelClosed,
}

/// A pending request awaiting its response from the server.
type PendingRequest = oneshot::Sender<Value>;

/// Shared state for the background WebSocket reader task.
#[derive(Debug)]
struct ConnectionInner {
    pending: HashMap<String, PendingRequest>,
    next_id: u64,
}

/// Manages a WebSocket connection to the game server.
///
/// Spawns a background task that reads from the WebSocket, routing
/// responses to waiting oneshots and push events to the event channel.
#[derive(Debug)]
pub struct GameConnection {
    inner: Arc<Mutex<ConnectionInner>>,
    write_tx: Option<mpsc::Sender<Message>>,
    event_tx: mpsc::UnboundedSender<Value>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl GameConnection {
    /// Creates a new unconnected game connection.
    ///
    /// Push events will be forwarded to `event_tx` for buffering.
    pub fn new(event_tx: mpsc::UnboundedSender<Value>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ConnectionInner {
                pending: HashMap::new(),
                next_id: 1,
            })),
            write_tx: None,
            event_tx,
            shutdown_tx: None,
        }
    }

    /// Returns true if a WebSocket connection is active.
    pub fn is_connected(&self) -> bool {
        self.write_tx.is_some()
    }

    /// Establishes a WebSocket connection to the game server.
    ///
    /// Spawns a background reader task that routes incoming messages.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::Connect` if the WebSocket handshake fails.
    pub async fn connect(&mut self, url: impl AsRef<str>) -> Result<(), ConnectionError> {
        let url = url.as_ref();
        let (ws_stream, _response) =
            tokio_tungstenite::connect_async(url)
                .await
                .map_err(|e| ConnectionError::Connect {
                    url: url.to_owned(),
                    source: e,
                })?;

        let (ws_write, ws_read) = ws_stream.split();

        // Channel for sending messages to the WebSocket writer task.
        let (write_tx, mut write_rx) = mpsc::channel::<Message>(64);

        // Shutdown signal for the background tasks.
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        // Writer task: forwards messages from the channel to the WebSocket.
        tokio::spawn(async move {
            let mut ws_write = ws_write;
            while let Some(msg) = write_rx.recv().await {
                if ws_write.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Reader task: routes incoming messages to pending requests or events.
        let inner = Arc::clone(&self.inner);
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            let mut ws_read = ws_read;
            loop {
                tokio::select! {
                    msg = ws_read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                route_message(&inner, &event_tx, &text).await;
                            }
                            Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                            _ => {}
                        }
                    }
                    _ = &mut shutdown_rx => break,
                }
            }
        });

        self.write_tx = Some(write_tx);
        self.shutdown_tx = Some(shutdown_tx);

        tracing::info!(server.url = url, "connection.established");
        Ok(())
    }

    /// Sends a command to the game server and awaits the response.
    ///
    /// Assigns a unique message ID for request/response correlation.
    /// Times out after 30 seconds.
    ///
    /// # Errors
    ///
    /// Returns `ConnectionError::NotConnected` if no connection is active,
    /// `ConnectionError::Timeout` if the server does not respond, or
    /// `ConnectionError::Send` if the write fails.
    pub async fn send_command(
        &self,
        action: impl AsRef<str>,
        params: Value,
    ) -> Result<Value, ConnectionError> {
        let write_tx = self
            .write_tx
            .as_ref()
            .ok_or(ConnectionError::NotConnected)?;

        let (msg_id, rx) = {
            let mut inner = self.inner.lock().await;
            let id = format!("msg-{:04}", inner.next_id);
            inner.next_id += 1;
            let (tx, rx) = oneshot::channel();
            inner.pending.insert(id.clone(), tx);
            (id, rx)
        };

        let payload = serde_json::json!({
            "id": msg_id,
            "action": action.as_ref(),
            "params": params,
        });

        let msg = Message::Text(payload.to_string().into());
        write_tx
            .send(msg)
            .await
            .map_err(|_send_err| ConnectionError::NotConnected)?;

        tracing::debug!(
            msg.id = %msg_id,
            msg.action = action.as_ref(),
            "connection.command.sent"
        );

        match tokio::time::timeout(RESPONSE_TIMEOUT, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(ConnectionError::ChannelClosed),
            Err(_) => {
                // Remove the stale pending entry.
                let mut inner = self.inner.lock().await;
                inner.pending.remove(&msg_id);
                Err(ConnectionError::Timeout(RESPONSE_TIMEOUT))
            }
        }
    }

    /// Gracefully closes the WebSocket connection.
    pub async fn disconnect(&mut self) {
        if let Some(write_tx) = self.write_tx.take() {
            let _ = write_tx.send(Message::Close(None)).await;
        }
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        tracing::info!("connection.closed");
    }
}

/// Routes an incoming WebSocket text message to either a pending request
/// or the event buffer.
async fn route_message(
    inner: &Arc<Mutex<ConnectionInner>>,
    event_tx: &mpsc::UnboundedSender<Value>,
    text: &str,
) {
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "connection.message.invalid_json");
            return;
        }
    };

    // If the message has an "id" field, it is a response to a pending request.
    if let Some(id) = value.get("id").and_then(Value::as_str) {
        let mut inner = inner.lock().await;
        if let Some(tx) = inner.pending.remove(id) {
            let _ = tx.send(value);
            return;
        }
    }

    // Otherwise treat it as a push event.
    let _ = event_tx.send(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_starts_disconnected() {
        let (event_tx, _rx) = mpsc::unbounded_channel();
        let conn = GameConnection::new(event_tx);
        assert!(!conn.is_connected());
    }

    #[tokio::test]
    async fn send_command_while_disconnected_returns_error() {
        let (event_tx, _rx) = mpsc::unbounded_channel();
        let conn = GameConnection::new(event_tx);
        let result = conn.send_command("look", serde_json::json!({})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConnectionError::NotConnected));
    }
}
