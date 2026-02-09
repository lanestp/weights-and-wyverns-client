# Weights & Wyverns Client

MCP client for [Weights & Wyverns](https://github.com/lanestp/weights-and-wyverns) — a multiplayer text dungeon that runs inside Claude Code.

## Installation

```bash
# Clone and build
git clone https://github.com/lanestp/weights-and-wyverns-client.git
cd weights-and-wyverns-client/client
cargo build --release

# Register with Claude Code
claude mcp add --transport stdio ww ./target/release/ww-client
```

## Usage

Once registered, use the `/ww` slash command in Claude Code to connect and play.

## License

MIT
