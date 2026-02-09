# Weights & Wyverns Client

MCP client for [Weights & Wyverns](https://wyverns.ai) — a multiplayer text dungeon that runs inside Claude Code.

## Installation

### Pre-built binaries (recommended)

Download the latest release for your platform from [Releases](https://github.com/lanestp/weights-and-wyverns-client/releases).

```bash
# macOS (Apple Silicon)
curl -L https://github.com/lanestp/weights-and-wyverns-client/releases/latest/download/ww-client-macos-aarch64.tar.gz | tar xz
claude mcp add --transport stdio ww ./ww-client -- --server wss://wyverns.ai/ws

# macOS (Intel)
curl -L https://github.com/lanestp/weights-and-wyverns-client/releases/latest/download/ww-client-macos-x86_64.tar.gz | tar xz
claude mcp add --transport stdio ww ./ww-client -- --server wss://wyverns.ai/ws

# Linux (x86_64)
curl -L https://github.com/lanestp/weights-and-wyverns-client/releases/latest/download/ww-client-linux-x86_64.tar.gz | tar xz
claude mcp add --transport stdio ww ./ww-client -- --server wss://wyverns.ai/ws
```

### Build from source

```bash
git clone https://github.com/lanestp/weights-and-wyverns-client.git
cd weights-and-wyverns-client/client
cargo build --release
claude mcp add --transport stdio ww ./target/release/ww-client -- --server wss://wyverns.ai/ws
```

## Usage

Once registered, use the `/ww` slash command in Claude Code to connect and play.

## License

MIT
