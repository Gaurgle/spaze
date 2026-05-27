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
    /// (`buffer_idx_if_selectable`, `row_rect`) for each sidebar row.
    /// `None` indicates a header row (no buffer associated).
    pub sidebar_items: Vec<(Option<usize>, Rect)>,
    /// (`buffer_idx`, `tab_rect`) for each tab in the tab strip.
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
    fn contains(rect: Rect, col: u16, row: u16) -> bool {
        col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
    }
    // Sidebar takes precedence (if visible and clicked).
    if let Some(sb) = layout.sidebar {
        if contains(sb, col, row) {
            // Find which sidebar row was hit.
            for (buffer_idx, item_rect) in &layout.sidebar_items {
                if contains(*item_rect, col, row) {
                    return Some(MouseHit::Sidebar {
                        selected_buffer_idx: *buffer_idx,
                    });
                }
            }
            // Click on sidebar but not on any item row (e.g., empty area at bottom).
            return Some(MouseHit::Sidebar {
                selected_buffer_idx: None,
            });
        }
    }
    // Tabs.
    if contains(layout.tabs, col, row) {
        for (buffer_idx, tab_rect) in &layout.tab_items {
            if contains(*tab_rect, col, row) {
                return Some(MouseHit::Tab {
                    buffer_idx: *buffer_idx,
                });
            }
        }
        // Click on tab strip but outside any tab — no region.
        return None;
    }
    // Input.
    if contains(layout.input, col, row) {
        return Some(MouseHit::Input);
    }
    // Buffer.
    if contains(layout.buffer, col, row) {
        return Some(MouseHit::Buffer);
    }
    None
}

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

// Per-region helpers.

fn draw_topbar(frame: &mut Frame, area: Rect, app: &App, theme: &crate::theme::Theme) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let short = format!("{:.8}", app.identity.user_id.as_uuid().simple());
    let conn_text = match &app.connection {
        crate::app::ConnectionState::Connecting => {
            Span::styled("  connecting…  ", Style::default().fg(theme.warning))
        }
        crate::app::ConnectionState::Connected { server_url } => Span::styled(
            format!("  connected  {server_url}  "),
            Style::default().fg(theme.success),
        ),
        crate::app::ConnectionState::Disconnected { reason } => Span::styled(
            format!("  disconnected: {reason}  "),
            Style::default().fg(theme.error),
        ),
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" spaze · {}@{}  ", app.identity.display_name, short),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        conn_text,
    ]);

    let para = Paragraph::new(line).style(Style::default().bg(theme.surface).fg(theme.foreground));
    frame.render_widget(para, area);
}

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![Span::styled(
        " SERVERS",
        Style::default().fg(theme.muted),
    )]));

    let server_label = match &app.connection {
        crate::app::ConnectionState::Connected { server_url } => {
            server_url.trim_start_matches("ws://").to_string()
        }
        _ => "—".to_string(),
    };
    lines.push(Line::from(vec![Span::styled(
        format!(" ▼ {server_label}"),
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "    › repo binding TBD".to_string(),
        Style::default().fg(theme.muted),
    )]));

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![Span::styled(
        "    ROOMS",
        Style::default().fg(theme.muted),
    )]));

    for (i, buf) in app.buffers.iter().enumerate() {
        if let crate::buffers::Buffer::Room(rb) = buf {
            let active = i == app.active;
            let style = if active {
                Style::default().fg(theme.tab_room).bg(theme.overlay)
            } else {
                Style::default().fg(theme.subtle)
            };
            lines.push(Line::from(vec![Span::styled(
                format!("    {}", rb.title()),
                style,
            )]));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![Span::styled(
        "    DIRECT MESSAGES",
        Style::default().fg(theme.muted),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "    (none)".to_string(),
        Style::default().fg(theme.subtle),
    )]));

    // Footer at the bottom of the sidebar.
    let lines_len_u16 = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let mut footer_y = area.height.saturating_sub(2);
    if footer_y < lines_len_u16 {
        footer_y = lines_len_u16.saturating_add(1);
    }
    while u16::try_from(lines.len()).unwrap_or(u16::MAX) < footer_y {
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(vec![Span::styled(
        " + add server…",
        Style::default().fg(theme.muted),
    )]));
    lines.push(Line::from(vec![Span::styled(
        " 🔍 search   ⚙ settings   ? help",
        Style::default().fg(theme.muted),
    )]));

    let para = Paragraph::new(lines).style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}

fn draw_tab_strip(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    const GUTTER: u16 = 1;

    let theme = &app.theme;
    let mut current_line: Vec<Span> = Vec::new();
    let mut current_width: u16 = 0;
    let mut lines: Vec<Line> = Vec::new();

    for (i, buf) in app.buffers.iter().enumerate() {
        let active = i == app.active;
        let label = format!(" {} ", buf.title());
        let label_w = u16::try_from(label.chars().count()).unwrap_or(u16::MAX);
        let kind = buf.kind();
        let fg = match kind {
            crate::buffers::BufferKind::Room => theme.tab_room,
            crate::buffers::BufferKind::DirectMessage => theme.tab_dm,
            crate::buffers::BufferKind::Help => theme.muted,
            _ => theme.tab_special,
        };
        let style = if active {
            Style::default()
                .fg(fg)
                .bg(theme.overlay)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(fg)
        };

        if current_width + label_w + GUTTER > area.width && !current_line.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current_line)));
            current_width = 0;
        }
        current_line.push(Span::styled(label, style));
        current_line.push(Span::raw(" "));
        current_width += label_w + GUTTER;

        if lines.len() >= 3 {
            current_line = vec![Span::styled(" … ", Style::default().fg(theme.muted))];
            break;
        }
    }
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    let para = Paragraph::new(lines).style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}

fn draw_room_header(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let active_buf = &app.buffers[app.active];
    let crate::buffers::Buffer::Room(rb) = active_buf else {
        return;
    };

    let mut spans = vec![Span::styled(
        format!(" {} ", rb.title()),
        Style::default()
            .fg(theme.tab_room)
            .add_modifier(Modifier::BOLD),
    )];
    if let Some(binding) = &rb.repo_binding {
        let binding_text = match &binding.subpath {
            Some(p) => format!("· {p}/"),
            None => format!("· {}", binding.repo_name),
        };
        spans.push(Span::styled(
            binding_text,
            Style::default().fg(theme.tab_special),
        ));
    }
    if let Some(t) = &rb.topic {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("— {t}"),
            Style::default().fg(theme.muted),
        ));
    }

    let para = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use spaze_commands::{InputClass, REGISTRY, classify_input};
    use unicode_width::UnicodeWidthStr;

    let theme = &app.theme;
    let prompt = match app.mode {
        crate::app::InputMode::Normal => Span::styled("› ", Style::default().fg(theme.muted)),
        crate::app::InputMode::Insert => Span::styled(
            "› ",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
    };

    // Live-color the input body based on what the user is typing.
    let class = classify_input(&app.input_buffer, REGISTRY);
    let fg = match class {
        InputClass::Text | InputClass::EscapedText => theme.foreground,
        InputClass::ValidCommand { .. } => theme.success,
        InputClass::InvalidCommand { .. } => theme.error,
    };

    let body = Span::styled(app.input_buffer.clone(), Style::default().fg(fg));

    let line = Line::from(vec![prompt, body]);
    let para = Paragraph::new(line).style(Style::default().bg(theme.surface));
    frame.render_widget(para, area);

    // In Insert mode, place the terminal cursor after the prompt (2 cells) and
    // after the input text. Use display-cell width, not byte length, so
    // multi-byte and wide characters are counted correctly.
    if matches!(app.mode, crate::app::InputMode::Insert) {
        let prompt_width: u16 = 2; // "› " is always 2 display cells
        let input_width =
            u16::try_from(UnicodeWidthStr::width(app.input_buffer.as_str())).unwrap_or(u16::MAX);
        let cursor_col = prompt_width.saturating_add(input_width);
        let max_col = area.right().saturating_sub(1);
        let col = (area.x + cursor_col).min(max_col);
        frame.set_cursor_position((col, area.y));
    }
}

fn draw_statusbar(frame: &mut Frame, area: Rect, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};

    let theme = &app.theme;
    let mode_text = match app.mode {
        crate::app::InputMode::Normal => "NORMAL",
        crate::app::InputMode::Insert => "INSERT",
    };
    let active_title = app.buffers[app.active].title().to_string();
    let msg_count = match &app.buffers[app.active] {
        crate::buffers::Buffer::Room(rb) => rb.messages.len(),
        crate::buffers::Buffer::Help(_) => 0,
    };

    let left = Line::from(vec![
        Span::styled(
            format!(" {mode_text} "),
            Style::default()
                .fg(theme.background)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("· {active_title} · {msg_count} messages"),
            Style::default().fg(theme.muted),
        ),
    ]);

    let right = Line::from(vec![Span::styled(
        format!("theme: {} ", theme.syntect_theme_name),
        Style::default().fg(theme.muted),
    )]);

    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(40)])
        .split(area);
    frame.render_widget(
        Paragraph::new(left).style(Style::default().bg(theme.surface)),
        parts[0],
    );
    frame.render_widget(
        Paragraph::new(right).style(Style::default().bg(theme.surface)),
        parts[1],
    );
}
