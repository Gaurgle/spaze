//! `HelpBuffer` — static keybindings reference. Demonstrates that the `Buffer`
//! enum supports >1 variant (proves the abstraction is real).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::Theme;

const KEYBINDINGS: &[(&str, &str)] = &[
    // Global (Normal mode)
    ("Tab", "next tab"),
    ("Shift-Tab", "previous tab"),
    ("Ctrl+B", "toggle sidebar"),
    ("i", "enter Insert mode (focus input)"),
    ("?", "open this help"),
    ("q", "quit"),
    ("Ctrl+C", "quit"),
    // Insert mode
    ("Esc", "Insert → Normal (input preserved)"),
    ("Enter", "send message"),
    ("Backspace", "delete char"),
    // Buffer-local (Normal mode)
    ("PageUp/PageDown", "scroll timeline (Room)"),
    ("Home/End", "top/bottom (Room)"),
    ("j/k or Up/Down", "scroll help text"),
];

pub struct HelpBuffer {
    pub scroll: u16,
}

impl HelpBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    #[must_use]
    pub fn title(&self) -> &'static str {
        "? help"
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "Spaze · Phase 1.B keybindings",
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::raw(""));
        for (key, desc) in KEYBINDINGS {
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:>16}  "), Style::default().fg(theme.primary)),
                Span::styled((*desc).to_string(), Style::default().fg(theme.muted)),
            ]));
        }
        let para = Paragraph::new(lines)
            .scroll((self.scroll, 0))
            .block(Block::default().borders(Borders::NONE));
        frame.render_widget(para, area);
    }

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
}

impl Default for HelpBuffer {
    fn default() -> Self {
        Self::new()
    }
}
