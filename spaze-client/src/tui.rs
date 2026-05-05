//! TUI module — terminal setup/teardown and the top-level draw function.
//!
//! `draw()` slices the screen into 7 named regions and dispatches to per-region
//! helpers (`draw_topbar`, `draw_sidebar`, etc.). Each helper takes a Rect and
//! a `&Theme`; rendering is theme-driven throughout.
//!
//! Phase 1.B uses placeholder content for most regions — they get real
//! rendering in Task 8. The structure is in place from this task.

use std::io;

use anyhow::{Context, Result};
use crossterm::ExecutableCommand;
use crossterm::event::DisableMouseCapture;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub const MIN_COLS: u16 = 60;
pub const MIN_ROWS: u16 = 20;

pub type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Enter raw mode + alt screen. Returns a Terminal handle.
///
/// # Errors
///
/// Returns an error if the terminal can't be put into raw mode.
pub fn setup() -> Result<Tui> {
    enable_raw_mode().context("enable_raw_mode")?;
    let mut stdout = io::stdout();
    stdout
        .execute(EnterAlternateScreen)
        .context("EnterAlternateScreen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("Terminal::new")?;
    Ok(terminal)
}

/// Leave raw mode + alt screen.
///
/// # Errors
///
/// Returns an error if the terminal can't be restored. Tries to do as much
/// cleanup as possible regardless.
pub fn teardown(mut terminal: Tui) -> Result<()> {
    let mut stdout = io::stdout();
    let _ = stdout.execute(DisableMouseCapture);
    let _ = stdout.execute(LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = terminal.show_cursor();
    Ok(())
}

/// Draw one frame given the current app state.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Floor check.
    let need_sidebar = app.sidebar_visible;
    let effective_min_cols = if need_sidebar { MIN_COLS } else { 50 };
    if area.width < effective_min_cols || area.height < MIN_ROWS {
        draw_too_small(frame, area, &app.theme);
        return;
    }

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

    let theme = app.theme;
    draw_topbar(frame, top, app, &theme);
    if let Some(sa) = sidebar_area {
        draw_sidebar(frame, sa, app);
    }
    draw_tab_strip(frame, tabs_area, app);
    if let Some(ha) = header_area {
        draw_room_header(frame, ha, app);
    }
    let active_buffer = &mut app.buffers[app.active];
    active_buffer.render(frame, buffer_area, &theme);
    draw_input(frame, input_area, app);
    draw_statusbar(frame, bottom, app);
}

fn draw_too_small(frame: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
    let msg = format!("terminal too small (need {MIN_COLS}×{MIN_ROWS})");
    let para = Paragraph::new(msg)
        .style(Style::default().fg(theme.error))
        .block(Block::default().borders(Borders::NONE));
    // Center-ish placement
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
    frame.render_widget(para, inner[1]);
}

// Per-region helpers — placeholder bodies; Task 8 fills these in.

fn draw_topbar(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    let _ = (frame, area, app, theme);
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_tab_strip(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_room_header(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}

fn draw_statusbar(frame: &mut Frame, area: Rect, app: &App) {
    let _ = (frame, area, app);
}
