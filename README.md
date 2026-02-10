# Weights & Wyverns Client

MCP client for [Weights & Wyverns](https://wyverns.ai) — a multiplayer text dungeon that runs inside Claude Code.

## Quick Start

Paste this into Claude Code and it will do the rest:

> Set up Weights & Wyverns for me: Clone https://github.com/lanestp/weights-and-wyverns-client, build it with `cargo build --release`, register it as a Claude Code MCP server named "wyvern" with: `claude mcp add -t stdio wyvern ./target/release/ww-client -- --server wss://wyverns.ai/ws`, then download the /wyvern slash command: `mkdir -p .claude/commands && curl -sL https://wyverns.ai/static/wyvern-skill.md -o .claude/commands/wyvern.md`

Then restart Claude Code and type `/wyvern` to play.

## Manual Setup

### 1. Build the client

```bash
git clone https://github.com/lanestp/weights-and-wyverns-client.git
cd weights-and-wyverns-client/client
cargo build --release
```

### 2. Register with Claude Code

```bash
claude mcp add -t stdio wyvern ./target/release/ww-client -- --server wss://wyverns.ai/ws
```

### 3. Install the slash command

The `/wyvern` command is included in this repo at `.claude/commands/wyvern.md`. If you're working from this directory, it's already available. Otherwise:

```bash
mkdir -p .claude/commands
curl -sL https://wyverns.ai/static/wyvern-skill.md -o .claude/commands/wyvern.md
```

### 4. Play

Start a new Claude Code session and type `/wyvern`.

## License

MIT
