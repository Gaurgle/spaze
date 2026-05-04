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
| 1 | 1 | Workspace, `spaze-proto`, server + client skeletons, buffer abstraction, slash command parser, theming scaffold, single hardcoded Space/Room, plaintext WebSocket — two terminals chat. **Decomposed into 1.A (bare WS chat ✅), 1.B (TUI + buffer + theming), 1.C (slash commands).** |
| 2 | 2 | SQLite persistence, multi-Space + multi-Room, room join/leave, reconnection catch-up via cursor protocol, TLS, **first-run configuration assistant** |
| 3 | 3 | GitHub OAuth Device Flow, per-device Ed25519 keypairs, keychain + `token-cmd`, refresh/session tokens, roles, **bootstrap admin invite-token flow** |
| 4 | 4 | Region-based mouse dispatch, full input model (vim + mouse + arrows + buttons + hover cursor), buffer-index quick jump, syntect, pulldown-cmark, **theming complete (8 built-ins + user-loadable)** |
| 5 | 5 | File upload/download, content-addressed storage, drag-and-drop, server-side thumbnails, file previews |
| 6 | 6 | Message pinning + pins buffer, room mute/pin, inline command parser (`#note`/`#todo`), Space-level notez/todoz store, notez/todoz buffers with filtering, clickable tags, highlight script notifications, **server-side FTS5 search** |
| 7 | 7 | Tier 1 repo linking (paste GitHub URL → bind), tag system integration with linked repos, export/sync server notez/todoz → repo notez/todoz files |
| 8 | 8 | Polish, `cargo-dist`, Docker image hardening, `DEPLOYMENT.md`, Homebrew tap |

## Sub-project tracking

Phase 1 was decomposed during brainstorming into three sub-projects, each with its own spec under `docs/superpowers/specs/` and plan under `docs/superpowers/plans/`:

| Sub-project | Status | Spec | Plan |
|---|---|---|---|
| 1.A — WS round-trip MVP | ✅ shipped | `2026-05-04-phase-1a-ws-mvp-design.md` | `2026-05-04-phase-1a-ws-mvp.md` |
| 1.B — TUI shell + buffer + theming scaffold | not yet planned | — | — |
| 1.C — Slash command parser | not yet planned | — | — |

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
