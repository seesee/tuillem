use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tuillem_core::actions::SessionSummary;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct Sidebar {
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_input: String,
    pub search_focused: bool,
    /// Session IDs that matched an FTS content search (None = no search active)
    pub content_match_ids: Option<std::collections::HashSet<String>>,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            search_input: String::new(),
            search_focused: false,
            content_match_ids: None,
        }
    }

    /// Filter sessions by search query.
    /// Matches on title, tags (client-side), AND conversation content (via FTS results).
    pub fn filtered_sessions<'a>(&self, sessions: &'a [SessionSummary]) -> Vec<&'a SessionSummary> {
        if self.search_input.is_empty() {
            sessions.iter().collect()
        } else {
            let query = self.search_input.to_lowercase();
            sessions
                .iter()
                .filter(|s| {
                    // Title or tag match (client-side)
                    let title_match = s.title.to_lowercase().contains(&query)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&query));
                    // Content match (from FTS results)
                    let content_match = self
                        .content_match_ids
                        .as_ref()
                        .is_some_and(|ids| ids.contains(&s.id));
                    title_match || content_match
                })
                .collect()
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        sessions: &[SessionSummary],
        focused: bool,
        theme: &Theme,
    ) {
        let border_style = if focused {
            Style::default().fg(theme.accent)
        } else {
            theme.border_style()
        };
        let title = if focused {
            Line::from(Span::styled(
                " Sessions [Tab] ",
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from(Span::styled(
                " Sessions ",
                Style::default().fg(theme.thinking_fg),
            ))
        };
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(border_style)
            .title_top(title)
            .style(theme.sidebar_style());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 2 {
            return;
        }

        // Search box at top
        let search_text = if self.search_input.is_empty() && !self.search_focused {
            Span::styled("/ search...", Style::default().fg(theme.thinking_fg))
        } else {
            Span::styled(
                format!("/ {}", self.search_input),
                Style::default().fg(theme.accent),
            )
        };
        let search_line = Paragraph::new(Line::from(search_text));
        let search_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        };
        frame.render_widget(search_line, search_area);

        // Session list below search
        let list_area = Rect {
            x: inner.x,
            y: inner.y + 1,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };

        let filtered = self.filtered_sessions(sessions);

        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(list_area.height as usize / 2) // 2 lines per item
            .map(|(i, session)| {
                let is_selected = i == self.selected;
                let style = if is_selected {
                    theme.sidebar_selected_style().add_modifier(Modifier::BOLD)
                } else {
                    theme.sidebar_style()
                };

                let mut title_spans: Vec<Span> = vec![Span::styled(&session.title, style)];
                for tag in &session.tags {
                    title_spans.push(Span::raw(" "));
                    title_spans.push(Span::styled(
                        format!("[{}]", tag),
                        Style::default().fg(theme.tag),
                    ));
                }

                let preview_text = session
                    .preview
                    .as_deref()
                    .unwrap_or("")
                    .replace('\n', " ");
                let max_w = inner.width.saturating_sub(4) as usize;
                let preview_truncated = tuillem_markdown::width::truncate_with_ellipsis(&preview_text, max_w);

                let preview_line = Line::from(Span::styled(
                    format!(" {}", preview_truncated),
                    Style::default().fg(theme.thinking_fg),
                ));

                ListItem::new(vec![Line::from(title_spans), preview_line])
                    .style(if is_selected {
                        Style::default().bg(theme.sidebar_bg)
                    } else {
                        Style::default()
                    })
            })
            .collect();

        let list = List::new(items);
        frame.render_widget(list, list_area);
    }

    pub fn move_up(&mut self, count: usize) {
        self.selected = self.selected.saturating_sub(count);
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
    }

    pub fn move_down(&mut self, session_count: usize, count: usize) {
        if session_count == 0 {
            return;
        }
        self.selected = (self.selected + count).min(session_count - 1);
        // Scrolling will be handled during render based on visible height,
        // but we do a basic adjustment here.
        if self.selected >= self.scroll_offset + 20 {
            self.scroll_offset = self.selected.saturating_sub(19);
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
