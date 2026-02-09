//! Weights & Wyverns MCP server entry point.
//!
//! Runs a local MCP server over stdio that bridges Claude Code
//! to the centralized game server via WebSocket. All game logic
//! lives on the server; this binary is a thin transport layer.

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod connection;
mod events;
mod tools;

use clap::Parser;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

/// MCP server for Weights & Wyverns.
#[derive(Debug, Parser)]
#[command(name = "ww-client", version, about)]
struct Args {
    /// WebSocket URL of the game server.
    #[arg(long, default_value = "ws://localhost:8080/ws")]
    server: String,

    /// Path to the authentication token file.
    #[arg(long, default_value = "~/.weights-and-wyverns/token")]
    token_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging must go to stderr — stdout is reserved for the MCP protocol.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    tracing::info!(
        server.url = %args.server,
        token.path = %args.token_path,
        "mcp.server.starting"
    );

    let handler = tools::GameHandler::new(args.server);
    let service = handler.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}
