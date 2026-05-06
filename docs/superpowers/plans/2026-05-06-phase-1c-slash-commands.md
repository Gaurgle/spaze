# Phase 1.C — Slash Command Parser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up a self-contained `spaze-commands` crate (parser + registry + four handlers + `Effect` enum), wire it into the TUI, add live input coloring, and add a `MessageBody::Action` proto variant for `/me`.

**Architecture:** `spaze-commands` is zero-dep std-only. Handlers are pure `(args, ctx) → Vec<Effect>`. The client's `App::apply_effect` is the only place effects become side effects (mutating buffers, sending WS frames, setting `should_quit`). Live coloring uses `classify_input` per-keystroke; submission uses `parse_input` per-Enter.

**Tech Stack:** Rust 2024 (MSRV 1.85), workspace at v0.4.0 after Task 1. New direct deps: `spaze-commands` (path), `unicode-width` (`0.2`). No new transitive deps. CI runs latest stable rustc on Ubuntu (1.95+ as of Phase 1.B).

**Spec:** `docs/superpowers/specs/2026-05-06-phase-1c-slash-commands-design.md`. All tasks reference its locked decisions.

---

### Task 1: Branch + workspace version 0.3.0 → 0.4.0

**Files:**
- Modify: `Cargo.toml` (workspace package version)
- Run: `cargo build` to refresh `Cargo.lock`

- [ ] **Step 1: Cut `phase-1c` from `main`**

```bash
git checkout main
git pull origin main
git checkout -b phase-1c
git status
```

Expected: clean tree, on branch `phase-1c`, up to date with `main`.

- [ ] **Step 2: Bump workspace version**

In `Cargo.toml`, change the `[workspace.package]` block:

```toml
[workspace.package]
version = "0.4.0"   # was "0.3.0"
edition = "2024"
rust-version = "1.85"
license = "Apache-2.0"
authors = ["Andreas <larsnilsandreas@pm.me>"]
repository = "https://github.com/Gaurgle/spaze"
```

- [ ] **Step 3: Refresh lockfile + sanity-check existing tests still pass**

Run: `cargo build && cargo test --workspace`
Expected: builds clean, all 32 existing tests pass.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump workspace to 0.4.0 for phase 1.C"
```

---

### Task 2: Proto — `MessageBody::Action` variant + serde round-trip tests

**Files:**
- Modify: `spaze-proto/src/messages.rs:34-42` (add `Action` variant)
- Modify: `spaze-proto/src/lib.rs` (add new tests at end of `#[cfg(test)] mod tests` block)

- [ ] **Step 1: Write the failing tests for Action serde**

Append to the existing `#[cfg(test)] mod tests { ... }` block in `spaze-proto/src/lib.rs` (find the closing `}` of the tests module — the new tests go just before it):

```rust
    #[test]
    fn action_body_serde_roundtrip() {
        let body = MessageBody::Action {
            content: "kicks the build".into(),
        };
        let json = serde_json::to_string(&body).expect("serialize Action");
        let parsed: MessageBody = serde_json::from_str(&json).expect("parse Action");
        match parsed {
            MessageBody::Action { content } => assert_eq!(content, "kicks the build"),
            other => panic!("wrong body variant: {other:?}"),
        }
    }

    #[test]
    fn action_body_utf8_roundtrip() {
        let body = MessageBody::Action {
            content: "waves at åse 🐧".into(),
        };
        let json = serde_json::to_string(&body).expect("serialize Action UTF-8");
        let parsed: MessageBody = serde_json::from_str(&json).expect("parse Action UTF-8");
        match parsed {
            MessageBody::Action { content } => assert_eq!(content, "waves at åse 🐧"),
            other => panic!("wrong body variant: {other:?}"),
        }
    }

    #[test]
    fn action_body_uses_action_kind_tag() {
        // The serde tag is "kind" with snake_case rename, so the JSON shape
        // must literally contain `"kind":"action"` to keep the wire format stable.
        let body = MessageBody::Action {
            content: "pings".into(),
        };
        let json = serde_json::to_string(&body).expect("serialize");
        assert!(json.contains(r#""kind":"action""#), "got: {json}");
        assert!(json.contains(r#""content":"pings""#), "got: {json}");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-proto action_body`
Expected: 3 FAIL with "no variant Action found" (or similar — variant doesn't exist yet).

- [ ] **Step 3: Add the `Action` variant**

In `spaze-proto/src/messages.rs:37-42`, change the `MessageBody` enum to:

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessageBody {
    /// A plain user-authored text message.
    Text { content: String },
    /// A server-generated system event rendered inline (joins, renames, etc.).
    System { content: String },
    /// An action message (the `/me` command). Renders as `* {author} {content}`.
    Action { content: String },
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-proto`
Expected: all proto tests pass (existing + 3 new). 16 + 3 = 19 proto tests.

- [ ] **Step 5: Verify the rest of the workspace still builds**

Run: `cargo build --workspace`
Expected: clean build. The wildcard `_ =>` arm in `spaze-client/src/buffers/room_timeline.rs:188` already absorbs the new variant via the `_` match — Action will render as debug repr until Task 13.

- [ ] **Step 6: Commit**

```bash
git add spaze-proto/src/messages.rs spaze-proto/src/lib.rs
git commit -m "feat(proto): add MessageBody::Action variant for /me"
```

---

### Task 3: `spaze-commands` crate — module skeleton

**Files:**
- Modify: `spaze-commands/src/lib.rs` (replace placeholder with module declarations + re-exports)
- Create: `spaze-commands/src/effect.rs` (empty stub for next task)
- Create: `spaze-commands/src/parser.rs` (empty stub)
- Create: `spaze-commands/src/registry.rs` (empty stub)
- Create: `spaze-commands/src/commands.rs` (empty stub)

- [ ] **Step 1: Replace `spaze-commands/src/lib.rs` with the module structure**

Overwrite `spaze-commands/src/lib.rs` with:

```rust
//! `spaze-commands` — slash command registry, parser, and effect model.
//!
//! This crate is intentionally zero-dependency (std-only). Commands are pure
//! functions of `(args, ctx) -> Vec<Effect>`. The client interprets effects.
//!
//! ## Public surface
//!
//! - [`Effect`] — what handlers return (semantic intent, not wire-level).
//! - [`parse_input`] / [`InputKind`] — Enter-time parser (allocates).
//! - [`classify_input`] / [`InputClass`] — keystroke-time classifier (zero alloc).
//! - [`Command`] / [`HandlerContext`] / [`Handler`] — registry types.
//! - [`lookup`] / [`REGISTRY`] — registry access.

pub mod commands;
pub mod effect;
pub mod parser;
pub mod registry;

pub use effect::Effect;
pub use parser::{InputClass, InputKind, classify_input, parse_input};
pub use registry::{Command, Handler, HandlerContext, REGISTRY, lookup};
```

- [ ] **Step 2: Create the four module stubs**

Create `spaze-commands/src/effect.rs` with:

```rust
//! `Effect` — what command handlers return. The client interprets each
//! variant in `App::apply_effect`. Effects represent semantic intent, not
//! wire-level commands; the client constructs `ClientCommand`s from app state.
```

Create `spaze-commands/src/parser.rs` with:

```rust
//! Input parsing: `classify_input` (per-keystroke, zero-alloc) and
//! `parse_input` (per-Enter, allocates owned strings).
```

Create `spaze-commands/src/registry.rs` with:

```rust
//! Command registry: `Command` struct, `HandlerContext`, `Handler` typedef,
//! the static `REGISTRY` slice, and `lookup` for name/alias resolution.
```

Create `spaze-commands/src/commands.rs` with:

```rust
//! The four 1.C command handlers: `/quit`, `/help`, `/me`, `/clear`.
//! Each is a pure function of `(args, ctx) -> Vec<Effect>`.
```

- [ ] **Step 3: Verify the crate still compiles (with the empty modules)**

Run: `cargo build -p spaze-commands`
Expected: warning about unused module imports in `lib.rs`, but compiles. Don't try to test yet — there's nothing to test.

- [ ] **Step 4: Commit**

```bash
git add spaze-commands/src/lib.rs spaze-commands/src/effect.rs spaze-commands/src/parser.rs spaze-commands/src/registry.rs spaze-commands/src/commands.rs
git commit -m "feat(commands): module skeleton for slash command system"
```

---

### Task 4: `Effect` enum (TDD)

**Files:**
- Modify: `spaze-commands/src/effect.rs`

- [ ] **Step 1: Write the failing test**

Append to `spaze-commands/src/effect.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_variants_have_expected_shape() {
        // Verify the four 1.C variants exist and have the correct payloads.
        let _ = Effect::Quit;
        let _ = Effect::ClearActiveBuffer;
        let _ = Effect::SystemLine("usage: ...".into());
        let _ = Effect::SendActionMessage("waves".into());
    }

    #[test]
    fn effect_equality_works() {
        assert_eq!(Effect::Quit, Effect::Quit);
        assert_eq!(
            Effect::SystemLine("a".into()),
            Effect::SystemLine("a".into())
        );
        assert_ne!(
            Effect::SystemLine("a".into()),
            Effect::SystemLine("b".into())
        );
        assert_ne!(Effect::Quit, Effect::ClearActiveBuffer);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p spaze-commands effect_`
Expected: FAIL with "cannot find type Effect" / "cannot find variant Quit".

- [ ] **Step 3: Implement the `Effect` enum**

Add to `spaze-commands/src/effect.rs` (above the `#[cfg(test)]` block):

```rust
/// What command handlers return. Each variant is a piece of *semantic intent*
/// — translation to wire-level commands or App-state mutations happens in the
/// client's `apply_effect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Quit the client cleanly.
    Quit,

    /// Empty the active buffer's local timeline. Session-only — no persistence
    /// concept yet (Phase 2 may revisit when scrollback exists).
    ClearActiveBuffer,

    /// Render a synthetic system message in the active buffer. Locally
    /// generated, never sent over the wire, never persisted. Visually
    /// identical to a server-issued `MessageBody::System`.
    SystemLine(String),

    /// Send an action message (`/me <content>`) to the server. The client
    /// constructs the full `PostMessage` from app state (room_id, author, etc.).
    SendActionMessage(String),
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p spaze-commands effect_`
Expected: PASS, 2/2 tests.

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/effect.rs
git commit -m "feat(commands): Effect enum with four 1.C variants"
```

---

### Task 5: `Command` struct, `HandlerContext`, `Handler`, `lookup` (TDD)

**Files:**
- Modify: `spaze-commands/src/registry.rs`

- [ ] **Step 1: Write the failing tests**

Append to `spaze-commands/src/registry.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;

    fn dummy_handler(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
        vec![Effect::Quit]
    }

    fn fixture() -> [Command; 2] {
        [
            Command {
                name: "quit",
                aliases: &["q"],
                short_help: "exit",
                long_help: "/quit — exit the client",
                handler: dummy_handler,
            },
            Command {
                name: "help",
                aliases: &["h", "?cmd"],
                short_help: "help",
                long_help: "/help — show help",
                handler: dummy_handler,
            },
        ]
    }

    #[test]
    fn lookup_finds_by_canonical_name() {
        let reg = fixture();
        let cmd = lookup("quit", &reg).expect("quit should be found");
        assert_eq!(cmd.name, "quit");
    }

    #[test]
    fn lookup_finds_by_alias() {
        let reg = fixture();
        let cmd = lookup("q", &reg).expect("q alias should resolve");
        assert_eq!(cmd.name, "quit");
        let cmd = lookup("?cmd", &reg).expect("?cmd alias should resolve");
        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn lookup_returns_none_on_miss() {
        let reg = fixture();
        assert!(lookup("nope", &reg).is_none());
    }

    #[test]
    fn lookup_is_case_sensitive() {
        // Slash commands are ASCII canonical-case. /Quit doesn't match /quit.
        let reg = fixture();
        assert!(lookup("Quit", &reg).is_none());
    }

    #[test]
    fn handler_context_holds_registry_reference() {
        let reg = fixture();
        let ctx = HandlerContext { registry: &reg };
        assert_eq!(ctx.registry.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-commands -- --include-ignored registry::tests`
Expected: FAIL — types `Command`, `HandlerContext`, `Handler`, `lookup` don't exist.

- [ ] **Step 3: Implement registry types and `lookup`**

Add to `spaze-commands/src/registry.rs` (above `#[cfg(test)]`):

```rust
use crate::effect::Effect;

/// Function signature for command handlers. Pure: no I/O, no mutation,
/// no async. Returns a list of effects the client will apply in order.
pub type Handler = fn(args: &[&str], ctx: &HandlerContext) -> Vec<Effect>;

/// Context passed to every handler. Carries the registry so `/help` can
/// introspect available commands. Future fields (current room name,
/// connection state, etc.) land here additively.
#[derive(Debug, Clone, Copy)]
pub struct HandlerContext<'a> {
    pub registry: &'a [Command],
}

/// One command in the registry. Lives in `&'static` data — entries are
/// const-constructible and shared across the process.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub short_help: &'static str,
    pub long_help: &'static str,
    pub handler: Handler,
}

/// Look up a command by canonical name OR alias. Case-sensitive.
#[must_use]
pub fn lookup<'a>(name: &str, registry: &'a [Command]) -> Option<&'a Command> {
    registry
        .iter()
        .find(|c| c.name == name || c.aliases.contains(&name))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-commands registry::tests`
Expected: PASS, 5/5 tests.

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/registry.rs
git commit -m "feat(commands): Command/HandlerContext/Handler types + lookup"
```

---

### Task 6: `parse_input` + `InputKind` (TDD)

**Files:**
- Modify: `spaze-commands/src/parser.rs`

- [ ] **Step 1: Write the failing tests**

Append to `spaze-commands/src/parser.rs`:

```rust
#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn empty_input_is_text() {
        assert_eq!(parse_input(""), InputKind::Text(String::new()));
    }

    #[test]
    fn plain_text_is_text() {
        assert_eq!(parse_input("hello world"), InputKind::Text("hello world".into()));
    }

    #[test]
    fn slash_mid_line_is_text() {
        assert_eq!(
            parse_input("hello /world"),
            InputKind::Text("hello /world".into())
        );
    }

    #[test]
    fn double_slash_is_escaped_text_with_one_slash_stripped() {
        assert_eq!(
            parse_input("//me kicks"),
            InputKind::EscapedText("/me kicks".into())
        );
    }

    #[test]
    fn triple_slash_is_escaped_text_with_one_slash_stripped() {
        assert_eq!(
            parse_input("///me"),
            InputKind::EscapedText("//me".into())
        );
    }

    #[test]
    fn single_slash_is_command_with_empty_name() {
        assert_eq!(
            parse_input("/"),
            InputKind::Command {
                name: String::new(),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn command_no_args() {
        assert_eq!(
            parse_input("/quit"),
            InputKind::Command {
                name: "quit".into(),
                args: Vec::new(),
            }
        );
    }

    #[test]
    fn command_with_args() {
        assert_eq!(
            parse_input("/me kicks the build"),
            InputKind::Command {
                name: "me".into(),
                args: vec!["kicks".into(), "the".into(), "build".into()],
            }
        );
    }

    #[test]
    fn command_collapses_internal_whitespace() {
        // split_whitespace collapses runs of any whitespace.
        assert_eq!(
            parse_input("/me  hi   there"),
            InputKind::Command {
                name: "me".into(),
                args: vec!["hi".into(), "there".into()],
            }
        );
    }

    #[test]
    fn command_with_utf8_args() {
        assert_eq!(
            parse_input("/me waves at åse 🐧"),
            InputKind::Command {
                name: "me".into(),
                args: vec!["waves".into(), "at".into(), "åse".into(), "🐧".into()],
            }
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-commands parse_tests`
Expected: FAIL — `parse_input` and `InputKind` don't exist.

- [ ] **Step 3: Implement `parse_input` + `InputKind`**

Add to `spaze-commands/src/parser.rs` (above `#[cfg(test)]`):

```rust
/// Result of parsing the input buffer at Enter time. Owned strings so the
/// caller can clear `app.input_buffer` immediately after dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputKind {
    /// A plain text message. Send the content as `MessageBody::Text`.
    Text(String),

    /// An escaped text message (input started with `//`). The leading slash
    /// has been stripped — send the content as `MessageBody::Text`.
    EscapedText(String),

    /// A slash command. `name` is the first whitespace-delimited token after
    /// the `/`; may be empty for bare `/`. `args` is the remaining tokens.
    Command { name: String, args: Vec<String> },
}

/// Parse the input buffer into an `InputKind` for Enter-time dispatch.
///
/// Rules:
/// - `"//..."` → `EscapedText` with one leading slash stripped.
/// - `"/..."`  → `Command { name, args }` (name may be empty for bare `/`).
/// - else      → `Text(input)`.
#[must_use]
pub fn parse_input(input: &str) -> InputKind {
    if input.starts_with("//") {
        // Strip ONE slash. "//me" → "/me" as text. "///x" → "//x" as text.
        return InputKind::EscapedText(input[1..].to_string());
    }
    if let Some(rest) = input.strip_prefix('/') {
        let mut tokens = rest.split_whitespace();
        let name = tokens.next().unwrap_or("").to_string();
        let args: Vec<String> = tokens.map(String::from).collect();
        return InputKind::Command { name, args };
    }
    InputKind::Text(input.to_string())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-commands parse_tests`
Expected: PASS, 10/10 tests.

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/parser.rs
git commit -m "feat(commands): parse_input + InputKind for Enter-time dispatch"
```

---

### Task 7: Handlers `/quit`, `/clear`, `/me` (TDD)

**Files:**
- Modify: `spaze-commands/src/commands.rs`

- [ ] **Step 1: Write the failing tests**

Append to `spaze-commands/src/commands.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::registry::HandlerContext;

    fn empty_ctx() -> HandlerContext<'static> {
        HandlerContext { registry: &[] }
    }

    #[test]
    fn quit_returns_quit_effect() {
        assert_eq!(handle_quit(&[], &empty_ctx()), vec![Effect::Quit]);
    }

    #[test]
    fn quit_ignores_args() {
        assert_eq!(
            handle_quit(&["with", "ignored", "args"], &empty_ctx()),
            vec![Effect::Quit]
        );
    }

    #[test]
    fn clear_returns_clear_effect() {
        assert_eq!(
            handle_clear(&[], &empty_ctx()),
            vec![Effect::ClearActiveBuffer]
        );
    }

    #[test]
    fn me_with_no_args_returns_usage_systemline() {
        assert_eq!(
            handle_me(&[], &empty_ctx()),
            vec![Effect::SystemLine("usage: /me <action text>".into())]
        );
    }

    #[test]
    fn me_with_one_arg_sends_action_message() {
        assert_eq!(
            handle_me(&["waves"], &empty_ctx()),
            vec![Effect::SendActionMessage("waves".into())]
        );
    }

    #[test]
    fn me_joins_multiple_args_with_single_space() {
        assert_eq!(
            handle_me(&["kicks", "the", "build"], &empty_ctx()),
            vec![Effect::SendActionMessage("kicks the build".into())]
        );
    }

    #[test]
    fn me_preserves_utf8_in_joined_content() {
        assert_eq!(
            handle_me(&["waves", "at", "åse", "🐧"], &empty_ctx()),
            vec![Effect::SendActionMessage("waves at åse 🐧".into())]
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-commands commands::tests`
Expected: FAIL — `handle_quit`, `handle_clear`, `handle_me` don't exist.

- [ ] **Step 3: Implement the three handlers**

Add to `spaze-commands/src/commands.rs` (above `#[cfg(test)]`):

```rust
use crate::effect::Effect;
use crate::registry::HandlerContext;

/// `/quit` — exit the client.
#[must_use]
pub fn handle_quit(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    vec![Effect::Quit]
}

/// `/clear` — empty the active buffer's local timeline.
#[must_use]
pub fn handle_clear(_args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    vec![Effect::ClearActiveBuffer]
}

/// `/me <action text>` — send the rest of the line as an action message.
#[must_use]
pub fn handle_me(args: &[&str], _ctx: &HandlerContext) -> Vec<Effect> {
    if args.is_empty() {
        return vec![Effect::SystemLine("usage: /me <action text>".into())];
    }
    vec![Effect::SendActionMessage(args.join(" "))]
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-commands commands::tests`
Expected: PASS, 7/7 tests.

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/commands.rs
git commit -m "feat(commands): /quit, /clear, /me handlers with tests"
```

---

### Task 8: Handler `/help` (TDD)

**Files:**
- Modify: `spaze-commands/src/commands.rs`

- [ ] **Step 1: Write the failing tests**

Append to the existing `#[cfg(test)] mod tests { ... }` block in `spaze-commands/src/commands.rs` (find the closing `}` of the tests module — the new tests go just before it):

```rust
    use crate::registry::Command;

    fn fixture_registry() -> [Command; 2] {
        [
            Command {
                name: "quit",
                aliases: &["q"],
                short_help: "exit the client",
                long_help: "/quit — exit cleanly",
                handler: handle_quit,
            },
            Command {
                name: "me",
                aliases: &[],
                short_help: "send an action message",
                long_help: "/me <text> — send action",
                handler: handle_me,
            },
        ]
    }

    #[test]
    fn help_no_args_lists_all_commands_with_header() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&[], &ctx);
        // Header + one line per command = 1 + 2 = 3 effects.
        assert_eq!(effects.len(), 3);
        assert_eq!(
            effects[0],
            Effect::SystemLine("available commands:".into())
        );
        assert_eq!(
            effects[1],
            Effect::SystemLine("  /quit — exit the client".into())
        );
        assert_eq!(
            effects[2],
            Effect::SystemLine("  /me — send an action message".into())
        );
    }

    #[test]
    fn help_with_known_name_returns_long_help() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&["me"], &ctx);
        assert_eq!(
            effects,
            vec![Effect::SystemLine("/me: /me <text> — send action".into())]
        );
    }

    #[test]
    fn help_strips_leading_slash_from_arg() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&["/me"], &ctx);
        assert_eq!(
            effects,
            vec![Effect::SystemLine("/me: /me <text> — send action".into())]
        );
    }

    #[test]
    fn help_with_unknown_name_returns_unknown_command() {
        let reg = fixture_registry();
        let ctx = HandlerContext { registry: &reg };
        let effects = handle_help(&["nope"], &ctx);
        assert_eq!(
            effects,
            vec![Effect::SystemLine("unknown command: /nope".into())]
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-commands commands::tests::help_`
Expected: FAIL — `handle_help` doesn't exist.

- [ ] **Step 3: Implement `handle_help`**

Add to `spaze-commands/src/commands.rs` (above `#[cfg(test)]`, after `handle_me`):

```rust
use crate::registry::lookup;

/// `/help` — list commands. `/help <name>` — show long_help for one.
///
/// Output is one `Effect::SystemLine` per visual line. Each line becomes its
/// own synthetic Message in the buffer; we don't embed `\n` and rely on the
/// renderer to split (the room timeline render path treats one Message as
/// one Line).
#[must_use]
pub fn handle_help(args: &[&str], ctx: &HandlerContext) -> Vec<Effect> {
    if args.is_empty() {
        let mut effects = vec![Effect::SystemLine("available commands:".into())];
        for cmd in ctx.registry {
            effects.push(Effect::SystemLine(format!(
                "  /{} — {}",
                cmd.name, cmd.short_help
            )));
        }
        return effects;
    }
    // Strip a leading slash if the user typed `/help /me`.
    let target = args[0].trim_start_matches('/');
    match lookup(target, ctx.registry) {
        Some(cmd) => vec![Effect::SystemLine(format!("/{}: {}", cmd.name, cmd.long_help))],
        None => vec![Effect::SystemLine(format!("unknown command: /{target}"))],
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-commands commands::tests`
Expected: PASS, 11/11 tests (7 from Task 7 + 4 new).

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/commands.rs
git commit -m "feat(commands): /help handler with registry introspection"
```

---

### Task 9: `REGISTRY` static + invariant tests

**Files:**
- Modify: `spaze-commands/src/registry.rs`

- [ ] **Step 1: Write the failing invariant tests**

Append to the existing `#[cfg(test)] mod tests { ... }` block in `spaze-commands/src/registry.rs` (just before its closing `}`):

```rust
    #[test]
    fn real_registry_has_four_commands() {
        // Sanity: 1.C ships exactly /quit, /help, /me, /clear.
        assert_eq!(REGISTRY.len(), 4);
        let names: Vec<_> = REGISTRY.iter().map(|c| c.name).collect();
        assert!(names.contains(&"quit"));
        assert!(names.contains(&"help"));
        assert!(names.contains(&"me"));
        assert!(names.contains(&"clear"));
    }

    #[test]
    fn real_registry_no_duplicate_names() {
        let mut names: Vec<_> = REGISTRY.iter().map(|c| c.name).collect();
        names.sort_unstable();
        let dedup_len = {
            let mut d = names.clone();
            d.dedup();
            d.len()
        };
        assert_eq!(names.len(), dedup_len, "duplicate command names: {names:?}");
    }

    #[test]
    fn real_registry_aliases_dont_collide() {
        // No alias may equal another command's canonical name or another
        // command's alias.
        for cmd in REGISTRY.iter() {
            for alias in cmd.aliases {
                // Not equal to any name except its own command's name.
                for other in REGISTRY.iter() {
                    if other.name == cmd.name {
                        continue;
                    }
                    assert_ne!(
                        *alias, other.name,
                        "alias {alias:?} of /{} collides with name /{}",
                        cmd.name, other.name
                    );
                    for other_alias in other.aliases {
                        assert_ne!(
                            alias, other_alias,
                            "alias {alias:?} of /{} collides with alias {other_alias:?} of /{}",
                            cmd.name, other.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn real_registry_every_command_has_help_text() {
        for cmd in REGISTRY.iter() {
            assert!(!cmd.name.is_empty(), "command has empty name");
            assert!(
                !cmd.short_help.is_empty(),
                "/{} has empty short_help",
                cmd.name
            );
            assert!(
                !cmd.long_help.is_empty(),
                "/{} has empty long_help",
                cmd.name
            );
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-commands registry::tests::real_registry`
Expected: FAIL — `REGISTRY` doesn't exist yet.

- [ ] **Step 3: Define `REGISTRY`**

Add to `spaze-commands/src/registry.rs` (above `#[cfg(test)]`, after `lookup`):

```rust
use crate::commands::{handle_clear, handle_help, handle_me, handle_quit};

/// The static slash command registry. Adding a command in 1.C+ is one new
/// entry here plus the handler in `commands.rs`.
pub static REGISTRY: &[Command] = &[
    Command {
        name: "quit",
        aliases: &["q"],
        short_help: "exit the client",
        long_help: "/quit — exit the Spaze client cleanly. Closes the WS connection and restores the terminal.",
        handler: handle_quit,
    },
    Command {
        name: "help",
        aliases: &["h"],
        short_help: "list commands or show help for one",
        long_help: "/help — list all commands. /help <name> — show detailed help for a specific command.",
        handler: handle_help,
    },
    Command {
        name: "me",
        aliases: &[],
        short_help: "send an action message",
        long_help: "/me <action text> — send your message as an action (rendered as '* yourname text' in the timeline).",
        handler: handle_me,
    },
    Command {
        name: "clear",
        aliases: &["cls"],
        short_help: "clear the current buffer's timeline",
        long_help: "/clear — empty the active buffer's local timeline. Session-local; messages are not re-fetched on reconnect (no scrollback yet).",
        handler: handle_clear,
    },
];
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-commands`
Expected: PASS, all `spaze-commands` tests including new registry invariants. Total ~22 tests in the crate so far.

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/registry.rs
git commit -m "feat(commands): REGISTRY static + invariant tests"
```

---

### Task 10: `classify_input` + `InputClass` (TDD)

**Files:**
- Modify: `spaze-commands/src/parser.rs`

- [ ] **Step 1: Write the failing tests**

Append to `spaze-commands/src/parser.rs` (a new tests module — keep `parse_tests` separate):

```rust
#[cfg(test)]
mod classify_tests {
    use super::*;
    use crate::registry::{Command, HandlerContext};
    use crate::effect::Effect;

    fn fixture_handler(_a: &[&str], _c: &HandlerContext) -> Vec<Effect> {
        vec![]
    }

    fn fixture() -> [Command; 2] {
        [
            Command {
                name: "quit",
                aliases: &["q"],
                short_help: "x",
                long_help: "x",
                handler: fixture_handler,
            },
            Command {
                name: "me",
                aliases: &[],
                short_help: "x",
                long_help: "x",
                handler: fixture_handler,
            },
        ]
    }

    #[test]
    fn empty_is_text() {
        assert_eq!(classify_input("", &fixture()), InputClass::Text);
    }

    #[test]
    fn plain_text_is_text() {
        assert_eq!(classify_input("hello", &fixture()), InputClass::Text);
    }

    #[test]
    fn slash_mid_line_is_text() {
        assert_eq!(
            classify_input("hello /world", &fixture()),
            InputClass::Text
        );
    }

    #[test]
    fn whitespace_before_slash_is_text() {
        assert_eq!(
            classify_input("   /quit", &fixture()),
            InputClass::Text
        );
    }

    #[test]
    fn double_slash_is_escaped_text() {
        assert_eq!(
            classify_input("//me kicks", &fixture()),
            InputClass::EscapedText
        );
    }

    #[test]
    fn registry_hit_by_name_is_valid() {
        assert_eq!(
            classify_input("/quit", &fixture()),
            InputClass::ValidCommand { name: "quit" }
        );
    }

    #[test]
    fn registry_hit_by_alias_is_valid() {
        assert_eq!(
            classify_input("/q", &fixture()),
            InputClass::ValidCommand { name: "q" }
        );
    }

    #[test]
    fn registry_hit_with_args_uses_command_name_only() {
        assert_eq!(
            classify_input("/me kicks the build", &fixture()),
            InputClass::ValidCommand { name: "me" }
        );
    }

    #[test]
    fn registry_miss_is_invalid() {
        assert_eq!(
            classify_input("/foo", &fixture()),
            InputClass::InvalidCommand { name: "foo" }
        );
    }

    #[test]
    fn bare_slash_is_invalid_with_empty_name() {
        assert_eq!(
            classify_input("/", &fixture()),
            InputClass::InvalidCommand { name: "" }
        );
    }

    #[test]
    fn slash_with_only_whitespace_is_invalid_with_empty_name() {
        assert_eq!(
            classify_input("/   ", &fixture()),
            InputClass::InvalidCommand { name: "" }
        );
    }

    #[test]
    fn utf8_command_name_misses_registry_and_is_invalid() {
        assert_eq!(
            classify_input("/qüit", &fixture()),
            InputClass::InvalidCommand { name: "qüit" }
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-commands classify_tests`
Expected: FAIL — `classify_input` and `InputClass` don't exist.

- [ ] **Step 3: Implement `classify_input` + `InputClass`**

Add to `spaze-commands/src/parser.rs` (above the existing `#[cfg(test)] mod parse_tests`):

```rust
use crate::registry::Command;

/// Per-keystroke classification of the input buffer. Allocates nothing —
/// the `name` field borrows from the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClass<'a> {
    /// Plain text (no `/` prefix, or whitespace before the prefix).
    Text,
    /// Escaped text (input started with `//`). Will be sent as text with one
    /// leading slash stripped.
    EscapedText,
    /// First token after `/` matched a registry entry by name or alias.
    ValidCommand { name: &'a str },
    /// First token after `/` did NOT match. Includes bare `/` (empty name).
    InvalidCommand { name: &'a str },
}

/// Classify the input buffer for the live-coloring path. Cheap enough to
/// call on every keystroke.
#[must_use]
pub fn classify_input<'a>(input: &'a str, registry: &[Command]) -> InputClass<'a> {
    if input.starts_with("//") {
        return InputClass::EscapedText;
    }
    let Some(rest) = input.strip_prefix('/') else {
        return InputClass::Text;
    };
    let name = rest.split_whitespace().next().unwrap_or("");
    if name.is_empty() {
        return InputClass::InvalidCommand { name };
    }
    if registry
        .iter()
        .any(|c| c.name == name || c.aliases.contains(&name))
    {
        InputClass::ValidCommand { name }
    } else {
        InputClass::InvalidCommand { name }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-commands`
Expected: PASS, all `spaze-commands` tests pass. Cumulative count: ~34 tests in `spaze-commands`.

- [ ] **Step 5: Commit**

```bash
git add spaze-commands/src/parser.rs
git commit -m "feat(commands): classify_input + InputClass for live coloring"
```

---

### Task 11: Wire `spaze-commands` + `unicode-width` into `spaze-client`

**Files:**
- Modify: `Cargo.toml` (workspace) — add `unicode-width` to `[workspace.dependencies]`
- Modify: `spaze-client/Cargo.toml` — add `spaze-commands` (path) + `unicode-width.workspace = true`

- [ ] **Step 1: Add `unicode-width` to workspace deps**

In the root `Cargo.toml`, append to `[workspace.dependencies]`:

```toml
unicode-width = "0.2"
```

- [ ] **Step 2: Add `spaze-commands` and `unicode-width` to spaze-client**

In `spaze-client/Cargo.toml`, append to the `[dependencies]` block:

```toml
spaze-commands = { path = "../spaze-commands" }
unicode-width.workspace = true
```

- [ ] **Step 3: Verify the workspace builds**

Run: `cargo build --workspace`
Expected: clean build. `unicode-width` 0.2.x is fetched (~50KB, no transitive deps); `spaze-commands` is used as a path dep.

- [ ] **Step 4: Verify all tests still pass**

Run: `cargo test --workspace`
Expected: ~70 tests across the workspace pass (32 from 1.B + 19 proto + 34 commands - some overlap).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock spaze-client/Cargo.toml
git commit -m "chore(client): add spaze-commands + unicode-width deps"
```

---

### Task 12: `App::apply_effect` (TDD)

**Files:**
- Modify: `spaze-client/src/app.rs` — add `apply_effect` and `push_local_system` helper
- Modify: `spaze-client/src/lib.rs` — add unit tests for `apply_effect`

- [ ] **Step 1: Write the failing unit tests**

Append to the `#[cfg(test)] mod tests { ... }` block in `spaze-client/src/lib.rs` (just before its closing `}`):

```rust
    #[test]
    fn apply_effect_quit_sets_should_quit_flag() {
        use super::app::{App, Identity};
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert!(!app.should_quit);
        app.apply_effect(Effect::Quit);
        assert!(app.should_quit);
    }

    #[test]
    fn apply_effect_clear_empties_active_room_buffer() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        // Push a couple of messages onto the active room buffer.
        if let Buffer::Room(rb) = &mut app.buffers[0] {
            rb.push_message(Message {
                id: MessageId::new(),
                room_id: RoomId::new(),
                author_id: UserId::new(),
                author_device_id: DeviceId::new(),
                author_display_name: "x".into(),
                created_at_ms: 0,
                edited_at_ms: None,
                deleted_at_ms: None,
                body: MessageBody::Text { content: "a".into() },
            });
            rb.push_message(Message {
                id: MessageId::new(),
                room_id: RoomId::new(),
                author_id: UserId::new(),
                author_device_id: DeviceId::new(),
                author_display_name: "x".into(),
                created_at_ms: 0,
                edited_at_ms: None,
                deleted_at_ms: None,
                body: MessageBody::Text { content: "b".into() },
            });
        }
        app.apply_effect(Effect::ClearActiveBuffer);
        if let Buffer::Room(rb) = &app.buffers[0] {
            assert_eq!(rb.messages.len(), 0);
        } else {
            panic!("expected room buffer");
        }
    }

    #[test]
    fn apply_effect_systemline_appends_local_system_message() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, MessageBody, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.apply_effect(Effect::SystemLine("usage: /me <text>".into()));

        if let Buffer::Room(rb) = &app.buffers[0] {
            assert_eq!(rb.messages.len(), 1);
            match &rb.messages[0].body {
                MessageBody::System { content } => {
                    assert_eq!(content, "usage: /me <text>");
                }
                other => panic!("expected System body, got {other:?}"),
            }
        } else {
            panic!("expected room buffer");
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client apply_effect_`
Expected: FAIL — `App::apply_effect` doesn't exist.

- [ ] **Step 3: Implement `apply_effect` and `push_local_system`**

In `spaze-client/src/app.rs`, add at the top of the file (alongside existing `use` lines):

```rust
use spaze_commands::Effect;
use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};
use uuid::Uuid;

use crate::buffers::{Buffer, HelpBuffer, RoomTimelineBuffer, room_timeline::RoomKind};
use crate::theme::{Theme, catppuccin_mocha::CATPPUCCIN_MOCHA};
```

(Replacing the existing imports — duplicates and reordering as shown.)

Then append to the `impl App` block (after `find_buffer`):

```rust
    /// Apply a command effect. The single mutation point for command-driven
    /// state changes. The wire-level send for `SendActionMessage` is *not*
    /// performed here — it requires async access to the WS sink, so the
    /// caller (`handle_key`) does that part. This method covers the
    /// synchronous, App-state-only effects: Quit, ClearActiveBuffer,
    /// SystemLine. For `SendActionMessage`, see `handle_key`'s Insert-mode
    /// branch in `lib.rs`.
    pub fn apply_effect(&mut self, effect: Effect) {
        match effect {
            Effect::Quit => {
                self.should_quit = true;
            }
            Effect::ClearActiveBuffer => {
                if let Some(Buffer::Room(rb)) = self.buffers.get_mut(self.active) {
                    rb.messages.clear();
                    rb.scroll.to_bottom();
                }
            }
            Effect::SystemLine(content) => {
                self.push_local_system(content);
            }
            Effect::SendActionMessage(_) => {
                // Wire-level effect; handled in `handle_key` where the WS
                // sink is in scope. Reaching here means the caller forgot
                // to filter — log and drop, don't panic.
                tracing::warn!(
                    "App::apply_effect received SendActionMessage; \
                     should be handled by caller before dispatch"
                );
            }
        }
    }

    /// Push a synthetic `MessageBody::System` message onto the active buffer
    /// (whichever it is). Locally generated, never sent over the wire,
    /// never persisted.
    fn push_local_system(&mut self, content: String) {
        let msg = Message {
            id: MessageId::from_uuid(Uuid::nil()),
            room_id: RoomId::from_uuid(Uuid::nil()),
            author_id: UserId::from_uuid(Uuid::nil()),
            author_device_id: DeviceId::from_uuid(Uuid::nil()),
            author_display_name: "spaze".into(),
            created_at_ms: chrono::Utc::now().timestamp_millis(),
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::System { content },
        };
        // For 1.C, system lines always render on the active room buffer if
        // one exists; HelpBuffer doesn't have a timeline.
        if let Some(Buffer::Room(rb)) = self.buffers.get_mut(self.active) {
            rb.push_message(msg);
            return;
        }
        // Active buffer isn't a room — fall back to the first room buffer.
        for buf in self.buffers.iter_mut() {
            if let Buffer::Room(rb) = buf {
                rb.push_message(msg);
                return;
            }
        }
    }
```

Note: this reuses `chrono` which is already a workspace dep brought in by `spaze-client`. No new deps.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-client apply_effect_`
Expected: PASS, 3/3 tests.

- [ ] **Step 5: Commit**

```bash
git add spaze-client/src/app.rs spaze-client/src/lib.rs
git commit -m "feat(client): App::apply_effect for command effects"
```

---

### Task 13: Render arm for `MessageBody::Action` + `body_text` update

**Files:**
- Modify: `spaze-client/src/buffers/room_timeline.rs` — add Action match arm in `render`, add Action to `body_text`

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests { ... }` block in `spaze-client/src/lib.rs` (just before its closing `}`):

```rust
    #[test]
    fn body_text_extracts_content_from_action_variant() {
        use super::buffers::room_timeline::body_text_for_test;
        use spaze_proto::MessageBody;

        let body = MessageBody::Action {
            content: "kicks the build".into(),
        };
        assert_eq!(body_text_for_test(&body), "kicks the build");
    }

    #[test]
    fn mention_inside_action_body_is_detectable() {
        // Indirect test: body_text returns content for Action, so the existing
        // mention detection (which lowercases body_text + contains-check)
        // works for actions without further changes.
        use super::buffers::room_timeline::body_text_for_test;
        use spaze_proto::MessageBody;

        let body = MessageBody::Action {
            content: "waves at @beth".into(),
        };
        let needle = "@beth".to_lowercase();
        assert!(body_text_for_test(&body).to_lowercase().contains(&needle));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client body_text_extracts mention_inside_action`
Expected: FAIL — `body_text_for_test` doesn't exist (the existing `body_text` is private).

- [ ] **Step 3: Update `body_text` to handle Action; expose for tests**

In `spaze-client/src/buffers/room_timeline.rs`, locate the `fn body_text(...)` function (currently around the bottom of the file) and update it:

```rust
/// Extract the user-visible text from any text-bearing body variant. Used
/// by mention detection. New variants added to `MessageBody` should be
/// covered here if they carry user-typed content.
pub(crate) fn body_text(body: &spaze_proto::MessageBody) -> &str {
    match body {
        spaze_proto::MessageBody::Text { content }
        | spaze_proto::MessageBody::System { content }
        | spaze_proto::MessageBody::Action { content } => content,
        _ => "",
    }
}

/// Test helper — re-exports `body_text` so unit tests in the client crate
/// can call it.
#[cfg(test)]
pub fn body_text_for_test(body: &spaze_proto::MessageBody) -> &str {
    body_text(body)
}
```

(If `body_text` was previously `fn body_text` without `pub(crate)`, the change widens its visibility within the crate. The `body_text_for_test` shim avoids exposing it publicly.)

- [ ] **Step 4: Run the new tests**

Run: `cargo test -p spaze-client body_text_extracts mention_inside_action`
Expected: PASS, 2/2 tests.

- [ ] **Step 5: Add the Action render arm + render test**

Append to `spaze-client/src/lib.rs` tests block:

```rust
    #[test]
    fn render_action_message_does_not_panic_and_uses_action_path() {
        // Smoke-level render check: rendering an Action message goes through
        // the Action arm (not the wildcard debug path) and produces non-empty
        // output. We can't easily assert visual output in unit tests without
        // a snapshot harness; this asserts the code path is wired up.
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".into(),
            RoomKind::Standard,
            "self".into(),
        );
        rb.push_message(Message {
            id: MessageId::new(),
            room_id: rb.room_id,
            author_id: UserId::new(),
            author_device_id: DeviceId::new(),
            author_display_name: "andreas".into(),
            created_at_ms: 0,
            edited_at_ms: None,
            deleted_at_ms: None,
            body: MessageBody::Action {
                content: "kicks the build".into(),
            },
        });

        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|f| {
                let theme = super::theme::catppuccin_mocha::CATPPUCCIN_MOCHA;
                rb.render(f, f.area(), &theme);
            })
            .expect("draw should not panic");
        // Spot-check that the rendered backend contains "andreas" and "kicks".
        let buf = terminal.backend().buffer().clone();
        let dump: String = (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| buf[(x, y)].symbol().to_string())
            .collect();
        assert!(dump.contains("andreas"), "rendered output: {dump}");
        assert!(dump.contains("kicks"), "rendered output: {dump}");
    }
```

This test will FAIL initially because `RoomTimelineBuffer::render` currently has no Action arm — Action falls through to the wildcard debug path which doesn't produce the readable `andreas` / `kicks` text we expect.

Run: `cargo test -p spaze-client render_action_message`
Expected: FAIL — output contains debug repr like `Action { content: "kicks the build" }`, not the expected `* andreas kicks the build` shape.

- [ ] **Step 6: Add the Action match arm in `render`**

In `spaze-client/src/buffers/room_timeline.rs`, locate the `match &msg.body { ... }` block in `render` (currently has arms for Text, System, and a wildcard `_`). Add an Action arm BEFORE the wildcard:

```rust
                MessageBody::Action { content } => {
                    // Action shape: replace the `<author>` chevroned span
                    // with `* author ` followed by italicized content. Drop
                    // the existing author chevron span and rewrite spans.
                    spans.clear();
                    spans.push(Span::styled(format!("{ts}  "), Style::default().fg(ts_fg)));
                    spans.push(Span::styled(
                        format!("* {author_display} "),
                        Style::default().fg(author_fg).add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(
                        content.clone(),
                        Style::default().fg(body_fg).add_modifier(Modifier::ITALIC),
                    ));
                }
```

The `spans.clear()` call drops the timestamp + chevron-author spans that the prior code already pushed, then we push the action shape. (Reviewing the existing render code: spans is constructed before the match block and pushed across all branches. For Action we want to override that. If the existing code structure makes `spans.clear()` awkward, the same effect can be achieved by checking `matches!(body, MessageBody::Action { .. })` *before* the chevron-author push. The implementer should pick whichever flow is cleaner; the rendered output is what matters.)

- [ ] **Step 7: Run all tests to verify**

Run: `cargo test -p spaze-client`
Expected: PASS — including the new `render_action_message_does_not_panic_and_uses_action_path` test that now sees `andreas` and `kicks` in the rendered output.

- [ ] **Step 8: Commit**

```bash
git add spaze-client/src/buffers/room_timeline.rs spaze-client/src/lib.rs
git commit -m "feat(client): render Action message variant + body_text update"
```

---

### Task 14: Insert mode Enter dispatch (parse + lookup + apply)

**Files:**
- Modify: `spaze-client/src/lib.rs:152-186` (Insert-mode branch of `handle_key`)

- [ ] **Step 1: Survey current behavior**

The current Insert-mode Enter handler at `spaze-client/src/lib.rs:158-176` always sends the input as a Text message. We replace it with a parse-then-dispatch flow that handles all four `InputKind` variants and registry hit/miss.

- [ ] **Step 2: Add the dispatch logic**

Replace the `KeyCode::Enter if !app.input_buffer.is_empty() => { ... }` arm in `spaze-client/src/lib.rs` with:

```rust
            KeyCode::Enter if !app.input_buffer.is_empty() => {
                use spaze_commands::{Effect, InputKind, REGISTRY, lookup, parse_input, HandlerContext};

                let kind = parse_input(&app.input_buffer);
                match kind {
                    InputKind::Text(content) | InputKind::EscapedText(content) => {
                        let frame = ClientFrame {
                            request_id: RequestId(request_counter.fetch_add(1, Ordering::Relaxed)),
                            command: ClientCommand::PostMessage {
                                room_id: config.room_id,
                                author_id: config.user_id,
                                author_device_id: config.device_id,
                                author_display_name: config.display_name.clone(),
                                body: MessageBody::Text { content },
                            },
                        };
                        let json = serde_json::to_string(&frame).context("serialize ClientFrame")?;
                        sink.send(WsMessage::Text(json))
                            .await
                            .map_err(|e| anyhow::anyhow!("ws sink write: {e}"))?;
                        app.input_buffer.clear();
                    }
                    InputKind::Command { name, args } => {
                        let Some(cmd) = lookup(&name, REGISTRY) else {
                            // Registry miss — leave input in bar, no submission,
                            // no system line. The visual red color was the feedback.
                            return Ok(());
                        };
                        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                        let ctx = HandlerContext { registry: REGISTRY };
                        let effects = (cmd.handler)(&arg_refs, &ctx);
                        for effect in effects {
                            match effect {
                                Effect::SendActionMessage(content) => {
                                    if !matches!(app.connection, ConnectionState::Connected { .. }) {
                                        app.apply_effect(Effect::SystemLine(
                                            "not connected — /me requires an active connection".into(),
                                        ));
                                        continue;
                                    }
                                    let frame = ClientFrame {
                                        request_id: RequestId(
                                            request_counter.fetch_add(1, Ordering::Relaxed),
                                        ),
                                        command: ClientCommand::PostMessage {
                                            room_id: config.room_id,
                                            author_id: config.user_id,
                                            author_device_id: config.device_id,
                                            author_display_name: config.display_name.clone(),
                                            body: MessageBody::Action { content },
                                        },
                                    };
                                    let json = serde_json::to_string(&frame)
                                        .context("serialize ClientFrame")?;
                                    sink.send(WsMessage::Text(json))
                                        .await
                                        .map_err(|e| anyhow::anyhow!("ws sink write: {e}"))?;
                                }
                                other => app.apply_effect(other),
                            }
                        }
                        app.input_buffer.clear();
                    }
                }
            }
```

(The change: instead of always constructing a `MessageBody::Text` PostMessage, we route through `parse_input`. Text/EscapedText paths are nearly identical to the prior code. Command path does registry lookup; on miss, return early with no-op (red Enter); on hit, run the handler and apply each effect, with the special `SendActionMessage` case handling the WS write directly because `apply_effect` can't access the sink.)

- [ ] **Step 3: Run tests to verify nothing broke**

Run: `cargo test -p spaze-client`
Expected: all 1.B + 1.C-so-far client tests pass. Behavior of plain text messages unchanged; new code paths exist but aren't covered by unit tests yet (integration tests in Task 16-17 will cover them).

- [ ] **Step 4: Manual sanity check (no smoke yet)**

Run: `cargo build -p spaze-client`
Expected: clean build, no clippy warnings.

Run: `cargo clippy -p spaze-client --all-targets`
Expected: no warnings. (If you get a `clippy::cognitive_complexity` warning on `handle_key`, the existing `#[allow(clippy::cognitive_complexity)]` attribute should still cover it — the new code adds one branch but not enough to warrant a rewrite.)

- [ ] **Step 5: Commit**

```bash
git add spaze-client/src/lib.rs
git commit -m "feat(client): Insert mode Enter dispatches via spaze-commands"
```

---

### Task 15: Input bar live coloring + unicode-width cursor

**Files:**
- Modify: `spaze-client/src/tui.rs` — locate the input bar render fn (search for "input" or "render_input" in the file). Adjust the foreground based on `classify_input` and use `unicode-width` for cursor positioning.

- [ ] **Step 1: Survey existing input render**

Read `spaze-client/src/tui.rs` and find the function (or block within the main `draw` function) that renders the input bar. It currently uses something like `Paragraph::new(app.input_buffer.as_str()).style(...)` with a fixed foreground (likely `theme.foreground`). The cursor is set with `frame.set_cursor_position` using `app.input_buffer.len()` as the column offset (which is bytes, not display cells).

- [ ] **Step 2: Update the render to use `classify_input` and `unicode-width`**

In the input render block of `spaze-client/src/tui.rs`, replace the existing fg-color and cursor-position logic with:

```rust
use spaze_commands::{InputClass, REGISTRY, classify_input};
use unicode_width::UnicodeWidthStr;

// ... inside the input bar render block ...

let class = classify_input(&app.input_buffer, REGISTRY);
let fg = match class {
    InputClass::Text | InputClass::EscapedText => theme.foreground,
    InputClass::ValidCommand { .. } => theme.success,
    InputClass::InvalidCommand { .. } => theme.error,
};

let para = ratatui::widgets::Paragraph::new(app.input_buffer.as_str())
    .style(ratatui::style::Style::default().fg(fg).bg(theme.background));
frame.render_widget(para, input_area);

if matches!(app.mode, crate::app::InputMode::Insert) {
    let cursor_col = UnicodeWidthStr::width(app.input_buffer.as_str()) as u16;
    // input_area.x + cursor_col may overflow the area; clamp to area.right() - 1
    let max_col = input_area.right().saturating_sub(1);
    let col = (input_area.x + cursor_col).min(max_col);
    frame.set_cursor_position((col, input_area.y));
}
```

Adapt variable names (`input_area`, `theme`, `app`) to whatever the existing render block uses. The semantic change is: (1) fg is picked from `classify_input`, (2) cursor column is `UnicodeWidthStr::width` not `.len()`.

- [ ] **Step 3: Add unit tests for color picking + cursor width**

Append to the `#[cfg(test)] mod tests { ... }` block in `spaze-client/src/lib.rs` (just before its closing `}`):

```rust
    #[test]
    fn classify_input_picks_correct_color_for_each_class() {
        use spaze_commands::{InputClass, REGISTRY, classify_input};

        // Validates that each variant has a distinct mapping. The actual
        // theme-color picking lives in tui.rs, but we test the classifier
        // outputs here so a regression in classify_input fails closer to
        // the source.
        assert_eq!(classify_input("hello", REGISTRY), InputClass::Text);
        assert_eq!(classify_input("//me", REGISTRY), InputClass::EscapedText);
        assert!(matches!(
            classify_input("/quit", REGISTRY),
            InputClass::ValidCommand { name: "quit" }
        ));
        assert!(matches!(
            classify_input("/foo", REGISTRY),
            InputClass::InvalidCommand { name: "foo" }
        ));
    }

    #[test]
    fn cursor_width_handles_multibyte_and_wide_chars() {
        use unicode_width::UnicodeWidthStr;
        // ASCII: width = byte len.
        assert_eq!(UnicodeWidthStr::width("abc"), 3);
        // Multi-byte but single-cell (Latin-1 supplement).
        assert_eq!(UnicodeWidthStr::width("ä"), 1);
        assert_eq!(UnicodeWidthStr::width("åäö"), 3);
        // Wide single-codepoint emoji: 2 cells.
        assert_eq!(UnicodeWidthStr::width("😀"), 2);
        // Mixed: ä(1) + 😀(2) + x(1) = 4 cells (but byte len is 2+4+1=7).
        assert_eq!(UnicodeWidthStr::width("ä😀x"), 4);
        assert_eq!("ä😀x".len(), 7); // byte len for comparison
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p spaze-client classify_input_picks cursor_width_handles`
Expected: PASS, 2/2 new tests.

- [ ] **Step 5: Build + clippy check**

Run: `cargo build -p spaze-client && cargo clippy -p spaze-client --all-targets`
Expected: clean build, no clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/tui.rs spaze-client/src/lib.rs
git commit -m "feat(client): live input coloring + unicode-width cursor"
```

---

### Task 16: Integration test — `/me` round-trip between two clients

**Files:**
- Modify: `spaze-server/tests/two_client_chat.rs` (append new test alongside the existing `two_clients_can_chat` test)

- [ ] **Step 1: Reference the existing test pattern**

The 1.A integration test at `spaze-server/tests/two_client_chat.rs` (lines 27–57) shows the inline server-spawning pattern: bind a TCP listener on an OS-assigned port, format `server_url` as `ws://{addr}/`, spawn an accept loop using `spaze_server::ServerState` and `spaze_server::connection::handle_connection`. We replicate that pattern in the new test.

- [ ] **Step 2: Append the `/me` round-trip test**

Append to `spaze-server/tests/two_client_chat.rs` (after the existing `two_clients_can_chat` and any other tests):

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn me_action_message_roundtrips_between_two_clients() -> Result<()> {
    // 1. Spawn the server (same pattern as `two_clients_can_chat`).
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = tokio::net::TcpListener::bind(bind_addr).await.context("bind")?;
    let actual_addr = listener.local_addr()?;
    let server_url = format!("ws://{actual_addr}/");

    let state = spaze_server::ServerState::new();
    let _server_handle = {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    return;
                };
                let conn_id = spaze_server::ConnectionId(
                    state
                        .next_connection_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    spaze_server::connection::handle_connection(stream, s, conn_id, peer).await;
                });
            }
        })
    };

    // 2. Connect two clients.
    let (a_user, a_device) = derive_identity("andreas");
    let (mut a_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("a connect")?;
    let (mut b_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("b connect")?;

    // Tiny delay so both registrations land before sending.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 3. A sends an Action message via PostMessage.
    let room_id = RoomId::from_uuid(Uuid::nil());
    let frame = ClientFrame {
        request_id: RequestId(1),
        command: ClientCommand::PostMessage {
            room_id,
            author_id: a_user,
            author_device_id: a_device,
            author_display_name: "andreas".into(),
            body: MessageBody::Action {
                content: "kicks the build".into(),
            },
        },
    };
    let json = serde_json::to_string(&frame).context("serialize")?;
    a_ws.send(WsMessage::Text(json)).await.context("a send")?;

    // 4. B should receive a ServerFrame::Event with Action body.
    let received = timeout(STEP_TIMEOUT, b_ws.next())
        .await
        .context("timeout waiting for B")?
        .ok_or_else(|| anyhow!("B stream ended"))?
        .context("ws error on B")?;
    let WsMessage::Text(text) = received else {
        return Err(anyhow!("expected text frame, got {received:?}"));
    };
    let server_frame: ServerFrame = serde_json::from_str(&text).context("parse server frame")?;
    match server_frame {
        ServerFrame::Event(ServerEvent::MessagePosted(m)) => match m.body {
            MessageBody::Action { content } => {
                assert_eq!(content, "kicks the build");
            }
            other => return Err(anyhow!("expected Action body, got {other:?}")),
        },
        other => return Err(anyhow!("expected Event(MessagePosted), got {other:?}")),
    }
    Ok(())
}
```

- [ ] **Step 3: Run the new test**

Run: `cargo test -p spaze-server me_action_message_roundtrips`
Expected: PASS, the new test runs in <1s.

- [ ] **Step 4: Commit**

```bash
git add spaze-server/tests/two_client_chat.rs
git commit -m "test(integration): /me action message roundtrips between clients"
```

---

### Task 17: Integration tests — `/clear`, `/quit`, UTF-8 round-trip

**Files:**
- Modify: same integration test file as Task 16

- [ ] **Step 1: Add `/clear` local-only test**

This test is at the *client* level — `/clear` doesn't touch the wire, so the integration test exercises the client's input dispatch directly without spinning up two clients.

Append to `spaze-client/src/lib.rs`'s `#[cfg(test)] mod tests { ... }` block (just before its closing `}`):

```rust
    #[test]
    fn clear_command_empties_active_buffer_only() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use spaze_commands::Effect;
        use spaze_proto::{DeviceId, Message, MessageBody, MessageId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());

        // Push messages onto the room buffer.
        if let Buffer::Room(rb) = &mut app.buffers[0] {
            for content in ["a", "b", "c"] {
                rb.push_message(Message {
                    id: MessageId::new(),
                    room_id: RoomId::new(),
                    author_id: UserId::new(),
                    author_device_id: DeviceId::new(),
                    author_display_name: "x".into(),
                    created_at_ms: 0,
                    edited_at_ms: None,
                    deleted_at_ms: None,
                    body: MessageBody::Text { content: content.into() },
                });
            }
        }
        // Apply ClearActiveBuffer.
        app.apply_effect(Effect::ClearActiveBuffer);
        if let Buffer::Room(rb) = &app.buffers[0] {
            assert_eq!(rb.messages.len(), 0, "active buffer should be empty");
        }
    }
```

- [ ] **Step 2: Add `/quit` test (App-level)**

Append:

```rust
    #[test]
    fn quit_command_sets_should_quit_through_handler_chain() {
        use super::app::{App, Identity};
        use spaze_commands::{HandlerContext, REGISTRY, lookup};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert!(!app.should_quit);

        let cmd = lookup("quit", REGISTRY).expect("/quit must be in registry");
        let ctx = HandlerContext { registry: REGISTRY };
        let effects = (cmd.handler)(&[], &ctx);
        for effect in effects {
            app.apply_effect(effect);
        }
        assert!(app.should_quit);
    }
```

- [ ] **Step 3: Add UTF-8 `/me` round-trip integration test**

Append to `spaze-server/tests/two_client_chat.rs` (after the test added in Task 16):

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn me_with_utf8_content_roundtrips_intact() -> Result<()> {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = tokio::net::TcpListener::bind(bind_addr).await.context("bind")?;
    let actual_addr = listener.local_addr()?;
    let server_url = format!("ws://{actual_addr}/");

    let state = spaze_server::ServerState::new();
    let _server_handle = {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    return;
                };
                let conn_id = spaze_server::ConnectionId(
                    state
                        .next_connection_id
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                );
                let s = state.clone();
                tokio::spawn(async move {
                    spaze_server::connection::handle_connection(stream, s, conn_id, peer).await;
                });
            }
        })
    };

    let (a_user, a_device) = derive_identity("andreas");
    let (mut a_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("a connect")?;
    let (mut b_ws, _) = tokio_tungstenite::connect_async(&server_url)
        .await
        .context("b connect")?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let frame = ClientFrame {
        request_id: RequestId(1),
        command: ClientCommand::PostMessage {
            room_id: RoomId::from_uuid(Uuid::nil()),
            author_id: a_user,
            author_device_id: a_device,
            author_display_name: "andreas".into(),
            body: MessageBody::Action {
                content: "waves at åse 🐧".into(),
            },
        },
    };
    a_ws.send(WsMessage::Text(serde_json::to_string(&frame)?))
        .await
        .context("a send")?;

    let received = timeout(STEP_TIMEOUT, b_ws.next())
        .await
        .context("timeout")?
        .ok_or_else(|| anyhow!("stream ended"))?
        .context("ws error")?;
    let WsMessage::Text(text) = received else {
        return Err(anyhow!("expected text"));
    };
    let frame: ServerFrame = serde_json::from_str(&text)?;
    let ServerFrame::Event(ServerEvent::MessagePosted(m)) = frame else {
        return Err(anyhow!("expected Event MessagePosted"));
    };
    let MessageBody::Action { content } = m.body else {
        return Err(anyhow!("expected Action body"));
    };
    assert_eq!(content, "waves at åse 🐧");
    Ok(())
}
```

- [ ] **Step 4: Run all new tests**

Run: `cargo test -p spaze-client clear_command quit_command_sets`
Run: `cargo test -p spaze-server me_with_utf8`
Expected: all PASS.

- [ ] **Step 5: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: all tests pass. Cumulative count: ~74 tests (32 from 1.B + ~42 new).

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/lib.rs spaze-server/tests/two_client_chat.rs
git commit -m "test: /clear, /quit, and UTF-8 /me roundtrip integration tests"
```

---

### Task 18: (gate) Manual smoke test

**This is a human-in-the-loop gate — no automated steps. Execute the spec's smoke test on a real terminal.**

- [ ] **Step 1: Start the server**

```bash
cargo run --release -p spaze-server
```

Expected: server logs `listening on 127.0.0.1:9876` (or whichever port).

- [ ] **Step 2: Start two clients in separate terminals**

```bash
# terminal 2
cargo run --release -p spaze-client -- --name andreas --server ws://127.0.0.1:9876

# terminal 3
cargo run --release -p spaze-client -- --name beth --server ws://127.0.0.1:9876
```

- [ ] **Step 3: Walk through every step of the spec's manual smoke test**

The script is in the spec at `## Manual smoke test (1.C merge gate)` — eleven steps. Each step has an explicit pass condition. Mark each one PASS or FAIL:

  - [ ] Step 1: both clients connect, sidebar shows room
  - [ ] Step 2: `/quit` shows red→green→exits cleanly
  - [ ] Step 3: `/me waves at @andreas` (green) → action shape on both sides, mention highlight on andreas's side
  - [ ] Step 4: `/foo bar` (red) → Enter is no-op, input stays
  - [ ] Step 5: `//me kicks` (foreground) → renders as text `<andreas> /me kicks`
  - [ ] Step 6: `hello åse 🐧` → renders correctly with UTF-8
  - [ ] Step 7: `/clear` → andreas's timeline empties, beth's untouched
  - [ ] Step 8: `/help` → andreas sees five system lines
  - [ ] Step 9: `/help me` → andreas sees one system line with /me's long_help
  - [ ] Step 10: kill server, `/me sad` → andreas sees "not connected" system line
  - [ ] Step 11: type `ä😀x` → cursor at column 4 (visual check)

- [ ] **Step 4: Capture findings**

If any step fails, fix it before proceeding to Task 19. Add the fix as additional commits. If all eleven pass, proceed.

---

### Task 19: DESIGN.md + README.md updates

**Files:**
- Modify: `DESIGN.md` (sub-project tracking table)
- Modify: `README.md` (phase status / current state section)

- [ ] **Step 1: Update sub-project tracking table in DESIGN.md**

Locate the table at `DESIGN.md:253-257` and update the 1.C row:

```markdown
| 1.C — Slash command parser | ✅ shipped | `2026-05-06-phase-1c-slash-commands-design.md` | `2026-05-06-phase-1c-slash-commands.md` |
```

If the table is followed by a "1.D — Sidebar navigation" row, leave it; if not (it likely doesn't exist yet), add:

```markdown
| 1.D — Sidebar navigation | not yet planned | — | — |
```

- [ ] **Step 2: Update README.md phase status**

Locate the phase status section in `README.md` and add a line for 1.C:

```markdown
- **Phase 1.C shipped 2026-05-06.** Slash command parser: `/quit`, `/help`, `/me`, `/clear` with live input coloring (green/red while typing), `MessageBody::Action` proto variant, and the `spaze-commands` crate as a self-contained registry/parser/effect-model. ~74 tests cumulative.
```

- [ ] **Step 3: Verify the docs render correctly**

Run: `grep -A 5 "Sub-project tracking" DESIGN.md | head -15`
Expected: 1.C marked ✅ shipped with both spec + plan filenames.

- [ ] **Step 4: Commit**

```bash
git add DESIGN.md README.md
git commit -m "docs: mark Phase 1.C complete"
```

---

### Task 20: Push the `phase-1c` branch

- [ ] **Step 1: Verify clean state**

```bash
git status
git log --oneline main..HEAD
```

Expected: clean tree, ~14-18 commits ahead of main.

- [ ] **Step 2: Push**

```bash
git push -u origin phase-1c
```

Expected: branch pushed, GitHub returns the PR-creation URL.

- [ ] **Step 3: Note the URL for Task 21**

Save the PR URL Github prints for the next task.

---

### Task 21: (gate) Open PR + merge

**Files:** GitHub PR — no local file changes.

- [ ] **Step 1: Open the PR**

```bash
gh pr create --title "Phase 1.C: slash command parser" --body "$(cat <<'EOF'
## Summary

Phase 1.C of the 8-week MVP. Stands up `spaze-commands` as a self-contained crate (parser, registry, four handlers, Effect enum), wires it into the TUI's Insert-mode Enter dispatch, adds live input coloring (green/red as the user types), and adds `MessageBody::Action` for `/me`.

## What's in scope

- `spaze-commands` crate with public API: `Effect`, `parse_input`, `classify_input`, `Command`, `HandlerContext`, `Handler`, `REGISTRY`, `lookup`.
- Four commands: `/quit`, `/help`, `/me`, `/clear`. `/help` and `/help <name>` both work.
- Live input coloring driven by `classify_input`. `theme.success` for valid commands, `theme.error` for invalid, `theme.foreground` for text/escaped.
- `MessageBody::Action` proto variant + render arm (italic, no chevrons, `* author content` shape).
- `unicode-width` for accurate cursor positioning on multi-byte / wide UTF-8 input.
- ~42 new tests across four layers (parser/handler/registry pure tests, App-level effect tests, render tests, server integration round-trip).

## Out of scope (per spec)

- Sidebar navigation (deferred to Phase 1.D).
- Quoted args / shell-style tokenization (when a command actually needs it).
- Plugin / runtime command registration (post-MVP).

## Test plan

- [x] `cargo test --workspace` — all ~74 tests pass.
- [x] `cargo clippy --workspace --all-targets` — clean.
- [x] Manual smoke test (per spec's 11-step script) — all steps pass.

EOF
)"
```

- [ ] **Step 2: Wait for CI**

```bash
until gh pr view --json statusCheckRollup -q '.statusCheckRollup | all(.conclusion != "")' 2>/dev/null | grep -q true; do sleep 5; done && gh pr view --json statusCheckRollup -q '.statusCheckRollup[] | "\(.name): \(.conclusion)"'
```

Expected: cargo check / cargo test / cargo clippy / cargo fmt all SUCCESS.

If any fail, fix locally, push, and re-wait.

- [ ] **Step 3: Hand off to user for merge decision**

The user merges manually (per repo convention; same as Phase 1.A and 1.B). Recommend:

```bash
gh pr merge --merge   # preserves per-task commit history
# OR
gh pr merge --rebase  # linear history (1.B precedent — fine either way)
```

- [ ] **Step 4: Post-merge cleanup**

After merge confirmation:

```bash
git checkout main
git pull origin main
git branch -d phase-1c
gh api -X DELETE repos/Gaurgle/spaze/git/refs/heads/phase-1c
git fetch --prune
```

Phase 1.C is shipped. Update memory to reflect — Phase 1 of the MVP is now complete.
