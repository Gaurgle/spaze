# Spaze

A terminal-first, project-centric team collaboration tool. Self-hostable, open source, GitHub-authenticated.

> **Status:** pre-alpha. Phase 1.C shipped 2026-05-06 — slash command parser with `/quit`, `/help`, `/me`, `/clear`, live input coloring (green/red while typing), `MessageBody::Action` proto variant, and `spaze-commands` crate. 91 tests cumulative. Phase 1.B complete — real TUI with Catppuccin Mocha theme, two terminals chat with author names.

## Why?

Slack and Discord are great until you want to run your own. Mattermost and Rocket.Chat are heavyweight. Spaze aims to be a single static binary, easy to self-host, and pleasant to use from a terminal — with notes and todos as first-class artifacts captured inline through chat.

## Highlights

- **Single static binary** for server and client — no runtime dependencies.
- **GitHub OAuth Device Flow** for identity — no password store on the server.
- **TUI-first client** built with `ratatui`, mouse + vim + buttons all first-class peers.
- **Inline `#note` and `#todo` capture** routed through the same parser as slash commands.
- **Server-side full-text search** via SQLite FTS5.
- **SQLite-backed**, with migrations baked into the server binary.
- **Self-hosting in 10 minutes** via Docker Compose + Caddy.

## Repository Layout

```
spaze-proto/      Wire protocol types
spaze-crypto/     Ed25519 device keys (message signing)
spaze-storage/    SQLite + sqlx persistence layer
spaze-server/     The daemon (`spaze-server` binary)
spaze-client/     The TUI client (`spaze` binary)
spaze-commands/   Slash command + inline tag registry
```

## Building

Requires Rust 1.85 or newer.

```bash
cargo build --workspace
```

## Running (Phase 1.B bare-WS TUI demo)

Three terminals — one server, two clients:

```bash
# Terminal 1
cargo run --bin spaze-server

# Terminal 2
cargo run --bin spaze -- --name andreas

# Terminal 3
cargo run --bin spaze -- --name beth
```

Each client opens an alt-screen TUI with sidebar (server + room), tab strip, room header, message timeline, and input box. Press `i` to enter Insert mode, type a message, press Enter to send. `Esc` returns to Normal mode. `Tab` cycles tabs (room ↔ help). `Ctrl+B` toggles the sidebar. `?` opens the help buffer. `Ctrl+C` quits cleanly. `q` also quits.

Authors render as their display names (the `--name` value) in `<andreas>` style. Catppuccin Mocha is the theme.

## Documentation

- [DESIGN.md](./DESIGN.md) — architecture, scope, and locked decisions.
- `LICENSE-APACHE` - Apache 2.0.

## Contributing

Pre-alpha; not accepting external contributions yet. Star and watch if you're curious - issues will open once the MVP is functional.

## License

Apache-2.0.
