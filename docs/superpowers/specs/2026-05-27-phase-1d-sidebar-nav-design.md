# Phase 1.D — Sidebar Navigation

> **Status:** design locked, ready for implementation plan.
> **Sub-project of:** Phase 1 (interstitial — split out from 1.C during brainstorming).
> **Sibling specs:** 1.A, 1.B, 1.C (all shipped). 1.D is the small fourth sub-project that closes Phase 1's keyboard/mouse UX gaps before Phase 2 begins.

## Context

Phase 1.B shipped a real TUI with a sidebar that renders servers/rooms/DMs, but the sidebar is visible-but-inert — there's no way to navigate it from the keyboard or click into it. Phase 1.C added the slash command system + live input coloring but kept the sidebar's behavior unchanged. 1.D fills the gap: arrow keys (and `hjkl`) drive sidebar selection, `Enter` activates the selected room as the active tab, mouse clicks change which region has focus, and the partial vim-motion set (`hjkl` + `Ctrl+u`/`Ctrl+d`) becomes available across the chrome.

The 1.D scope is small (~30–40% of 1.C). In 1.D's world there is still only one server and one room (multi-server arrives in Phase 2), so most of the sidebar selection logic exercises code paths that won't have meaningful visual impact until Phase 2. The work is foundational: the `FocusedRegion` enum and the layout-rect cache that 1.D introduces are what Phase 4's region-based mouse dispatch and keyboard focus cycling build on.

## Goal of 1.D

A user can navigate Spaze entirely from the keyboard *or* the mouse without the sidebar being a decorative dead-end. Specifically: arrow keys drive sidebar selection, `Enter` activates a room, mouse clicks set focus and trigger actions in the clicked region, and the buffer responds to `Up`/`Down`/`k`/`j` for line scroll plus `Ctrl+u`/`Ctrl+d` for half-page jumps. The architectural deliverable: `FocusedRegion` enum + `LayoutRects` cache, both forward-compatible with Phase 4's bigger input/mouse work.

The implicit deliverable: a Spaze TUI that feels like a real client to anyone coming from WeeChat, irssi, or senpai. Arrow keys do what arrow keys do.

## Scope

### In scope

- New `FocusedRegion` enum (`Sidebar | Tabs | Buffer | Input`) on `App`. 4 variants; in 1.D, focus drives keyboard routing for arrow / `hjkl` / `Enter` keys.
- New `sidebar_selected: Option<usize>` on `App` (index into `app.buffers`, restricted to Room/DM kinds during nav).
- New `App` methods: `sidebar_move_up`, `sidebar_move_down`, `sidebar_activate`, `set_focus`.
- Sidebar render gains an inverted-bg cursor on the selected row when `focus == Sidebar`; dimmed when focus is elsewhere.
- Headers (server URL, `Direct Messages` subheader) render in `theme.muted` and are not selectable — arrow nav skips them.
- Arrow / `hjkl` / `Enter` key routing based on `app.focus`:
  - `Sidebar` focus: `Up`/`k` → move up; `Down`/`j` → move down; `Enter` → activate.
  - `Tabs` focus: `Left`/`h` → previous tab; `Right`/`l` → next tab.
  - `Buffer` focus: forwards Up/Down/k/j to active buffer's `handle_key` for line scroll.
  - `Input` focus: passes keys to input buffer (Insert mode behavior, unchanged from 1.C).
- New global Normal-mode shortcuts (work regardless of focus): `Ctrl+u` / `Ctrl+d` → half-page scroll of active buffer.
- Extension to `RoomTimelineBuffer::handle_key`: `Up`/`k` → scroll one line up, `Down`/`j` → scroll one line down, `Ctrl+u`/`Ctrl+d` → half page.
- Extension to `HelpBuffer::handle_key`: `Ctrl+u`/`Ctrl+d` → half page (it already has `Up`/`Down`/`k`/`j`).
- Mouse capture enabled in `crossterm`. New main-loop branch handles `MouseEventKind::Down(MouseButton::Left)`.
- Mouse click routes:
  - Click on sidebar row → focus = Sidebar; if row maps to a buffer, set `sidebar_selected` + activate.
  - Click on tab → focus = Tabs; `app.active = clicked_tab_idx`.
  - Click on buffer → focus = Buffer (no further action — Phase 4 adds message interaction).
  - Click on input → focus = Input; auto-enter Insert mode (equivalent to pressing `i`).
- New `LayoutRects` struct cached on `App.last_layout` after each `tui::draw`. Holds per-region rects + per-tab and per-sidebar-item rects for hit-testing.
- New helper `tui::region_at(col, row, &LayoutRects) -> Option<MouseHit>` — pure function, testable.
- Workspace version bump 0.4.0 → 0.5.0.
- ~18 new unit tests (no integration tests — all changes are client-local).
- 13-step manual smoke test as the merge gate.

### Out of scope (deferred)

| Feature | Lands in |
|---|---|
| Visual region-level focus indication (dim non-focused regions, highlight borders) | Phase 4 |
| Keyboard cycling between regions (`Ctrl+W h/j/k/l` vim-split-style or similar) | Phase 4 |
| Full region-based mouse dispatch (click messages to react, drag-select, etc.) | Phase 4 |
| Mouse scroll wheel scrolling buffer | Phase 4 |
| `h` / `l` on sidebar to collapse/expand groups | when groups are real (Phase 5/6 if ever) |
| `h` / `l` on buffer for horizontal scroll | only if needed for wide code blocks (Phase 4) |
| Multi-room sidebar (multiple selectable rooms in the cycle) | Phase 2 (data layer makes multi-room real) |
| Multi-server sidebar (multiple server sections) | Phase 2 |
| DM list real entries | Phase 2 alongside DM creation flow |
| `Ctrl+u` / `Ctrl+d` in Insert mode (readline-style kill-line, etc.) | Phase 4 if we ever want it; current call is "leave Insert alone" |
| Right-click, middle-click, drag, double-click | Phase 4+ if any of these prove useful |
| Sidebar collapse/expand of subsections (rooms / DMs) | Phase 5/6 polish |

## Locked Decisions

These were settled during the 2026-05-27 brainstorming session. Each is non-negotiable for 1.D.

1. **Focus model = `FocusedRegion` enum on App.** 4 variants (`Sidebar | Tabs | Buffer | Input`). Focus is orthogonal to mode (Normal/Insert). In Normal mode, focus determines which region receives arrow / `hjkl` / `Enter` keys. Insert mode forces `focus = Input` implicitly.

2. **Sidebar is the default focus** when sidebar is visible. Default is `Buffer` when sidebar is hidden (Ctrl+B). Matches WeeChat/irssi/senpai convention — arrow keys drive the chrome by default.

3. **Partial vim motions in 1.D.** `hjkl` + `Ctrl+u`/`Ctrl+d` included. The full input model (more motions, register, ex commands, etc.) remains a Phase 4 deliverable. Shipping the partial set now is scope creep, accepted because the cost is small (~15 LoC extension to existing `handle_key` methods) and because it makes 1.D actually useful for vim users immediately.

4. **Mouse click changes focus.** No keyboard mechanism for changing focus in 1.D (that's Phase 4). Click on a region sets `app.focus` to that region. Click on a clickable element within the region (sidebar row, tab, input) additionally performs the obvious action.

5. **Headers in the sidebar are not selectable.** Server URL row, `Direct Messages` subheader — rendered in `theme.muted`, skipped by arrow nav, ignored by Enter. Mouse click on a header sets focus to Sidebar but does not change selection or activate.

6. **Inverted-bg selection cursor.** Selected sidebar row renders with `theme.background` fg + `theme.foreground` bg when focus is on Sidebar. Dimmed (`theme.surface` bg, `theme.foreground` fg unchanged) when focus is elsewhere. No `>` prefix marker — cleaner.

7. **Enter activates = switch active tab.** `sidebar_activate()` sets `app.active = sidebar_selected` (if Some). In 1.D the only room is always already the active tab, so this is visually a no-op — but the code path is correct for Phase 2's "click a room not yet open as a tab → open it" flow.

8. **Arrow wrap-around.** `Up` from first item wraps to last, `Down` from last wraps to first. Matches the existing `Tab`/`Shift+Tab` wrap behavior in `cycle_tab_forward`/`backward`.

9. **Layout rects cached on `App.last_layout` after each render.** `tui::draw` populates a `LayoutRects` struct containing per-region rects and per-tab/per-sidebar-item rects. Mouse hit-testing reads this. Mouse events arriving before the first render (a race-condition edge case) are silently dropped.

10. **Visual region-level focus indication is deferred to Phase 4.** In 1.D, the only visible focus signal is the sidebar selection cursor (which dims when sidebar isn't focused). Other regions (Tabs, Buffer) have no visual indication of focus state. Documented as a Phase 4 deliverable alongside the rest of mouse/visual polish.

11. **`Ctrl+u` / `Ctrl+d` are global in Normal mode** — always scroll the active buffer half page, regardless of focus. Same lifecycle as PageUp/PageDown. Not handled in Insert mode (Insert keys pass through to input buffer literally).

## Architecture

### Dependency direction

```
spaze-proto   (unchanged)
    ↑
spaze-commands (unchanged)
    ↑
spaze-client  (this is where all 1.D changes land)
```

No new workspace crates. No new dependencies (crossterm 0.28 already includes mouse support).

### File layout

| File | Change | Approx. lines |
|---|---|---|
| `spaze-client/src/app.rs` | Add `FocusedRegion` enum, `sidebar_selected` + `focus` + `last_layout` fields on `App`, 4 new methods (`sidebar_move_up`, `sidebar_move_down`, `sidebar_activate`, `set_focus`). | ~80 |
| `spaze-client/src/tui.rs` | Sidebar render applies inverted-bg to selected row (dimmed when focus elsewhere); cache `LayoutRects` on App after each draw; new `region_at` pure helper for hit-testing. | ~100 |
| `spaze-client/src/lib.rs` | `handle_key` routes Up/Down/Enter/hjkl to sidebar / tabs / buffer based on focus; new global `Ctrl+u`/`Ctrl+d`; mouse event branch in main loop; enable/disable mouse capture in setup/teardown. | ~70 |
| `spaze-client/src/buffers/room_timeline.rs` | Extend `handle_key`: Up/k → scroll 1 up, Down/j → scroll 1 down, Ctrl+u/Ctrl+d → half page. Add `scroll_half_page_up`/`down` methods. | ~25 |
| `spaze-client/src/buffers/help.rs` | Extend `handle_key`: Ctrl+u/Ctrl+d → half page (Up/Down/k/j already bound). | ~10 |
| `Cargo.toml` (workspace) | Version bump 0.4.0 → 0.5.0. | 1 |

**No new dependencies.**

### New types

```rust
// spaze-client/src/app.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedRegion {
    Sidebar,
    Tabs,
    Buffer,
    Input,
}

// On App:
pub focus: FocusedRegion,
pub sidebar_selected: Option<usize>,  // index into app.buffers, restricted to Room/DM kinds
pub last_layout: Option<LayoutRects>,
```

```rust
// spaze-client/src/tui.rs
#[derive(Debug, Clone)]
pub struct LayoutRects {
    pub sidebar: Option<Rect>,      // None when sidebar is hidden
    pub tabs: Rect,
    pub buffer: Rect,
    pub input: Rect,
    pub status: Rect,
    pub topbar: Rect,
    pub room_header: Option<Rect>,
    /// Per-sidebar-row hit data. Each entry: (buffer_idx if selectable else None, rect).
    pub sidebar_items: Vec<(Option<usize>, Rect)>,
    /// Per-tab hit data: (buffer_idx, tab_rect).
    pub tab_items: Vec<(usize, Rect)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseHit {
    Sidebar { selected_buffer_idx: Option<usize> },
    Tab { buffer_idx: usize },
    Buffer,
    Input,
}

#[must_use]
pub fn region_at(col: u16, row: u16, layout: &LayoutRects) -> Option<MouseHit>;
```

### New methods on `App`

```rust
impl App {
    pub fn sidebar_move_up(&mut self);
    pub fn sidebar_move_down(&mut self);
    pub fn sidebar_activate(&mut self);
    pub fn set_focus(&mut self, region: FocusedRegion);
}
```

All sync, App-state-only. None of them touch the WS sink (Phase 1.C's pattern: WS work happens in `handle_key`, not in App methods).

`sidebar_move_up`/`down` walk `app.buffers` filtered to Room/DM kinds (skipping Help). Wrap-around on overflow/underflow. No-op on empty selectable list.

`sidebar_activate` sets `app.active = sidebar_selected` if Some, else no-op.

`set_focus` is a setter that maintains the invariant **`focus == Input` iff `mode == Insert`**. Concretely:

- `set_focus(Input)` → also sets `app.mode = InputMode::Insert` (mouse-click-on-input shortcut).
- `set_focus(non-Input)` from Insert mode → also sets `app.mode = InputMode::Normal` (clicking outside input exits Insert).
- `set_focus(non-Input)` from Normal mode → no mode change (already Normal).

This means clicking any region while in Insert mode implicitly exits Insert. Pragmatic for mouse users; explicit-but-invisible behavior for keyboard users (who would normally use Esc instead of clicking).

## Data flow

### Keystroke flow (Normal mode)

```
KeyPress
  ↓
1. Global routes (always handled, regardless of focus):
     Tab / Shift+Tab    → cycle tabs
     Ctrl+C / q         → quit
     Ctrl+B             → toggle sidebar (and update default focus
                          based on visibility: sidebar visible → Sidebar,
                          else → Buffer)
     ?                  → open help buffer
     i                  → enter Insert mode (sets focus = Input)
     /                  → enter Insert mode prepending '/' to input
     Ctrl+u / Ctrl+d    → scroll active buffer half page
     PageUp / PageDown  → forward to active buffer's handle_key
     Home / End         → forward to active buffer's handle_key
  ↓ (if not matched)
2. Focus-dependent routes (Up/Down/Left/Right/h/j/k/l/Enter):
     match app.focus {
         Sidebar:
             Up | k             → sidebar_move_up
             Down | j           → sidebar_move_down
             Enter              → sidebar_activate
             Left | h | Right | l → no-op (Phase 4+)
         Tabs:
             Left | h           → cycle_tab_backward
             Right | l          → cycle_tab_forward
             Up | k | Down | j | Enter → no-op
         Buffer:
             Up | k | Down | j | Enter
                                → forward to app.buffers[active].handle_key
             Left | h | Right | l → no-op (Phase 4+)
         Input:
             unreachable in Normal mode (only set during Insert mode)
     }
  ↓ (if still not matched)
3. No-op — unknown key silently dropped
```

### Keystroke flow (Insert mode)

**Unchanged from 1.C.** Insert mode always forces `focus = Input`. The parse+lookup+dispatch flow from 1.C is preserved verbatim. `Ctrl+u` / `Ctrl+d` are NOT special-cased in Insert mode — they pass through as control characters (most terminals don't surface them anyway).

### Mouse event flow (Normal mode)

```
crossterm: Event::Mouse(MouseEvent { kind, column, row, .. })
  ↓
Filter: only MouseEventKind::Down(MouseButton::Left)
  (Right/middle button, drag, scroll wheel → silently ignored)
  ↓
If app.last_layout is None: drop event (no render has happened yet — extremely rare)
  ↓
region_at(column, row, &app.last_layout) → Option<MouseHit>
  ↓
match hit {
    Some(MouseHit::Sidebar { selected_buffer_idx }) => {
        app.set_focus(FocusedRegion::Sidebar);
        if let Some(idx) = selected_buffer_idx {
            app.sidebar_selected = Some(idx);
            app.sidebar_activate();
        }
        // else: clicked on header → focus set but no activation
    }
    Some(MouseHit::Tab { buffer_idx }) => {
        app.set_focus(FocusedRegion::Tabs);
        app.active = buffer_idx;
    }
    Some(MouseHit::Buffer) => {
        app.set_focus(FocusedRegion::Buffer);
        // Phase 4 adds message-click handling
    }
    Some(MouseHit::Input) => {
        app.set_focus(FocusedRegion::Input);
        // set_focus(Input) also flips app.mode = Insert
    }
    None => { /* click on a gutter or border — no-op */ }
}
```

### Mouse event flow (Insert mode)

Same as Normal mode, with the mode transition handled by `set_focus`'s invariant (see *New methods on App* above):
- Click on input → focus already Input, no change.
- Click outside input → `set_focus(other)` runs, which sees `mode == Insert` and exits to Normal.

So clicking the sidebar while in Insert mode is an implicit "Esc + click" — exits Insert mode and changes focus + selection in one click. The mouse handler doesn't need any Insert-vs-Normal branching; the invariant in `set_focus` handles it.

### Sidebar selection cursor render flow

```
tui::render_sidebar(frame, sidebar_rect, app, theme)
  ↓
For each row in the sidebar:
  match row {
      Header (server URL, "Direct Messages") =>
          style = Style::default().fg(theme.muted)
      Selectable item (Room / DM) =>
          if app.sidebar_selected == Some(this_buffer_idx) && app.focus == Sidebar =>
              style = Style::default().fg(theme.background).bg(theme.foreground)  // inverted
          else if app.sidebar_selected == Some(this_buffer_idx) =>
              style = Style::default().fg(theme.foreground).bg(theme.surface)     // dimmed
          else =>
              style = Style::default().fg(theme.foreground)
  }
  Render row with style
```

The sidebar's rendering also populates the `sidebar_items` field of `LayoutRects` while drawing — each row's rect + its associated buffer index (or None for headers) is recorded for later mouse hit-testing.

### Layout rect caching flow

```
tui::draw(frame, app)
  ↓
Compute outer split (topbar / body / status)
Compute body split (sidebar / main) based on app.sidebar_visible
Compute main split (tabs / room_header / buffer / input)
  ↓
Render each region (existing 1.B/1.C code paths, with sidebar render updated per above)
  ↓
After all renders, populate app.last_layout = Some(LayoutRects { ... });
```

The `sidebar_items` and `tab_items` vectors are built incrementally during rendering — each render function pushes its rects into a builder, which is finalized after all renders complete.

## Testing strategy

**Total: ~22 new unit tests** (7 sidebar nav + 4 focus + 5 buffer keybindings + 6 mouse hit-testing). No integration tests. Cumulative after 1.D: ~113 tests (91 from 1.C + 22 new).

### Sidebar navigation tests (`spaze-client/src/lib.rs` test module)

| Test | Asserts |
|---|---|
| `sidebar_initial_selection_is_first_room_buffer` | `app.sidebar_selected == Some(0)` after `App::new` (the one room is index 0) |
| `sidebar_move_with_single_item_is_no_op` | 1.D's actual case — single Room → up/down stays at idx 0 |
| `sidebar_move_up_wraps_from_top_to_bottom` | Push extra Room buffer for fixture; from idx 0 → last idx |
| `sidebar_move_down_wraps_from_bottom_to_top` | mirror |
| `sidebar_move_skips_help_buffer` | Help is in `buffers[1]` but not selectable; with 2 Rooms (idx 0,2) skipping Help (idx 1), move from 0 → 2 |
| `sidebar_activate_switches_active_tab` | `app.active` mutates to `sidebar_selected` |
| `sidebar_activate_with_none_selection_is_no_op` | defensive — no crash, no mutation |

### Focus state tests

| Test | Asserts |
|---|---|
| `app_initial_focus_is_sidebar_when_sidebar_visible` | `app.focus == Sidebar` after `App::new` (default sidebar_visible = true) |
| `set_focus_input_enters_insert_mode` | `set_focus(Input)` from Normal → `app.mode == Insert` |
| `set_focus_non_input_exits_insert_mode` | `set_focus(Sidebar)` (or any non-Input) from Insert → `app.mode == Normal` |
| `set_focus_non_input_from_normal_does_not_change_mode` | `set_focus(Sidebar)` from Normal stays Normal |

### New buffer key bindings

| Test | Asserts |
|---|---|
| `room_buffer_up_and_k_scroll_one_line_up` | `Up` and `k` both decrement scroll by 1 |
| `room_buffer_down_and_j_scroll_one_line_down` | mirror |
| `room_buffer_ctrl_u_scrolls_half_page` | `Ctrl+u` decrements scroll by approximately `area.height / 2` (use a fixture height) |
| `room_buffer_ctrl_d_scrolls_half_page` | mirror |
| `help_buffer_ctrl_u_and_ctrl_d_scroll_half_page` | extend existing HelpBuffer test pattern |

### Mouse hit-testing (`spaze-client/src/tui.rs` test module)

| Test | Asserts |
|---|---|
| `region_at_maps_sidebar_row_to_sidebar_with_buffer_idx` | hit on a known sidebar row → `Some(MouseHit::Sidebar { Some(idx) })` |
| `region_at_maps_sidebar_header_to_sidebar_none` | clicking the server URL row → `Some(MouseHit::Sidebar { None })` |
| `region_at_maps_tab_strip_to_tab_idx` | clicking a specific tab → `Some(MouseHit::Tab { idx })` |
| `region_at_maps_input_to_input` | clicking input → `Some(MouseHit::Input)` |
| `region_at_outside_all_rects_is_none` | clicking outside all rects → `None` |
| `region_at_with_sidebar_hidden_treats_left_columns_as_buffer` | Ctrl+B hidden case — sidebar = None, full body width is buffer |

That's 22 unit tests.

### Manual smoke test (1.D merge gate)

Three-terminal smoke (server + two clients), but most steps are single-client since changes are client-only.

1. Start one server + one client. Sidebar shows `# general` with inverted-bg cursor on that row.
2. Press `j` → no visible change (single room). Press `k` → same. No crash.
3. Press `Enter` on selected room → no visible change (already active tab). No crash.
4. Press `Tab` → switches to Help tab. Sidebar cursor still visible (sidebar still has focus per our model — focus doesn't change on Tab).
5. Click the Room tab in the tab strip with the mouse → switches back to Room tab. Sidebar cursor visually dims (focus shifted to Tabs).
6. Click directly on the input bar with the mouse → enters Insert mode, status bar shows `INSERT`, cursor in input bar.
7. Press Esc → back to Normal mode. Click on a sidebar row → focus moves to Sidebar, cursor becomes inverted-bg again.
8. Click on the room buffer area → focus moves to Buffer, sidebar cursor dims.
9. With focus on Buffer: press `Up`/`k` → scrolls buffer one line up. `Down`/`j` → scrolls one line down. `PageUp`/`PageDown` still work for full-page scroll.
10. With focus on Buffer (or any focus): press `Ctrl+u` → half page up. `Ctrl+d` → half page down.
11. Click on the sidebar row again → focus = Sidebar, cursor inverted, active tab is the room.
12. Press `Ctrl+B` → sidebar hides. Cursor invisible. Default focus shifts to Buffer (arrow keys would now scroll buffer line-by-line per the focus model).
13. Press `Ctrl+B` → sidebar reappears with cursor on the same row it was on before hiding (selection preserved). Focus shifts back to Sidebar.

If all 13 steps pass → ready to merge.

## Workspace version bump

`[workspace.package].version` 0.4.0 → 0.5.0. Pattern: each Phase 1 sub-project bumps the minor.

## Implementation discipline

- **TDD throughout.** Each task starts with the failing test, makes it pass, refactors. Same pattern as 1.A/1.B/1.C.
- **Single branch** `phase-1d` from `main`. PR opens against `main` once smoke test passes.
- **CI must be green** on Ubuntu (rustc latest stable).
- **Pre-push hook** installed during the same session — will fire on push and catch fmt/clippy/test regressions before they reach CI.
- **No `Co-Authored-By` lines** in commit messages.
- **Conventional Commit prefixes:** `feat(client):`, `feat(tui):`, `test(client):`, `chore:`, `docs:`.

## Open questions / follow-ups for later phases

(Not blocking 1.D; captured here so they don't drift.)

- **Focus visibility when sidebar is hidden.** When `sidebar_visible == false` and `app.focus == Sidebar`, what should arrow keys do? Current call: arrows continue to nav sidebar selection (invisibly). Toggling sidebar back shows the cursor where it was. Alternative: hiding sidebar implicitly shifts focus to Buffer. Punted to Phase 4.
- **Default focus on app startup.** Currently `Sidebar` when visible. An empirical user-research question: do users prefer keys to immediately scroll the buffer (focus = Buffer) or to nav the sidebar (focus = Sidebar) on startup? Phase 4 can revisit.
- **`set_focus(Input)` auto-entering Insert mode.** This is a convenience for the mouse-click-on-input path. Some users might find it confusing (clicking input from Normal mode skips the "press i" step). Documented as intentional; revisit if users complain.
- **Mouse capture and terminal compatibility.** crossterm's mouse capture requires terminal support; some VT100-only terminals may not work. If smoke testing reveals issues, document the requirement in DEPLOYMENT.md.
- **Tab strip horizontal scroll** (when many tabs are open). 1.B deferred this. Click-to-switch in 1.D doesn't change the deferral — click only works on visible tabs. Phase 4 polish.
