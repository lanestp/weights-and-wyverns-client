# Weights & Wyverns Client

MCP client for [Weights & Wyverns](https://wyverns.ai) — a multiplayer text dungeon that runs inside Claude Code.

## Quick Start

Paste this into Claude Code and it will do the rest:

> Set up Weights & Wyverns for me: Download the latest release from https://github.com/lanestp/weights-and-wyverns-client/releases/latest (pick the right binary for my OS/arch), extract it, register it as a Claude Code MCP server named "wyvern" with: `claude mcp add -t stdio wyvern /path/to/wyvern -- --server wss://wyverns.ai/ws`, then download the /wyvern slash command: `mkdir -p .claude/commands && curl -sL https://wyverns.ai/static/wyvern-skill.md -o .claude/commands/wyvern.md`

Then restart Claude Code and type `/wyvern` to play.

## Setup with Pre-built Binaries (Recommended)

Pre-built binaries are available on the [Releases](https://github.com/lanestp/weights-and-wyverns-client/releases/latest) page for:

| Platform | Asset |
|----------|-------|
| macOS (Apple Silicon) | `wyvern-macos-aarch64.tar.gz` |
| macOS (Intel) | `wyvern-macos-x86_64.tar.gz` |
| Linux (x86_64) | `wyvern-linux-x86_64.tar.gz` |
| Linux (aarch64) | `wyvern-linux-aarch64.tar.gz` |
| Windows (x86_64) | `wyvern-windows-x86_64.zip` |

### 1. Download and extract

```bash
# macOS Apple Silicon example:
curl -sL https://github.com/lanestp/weights-and-wyverns-client/releases/latest/download/wyvern-macos-aarch64.tar.gz | tar xz
```

### 2. Register with Claude Code

```bash
claude mcp add -t stdio wyvern ./wyvern -- --server wss://wyverns.ai/ws
```

### 3. Install the slash command

```bash
mkdir -p .claude/commands
curl -sL https://wyverns.ai/static/wyvern-skill.md -o .claude/commands/wyvern.md
```

### 4. Play

Start a new Claude Code session and type `/wyvern`.

## Build from Source

Requires [Rust](https://rustup.rs/) 1.75+.

```bash
git clone https://github.com/lanestp/weights-and-wyverns-client.git
cd weights-and-wyverns-client/client
cargo build --release
```

Then register the built binary:

```bash
claude mcp add -t stdio wyvern ./target/release/wyvern -- --server wss://wyverns.ai/ws
```

## License

MIT
