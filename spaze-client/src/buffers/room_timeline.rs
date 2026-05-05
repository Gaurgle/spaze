//! `RoomTimelineBuffer` — chat timeline for a single Room (or DM).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use spaze_proto::{Message, RoomId};

use crate::buffers::BufferKind;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    Standard,
    Direct,
}

#[derive(Debug, Clone)]
pub struct RepoBinding {
    pub repo_name: String,
    pub subpath: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    pub offset_from_bottom: u16,
    pub stuck_to_bottom: bool,
}

impl ScrollState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            offset_from_bottom: 0,
            stuck_to_bottom: true,
        }
    }

    pub fn page_up(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_add(10);
        self.stuck_to_bottom = false;
    }

    pub fn page_down(&mut self) {
        self.offset_from_bottom = self.offset_from_bottom.saturating_sub(10);
        if self.offset_from_bottom == 0 {
            self.stuck_to_bottom = true;
        }
    }

    pub fn to_bottom(&mut self) {
        self.offset_from_bottom = 0;
        self.stuck_to_bottom = true;
    }

    pub fn to_top(&mut self) {
        self.offset_from_bottom = u16::MAX;
        self.stuck_to_bottom = false;
    }

    pub fn stick_to_bottom_if_was(&mut self) {
        if self.stuck_to_bottom {
            self.offset_from_bottom = 0;
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RoomTimelineBuffer {
    pub room_id: RoomId,
    pub display_name: String,
    pub kind: RoomKind,
    pub repo_binding: Option<RepoBinding>,
    pub topic: Option<String>,
    pub messages: Vec<Message>,
    pub scroll: ScrollState,
    /// Used for mention detection in render. Set from Identity at construction.
    pub self_display_name: String,
}

impl RoomTimelineBuffer {
    #[must_use]
    pub fn new(
        room_id: RoomId,
        display_name: String,
        kind: RoomKind,
        self_display_name: String,
    ) -> Self {
        Self {
            room_id,
            display_name,
            kind,
            repo_binding: None,
            topic: None,
            messages: Vec::new(),
            scroll: ScrollState::new(),
            self_display_name,
        }
    }

    pub fn push_message(&mut self, msg: Message) {
        self.messages.push(msg);
        self.scroll.stick_to_bottom_if_was();
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn kind(&self) -> BufferKind {
        match self.kind {
            RoomKind::Standard => BufferKind::Room,
            RoomKind::Direct => BufferKind::DirectMessage,
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        use chrono::TimeZone;
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        use spaze_proto::MessageBody;

        let mut lines: Vec<Line> = Vec::with_capacity(self.messages.len());

        for msg in &self.messages {
            let ts = chrono::Utc
                .timestamp_millis_opt(msg.created_at_ms)
                .single()
                .map_or_else(|| "??:??".into(), |dt| dt.format("%H:%M").to_string());

            let author_display = if msg.author_display_name.is_empty() {
                // Fallback to 8-char hex prefix (same shape as 1.A's stdout output).
                format!("{:.8}", msg.author_id.as_uuid().simple())
            } else {
                msg.author_display_name.clone()
            };

            let mention = !self.self_display_name.is_empty()
                && body_text(&msg.body)
                    .to_lowercase()
                    .contains(&format!("@{}", self.self_display_name).to_lowercase());

            // When a line is a mention, swap to highlighter mode: pink bg with
            // dark text so the line is eye-catching AND readable (the previous
            // light-text-on-light-pink failed contrast).
            let (line_style, ts_fg, author_fg, body_fg, dim_fg) = if mention {
                (
                    Style::default().bg(theme.mention),
                    theme.background,
                    theme.background,
                    theme.background,
                    theme.surface,
                )
            } else {
                (
                    Style::default(),
                    theme.muted,
                    theme.info,
                    theme.foreground,
                    theme.subtle,
                )
            };

            let mut spans = vec![
                Span::styled(format!("{ts} "), Style::default().fg(ts_fg)),
                Span::styled(
                    format!("<{author_display}> "),
                    Style::default().fg(author_fg).add_modifier(Modifier::BOLD),
                ),
            ];
            match &msg.body {
                MessageBody::Text { content } => {
                    spans.push(Span::styled(content.clone(), Style::default().fg(body_fg)));
                }
                MessageBody::System { content } => {
                    spans.push(Span::styled(
                        format!("-- system: {content}"),
                        Style::default().fg(dim_fg).add_modifier(Modifier::ITALIC),
                    ));
                }
                _ => {
                    // MessageBody is non_exhaustive; future variants render as raw debug.
                    spans.push(Span::styled(
                        format!("{:?}", msg.body),
                        Style::default().fg(dim_fg),
                    ));
                }
            }
            if msg.deleted_at_ms.is_some() {
                spans.push(Span::styled(
                    "  [deleted]".to_string(),
                    Style::default().fg(dim_fg).add_modifier(Modifier::ITALIC),
                ));
            }
            if msg.edited_at_ms.is_some() {
                spans.push(Span::styled(
                    "  (edited)".to_string(),
                    Style::default().fg(ts_fg),
                ));
            }
            lines.push(Line::from(spans).style(line_style));
        }

        let para = ratatui::widgets::Paragraph::new(lines)
            .scroll((self.scroll.offset_from_bottom, 0))
            .style(Style::default().fg(theme.foreground).bg(theme.background));
        frame.render_widget(para, area);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
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
            _ => false,
        }
    }
}

fn body_text(body: &spaze_proto::MessageBody) -> &str {
    match body {
        spaze_proto::MessageBody::Text { content }
        | spaze_proto::MessageBody::System { content } => content,
        _ => "",
    }
}
