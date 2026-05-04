# Spaze Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the Spaze repository skeleton: Cargo workspace, license, design doc, README, a real first sketch of the `spaze-proto` crate (IDs, messages, events with request/response correlation, errors) with passing tests, and a CI skeleton — ready for ongoing development.

**Architecture:** A six-crate Cargo workspace using prefix-style names (`spaze-proto`, `spaze-crypto`, `spaze-storage`, `spaze-server`, `spaze-client`, `spaze-commands`). All but `spaze-proto` are empty stubs in this bootstrap session. `spaze-proto` defines the wire protocol — newtype-wrapped UUIDv7 IDs, JSON-serializable messages and events via serde, request/response correlation frames (`ClientFrame` / `ServerFrame`), and a typed `ProtocolError` enum. **No theming or buffer abstraction in this bootstrap** — those are Phase 1 work, not part of the workspace skeleton.

**Tech Stack:** Rust 2024 edition, cargo workspace, serde + serde_json, uuid (v7), thiserror, GitHub Actions CI (cargo check / test / clippy / fmt).

**Conventions for this plan (per user's CLAUDE.md):**
- **Commits:** Conventional Commits format (`feat:`, `chore:`, etc.). First line ≤72 chars. **No `Co-Authored-By` lines.** Author runs commits and pushes themselves — these tasks present the exact command for review, they do not auto-execute.
- **Authorization gates:** Tasks that create commits, create the GitHub repo, or push are flagged. Pause and confirm with the user before running them.

---

## File Structure

Top-level layout this plan creates:

```
Spaze/
├── Cargo.toml                # workspace manifest
├── LICENSE-APACHE            # full Apache-2.0 text
├── README.md                 # vision summary + status + DESIGN link
├── DESIGN.md                 # canonical design doc
├── .gitignore                # Rust-flavored
├── .github/
│   └── workflows/
│       └── ci.yml            # check / test / clippy / fmt
├── docs/
│   └── superpowers/
│       └── plans/
│           └── 2026-05-02-spaze-bootstrap.md  # (this file)
├── spaze-proto/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # re-exports + version constant + crate-level tests
│       ├── ids.rs            # UserId, DeviceId, SpaceId, RoomId, MessageId
│       ├── messages.rs       # Message (with edited_at/deleted_at), MessageBody
│       ├── events.rs         # ClientFrame/ServerFrame, ClientCommand, ServerEvent, ResponsePayload, RequestId
│       └── error.rs          # ProtocolError
├── spaze-crypto/
│   └── src/lib.rs            # empty stub
├── spaze-storage/
│   └── src/lib.rs            # empty stub
├── spaze-server/
│   └── src/main.rs           # binary stub (`spaze-server`)
├── spaze-client/
│   └── src/main.rs           # binary stub (`spaze`)
└── spaze-commands/
    └── src/lib.rs            # empty stub
```

Responsibility split:
- **`spaze-proto`** — wire-level types only. No I/O, no business logic. Pure serde-derivable data + a small `ProtocolError`. Importable by every other crate.
- **`spaze-crypto`** (stub) — Ed25519 device keypairs for message signing. (E2E was dropped from MVP — no OpenMLS.)
- **`spaze-storage`** (stub) — sqlx + SQLite migrations later.
- **`spaze-server`** (stub binary) — daemon. Top-level binary name: `spaze-server`.
- **`spaze-client`** (stub binary) — TUI client. Top-level binary name: `spaze` (per locked decision).
- **`spaze-commands`** (stub) — slash command registry / parser later.

---

## Task 1: Workspace skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `spaze-proto/Cargo.toml`, `spaze-proto/src/lib.rs`
- Create: `spaze-crypto/Cargo.toml`, `spaze-crypto/src/lib.rs`
- Create: `spaze-storage/Cargo.toml`, `spaze-storage/src/lib.rs`
- Create: `spaze-commands/Cargo.toml`, `spaze-commands/src/lib.rs`
- Create: `spaze-server/Cargo.toml`, `spaze-server/src/main.rs`
- Create: `spaze-client/Cargo.toml`, `spaze-client/src/main.rs`

- [ ] **Step 1: Write workspace `Cargo.toml`**

```toml
[workspace]
resolver = "3"
members = [
    "spaze-proto",
    "spaze-crypto",
    "spaze-storage",
    "spaze-server",
    "spaze-client",
    "spaze-commands",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "Apache-2.0"
authors = ["Andreas <larsnilsandreas@pm.me>"]
repository = "https://github.com/Gaurgle/spaze"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
uuid = { version = "1.10", features = ["v7", "serde"] }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
```

- [ ] **Step 2: Write each member crate's `Cargo.toml`**

`spaze-proto/Cargo.toml`:
```toml
[package]
name = "spaze-proto"
description = "Wire protocol types for Spaze (IDs, messages, events, errors)"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true

[lints]
workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uuid.workspace = true
```

`spaze-crypto/Cargo.toml`, `spaze-storage/Cargo.toml`, `spaze-commands/Cargo.toml` — same shape, no dependencies, swap `name` and `description`:
- `spaze-crypto`: "Cryptographic primitives for Spaze (Ed25519 device keys for message signing)"
- `spaze-storage`: "SQLite-backed persistence layer for Spaze"
- `spaze-commands`: "Slash command registry and parser for Spaze"

`spaze-server/Cargo.toml`:
```toml
[package]
name = "spaze-server"
description = "Spaze server daemon"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true

[lints]
workspace = true

[[bin]]
name = "spaze-server"
path = "src/main.rs"

[dependencies]
spaze-proto = { path = "../spaze-proto" }
```

`spaze-client/Cargo.toml`:
```toml
[package]
name = "spaze-client"
description = "Spaze TUI client"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true
repository.workspace = true

[lints]
workspace = true

[[bin]]
name = "spaze"
path = "src/main.rs"

[dependencies]
spaze-proto = { path = "../spaze-proto" }
```

- [ ] **Step 3: Write empty source files for the stub crates**

`spaze-crypto/src/lib.rs`, `spaze-storage/src/lib.rs`, `spaze-commands/src/lib.rs` — identical placeholder content (substitute the crate name in the doc comment):

```rust
//! `spaze-crypto` — placeholder. Implementation lands in a later phase.
```

`spaze-server/src/main.rs`:
```rust
fn main() {
    println!("spaze-server: not yet implemented");
}
```

`spaze-client/src/main.rs`:
```rust
fn main() {
    println!("spaze: not yet implemented");
}
```

`spaze-proto/src/lib.rs` — minimal placeholder for now (real content in Task 4):
```rust
//! `spaze-proto` — wire protocol types. Filled in by Task 4.
```

- [ ] **Step 4: Verify the workspace builds**

Run: `cd /Users/at-a/Repos/Spaze && cargo check --workspace`
Expected: clean compile of all six crates, no errors. Warnings about empty crates are acceptable; warnings about missing `Cargo.lock` are not (it should be generated).

- [ ] **Step 5: Verify formatting and lints are clean**

Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: both pass with no output.

---

## Task 2: License, gitignore, supporting files

**Files:**
- Create: `LICENSE-APACHE`
- Create: `.gitignore`
- Create: `rust-toolchain.toml` (optional but useful — pins toolchain for contributors)

- [ ] **Step 1: Write `LICENSE-APACHE`**

Fetch the canonical Apache-2.0 text. From the project root:

```bash
curl -fsSL https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE-APACHE
```

Verify the file is ~11 KB and starts with `Apache License`. If `curl` is unavailable or the URL is unreachable, copy the text from any other Apache-2.0 project on the machine (e.g., a repo under `~/Repos/`).

- [ ] **Step 2: Write `.gitignore`**

```gitignore
# Rust
/target
**/*.rs.bk
Cargo.lock.bak

# Editor / OS
.DS_Store
*.swp
.idea/
.vscode/
.zed/

# Local environment
.env
.env.local

# Project-local data (e.g., dev SQLite DBs)
*.sqlite
*.sqlite-journal
*.sqlite-wal
*.sqlite-shm
/data/
```

Note: do NOT gitignore `Cargo.lock`. This is a workspace with binaries — the lockfile is committed.

- [ ] **Step 3: Write `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

This is intentionally loose (`stable`) rather than pinned to a specific version, so contributors stay current. The `rust-version` field in `Cargo.toml` enforces the minimum.

---

## Task 3: DESIGN.md and README.md

**Files:**
- Create: `DESIGN.md`
- Create: `README.md`

- [ ] **Step 1: Write `DESIGN.md`**

This is the canonical design doc — the locked decisions from the multi-day planning conversation. Structure it with these sections (full content below).

```markdown
# Spaze — Design Document

> **Status:** locked spec for the 8-week MVP. This document records the decisions made during planning. Update as the project evolves; treat it as the single source of truth for product scope and architecture.

## Vision

Spaze is a terminal-first, project-centric team collaboration tool written in Rust. Each Space represents a project/team/product. Rooms within a Space organize conversations and optionally bind to repos (supports monorepo and multi-repo team structures). Notes and todos are first-class Space-level artifacts captured inline through chat, filterable across rooms and repos, exportable to linked repos. Chat is the capture surface; the structured knowledge base is the value.

Keyboard-first but not keyboard-only — mouse, vim motions, arrow keys, and dedicated buttons are first-class peers. TUI is primary; a desktop GUI sharing the same protocol crates is planned as a secondary client post-MVP.

Spaze takes architectural cues from **WeeChat** (buffer abstraction, slash commands, lightweight extensible core), **gomuks** (split client/backend architecture, local cache patterns), and **senpai** (modern terminal UX — first-run assistant, mouse everywhere, hover-clickable elements, drag-drop file upload, mute/pin rooms, highlight scripts).

## Project Identity

- **Name:** Spaze (the brand "z" stays in the product name; code uses `Space`/`Room`/etc. without z — the z is brand voice, not codebase voice)
- **Repo:** github.com/Gaurgle/spaze
- **License:** Apache-2.0 (matches Rust ecosystem norms; includes patent grant)
- **Workspace crate naming:** `spaze-*` prefix style
- **Top-level binaries:** `spaze` (TUI client) and `spaze-server` (daemon)
- **Workspace members:** `spaze-proto`, `spaze-crypto`, `spaze-storage`, `spaze-server`, `spaze-client`, `spaze-commands`
- **Stance:** Fully open source, self-hostable. Hosted/managed offering deferred indefinitely.

## Three-level hierarchy

| Level | What it is | Identity | Wire-level type |
|-------|-----------|----------|-----------------|
| **Server** | A running `spaze-server` daemon (one machine, one URL) | hostname + port | not a wire ID — connect by URL |
| **Space** | A project workspace inside a server (multi-Space per server supported) | UUIDv7 | `SpaceId` |
| **Room** | A conversation channel inside a Space, optionally repo-bound | UUIDv7 | `RoomId` |

DMs are modeled as Rooms with `kind = direct` and exactly two members. Presented in a separate "Direct messages" UX category, not mixed with project rooms.

## Architecture Principles

1. **Single static binary** for both server and client. No runtime dependencies beyond compile-in.
2. **Sensible defaults, zero-config first run.** `spaze-server` with no flags works: self-signed cert, SQLite DB in `~/.local/share/spaze/`, default port, prints a one-time admin invite code.
3. **Schema migrations on startup** via sqlx — upgrades are safe and automatic.
4. **TLS via rustls only** — no OpenSSL.
5. **JSON over WebSocket** as wire format. `spaze-proto` types stay format-agnostic via serde so a binary format swap is a one-line change later.
6. **Universal buffer abstraction** as the foundational TUI primitive. Rooms, pins view, notez/todoz views, settings, help — all are buffers sharing one navigation/scroll/focus model.
7. **Composable bars** as the screen-layout mental model: top status, room list (left or top, configurable), main timeline, input, bottom status.
8. **Region-based mouse dispatch** as foundational TUI infrastructure.
9. **Slash commands and inline tags share one parser** — `/join` and `#note` route through the same dispatcher.
10. **Pure-library cores** (`spaze-proto`, `spaze-storage`, `spaze-commands`) are I/O-free and testable in isolation.
11. **Request/response correlation** at the protocol layer: every client command carries a `RequestId`, every server response echoes it back. Unsolicited pushes (typing, new message, user joined) arrive as separate event frames.

## Authentication & Identity

- Primary login: GitHub OAuth Device Flow (per-instance OAuth App on self-hosted deployments)
- Canonical user record = GitHub **numeric** user ID (usernames change; numeric IDs don't)
- Each device generates an Ed25519 keypair on first run, registered post-OAuth — used for **signing** messages (audit/abuse prevention), not encryption
- Keychain storage via `keyring` crate; `token-cmd` config directive for users who prefer pass/gopass/himitsu/1Password CLI
- Server-issued rotating refresh tokens (~90 days) + short-lived session tokens (~1 hour)
- Per-device revocation supported
- Initial OAuth scopes minimal (`read:user`, `public_repo`); incremental upgrade for private repo features

### Bootstrap admin

`spaze-server` prints a one-time admin invite code on first start. The first GitHub login redeeming the code becomes Owner. Token-based — an attacker hitting the server before the operator can't claim ownership.

### Invite flow for users 2…N

Invite codes — Owner/Admin generates a code, shares out-of-band, new user redeems on first GitHub OAuth.

### Roles (MVP)

Owner (Space creator), Admin (invite/kick/manage rooms), Member (participate). Per-room ACLs deferred post-MVP.

## Privacy & Security

**Spaze is not end-to-end encrypted in the MVP.** For self-hosted team chat the threat model that E2E protects against (untrusted server operator) is largely empty — the operator is the team. The cost of E2E (no server-side search, complex new-device sync, no file dedup, no thumbnails) outweighs the realistic benefit.

What IS in place:
- TLS in transit (rustls)
- Encryption at rest with a server master key (database file + blob directory)
- Per-device Ed25519 signing for message authenticity
- TLS-only certificate pinning option for paranoid deployments

E2E DMs are a post-1.0 option if demanded.

## Chat Core

- Multiple persistent rooms per Space, full history
- Cursor-based message catch-up on reconnect (protocol shape inspired by IRC's CHATHISTORY)
- Presence indicators
- Typed message bodies (text, code block, markdown, file, command) — not strings
- **Edits and deletes are MVP** — `Message` carries `edited_at` / `deleted_at`; `EditMessage` / `DeleteMessage` commands; `MessageEdited` / `MessageDeleted` events
- Reactions and replies (scope-permitting in MVP)
- `/reply` to reply to last DM sender (senpai-borrowed)
- **Server-side full-text search** via SQLite FTS5 — the killer feature for team chat

## Code & Content Rendering

- `syntect` for syntax-highlighted code blocks (auto-detect or explicit language)
- `pulldown-cmark` for markdown → ratatui widgets
- Inline `.md` file preview

## File Sharing

- Drag-and-drop upload (TUI captures terminal drag events where supported, paste-path fallback; GUI native drag-drop)
- Content-addressed blob storage on server (SHA-256, dedup, server-side thumbnails)
- Per-file and per-room size limits
- Click/keystroke to download

## Pinned Messages

- Pin/unpin any message in any room
- Per-room "Pins" buffer
- Pin events broadcast to all room members
- Distinct from room-level pin (which controls room ordering in the buffer list)

## Inline Command System (notez/todoz integration)

- Slash commands and inline tags share one parser/dispatcher (WeeChat pattern)
- Slash commands for application actions: `/join`, `/leave`, `/pin`, `/unpin`, `/mute`, `/unmute`, `/quit`, `/help`, `/set`, `/reply`, `/theme`
- Inline tags in chat for content capture: `#note "..."`, `#todo "..."`
- Default destination: Space-level notez/todoz store (not user's local repo)
- Tag-aware: `#todo @backend +urgent "fix auth bug"`
- Full tag taxonomy from notez-cli/todoz: `#prio`, `#important`, `#longterm`, `#idea`, `#blocked`
- Optional flag/syntax to write directly to a linked repo's notez/todoz
- Drag-and-drop into notez/todoz panes (drop file → attach)
- Other users can clone captured notes/todos into their personal notez/todoz with one keystroke

### Notez/Todoz views

- Dedicated Space-wide buffers for all notes and all todos
- Filter by: room, linked repo, tag, author, date, status
- Full-text search across content
- Export action: push selected notes/todos into a linked repo's notez/todoz files (manual or scheduled sync)
- Public-by-default within a Space (inverts notez-cli's private default)

## Universal Input Model

- Mouse, vim motions, arrow keys, dedicated buttons functional everywhere
- Modeless compose box, modal navigation elsewhere; `Esc` returns to normal mode
- **Hover cursor change on clickable elements** (where terminal supports it — senpai pattern)
- Clickable: tags, todos, pins, room names, file attachments, authors, member list entries, links
- Click tag → filter; right-click or `m` → context menu
- **Buffer-index quick jump** (`Alt-1` through `Alt-9` for first nine buffers)
- Discoverable action buttons for proprietary features with visible keybindings
- **Multi-server client:** the `spaze` TUI can be logged into N Spaze servers simultaneously (Slack workspace-switcher style)

## First-Run Experience

- Interactive configuration assistant on first launch (senpai-borrowed)
- No "edit this TOML file" required for basic setup
- Walks through: server URL, GitHub OAuth Device Flow, default room subscription
- Writes config with sensible defaults; advanced users can hand-edit

## Notifications

- **Highlight script pattern** (senpai-borrowed): on @-mention, run `~/.config/spaze/highlight` if present, with environment variables (sender, room, Space, message, server)
- Spaze ships sensible defaults: `notify-send` on Linux, `terminal-notifier` on macOS, native toast on Windows
- Users override by writing a script — full flexibility, no bundled notification framework

## Theming (foundational from day 1)

Components never reference literal colors; they reference **semantic slots** on a `Theme` struct. Themes are bindings of slots → colors. The buffer abstraction takes `&Theme` from Phase 1 — no hardcoded colors anywhere.

Theme struct slots (initial):
- Surfaces: `background`, `surface` (sidebar/status), `overlay` (selection/hover)
- Text: `foreground`, `muted`, `subtle`
- State: `primary`, `success`, `warning`, `error`, `info`
- Chat-specific: `mention`, `link`, `code_bg`, `border`, `border_focused`
- Syntect pairing: `syntect_theme_name` → matching syntax-highlight theme

### Built-in themes (8)

**Dark:** Catppuccin Mocha (default), Tokyo Night, Gruvbox Dark, Nord, Rose Pine
**Light:** Catppuccin Latte (default light), Solarized Light, Gruvbox Light

### Loading

- Built-ins compiled into the binary as `const Theme` values
- User-loadable themes via `~/.config/spaze/themes/<name>.toml`
- Config selects: `theme = "catppuccin-mocha"`
- Slash command: `/theme <name>` for live switching

Requires true-color (24-bit) terminal — modern terminals (Alacritty, Kitty, WezTerm, iTerm2, Ghostty, Windows Terminal) all support it. Graceful 256-color degradation deferred post-MVP.

## Persistence

- SQLite (WAL mode) on both server and client via sqlx
- **Server:** message store + room/Space metadata + pin sets + blob index + notez/todoz store + user pubkeys + OAuth tokens (encrypted at rest with server master key) + FTS5 virtual table for search
- **Client:** local cache (decrypted as fetched), draft state, UI preferences, room mute/pin state

## Clients

### Primary: TUI

- ratatui + crossterm
- Vim-spirited keybindings, modal navigation outside compose box
- Buffer abstraction unifies rooms, pins, notez, todoz, settings, help
- Composable bar layout

### Secondary (post-MVP): Desktop GUI

- Shares `spaze-proto`, `spaze-crypto`, `spaze-storage`, `spaze-commands` crates
- Tauri or egui (decide later based on packaging and feel)
- Same feature set, GUI-native interactions (real drag-drop, native file pickers, system notifications)

## Tech Stack

| Concern | Crate / Tool |
|---------|-------------|
| Language | Rust (edition 2024) |
| Async runtime | tokio |
| Transport | WebSocket over TLS (`tokio-tungstenite` + `rustls`) |
| Wire format | JSON via `serde_json` (format-agnostic via serde — swappable later) |
| Serialization | serde |
| Storage | `sqlx` + SQLite (WAL mode) + FTS5 |
| Crypto | `ed25519-dalek` (per-device signing only — no encryption) |
| Credential storage | `keyring` (default) + `token-cmd` directive |
| Hashing | `sha2` |
| OAuth | `oauth2` |
| GitHub API | `octocrab` |
| TUI | `ratatui` + `crossterm` |
| Syntax highlighting | `syntect` |
| Markdown | `pulldown-cmark` |
| CLI args | `clap` |
| Distribution | `cargo-dist` + Homebrew tap + Docker (GHCR + Docker Hub) |
| IDs | UUID v7 (time-ordered) |

## Self-Hosting

- Static Rust binaries via `cargo-dist` for Linux/macOS/Windows (server + client)
- Official Docker image (GHCR + Docker Hub)
- Reference `docker-compose.yml` with Caddy for automatic Let's Encrypt TLS
- `DEPLOYMENT.md` covering Docker, Compose+Caddy (recommended), bare-metal systemd
- Per-instance GitHub OAuth App (~5 min of operator work; `spaze-server init-oauth` helper prints prompts)
- Backups: `sqlite3 spaze.db ".backup ..."` + blob directory rsync
- Schema migrations run automatically on server startup
- Generic OIDC (Authelia, Keycloak, Auth0) deferred post-1.0

## Phased Scope

| Phase | Week | Deliverable |
|-------|------|-------------|
| 1 | 1 | Workspace, `spaze-proto` (IDs/errors/messages/events with correlation frames), server + client skeletons, **buffer abstraction** in TUI from day one, **slash command parser**, **theming module scaffold**, single hardcoded Space/Room, plaintext WebSocket — two terminals chat |
| 2 | 2 | SQLite persistence, multi-Space + multi-Room, room join/leave, reconnection catch-up via cursor protocol, TLS, **first-run configuration assistant** |
| 3 | 3 | GitHub OAuth Device Flow, per-device Ed25519 keypairs, keychain + `token-cmd`, refresh/session tokens, roles, **bootstrap admin invite-token flow** |
| 4 | 4 | Region-based mouse dispatch, full input model (vim + mouse + arrows + buttons + hover cursor), buffer-index quick jump, syntect, pulldown-cmark, **theming complete (8 built-ins + user-loadable)** |
| 5 | 5 | File upload/download, content-addressed storage, drag-and-drop, server-side thumbnails, file previews |
| 6 | 6 | Message pinning + pins buffer, room mute/pin, inline command parser (`#note`/`#todo`), Space-level notez/todoz store, notez/todoz buffers with filtering, clickable tags, highlight script notifications, **server-side FTS5 search** |
| 7 | 7 | Tier 1 repo linking (paste GitHub URL → bind), tag system integration with linked repos, export/sync server notez/todoz → repo notez/todoz files |
| 8 | 8 | Polish, `cargo-dist`, Docker image hardening, `DEPLOYMENT.md`, Homebrew tap |

## Roadmap (post-1.0)

- Tier 2 GitHub integration: repo metadata cards, auto-linked issue/PR/commit refs
- Tier 3 GitHub integration: bidirectional sync (push `#todo` to GitHub Issues, pull issues into todoz view), webhook subscriptions for repo events as system messages
- Generic OIDC auth alongside GitHub (Authelia, Keycloak, Auth0)
- **Optional E2E DMs** (DM-only first; channel-level E2E only if seriously demanded)
- Voice channels (Opus, cpal, webrtc-rs)
- Per-room ACLs and private rooms
- Hosted/managed Spaze instance (decision deferred)
- Mobile clients (only if demanded)
- Interactive settings buffer (WeeChat `iset`-style)
- Plugin system (design-with-seams from day one but build only when demanded)
- Desktop GUI (Tauri or egui)

## Deferred Design Questions

- Threading/replies depth — flat replies vs nested threads
- Sync conflict resolution for server↔repo notez/todoz export
- Notification system fine-tuning (mention parsing rules, do-not-disturb hours)
- Federation between Spaze servers (probably never — staying focused)

## Non-Goals (for now)

- Federation
- Native mobile apps
- Plugin system
- Hosted billing / SaaS tooling
- E2E (post-1.0 if demanded)
```

- [ ] **Step 2: Write `README.md`**

Keep it focused — short pitch, status, deeper doc links. No marketing fluff.

```markdown
# Spaze

A terminal-first, project-centric team collaboration tool. Self-hostable, open source, GitHub-authenticated.

> **Status:** pre-alpha. Bootstrapping the workspace. Not usable yet.

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

## Documentation

- [DESIGN.md](./DESIGN.md) — architecture, scope, and locked decisions.
- `LICENSE-APACHE` — Apache 2.0.

## Contributing

Pre-alpha; not accepting external contributions yet. Star and watch if you're curious — issues will open once the MVP is functional.

## License

Apache-2.0.
```

---

## Task 4: `spaze-proto` — IDs

**Files:**
- Create: `spaze-proto/src/ids.rs`
- Modify: `spaze-proto/src/lib.rs` (add `pub mod ids;` and re-exports)

**TDD note:** the test for ID uniqueness goes in `lib.rs` after the module is wired up (Step 4 of this task), so the cycle is: write the type with no real test → wire it into lib.rs → write the test that exercises both ID generation and serde roundtrip → confirm it passes. This is honest TDD even though the type itself is small.

- [ ] **Step 1: Write the failing test in `spaze-proto/src/lib.rs`**

Replace `spaze-proto/src/lib.rs` with:

```rust
//! `spaze-proto` — wire protocol types for Spaze.
//!
//! This crate intentionally contains no I/O and no business logic.
//! It defines IDs, messages, events, and protocol errors that all other
//! crates serialize over the wire.

pub mod ids;

pub use ids::{DeviceId, MessageId, RoomId, SpaceId, UserId};

/// Wire protocol version. Bumped when message/event shapes change incompatibly.
pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_per_call() {
        let a = UserId::new();
        let b = UserId::new();
        assert_ne!(a, b, "two fresh UserIds must not collide");
    }

    #[test]
    fn user_id_roundtrips_json() {
        let id = UserId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p spaze-proto`
Expected: compile failure — `ids` module does not exist.

- [ ] **Step 3: Implement `spaze-proto/src/ids.rs`**

```rust
//! Newtype-wrapped UUIDv7 IDs for the Spaze wire protocol.
//!
//! UUIDv7 is time-ordered, so values sort lexically by creation time.
//! That property matters for chat (message ordering) and for SQLite
//! primary-key locality.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a fresh UUIDv7-backed identifier.
            #[must_use]
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wrap an existing UUID.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Unwrap to the underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(
    /// Identifies a Spaze user (one human, possibly many devices).
    UserId
);
id_type!(
    /// Identifies a single registered client device for a user.
    DeviceId
);
id_type!(
    /// Identifies a Space — a project workspace inside a server.
    /// One `spaze-server` daemon may host multiple Spaces.
    SpaceId
);
id_type!(
    /// Identifies a Room — a conversation channel inside a Space.
    /// DMs are Rooms with `kind = direct` and exactly two members.
    RoomId
);
id_type!(
    /// Identifies a single message.
    MessageId
);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-proto`
Expected: 2 tests pass (`ids_are_unique_per_call`, `user_id_roundtrips_json`). No clippy warnings — also run `cargo clippy -p spaze-proto --all-targets -- -D warnings`.

---

## Task 5: `spaze-proto` — ProtocolError

**Files:**
- Create: `spaze-proto/src/error.rs`
- Modify: `spaze-proto/src/lib.rs` (add module + re-export)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `spaze-proto/src/lib.rs`:

```rust
#[test]
fn protocol_error_roundtrips_json() {
    let err = ProtocolError::NotFound { what: "channel".into() };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("\"not_found\""), "tag missing in {json}");
    let parsed: ProtocolError = serde_json::from_str(&json).unwrap();
    match parsed {
        ProtocolError::NotFound { what } => assert_eq!(what, "channel"),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn protocol_error_displays_human_readable() {
    let err = ProtocolError::RateLimited { retry_after_secs: 30 };
    assert_eq!(err.to_string(), "rate limited; retry after 30s");
}
```

Also add to `lib.rs` (top of the file, alongside `pub mod ids;`):
```rust
pub mod error;

pub use error::ProtocolError;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-proto`
Expected: compile failure — `error` module does not exist.

- [ ] **Step 3: Implement `spaze-proto/src/error.rs`**

```rust
//! Protocol-level error type sent over the wire.
//!
//! This is intentionally small and stable. Internal server errors
//! (database failures, panics, etc.) are surfaced to clients as
//! [`ProtocolError::InternalError`] without leaking detail.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ProtocolError {
    #[error("not authenticated")]
    NotAuthenticated,

    #[error("permission denied")]
    PermissionDenied,

    #[error("not found: {what}")]
    NotFound { what: String },

    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("rate limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u32 },

    #[error("internal server error")]
    InternalError,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-proto && cargo clippy -p spaze-proto --all-targets -- -D warnings`
Expected: 4 tests pass, clippy clean.

---

## Task 6: `spaze-proto` — Messages

**Files:**
- Create: `spaze-proto/src/messages.rs`
- Modify: `spaze-proto/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `spaze-proto/src/lib.rs`:

```rust
#[test]
fn message_roundtrips_json() {
    let msg = Message {
        id: MessageId::new(),
        room_id: RoomId::new(),
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        created_at_ms: 1_700_000_000_000,
        edited_at_ms: None,
        deleted_at_ms: None,
        body: MessageBody::Text { content: "hello world".into() },
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, msg.id);
    assert_eq!(parsed.edited_at_ms, None);
    assert_eq!(parsed.deleted_at_ms, None);
    match parsed.body {
        MessageBody::Text { content } => assert_eq!(content, "hello world"),
        other => panic!("wrong body variant: {other:?}"),
    }
}

#[test]
fn edited_message_carries_edit_timestamp() {
    let msg = Message {
        id: MessageId::new(),
        room_id: RoomId::new(),
        author_id: UserId::new(),
        author_device_id: DeviceId::new(),
        created_at_ms: 1_700_000_000_000,
        edited_at_ms: Some(1_700_000_500_000),
        deleted_at_ms: None,
        body: MessageBody::Text { content: "edited content".into() },
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.edited_at_ms, Some(1_700_000_500_000));
}

#[test]
fn system_message_serializes_with_tag() {
    let body = MessageBody::System { content: "user joined".into() };
    let json = serde_json::to_string(&body).unwrap();
    assert!(json.contains("\"kind\":\"system\""), "tag missing in {json}");
}
```

Add to `lib.rs`:
```rust
pub mod messages;

pub use messages::{Message, MessageBody};
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-proto`
Expected: compile failure — `messages` module does not exist.

- [ ] **Step 3: Implement `spaze-proto/src/messages.rs`**

```rust
//! Chat message types.
//!
//! [`Message`] is the durable, server-authoritative shape stored in SQLite
//! and broadcast to room subscribers. [`MessageBody`] is intentionally
//! a small enum: text and system today; attachments, code blocks, and
//! reactions are added in later phases.
//!
//! `edited_at_ms` and `deleted_at_ms` are `Option<i64>` rather than baking
//! state into `MessageBody` — server-assigned, monotonic, and easy to query.

use serde::{Deserialize, Serialize};

use crate::ids::{DeviceId, MessageId, RoomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub room_id: RoomId,
    pub author_id: UserId,
    pub author_device_id: DeviceId,
    /// Unix timestamp in milliseconds (server-assigned at post time).
    pub created_at_ms: i64,
    /// Server-assigned timestamp of the most recent edit, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at_ms: Option<i64>,
    /// Server-assigned timestamp of soft-delete, if any. Body is replaced
    /// with a tombstone variant on delete; this field marks the moment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at_ms: Option<i64>,
    pub body: MessageBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageBody {
    /// A plain user-authored text message.
    Text { content: String },
    /// A server-generated system event rendered inline (joins, renames, etc.).
    System { content: String },
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-proto && cargo clippy -p spaze-proto --all-targets -- -D warnings`
Expected: 7 tests pass (`ids_are_unique_per_call`, `user_id_roundtrips_json`, `protocol_error_roundtrips_json`, `protocol_error_displays_human_readable`, `message_roundtrips_json`, `edited_message_carries_edit_timestamp`, `system_message_serializes_with_tag`), clippy clean.

---

## Task 7: `spaze-proto` — Frames, Events and Commands

**Files:**
- Create: `spaze-proto/src/events.rs`
- Modify: `spaze-proto/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `tests` module in `spaze-proto/src/lib.rs`:

```rust
#[test]
fn server_event_roundtrips_json() {
    let event = ServerEvent::UserJoined {
        user_id: UserId::new(),
        room_id: RoomId::new(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"type\":\"user_joined\""), "tag missing in {json}");
    let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, ServerEvent::UserJoined { .. }));
}

#[test]
fn client_frame_carries_request_id_and_command() {
    let frame = ClientFrame {
        request_id: RequestId(42),
        command: ClientCommand::PostMessage {
            room_id: RoomId::new(),
            body: MessageBody::Text { content: "hi".into() },
        },
    };
    let json = serde_json::to_string(&frame).unwrap();
    assert!(json.contains("\"request_id\":42"), "request_id missing in {json}");
    assert!(json.contains("\"type\":\"post_message\""), "command tag missing in {json}");
    let parsed: ClientFrame = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.request_id, RequestId(42));
    assert!(matches!(parsed.command, ClientCommand::PostMessage { .. }));
}

#[test]
fn server_response_frame_roundtrips_ok_outcome() {
    let frame = ServerFrame::Response {
        request_id: RequestId(7),
        result: CommandOutcome::Ok { payload: ResponsePayload::Empty },
    };
    let json = serde_json::to_string(&frame).unwrap();
    assert!(json.contains("\"frame\":\"response\""), "outer tag missing in {json}");
    assert!(json.contains("\"outcome\":\"ok\""), "outcome tag missing in {json}");
    let parsed: ServerFrame = serde_json::from_str(&json).unwrap();
    match parsed {
        ServerFrame::Response { request_id, result } => {
            assert_eq!(request_id, RequestId(7));
            assert!(matches!(result, CommandOutcome::Ok { .. }));
        }
        other => panic!("wrong frame variant: {other:?}"),
    }
}

#[test]
fn server_response_frame_roundtrips_err_outcome() {
    let frame = ServerFrame::Response {
        request_id: RequestId(9),
        result: CommandOutcome::Err {
            error: ProtocolError::PermissionDenied,
        },
    };
    let json = serde_json::to_string(&frame).unwrap();
    let parsed: ServerFrame = serde_json::from_str(&json).unwrap();
    match parsed {
        ServerFrame::Response { result: CommandOutcome::Err { error }, .. } => {
            assert_eq!(error, ProtocolError::PermissionDenied);
        }
        other => panic!("expected Err outcome, got {other:?}"),
    }
}

#[test]
fn server_event_frame_roundtrips() {
    let frame = ServerFrame::Event(ServerEvent::Typing {
        user_id: UserId::new(),
        room_id: RoomId::new(),
    });
    let json = serde_json::to_string(&frame).unwrap();
    assert!(json.contains("\"frame\":\"event\""), "outer tag missing in {json}");
    assert!(json.contains("\"type\":\"typing\""), "event tag missing in {json}");
    let parsed: ServerFrame = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, ServerFrame::Event(ServerEvent::Typing { .. })));
}
```

Add to `lib.rs`:
```rust
pub mod events;

pub use events::{
    ClientCommand, ClientFrame, CommandOutcome, RequestId, ResponsePayload, ServerEvent,
    ServerFrame,
};
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-proto`
Expected: compile failure — `events` module does not exist.

- [ ] **Step 3: Implement `spaze-proto/src/events.rs`**

```rust
//! Wire frames exchanged between client and server.
//!
//! Top-level frame types are [`ClientFrame`] (client → server) and
//! [`ServerFrame`] (server → client). Client frames always carry a
//! [`RequestId`] so the matching [`ServerFrame::Response`] can be
//! correlated back. Unsolicited pushes (typing, new message, user joined)
//! arrive as [`ServerFrame::Event`].
//!
//! All variants are serde-tagged unions; the wire format is JSON. Keep
//! enums under careful protocol versioning — see [`crate::PROTOCOL_VERSION`].

use serde::{Deserialize, Serialize};

use crate::error::ProtocolError;
use crate::ids::{MessageId, RoomId, UserId};
use crate::messages::{Message, MessageBody};

/// Per-connection correlation token for client commands and their responses.
///
/// The client increments a counter for each command sent on a given
/// WebSocket session. RequestIds are unique within a single connection,
/// not globally — they are not persisted server-side beyond the response.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct RequestId(pub u64);

/// Top-level frame sent client → server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientFrame {
    pub request_id: RequestId,
    pub command: ClientCommand,
}

/// Top-level frame sent server → client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ServerFrame {
    /// Reply to a specific [`ClientFrame`]. `request_id` matches the original.
    Response {
        request_id: RequestId,
        result: CommandOutcome,
    },
    /// Unsolicited server-pushed event (no matching request).
    Event(ServerEvent),
}

/// Outcome of a client command. Explicit enum (rather than `Result`) so the
/// JSON shape is `{"outcome": "ok", "payload": ...}` / `{"outcome": "err", "error": ...}`
/// instead of serde's default `{"Ok": ...}` / `{"Err": ...}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CommandOutcome {
    Ok { payload: ResponsePayload },
    Err { error: ProtocolError },
}

/// Successful response payloads. Most commands respond with [`ResponsePayload::Empty`];
/// commands that return data attach it via a dedicated variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    /// The command succeeded with no return data.
    Empty,
    /// `PostMessage` succeeded — server returns the canonical [`Message`]
    /// (with server-assigned id and timestamp) for the client to swap in.
    MessagePosted(Message),
}

/// Server-pushed events (not direct responses to client commands).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// A new message was posted in a room the client is subscribed to.
    MessagePosted(Message),
    /// A previously-posted message was edited.
    MessageEdited {
        message_id: MessageId,
        body: MessageBody,
        edited_at_ms: i64,
    },
    /// A previously-posted message was deleted.
    MessageDeleted {
        message_id: MessageId,
        deleted_at_ms: i64,
    },
    /// A user joined a room.
    UserJoined { user_id: UserId, room_id: RoomId },
    /// A user left a room.
    UserLeft { user_id: UserId, room_id: RoomId },
    /// Typing indicator (best-effort fan-out).
    Typing { user_id: UserId, room_id: RoomId },
}

/// Client commands carried inside a [`ClientFrame`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    PostMessage {
        room_id: RoomId,
        body: MessageBody,
    },
    EditMessage {
        message_id: MessageId,
        body: MessageBody,
    },
    DeleteMessage {
        message_id: MessageId,
    },
    JoinRoom {
        room_id: RoomId,
    },
    LeaveRoom {
        room_id: RoomId,
    },
    StartTyping {
        room_id: RoomId,
    },
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-proto && cargo clippy -p spaze-proto --all-targets -- -D warnings`
Expected: 12 tests pass total (2 from ids, 2 from error, 3 from messages, 5 from events), clippy clean.

- [ ] **Step 5: Final whole-workspace check**

Run: `cargo check --workspace --all-targets && cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: all green.

---

## Task 8: GitHub Actions CI skeleton

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the workflow**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: cargo check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets

  test:
    name: cargo test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --all-targets

  clippy:
    name: cargo clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  fmt:
    name: cargo fmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check
```

Note on action pinning: actions are tagged at the major version (`@v4`, `@v2`) for now — common practice and easy to read. If we later want SHA-pinning for supply-chain hardening, dependabot can rewrite these.

---

## Task 9: Initialize git and prepare initial commit (AUTHORIZATION GATE)

**Reminder per user CLAUDE.md:** "Ask before creating git commits or pushing." This task surfaces the exact commands; the user reviews and runs them. The commit is **not** auto-executed by the agent.

- [ ] **Step 1: Initialize the repo**

Run from `/Users/at-a/Repos/Spaze`:

```bash
git init -b main
```

Expected: `Initialized empty Git repository in /Users/at-a/Repos/Spaze/.git/`. The default branch is `main`.

- [ ] **Step 2: Stage everything**

```bash
git add Cargo.toml Cargo.lock LICENSE-APACHE README.md DESIGN.md \
        .gitignore rust-toolchain.toml \
        .github/ docs/ \
        spaze-proto/ spaze-crypto/ spaze-storage/ \
        spaze-server/ spaze-client/ spaze-commands/
```

Then `git status` and verify nothing unexpected is staged. No `.env`, no `target/`, no editor cruft.

- [ ] **Step 3: Present the commit command for the user**

**Do not run.** Show the user this exact command and pause:

```bash
git commit -m "chore: bootstrap workspace, design doc, proto crate, CI"
```

Optional richer body if the user prefers:

```bash
git commit -m "chore: bootstrap workspace, design doc, proto crate, CI" -m "
- Cargo workspace with six prefix-style member crates (proto, crypto, storage, server, client, commands)
- Apache-2.0 license, Rust 2024 edition, MSRV 1.85
- DESIGN.md captures locked product/architecture decisions: 3-level Server/Space/Room hierarchy, GitHub OAuth + Ed25519 device signing, no E2E in MVP, server-side FTS5 search, buffer abstraction + theming foundational in Phase 1, prior-art borrowings from WeeChat/gomuks/senpai
- spaze-proto: UUIDv7 IDs (UserId/DeviceId/SpaceId/RoomId/MessageId), ProtocolError, Message with edited_at/deleted_at, ClientFrame/ServerFrame with RequestId correlation, ClientCommand (Post/Edit/Delete/Join/Leave/Typing), ServerEvent (Message Posted/Edited/Deleted, User Joined/Left, Typing) — all serde-roundtrip tested
- GitHub Actions CI: check, test, clippy, fmt
"
```

The user runs whichever they prefer. Per their CLAUDE.md: no `Co-Authored-By` lines.

---

## Task 10: Create the GitHub repo and push (AUTHORIZATION GATE)

**Reminder per user CLAUDE.md:** "I commit and push myself — present the message and let me run it." This task gathers the exact commands; the user runs them.

- [ ] **Step 1: Create the GitHub repo**

```bash
gh repo create Gaurgle/spaze \
  --public \
  --description "Terminal-first, project-centric team collaboration tool. Self-hostable, open source, GitHub-authenticated. With inline #note/#todo capture and server-side search." \
  --source . \
  --remote origin
```

This creates the repo and adds an `origin` remote in one step. If the user prefers private-while-developing, swap `--public` for `--private`.

- [ ] **Step 2: Push**

```bash
git push -u origin main
```

- [ ] **Step 3: Verify CI runs**

Open: `https://github.com/Gaurgle/spaze/actions`
Expected: the four CI jobs (`check`, `test`, `clippy`, `fmt`) start and pass within a few minutes.

If a job fails, capture the error and treat it as a Task 11 follow-up rather than rolling everything back. CI failures on a brand-new repo are usually trivial (formatting, missing component) and worth fixing in a follow-up commit.

---

## Done

After Task 10, the repo is bootstrapped: workspace skeleton in place, proto crate has real (tested) types with request/response correlation, design doc is the canonical reference, CI is green. Next session focus: **Phase 1 of the MVP** — server + client skeletons that talk over plaintext WebSocket, the buffer abstraction in the TUI from day one, the slash-command parser, and the theming module scaffold (so no hardcoded colors land anywhere). After that: Phase 2 brings in `spaze-storage` (SQLite schema + sqlx migrations + FTS5).
