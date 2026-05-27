# Phase 1.D — Sidebar Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the sidebar keyboard-navigable + mouse-clickable. Add `FocusedRegion` enum + layout rect cache as forward-compatible infrastructure for Phase 4's full input model. Extend buffers with partial vim motions (`hjkl` + `Ctrl+u`/`Ctrl+d`).

**Architecture:** Focus state on `App` (`FocusedRegion` enum) routes Up/Down/`hjkl`/Enter to the right region. `tui::draw` populates a `LayoutRects` cache; mouse events read it to hit-test clicks. Sidebar render gains an inverted-bg cursor on the selected row. All changes are client-local — no proto change, no new crate, no new deps.

**Tech Stack:** Rust 2024 (MSRV 1.85), workspace at v0.5.0 after Task 1. `crossterm` 0.28's existing mouse support (no new deps).

**Spec:** `docs/superpowers/specs/2026-05-27-phase-1d-sidebar-nav-design.md`. All tasks reference its locked decisions.

**Pre-push discipline:** A pre-push git hook installed at `/Users/at-a/Repos/Spaze/.git/hooks/pre-push` runs `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` before every push. CI failures from fmt/clippy regressions should be near-impossible.

---

### Task 1: Branch + workspace 0.4.0 → 0.5.0

**Files:**
- Modify: `Cargo.toml` (workspace package version)

- [ ] **Step 1: Cut `phase-1d` from `main`**

```bash
git checkout main
git pull origin main
git checkout -b phase-1d
git status
```

Expected: clean tree, on branch `phase-1d`, up to date with `main`.

- [ ] **Step 2: Bump workspace version**

In `Cargo.toml`, change the `[workspace.package]` block's version line:

```toml
[workspace.package]
version = "0.5.0"   # was "0.4.0"
```

- [ ] **Step 3: Sanity-check existing tests + build**

Run: `cargo build && cargo test --workspace`
Expected: clean build, 91 tests pass (no regressions).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump workspace to 0.5.0 for phase 1.D"
```

---

### Task 2: `FocusedRegion` enum + `App` fields (TDD)

**Files:**
- Modify: `spaze-client/src/app.rs` (add enum, fields, default init in `App::new`)
- Modify: `spaze-client/src/lib.rs` (add unit tests in the test module)

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests { ... }` block in `spaze-client/src/lib.rs` (just before its closing `}`):

```rust
    #[test]
    fn app_initial_focus_is_sidebar_when_sidebar_visible() {
        use super::app::{App, FocusedRegion, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let app = App::new(identity, "ws://localhost".into());
        assert!(app.sidebar_visible);
        assert_eq!(app.focus, FocusedRegion::Sidebar);
    }

    #[test]
    fn app_has_sidebar_selected_field_initialized_to_first_room() {
        use super::app::{App, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let app = App::new(identity, "ws://localhost".into());
        // The first Room buffer is at index 0 (Help is at index 1).
        assert_eq!(app.sidebar_selected, Some(0));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client app_initial_focus app_has_sidebar_selected`
Expected: FAIL — `FocusedRegion` and `app.focus` / `app.sidebar_selected` don't exist.

- [ ] **Step 3: Add the enum + fields + default init**

In `spaze-client/src/app.rs`, add `FocusedRegion` between the existing `Identity` struct and `App` struct definitions:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedRegion {
    Sidebar,
    Tabs,
    Buffer,
    Input,
    // Future: Popup (Phase 2's first-run assistant)
}
```

Extend the `App` struct (between `should_quit` and the closing `}`):

```rust
pub struct App {
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub sidebar_visible: bool,
    pub mode: InputMode,
    pub input_buffer: String,
    pub theme: Theme,
    pub connection: ConnectionState,
    pub identity: Identity,
    pub should_quit: bool,
    pub focus: FocusedRegion,
    pub sidebar_selected: Option<usize>,
}
```

Extend the `App::new` constructor — add the two new field initializers in the struct literal:

```rust
Self {
    buffers: vec![Buffer::Room(room), Buffer::Help(help)],
    active: 0,
    sidebar_visible: true,
    mode: InputMode::Normal,
    input_buffer: String::new(),
    theme: CATPPUCCIN_MOCHA,
    connection: ConnectionState::Connecting,
    identity,
    should_quit: false,
    focus: FocusedRegion::Sidebar,
    sidebar_selected: Some(0),
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-client app_initial_focus app_has_sidebar_selected`
Expected: PASS, 2/2 tests.

- [ ] **Step 5: Run the full workspace test suite**

Run: `cargo test --workspace`
Expected: 93 tests pass (91 + 2). Some existing tests construct `App` directly via struct literal — if any fail with "missing fields," update those constructors with the new fields. (Most tests use `App::new` so they pick up the defaults automatically.)

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/app.rs spaze-client/src/lib.rs
git commit -m "feat(client): FocusedRegion enum + sidebar_selected + focus on App"
```

---

### Task 3: `set_focus` method with mode invariant (TDD)

**Files:**
- Modify: `spaze-client/src/app.rs` (add `set_focus` method)
- Modify: `spaze-client/src/lib.rs` (add 3 tests)

- [ ] **Step 1: Write the failing tests**

Append to the test module in `spaze-client/src/lib.rs`:

```rust
    #[test]
    fn set_focus_input_enters_insert_mode() {
        use super::app::{App, FocusedRegion, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert_eq!(app.mode, InputMode::Normal);
        app.set_focus(FocusedRegion::Input);
        assert_eq!(app.focus, FocusedRegion::Input);
        assert_eq!(app.mode, InputMode::Insert);
    }

    #[test]
    fn set_focus_non_input_exits_insert_mode() {
        use super::app::{App, FocusedRegion, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.mode = InputMode::Insert;
        app.focus = FocusedRegion::Input;
        app.set_focus(FocusedRegion::Sidebar);
        assert_eq!(app.focus, FocusedRegion::Sidebar);
        assert_eq!(app.mode, InputMode::Normal);
    }

    #[test]
    fn set_focus_non_input_from_normal_does_not_change_mode() {
        use super::app::{App, FocusedRegion, Identity, InputMode};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert_eq!(app.mode, InputMode::Normal);
        app.set_focus(FocusedRegion::Buffer);
        assert_eq!(app.focus, FocusedRegion::Buffer);
        assert_eq!(app.mode, InputMode::Normal);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client set_focus_`
Expected: FAIL — `App::set_focus` doesn't exist.

- [ ] **Step 3: Implement `set_focus`**

In `spaze-client/src/app.rs`, append to the `impl App` block (after the existing `find_buffer` method):

```rust
    /// Set the focused region, maintaining the invariant:
    /// `focus == Input` iff `mode == Insert`.
    ///
    /// Setting `Input` enters Insert mode (mouse-click-on-input shortcut).
    /// Setting non-Input from Insert mode returns to Normal (click elsewhere).
    pub fn set_focus(&mut self, region: FocusedRegion) {
        self.focus = region;
        match region {
            FocusedRegion::Input => self.mode = InputMode::Insert,
            _ => {
                if matches!(self.mode, InputMode::Insert) {
                    self.mode = InputMode::Normal;
                }
            }
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-client set_focus_`
Expected: PASS, 3/3 tests.

- [ ] **Step 5: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass, 96 tests total.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/app.rs spaze-client/src/lib.rs
git commit -m "feat(client): App::set_focus with focus/mode invariant"
```

---

### Task 4: `sidebar_move_up`/`down` + `sidebar_activate` (TDD)

**Files:**
- Modify: `spaze-client/src/app.rs` (add 3 methods)
- Modify: `spaze-client/src/lib.rs` (add 5 tests)

- [ ] **Step 1: Write the failing tests**

Append to the test module in `spaze-client/src/lib.rs`:

```rust
    #[test]
    fn sidebar_move_with_single_item_is_no_op() {
        use super::app::{App, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        // Only one Room buffer (index 0). Move up/down stays at 0.
        assert_eq!(app.sidebar_selected, Some(0));
        app.sidebar_move_up();
        assert_eq!(app.sidebar_selected, Some(0));
        app.sidebar_move_down();
        assert_eq!(app.sidebar_selected, Some(0));
    }

    #[test]
    fn sidebar_move_down_wraps_with_multiple_rooms() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use spaze_proto::{DeviceId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        // Push two more Room buffers (so indices 0, 2, 3 are Rooms; 1 is Help).
        app.buffers.push(Buffer::Room(RoomTimelineBuffer::new(
            RoomId::new(),
            "# second".into(),
            RoomKind::Standard,
            "test".into(),
        )));
        app.buffers.push(Buffer::Room(RoomTimelineBuffer::new(
            RoomId::new(),
            "# third".into(),
            RoomKind::Standard,
            "test".into(),
        )));
        // Selection cycles 0 → 2 → 3 → 0.
        app.sidebar_selected = Some(0);
        app.sidebar_move_down();
        assert_eq!(app.sidebar_selected, Some(2));
        app.sidebar_move_down();
        assert_eq!(app.sidebar_selected, Some(3));
        app.sidebar_move_down();
        assert_eq!(app.sidebar_selected, Some(0));
    }

    #[test]
    fn sidebar_move_up_wraps_with_multiple_rooms() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use spaze_proto::{DeviceId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.buffers.push(Buffer::Room(RoomTimelineBuffer::new(
            RoomId::new(),
            "# second".into(),
            RoomKind::Standard,
            "test".into(),
        )));
        app.sidebar_selected = Some(0);
        app.sidebar_move_up();
        assert_eq!(app.sidebar_selected, Some(2));
        app.sidebar_move_up();
        assert_eq!(app.sidebar_selected, Some(0));
    }

    #[test]
    fn sidebar_activate_switches_active_tab() {
        use super::app::{App, Identity};
        use super::buffers::Buffer;
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use spaze_proto::{DeviceId, RoomId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.buffers.push(Buffer::Room(RoomTimelineBuffer::new(
            RoomId::new(),
            "# second".into(),
            RoomKind::Standard,
            "test".into(),
        )));
        assert_eq!(app.active, 0);
        app.sidebar_selected = Some(2);
        app.sidebar_activate();
        assert_eq!(app.active, 2);
    }

    #[test]
    fn sidebar_activate_with_none_selection_is_no_op() {
        use super::app::{App, Identity};
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        app.sidebar_selected = None;
        let prior_active = app.active;
        app.sidebar_activate();
        assert_eq!(app.active, prior_active);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client sidebar_move sidebar_activate`
Expected: FAIL — none of `sidebar_move_up`, `sidebar_move_down`, `sidebar_activate` exist.

- [ ] **Step 3: Implement the three methods**

In `spaze-client/src/app.rs`, append to the `impl App` block (after `set_focus`):

```rust
    /// Move sidebar selection to the previous Room/DM buffer (wraps around).
    /// No-op if there are zero or one selectable items.
    pub fn sidebar_move_up(&mut self) {
        let selectable: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, Buffer::Room(_)))
            .map(|(i, _)| i)
            .collect();
        if selectable.is_empty() {
            self.sidebar_selected = None;
            return;
        }
        let cur = self.sidebar_selected.unwrap_or(selectable[0]);
        let pos = selectable.iter().position(|&i| i == cur).unwrap_or(0);
        let prev = if pos == 0 {
            selectable[selectable.len() - 1]
        } else {
            selectable[pos - 1]
        };
        self.sidebar_selected = Some(prev);
    }

    /// Move sidebar selection to the next Room/DM buffer (wraps around).
    /// No-op if there are zero or one selectable items.
    pub fn sidebar_move_down(&mut self) {
        let selectable: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b, Buffer::Room(_)))
            .map(|(i, _)| i)
            .collect();
        if selectable.is_empty() {
            self.sidebar_selected = None;
            return;
        }
        let cur = self.sidebar_selected.unwrap_or(selectable[0]);
        let pos = selectable.iter().position(|&i| i == cur).unwrap_or(0);
        let next = selectable[(pos + 1) % selectable.len()];
        self.sidebar_selected = Some(next);
    }

    /// Activate the currently selected sidebar item (switch active tab).
    /// No-op if `sidebar_selected == None`.
    pub fn sidebar_activate(&mut self) {
        if let Some(idx) = self.sidebar_selected {
            if idx < self.buffers.len() {
                self.active = idx;
            }
        }
    }
```

NOTE: DMs are represented as `Buffer::Room(RoomTimelineBuffer)` with `RoomKind::Direct` on the inner struct (per `spaze-client/src/buffers/mod.rs:19` comment "also used for DMs (kind=Direct on inner struct)"). So `Buffer::Room(_)` alone covers both standard rooms and DMs in 1.D. A future `Buffer` enum split (if Phase 2 introduces a distinct `Buffer::DirectMessage` variant) would update the filter then.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-client sidebar_move sidebar_activate`
Expected: PASS, 5/5 tests.

- [ ] **Step 5: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all pass, 101 tests total.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/app.rs spaze-client/src/lib.rs
git commit -m "feat(client): sidebar_move_up/down + sidebar_activate"
```

---

### Task 5: `RoomTimelineBuffer` Up/k/Down/j/Ctrl+u/Ctrl+d (TDD)

**Files:**
- Modify: `spaze-client/src/buffers/room_timeline.rs` (extend `handle_key`, add `scroll_one_up`/`scroll_one_down`/`scroll_half_page_up`/`scroll_half_page_down` methods to `ScrollState`)
- Modify: `spaze-client/src/lib.rs` (add 4 tests)

- [ ] **Step 1: Write the failing tests**

Append to the test module in `spaze-client/src/lib.rs`:

```rust
    #[test]
    fn room_buffer_up_and_k_scroll_one_line_up() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use spaze_proto::{RoomId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".into(),
            RoomKind::Standard,
            "self".into(),
        );
        rb.scroll.offset_from_bottom = 5;
        rb.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(rb.scroll.offset_from_bottom, 6);
        rb.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(rb.scroll.offset_from_bottom, 7);
    }

    #[test]
    fn room_buffer_down_and_j_scroll_one_line_down() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use spaze_proto::{RoomId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".into(),
            RoomKind::Standard,
            "self".into(),
        );
        rb.scroll.offset_from_bottom = 5;
        rb.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(rb.scroll.offset_from_bottom, 4);
        rb.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(rb.scroll.offset_from_bottom, 3);
    }

    #[test]
    fn room_buffer_ctrl_u_scrolls_half_page() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use spaze_proto::{RoomId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".into(),
            RoomKind::Standard,
            "self".into(),
        );
        rb.scroll.offset_from_bottom = 0;
        rb.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        // Half-page constant defaults to 5 lines (configurable later).
        assert_eq!(rb.scroll.offset_from_bottom, 5);
    }

    #[test]
    fn room_buffer_ctrl_d_scrolls_half_page() {
        use super::buffers::room_timeline::{RoomKind, RoomTimelineBuffer};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use spaze_proto::{RoomId};

        let mut rb = RoomTimelineBuffer::new(
            RoomId::new(),
            "# test".into(),
            RoomKind::Standard,
            "self".into(),
        );
        rb.scroll.offset_from_bottom = 10;
        rb.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(rb.scroll.offset_from_bottom, 5);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client room_buffer_up_and_k room_buffer_down_and_j room_buffer_ctrl_u room_buffer_ctrl_d`
Expected: FAIL — the new keys aren't handled (current `handle_key` only matches PageUp/Down/Home/End).

- [ ] **Step 3: Add scroll methods to `ScrollState`**

In `spaze-client/src/buffers/room_timeline.rs`, append these methods to the existing `impl ScrollState` block (after `to_top`):

```rust
    pub fn scroll_one_up(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_add(1);
        self.stuck_to_bottom = false;
    }

    pub fn scroll_one_down(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_sub(1);
        if self.offset_from_bottom == 0 {
            self.stuck_to_bottom = true;
        }
    }

    pub fn scroll_half_page_up(&mut self) {
        // Half-page is 5 lines in 1.D (fixed). Phase 4 may make this
        // viewport-relative once region rects are easier to pipe through.
        self.offset_from_bottom = self.offset_from_bottom.saturating_add(5);
        self.stuck_to_bottom = false;
    }

    pub fn scroll_half_page_down(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_sub(5);
        if self.offset_from_bottom == 0 {
            self.stuck_to_bottom = true;
        }
    }
```

- [ ] **Step 4: Extend `RoomTimelineBuffer::handle_key`**

In the same file, locate the existing `handle_key` method at ~line 216 and update its `match key.code { ... }` block. Replace the existing match with:

```rust
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        use crossterm::event::KeyModifiers;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::PageUp => {
                self.scroll.page_up();
                true
            }
            KeyCode::PageDown => {
                self.scroll.page_down();
                true
            }
            KeyCode::Home => {
                self.scroll.to_top();
                true
            }
            KeyCode::End => {
                self.scroll.to_bottom();
                true
            }
            KeyCode::Up | KeyCode::Char('k') if !ctrl => {
                self.scroll.scroll_one_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl => {
                self.scroll.scroll_one_down();
                true
            }
            KeyCode::Char('u') if ctrl => {
                self.scroll.scroll_half_page_up();
                true
            }
            KeyCode::Char('d') if ctrl => {
                self.scroll.scroll_half_page_down();
                true
            }
            _ => false,
        }
    }
```

The `if !ctrl` guards on the Up/Down/k/j arms prevent Ctrl+Up/Down or Ctrl+k/Ctrl+j (which aren't bound here) from being absorbed as single-line scroll. Ctrl+u/Ctrl+d are handled by their own arms.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p spaze-client room_buffer_up_and_k room_buffer_down_and_j room_buffer_ctrl_u room_buffer_ctrl_d`
Expected: PASS, 4/4 tests.

- [ ] **Step 6: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: 105 tests pass.

- [ ] **Step 7: Commit**

```bash
git add spaze-client/src/buffers/room_timeline.rs spaze-client/src/lib.rs
git commit -m "feat(client): room buffer vim motions + half-page scroll"
```

---

### Task 6: `HelpBuffer` Ctrl+u/Ctrl+d (TDD)

**Files:**
- Modify: `spaze-client/src/buffers/help.rs` (extend `handle_key`)
- Modify: `spaze-client/src/lib.rs` (add 1 test)

- [ ] **Step 1: Write the failing test**

Append to the test module in `spaze-client/src/lib.rs`:

```rust
    #[test]
    fn help_buffer_ctrl_u_and_ctrl_d_scroll_half_page() {
        use super::buffers::HelpBuffer;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut hb = HelpBuffer::new();
        hb.scroll = 0;
        hb.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(hb.scroll, 5);
        hb.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(hb.scroll, 0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p spaze-client help_buffer_ctrl_u_and_ctrl_d`
Expected: FAIL — Ctrl+u and Ctrl+d aren't handled in HelpBuffer.

- [ ] **Step 3: Extend `HelpBuffer::handle_key`**

In `spaze-client/src/buffers/help.rs`, locate the existing `handle_key` method. Update the match block to add Ctrl+u and Ctrl+d arms — BEFORE the existing `KeyCode::Up | KeyCode::Char('k')` and `KeyCode::Down | KeyCode::Char('j')` arms, so that `Ctrl+u` doesn't get absorbed by the bare-`k` arm via single-char-down/up logic. The full updated method:

```rust
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        use crossterm::event::KeyModifiers;
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('d') if ctrl => {
                self.scroll = self.scroll.saturating_add(5);
                true
            }
            KeyCode::Char('u') if ctrl => {
                self.scroll = self.scroll.saturating_sub(5);
                true
            }
            KeyCode::Up | KeyCode::Char('k') if !ctrl => {
                self.scroll = self.scroll.saturating_sub(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl => {
                self.scroll = self.scroll.saturating_add(1);
                true
            }
            KeyCode::Home => {
                self.scroll = 0;
                true
            }
            _ => false,
        }
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-client help_buffer`
Expected: PASS — both the new test and the existing `help_buffer_scrolls_with_jk_and_arrows`.

- [ ] **Step 5: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: 106 tests pass.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/buffers/help.rs spaze-client/src/lib.rs
git commit -m "feat(client): help buffer Ctrl+u/Ctrl+d half-page scroll"
```

---

### Task 7: `LayoutRects` struct + `region_at` helper (TDD)

**Files:**
- Modify: `spaze-client/src/tui.rs` (add `LayoutRects`, `MouseHit`, `region_at`)
- Modify: `spaze-client/src/app.rs` (add `last_layout: Option<LayoutRects>` field)
- Modify: `spaze-client/src/lib.rs` (add 6 tests for `region_at`)

- [ ] **Step 1: Write the failing tests**

Append to the test module in `spaze-client/src/lib.rs`:

```rust
    fn make_test_layout(sidebar_visible: bool) -> super::tui::LayoutRects {
        use ratatui::layout::Rect;
        use super::tui::LayoutRects;

        let sidebar = if sidebar_visible {
            Some(Rect::new(0, 1, 24, 18))
        } else {
            None
        };
        LayoutRects {
            sidebar,
            tabs: Rect::new(if sidebar_visible { 24 } else { 0 }, 1, 56, 2),
            buffer: Rect::new(if sidebar_visible { 24 } else { 0 }, 4, 56, 14),
            input: Rect::new(if sidebar_visible { 24 } else { 0 }, 18, 56, 1),
            status: Rect::new(0, 19, 80, 1),
            topbar: Rect::new(0, 0, 80, 1),
            room_header: Some(Rect::new(if sidebar_visible { 24 } else { 0 }, 3, 56, 1)),
            // (Some(buffer_idx_or_none), rect): row 2 = header, row 3 = # general (idx 0)
            sidebar_items: if sidebar_visible {
                vec![
                    (None, Rect::new(0, 2, 24, 1)),    // server header
                    (Some(0), Rect::new(0, 3, 24, 1)), // # general
                ]
            } else {
                vec![]
            },
            tab_items: vec![
                (0, Rect::new(if sidebar_visible { 24 } else { 0 }, 1, 12, 1)), // # general tab
                (1, Rect::new(if sidebar_visible { 36 } else { 12 }, 1, 8, 1)), // ? help tab
            ],
        }
    }

    #[test]
    fn region_at_maps_sidebar_row_to_sidebar_with_buffer_idx() {
        use super::tui::{MouseHit, region_at};
        let layout = make_test_layout(true);
        // Click on row 3 (= # general row)
        assert_eq!(
            region_at(5, 3, &layout),
            Some(MouseHit::Sidebar { selected_buffer_idx: Some(0) })
        );
    }

    #[test]
    fn region_at_maps_sidebar_header_to_sidebar_none() {
        use super::tui::{MouseHit, region_at};
        let layout = make_test_layout(true);
        // Click on row 2 (= server header)
        assert_eq!(
            region_at(5, 2, &layout),
            Some(MouseHit::Sidebar { selected_buffer_idx: None })
        );
    }

    #[test]
    fn region_at_maps_tab_strip_to_tab_idx() {
        use super::tui::{MouseHit, region_at};
        let layout = make_test_layout(true);
        // Click on the second tab (Help) — x range 36..44 at y=1
        assert_eq!(
            region_at(40, 1, &layout),
            Some(MouseHit::Tab { buffer_idx: 1 })
        );
    }

    #[test]
    fn region_at_maps_input_to_input() {
        use super::tui::{MouseHit, region_at};
        let layout = make_test_layout(true);
        // Input row is at y=18, x range 24..80
        assert_eq!(region_at(30, 18, &layout), Some(MouseHit::Input));
    }

    #[test]
    fn region_at_outside_all_rects_is_none() {
        use super::tui::region_at;
        let layout = make_test_layout(true);
        // Click on status bar (y=19) is not mapped to any focusable region.
        assert_eq!(region_at(40, 19, &layout), None);
    }

    #[test]
    fn region_at_with_sidebar_hidden_treats_left_columns_as_buffer() {
        use super::tui::{MouseHit, region_at};
        let layout = make_test_layout(false);
        // x=5, y=10 — sidebar is hidden, so this hits the buffer (which starts at x=0).
        assert_eq!(region_at(5, 10, &layout), Some(MouseHit::Buffer));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p spaze-client region_at_`
Expected: FAIL — `LayoutRects`, `MouseHit`, `region_at` don't exist.

- [ ] **Step 3: Add types + helper to `tui.rs`**

In `spaze-client/src/tui.rs`, add at the top of the file (after the existing `use` block, before `MIN_COLS`):

```rust
/// Layout rectangles cached after each `draw()` for mouse hit-testing.
#[derive(Debug, Clone)]
pub struct LayoutRects {
    pub sidebar: Option<Rect>,
    pub tabs: Rect,
    pub buffer: Rect,
    pub input: Rect,
    pub status: Rect,
    pub topbar: Rect,
    pub room_header: Option<Rect>,
    /// (buffer_idx_if_selectable, row_rect) for each sidebar row.
    /// `None` indicates a header row (no buffer associated).
    pub sidebar_items: Vec<(Option<usize>, Rect)>,
    /// (buffer_idx, tab_rect) for each tab in the tab strip.
    pub tab_items: Vec<(usize, Rect)>,
}

/// Result of `region_at` — what was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseHit {
    Sidebar { selected_buffer_idx: Option<usize> },
    Tab { buffer_idx: usize },
    Buffer,
    Input,
}

/// Map a mouse click at (col, row) to a region. Returns `None` for clicks
/// outside all focusable regions (status bar, topbar, gutters).
#[must_use]
pub fn region_at(col: u16, row: u16, layout: &LayoutRects) -> Option<MouseHit> {
    fn contains(rect: &Rect, col: u16, row: u16) -> bool {
        col >= rect.x && col < rect.x + rect.width
            && row >= rect.y && row < rect.y + rect.height
    }
    // Sidebar takes precedence (if visible and clicked).
    if let Some(sb) = layout.sidebar {
        if contains(&sb, col, row) {
            // Find which sidebar row was hit.
            for (buffer_idx, item_rect) in &layout.sidebar_items {
                if contains(item_rect, col, row) {
                    return Some(MouseHit::Sidebar {
                        selected_buffer_idx: *buffer_idx,
                    });
                }
            }
            // Click on sidebar but not on any item row (e.g., empty area at bottom).
            return Some(MouseHit::Sidebar { selected_buffer_idx: None });
        }
    }
    // Tabs.
    if contains(&layout.tabs, col, row) {
        for (buffer_idx, tab_rect) in &layout.tab_items {
            if contains(tab_rect, col, row) {
                return Some(MouseHit::Tab { buffer_idx: *buffer_idx });
            }
        }
        // Click on tab strip but outside any tab — no region.
        return None;
    }
    // Input.
    if contains(&layout.input, col, row) {
        return Some(MouseHit::Input);
    }
    // Buffer.
    if contains(&layout.buffer, col, row) {
        return Some(MouseHit::Buffer);
    }
    None
}
```

- [ ] **Step 4: Add `last_layout` field to App**

In `spaze-client/src/app.rs`, add the field to the `App` struct (after `sidebar_selected`):

```rust
pub struct App {
    pub buffers: Vec<Buffer>,
    pub active: usize,
    pub sidebar_visible: bool,
    pub mode: InputMode,
    pub input_buffer: String,
    pub theme: Theme,
    pub connection: ConnectionState,
    pub identity: Identity,
    pub should_quit: bool,
    pub focus: FocusedRegion,
    pub sidebar_selected: Option<usize>,
    pub last_layout: Option<crate::tui::LayoutRects>,
}
```

Update `App::new` to initialize `last_layout: None`:

```rust
Self {
    // ... existing fields ...
    focus: FocusedRegion::Sidebar,
    sidebar_selected: Some(0),
    last_layout: None,
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p spaze-client region_at_`
Expected: PASS, 6/6 tests.

- [ ] **Step 6: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: 112 tests pass.

- [ ] **Step 7: Commit**

```bash
git add spaze-client/src/tui.rs spaze-client/src/app.rs spaze-client/src/lib.rs
git commit -m "feat(tui): LayoutRects + region_at helper for mouse hit-testing"
```

---

### Task 8: Sidebar render with selection cursor + populate `sidebar_items`

**Files:**
- Modify: `spaze-client/src/tui.rs` (update `draw_sidebar` to take `&mut LayoutRectsBuilder` and apply selection-cursor styling)

This task adds the visible sidebar selection cursor AND records per-row rects into a builder that the next task will assemble into `LayoutRects`.

- [ ] **Step 1: Add a `LayoutRectsBuilder` struct to `tui.rs`**

In `spaze-client/src/tui.rs`, after the `LayoutRects` definition added in Task 7:

```rust
/// Builder used during `draw()` to collect rects from each render helper
/// into a final `LayoutRects`. Finalized after all renders complete.
#[derive(Debug, Default)]
pub(crate) struct LayoutRectsBuilder {
    pub sidebar: Option<Rect>,
    pub tabs: Option<Rect>,
    pub buffer: Option<Rect>,
    pub input: Option<Rect>,
    pub status: Option<Rect>,
    pub topbar: Option<Rect>,
    pub room_header: Option<Rect>,
    pub sidebar_items: Vec<(Option<usize>, Rect)>,
    pub tab_items: Vec<(usize, Rect)>,
}

impl LayoutRectsBuilder {
    pub(crate) fn build(self) -> Option<LayoutRects> {
        Some(LayoutRects {
            sidebar: self.sidebar,
            tabs: self.tabs?,
            buffer: self.buffer?,
            input: self.input?,
            status: self.status?,
            topbar: self.topbar?,
            room_header: self.room_header,
            sidebar_items: self.sidebar_items,
            tab_items: self.tab_items,
        })
    }
}
```

- [ ] **Step 2: Update `draw_sidebar` signature and body**

In `spaze-client/src/tui.rs`, locate the existing `fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App)` (~line 198). Change its signature to also take `&mut LayoutRectsBuilder`:

```rust
fn draw_sidebar(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    builder: &mut LayoutRectsBuilder,
) {
```

Then update the body to:
1. Apply inverted-bg / dimmed styling to the selected row based on `app.sidebar_selected` and `app.focus`
2. Push `(Option<buffer_idx>, row_rect)` into `builder.sidebar_items` for each row

The room-rendering loop (currently ~line 233) becomes:

```rust
    for (i, buf) in app.buffers.iter().enumerate() {
        if let crate::buffers::Buffer::Room(rb) = buf {
            let is_selected = app.sidebar_selected == Some(i);
            let focused = matches!(app.focus, crate::app::FocusedRegion::Sidebar);
            let style = if is_selected && focused {
                // Inverted-bg cursor when sidebar has focus.
                Style::default().fg(theme.background).bg(theme.foreground)
            } else if is_selected {
                // Dimmed cursor when sidebar lost focus.
                Style::default().fg(theme.foreground).bg(theme.surface)
            } else if i == app.active {
                Style::default().fg(theme.tab_room).bg(theme.overlay)
            } else {
                Style::default().fg(theme.subtle)
            };
            lines.push(Line::from(vec![Span::styled(
                format!("    {}", rb.title()),
                style,
            )]));
            // Record this row's rect for mouse hit-testing.
            // The row's y in the sidebar area is `area.y + lines.len() - 1`.
            let row_y = area.y + u16::try_from(lines.len()).unwrap_or(u16::MAX).saturating_sub(1);
            builder.sidebar_items.push((Some(i), Rect::new(area.x, row_y, area.width, 1)));
        }
    }
```

For the header lines (server URL, "ROOMS" label, "DIRECT MESSAGES" label, etc.), record them with `None` buffer index so clicking sets focus but doesn't activate:

```rust
    // Example for the server URL header line — wrap each `lines.push(...)` for a header:
    let row_y = area.y + u16::try_from(lines.len()).unwrap_or(u16::MAX);
    builder.sidebar_items.push((None, Rect::new(area.x, row_y, area.width, 1)));
    lines.push(Line::from(vec![Span::styled(...)]));
```

NOTE: there are multiple `lines.push(...)` calls for headers in the existing function (server URL, blank line, "ROOMS", "DIRECT MESSAGES", "(none)", blank, "+ add server", "search/settings/help"). Wrap each header push with a builder.sidebar_items push (using `None` for the buffer index). For pure spacers (blank `Line::raw("")`), it's safe to record them with `None` too — click is just no-op there.

A cleaner pattern: track current row index as a local variable and push rect + line together for every visual row.

- [ ] **Step 3: Save `area` to the builder**

At the top of `draw_sidebar`:

```rust
    builder.sidebar = Some(area);
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p spaze-client`
Expected: clean build. (Tests will be added in next task wave; this task is rendering-only.)

This task doesn't have a dedicated unit test — the visible behavior is verified by the manual smoke test (Task 12). The `region_at` tests already cover the hit-testing path; this task's render code is exercised by integration in subsequent tasks.

- [ ] **Step 5: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: 112 tests pass (no test count change — render changes are smoke-tested).

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/tui.rs
git commit -m "feat(tui): sidebar selection cursor + per-row rect tracking"
```

---

### Task 9: `draw()` populates `app.last_layout` after rendering

**Files:**
- Modify: `spaze-client/src/tui.rs` (the `draw` function)
- Modify: `spaze-client/src/lib.rs` (add 1 test)

- [ ] **Step 1: Write the failing test**

Append to the test module in `spaze-client/src/lib.rs`:

```rust
    #[test]
    fn draw_populates_last_layout() {
        use super::app::{App, Identity};
        use super::tui::draw;
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;
        use spaze_proto::{DeviceId, UserId};

        let identity = Identity {
            user_id: UserId::new(),
            device_id: DeviceId::new(),
            display_name: "test".into(),
        };
        let mut app = App::new(identity, "ws://localhost".into());
        assert!(app.last_layout.is_none());

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal.draw(|f| draw(f, &mut app)).expect("draw");

        assert!(app.last_layout.is_some(), "draw should populate last_layout");
        let layout = app.last_layout.as_ref().unwrap();
        assert!(layout.sidebar.is_some(), "sidebar visible by default");
        assert!(!layout.sidebar_items.is_empty(), "sidebar items recorded");
        assert!(!layout.tab_items.is_empty(), "tab items recorded");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p spaze-client draw_populates_last_layout`
Expected: FAIL — `last_layout` stays `None` after `draw`.

- [ ] **Step 3: Update `draw()` to populate the builder + commit to `app.last_layout`**

In `spaze-client/src/tui.rs`, update the `draw` function (~line 64). At the top of the function, create the builder; after each region renders, populate it; at the end, finalize:

```rust
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Floor check.
    let need_sidebar = app.sidebar_visible;
    let effective_min_cols = if need_sidebar { MIN_COLS } else { 50 };
    if area.width < effective_min_cols || area.height < MIN_ROWS {
        draw_too_small(frame, area, &app.theme);
        // Don't populate layout on the too-small path.
        app.last_layout = None;
        return;
    }

    let mut builder = LayoutRectsBuilder::default();

    // Vertical: topbar (1) | middle (flex) | statusbar (1).
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    let top = outer[0];
    let middle = outer[1];
    let bottom = outer[2];

    builder.topbar = Some(top);
    builder.status = Some(bottom);

    // Horizontal in middle: optional sidebar | main.
    let (sidebar_area, main_area) = if app.sidebar_visible {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(24), Constraint::Min(0)])
            .split(middle);
        (Some(split[0]), split[1])
    } else {
        (None, middle)
    };

    // Vertical in main: tabs (auto, 1-3) | header (0 or 1) | buffer (flex) | input (1-3).
    let active_kind = app.buffers[app.active].kind();
    let needs_room_header = matches!(
        active_kind,
        crate::buffers::BufferKind::Room | crate::buffers::BufferKind::DirectMessage
    );
    let main_constraints: Vec<Constraint> = if needs_room_header {
        vec![
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ]
    };
    let main_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints(main_constraints)
        .split(main_area);

    let tabs_area = main_split[0];
    let (header_area, buffer_area, input_area) = if needs_room_header {
        (Some(main_split[1]), main_split[2], main_split[3])
    } else {
        (None, main_split[1], main_split[2])
    };

    builder.tabs = Some(tabs_area);
    builder.buffer = Some(buffer_area);
    builder.input = Some(input_area);
    builder.room_header = header_area;

    let theme = app.theme;
    draw_topbar(frame, top, app, &theme);
    if let Some(sa) = sidebar_area {
        draw_sidebar(frame, sa, app, &mut builder);
    }
    draw_tab_strip(frame, tabs_area, app, &mut builder);
    if let Some(ha) = header_area {
        draw_room_header(frame, ha, app);
    }
    let active_buffer = &mut app.buffers[app.active];
    active_buffer.render(frame, buffer_area, &theme);
    draw_input(frame, input_area, app);
    draw_statusbar(frame, bottom, app);

    app.last_layout = builder.build();
}
```

NOTE: `draw_tab_strip` also takes `&mut LayoutRectsBuilder` now. Update its signature similarly to `draw_sidebar`. In the tab-rendering loop (~line 291), record each rendered tab's rect into `builder.tab_items`. Tracking the per-tab rect requires knowing the tab's x-range within `tabs_area`; the existing code has `current_width` as a running width counter, so each tab's rect is `Rect::new(tabs_area.x + current_width - label_w, tabs_area.y + lines.len() as u16, label_w, 1)` (or similar; implementer should adapt to the actual local variable names).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p spaze-client draw_populates_last_layout`
Expected: PASS, 1/1 test.

- [ ] **Step 5: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: 113 tests pass.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/tui.rs spaze-client/src/lib.rs
git commit -m "feat(tui): populate app.last_layout after each draw"
```

---

### Task 10: Enable mouse capture in `setup` (teardown already disables)

**Files:**
- Modify: `spaze-client/src/tui.rs` (add `EnableMouseCapture` to `setup`)

- [ ] **Step 1: Add `EnableMouseCapture` import and call**

In `spaze-client/src/tui.rs`, update the imports at the top:

```rust
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
```

Then in the `setup` function (~line 37), call `EnableMouseCapture` after `EnterAlternateScreen`:

```rust
pub fn setup() -> Result<Tui> {
    enable_raw_mode().context("enable_raw_mode")?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("EnterAlternateScreen")?;
    stdout
        .execute(EnableMouseCapture)
        .context("EnableMouseCapture")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("Terminal::new")?;
    Ok(terminal)
}
```

The existing `teardown` already calls `DisableMouseCapture` (line 56) — defensive, but now actually needed.

- [ ] **Step 2: Verify it compiles + tests pass**

Run: `cargo build -p spaze-client && cargo test --workspace`
Expected: clean build, 113 tests pass. (No new test for this — the mouse capture is verified by Task 11's mouse event handling integration.)

- [ ] **Step 3: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add spaze-client/src/tui.rs
git commit -m "feat(tui): enable mouse capture on setup"
```

---

### Task 11: Main loop mouse handling + focus-based key routing + global Ctrl+u/Ctrl+d

**Files:**
- Modify: `spaze-client/src/lib.rs` (update the `tokio::select!` to handle mouse events, update `handle_key` to route based on focus)

This is the most substantial task — it integrates everything that landed in Tasks 2-10.

- [ ] **Step 1: Add mouse event branch in the main loop**

In `spaze-client/src/lib.rs`, locate the `tokio::select!` block in `run` (~line 67). The existing structure handles key events via `Some(Ok(CrosstermEvent::Key(key)))`. Update the match arm in the events branch to also handle mouse:

```rust
                Some(Ok(CrosstermEvent::Key(key))) => {
                    handle_key(key, &mut app, &mut sink, &config, &request_counter).await?;
                }
                Some(Ok(CrosstermEvent::Mouse(mouse))) => {
                    handle_mouse(mouse, &mut app);
                }
                // Resize triggers redraw at end of loop. Paste/focus events ignored.
                Some(Ok(_)) => {}
```

- [ ] **Step 2: Add `handle_mouse` function**

In `spaze-client/src/lib.rs`, add a new function (place after `handle_key`):

```rust
fn handle_mouse(mouse: crossterm::event::MouseEvent, app: &mut App) {
    use crossterm::event::{MouseButton, MouseEventKind};
    use crate::app::FocusedRegion;
    use crate::tui::{MouseHit, region_at};

    // Only handle left-button down. Other buttons / drag / scroll → ignored in 1.D.
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return;
    }
    let Some(layout) = &app.last_layout else {
        return;  // no render yet — drop event
    };
    let Some(hit) = region_at(mouse.column, mouse.row, layout) else {
        return;  // click on gutter / border / status
    };
    match hit {
        MouseHit::Sidebar { selected_buffer_idx } => {
            app.set_focus(FocusedRegion::Sidebar);
            if let Some(idx) = selected_buffer_idx {
                app.sidebar_selected = Some(idx);
                app.sidebar_activate();
            }
        }
        MouseHit::Tab { buffer_idx } => {
            app.set_focus(FocusedRegion::Tabs);
            if buffer_idx < app.buffers.len() {
                app.active = buffer_idx;
            }
        }
        MouseHit::Buffer => {
            app.set_focus(FocusedRegion::Buffer);
            // No further action in 1.D.
        }
        MouseHit::Input => {
            // set_focus(Input) also enters Insert mode via the invariant.
            app.set_focus(FocusedRegion::Input);
        }
    }
}
```

- [ ] **Step 3: Update `handle_key` for focus-based routing + global Ctrl+u/Ctrl+d**

In `spaze-client/src/lib.rs`, locate the Normal mode portion of `handle_key` (~line 188, after the Insert mode `return Ok(())`). The existing structure handles globals (Tab, Ctrl+B, Ctrl+C, q, i, ?) and then falls through to `app.buffers[app.active].handle_key(key)`.

Update it to:
1. Add global `Ctrl+u` / `Ctrl+d` handling (always scrolls active buffer half page).
2. After globals, branch on `app.focus` to route Up/Down/k/j/h/l/Enter to the right destination.

Replace the existing Normal-mode block (from line 188 through the `let _ = app.buffers[app.active].handle_key(key); Ok(())` at line 222-223) with:

```rust
    // Normal mode: globals first, then focus-based routing.
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Global routes (regardless of focus).
    match (key.code, ctrl) {
        (KeyCode::Char('c'), true) | (KeyCode::Char('q'), false) => {
            app.should_quit = true;
            return Ok(());
        }
        (KeyCode::Char('b'), true) => {
            app.toggle_sidebar();
            // Re-establish default focus based on visibility.
            app.set_focus(if app.sidebar_visible {
                crate::app::FocusedRegion::Sidebar
            } else {
                crate::app::FocusedRegion::Buffer
            });
            return Ok(());
        }
        (KeyCode::Char('i'), false) => {
            app.set_focus(crate::app::FocusedRegion::Input);
            return Ok(());
        }
        (KeyCode::Char('?'), false) => {
            if let Some(idx) = app.find_buffer(|b| matches!(b, Buffer::Help(_))) {
                app.active = idx;
            }
            return Ok(());
        }
        (KeyCode::Tab, false) => {
            app.cycle_tab_forward();
            return Ok(());
        }
        (KeyCode::BackTab, _) => {
            app.cycle_tab_backward();
            return Ok(());
        }
        // Global Ctrl+u / Ctrl+d → scroll active buffer half page.
        (KeyCode::Char('u'), true) => {
            let _ = app.buffers[app.active].handle_key(key);
            return Ok(());
        }
        (KeyCode::Char('d'), true) => {
            let _ = app.buffers[app.active].handle_key(key);
            return Ok(());
        }
        // PageUp/Down/Home/End → always forward to active buffer.
        (KeyCode::PageUp, _) | (KeyCode::PageDown, _) | (KeyCode::Home, _) | (KeyCode::End, _) => {
            let _ = app.buffers[app.active].handle_key(key);
            return Ok(());
        }
        _ => {}
    }

    // Focus-based routing for Up/Down/Left/Right/h/j/k/l/Enter.
    use crate::app::FocusedRegion;
    match app.focus {
        FocusedRegion::Sidebar => match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.sidebar_move_up(),
            KeyCode::Down | KeyCode::Char('j') => app.sidebar_move_down(),
            KeyCode::Enter => app.sidebar_activate(),
            _ => {}  // h, l, others: no-op in 1.D
        },
        FocusedRegion::Tabs => match key.code {
            KeyCode::Left | KeyCode::Char('h') => app.cycle_tab_backward(),
            KeyCode::Right | KeyCode::Char('l') => app.cycle_tab_forward(),
            _ => {}
        },
        FocusedRegion::Buffer => {
            let _ = app.buffers[app.active].handle_key(key);
        }
        FocusedRegion::Input => {
            // Unreachable in Normal mode (Input focus only set during Insert mode).
        }
    }
    Ok(())
```

NOTE: This change is structural. Read the existing `handle_key` carefully before replacing — there may be small details (e.g., the `let ctrl = ...` line already exists). Adapt the replacement to preserve any handler logic the spec doesn't explicitly change.

The `i` arm now uses `set_focus(Input)` instead of directly setting `app.mode = Insert` — `set_focus`'s invariant handles the mode transition.

- [ ] **Step 4: Run all client tests**

Run: `cargo test -p spaze-client`
Expected: PASS — all existing + new tests still pass (no integration test for the main loop wiring; manual smoke test is the gate).

- [ ] **Step 5: Run pre-push checks**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: clean, 113 tests.

- [ ] **Step 6: Commit**

```bash
git add spaze-client/src/lib.rs
git commit -m "feat(client): main loop mouse + focus-based key routing + global Ctrl+u/d"
```

---

### Task 12: (gate) Manual smoke test

**This is a human-in-the-loop gate — no automated steps. Execute the spec's 13-step smoke test on a real terminal.**

- [ ] **Step 1: Start the server**

```bash
cargo run --release -p spaze-server
```

Expected: server logs `listening on 127.0.0.1:9876`.

- [ ] **Step 2: Start a client in another terminal**

```bash
cargo run --release -p spaze-client -- --name andreas --server ws://127.0.0.1:9876
```

- [ ] **Step 3: Walk through every step of the spec's manual smoke test**

The full 13-step script is in `docs/superpowers/specs/2026-05-27-phase-1d-sidebar-nav-design.md` under `## Testing strategy` → `### Manual smoke test (1.D merge gate)`. Mark each:

  - [ ] Step 1: Sidebar shows `# general` with inverted-bg cursor
  - [ ] Step 2: `j`/`k` no visible change (single room), no crash
  - [ ] Step 3: `Enter` on selected room — no visible change (already active)
  - [ ] Step 4: `Tab` cycles to Help tab; sidebar cursor still visible (focus = Sidebar unchanged)
  - [ ] Step 5: Mouse click on Room tab → switches back; sidebar cursor dims (focus shifted to Tabs)
  - [ ] Step 6: Mouse click on input bar → enters Insert mode (`INSERT` in status bar, cursor in input)
  - [ ] Step 7: Esc returns to Normal; click on sidebar row → focus = Sidebar, cursor inverted
  - [ ] Step 8: Click on room buffer area → focus = Buffer, sidebar cursor dims
  - [ ] Step 9: With Buffer focus: `Up`/`k` scroll buffer one line, `Down`/`j` opposite, PageUp/Down still work
  - [ ] Step 10: `Ctrl+u` half page up, `Ctrl+d` half page down (works regardless of focus)
  - [ ] Step 11: Click sidebar row → focus = Sidebar, cursor inverted, room is active
  - [ ] Step 12: `Ctrl+B` hides sidebar (cursor invisible); `Ctrl+B` again shows it with cursor preserved
  - [ ] Step 13: Resize terminal smaller (still above 60×20) → layout adjusts, sidebar selection preserved

- [ ] **Step 4: Capture findings**

If any step fails, fix it before proceeding to Task 13. Add the fix as additional commits. If all 13 pass, proceed.

---

### Task 13: DESIGN.md + README.md updates

**Files:**
- Modify: `DESIGN.md` (sub-project tracking table)
- Modify: `README.md` (phase status section)

- [ ] **Step 1: Update sub-project tracking table in `DESIGN.md`**

Locate the table at `DESIGN.md:253-258`. Update the 1.D row from "not yet planned" to:

```markdown
| 1.D — Sidebar navigation | ✅ shipped | `2026-05-27-phase-1d-sidebar-nav-design.md` | `2026-05-27-phase-1d-sidebar-nav.md` |
```

- [ ] **Step 2: Add a Phase 1.D line to `README.md` phase status section**

Find the existing Phase 1.C line in `README.md` and add 1.D right after:

```markdown
- **Phase 1.D shipped 2026-05-27.** Sidebar navigation: arrow keys and `hjkl` drive sidebar selection, `Enter` activates a room, mouse clicks set focus and trigger region actions, buffers gain `Ctrl+u`/`Ctrl+d` half-page scroll. `FocusedRegion` enum + `LayoutRects` cache as forward infra for Phase 4. ~113 tests cumulative.
```

If the existing format uses different phrasing, adapt to match.

- [ ] **Step 3: Verify docs**

Run: `grep -A 6 "Sub-project tracking" DESIGN.md | head -12`
Expected: 1.D row shows ✅ shipped with spec + plan filenames.

- [ ] **Step 4: Commit**

```bash
git add DESIGN.md README.md
git commit -m "docs: mark Phase 1.D complete"
```

---

### Task 14: Push `phase-1d` branch

- [ ] **Step 1: Verify clean state**

```bash
git status
git log --oneline main..HEAD
```

Expected: clean tree, ~12-14 commits ahead of main.

- [ ] **Step 2: Push (pre-push hook will fire)**

```bash
git push -u origin phase-1d
```

Expected: pre-push hook runs fmt + clippy + tests, all pass, push succeeds.

If the hook fails, fix locally, commit, push again. (It shouldn't fail since we ran the checks at each task — but it's the safety net.)

- [ ] **Step 3: Note the PR URL for Task 15**

GitHub will print a PR-creation URL after push. Use it (or the `gh pr create` command in Task 15).

---

### Task 15: (gate) Open PR + merge

**Files:** GitHub PR — no local file changes.

- [ ] **Step 1: Open the PR**

```bash
gh pr create --title "Phase 1.D: sidebar navigation" --body "$(cat <<'EOF'
## Summary

Phase 1.D of the 8-week MVP. Makes the sidebar keyboard-navigable and mouse-clickable, adds the `FocusedRegion` enum + `LayoutRects` cache as forward infrastructure for Phase 4's full input model, and extends buffers with partial vim motions (hjkl + Ctrl+u/Ctrl+d half-page).

## What's in scope

- `FocusedRegion` enum on App with 4 variants (Sidebar/Tabs/Buffer/Input). `set_focus` maintains the invariant `focus == Input` iff `mode == Insert`.
- New App fields: `focus`, `sidebar_selected`, `last_layout`.
- New App methods: `sidebar_move_up`, `sidebar_move_down`, `sidebar_activate`, `set_focus`.
- Sidebar render gains an inverted-bg cursor on the selected row (dimmed when focus is elsewhere). Headers in the sidebar render as muted and are not selectable.
- `RoomTimelineBuffer::handle_key` extended with Up/k/Down/j (one line scroll) and Ctrl+u/Ctrl+d (half page).
- `HelpBuffer::handle_key` extended with Ctrl+u/Ctrl+d (already had Up/Down/k/j).
- `LayoutRects` struct + `region_at` helper in tui.rs. tui::draw populates `app.last_layout` after each render.
- Mouse capture enabled in setup; left-click handling in the main loop routes to region + sets focus + triggers action.
- ~22 new tests covering sidebar nav, focus state, new buffer keys, and mouse hit-testing.

## Out of scope (per spec)

- Visual region-level focus indication (Phase 4 alongside region polish).
- Keyboard cycling between regions (Phase 4, likely Ctrl+W h/j/k/l).
- Full region-based mouse dispatch (Phase 4).
- Popup as a 5th FocusedRegion variant (Phase 2 alongside first-run assistant).

## Test plan

- [x] cargo test --workspace — ~113 tests pass.
- [x] cargo clippy --workspace --all-targets -- -D warnings — clean.
- [x] cargo fmt --check — clean.
- [x] Pre-push hook fires + passes locally.
- [x] Manual smoke test (per spec's 13-step script) — all steps pass.

EOF
)"
```

- [ ] **Step 2: Wait for CI**

```bash
until gh pr view --json statusCheckRollup -q '.statusCheckRollup | map(select(.status == "COMPLETED")) | length' 2>/dev/null | grep -q "^4$"; do sleep 5; done && gh pr view --json statusCheckRollup -q '.statusCheckRollup[] | "\(.name): \(.conclusion)"'
```

Expected: all four checks SUCCESS. (Pre-push hook should have caught everything locally.)

If any fail: fix, push, re-wait.

- [ ] **Step 3: Hand off to user for merge decision**

The user merges manually. Recommend `--rebase` per the 1.A/1.B/1.C/PR#4 precedent:

```bash
gh pr merge --rebase
```

- [ ] **Step 4: Post-merge cleanup**

```bash
git checkout main
git pull origin main
git branch -d phase-1d
gh api -X DELETE repos/Gaurgle/spaze/git/refs/heads/phase-1d
git fetch --prune
```

Phase 1.D shipped. Update memory: Phase 1 fully complete (1.A + 1.B + 1.C + 1.D shipped).
