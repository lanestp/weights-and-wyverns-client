//! Push event buffer for game server events.
//!
//! Accumulates unsolicited events (`player_entered`, combat updates, etc.)
//! between MCP tool calls. Events are drained and included with each
//! tool response so Claude can narrate them naturally.

use serde_json::Value;
use tokio::sync::mpsc;

/// Maximum events to buffer before producing an overflow warning.
///
/// Prevents unbounded memory growth if the player is idle for
/// a long time while many server events arrive.
const MAX_BUFFERED_EVENTS: usize = 200;

/// Buffers push events from the game server between tool calls.
///
/// Events accumulate via an unbounded channel receiver and are
/// drained in bulk when a tool response is being assembled.
#[derive(Debug)]
pub struct EventBuffer {
    rx: mpsc::UnboundedReceiver<Value>,
}

impl EventBuffer {
    /// Creates a new event buffer from the given channel receiver.
    pub fn new(rx: mpsc::UnboundedReceiver<Value>) -> Self {
        Self { rx }
    }

    /// Drains all buffered events, returning them as a vector.
    ///
    /// If more than 200 events have accumulated, the excess is
    /// discarded and a synthetic `events_overflow` event is appended
    /// to signal that some events were lost.
    pub fn drain(&mut self) -> Vec<Value> {
        let mut events = Vec::new();

        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
            if events.len() >= MAX_BUFFERED_EVENTS {
                // Discard remaining events and signal overflow.
                let mut overflow_count: usize = 0;
                while self.rx.try_recv().is_ok() {
                    overflow_count += 1;
                }
                events.push(serde_json::json!({
                    "type": "event",
                    "data": {
                        "event": "events_overflow",
                        "message": format!(
                            "{overflow_count} events were dropped due to buffer overflow"
                        )
                    }
                }));
                break;
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_empty_buffer() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let mut buffer = EventBuffer::new(rx);
        let events = buffer.drain();
        assert!(events.is_empty());
    }

    #[test]
    fn drain_returns_buffered_events() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut buffer = EventBuffer::new(rx);

        tx.send(serde_json::json!({"type": "event", "data": {"event": "player_entered"}}))
            .expect("send should succeed");
        tx.send(serde_json::json!({"type": "event", "data": {"event": "combat_update"}}))
            .expect("send should succeed");

        let events = buffer.drain();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn drain_caps_at_max_with_overflow_event() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut buffer = EventBuffer::new(rx);

        for i in 0..250 {
            tx.send(serde_json::json!({"type": "event", "data": {"index": i}}))
                .expect("send should succeed");
        }

        let events = buffer.drain();
        // 200 real events + 1 overflow event
        assert_eq!(events.len(), MAX_BUFFERED_EVENTS + 1);

        let last = events.last().expect("should have events");
        let event_name = last
            .get("data")
            .and_then(|d| d.get("event"))
            .and_then(|e| e.as_str());
        assert_eq!(event_name, Some("events_overflow"));
    }
}
