use chrono::{DateTime, Local, NaiveDate, Utc};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use tuillem_core::actions::SessionSummary;

use crate::theme::Theme;

/// Split `text` into spans, highlighting case-insensitive matches of `query`.
fn highlight_matches<'a>(
    text: &str,
    query: &str,
    normal_style: Style,
    highlight_style: Style,
) -> Vec<Span<'a>> {
    if query.is_empty() {
        return vec![Span::styled(text.to_string(), normal_style)];
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut spans = Vec::new();
    let mut last = 0;
    for (start, _) in lower_text.match_indices(&lower_query) {
        if start < last {
            // Overlapping match — skip
            continue;
        }
        if start > last {
            spans.push(Span::styled(text[last..start].to_string(), normal_style));
        }
        spans.push(Span::styled(
            text[start..start + lower_query.len()].to_string(),
            highlight_style,
        ));
        last = start + lower_query.len();
    }
    if last < text.len() {
        spans.push(Span::styled(text[last..].to_string(), normal_style));
    }
    spans
}

#[derive(Debug, Clone)]
pub struct Sidebar {
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_input: String,
    pub search_focused: bool,
    /// Session IDs that matched an FTS content search (None = no search active)
    pub content_match_ids: Option<std::collections::HashSet<String>>,
    /// Number of session items visible in the last render (used for scroll calculations)
    visible_count: usize,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            search_input: String::new(),
            search_focused: false,
            content_match_ids: None,
            visible_count: 0,
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
                    let title_match = s.title.to_lowercase().contains(&query)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&query));
                    let content_match = self
                        .content_match_ids
                        .as_ref()
                        .is_some_and(|ids| ids.contains(&s.id));
                    title_match || content_match
                })
                .collect()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        sessions: &[SessionSummary],
        focused: bool,
        theme: &Theme,
        layout: &str,
        date_format: &str,
        confirm_delete: Option<&str>,   // session_id pending delete
        renaming: Option<(&str, &str)>, // (session_id, edit_buffer)
    ) {
        let border_style = if focused {
            Style::default().fg(theme.accent)
        } else {
            theme.border_style()
        };
        let title = if focused {
            Line::from(Span::styled(
                " Sessions [Tab] ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
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

        if inner.height < 3 {
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

        // Blank line after search (both tight and loose)
        let list_area = Rect {
            x: inner.x,
            y: inner.y + 2, // +1 search, +1 blank
            width: inner.width,
            height: inner.height.saturating_sub(2),
        };

        let filtered = self.filtered_sessions(sessions);
        let is_loose = layout == "loose";
        let today = Local::now().date_naive();

        // Build list items with date group headers
        let mut items: Vec<ListItem> = Vec::new();
        let mut current_group: Option<String> = None;
        let mut item_index = 0;
        let mut rename_line_y: Option<u16> = None; // track Y position of renaming item
        let mut current_line: u16 = 0;
        let header_style = Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::ITALIC);

        for session in filtered.iter().skip(self.scroll_offset) {
            let group = date_group_label(&session.updated_at, today, date_format);
            if current_group.as_ref() != Some(&group) {
                // Add group header
                if current_group.is_some() && is_loose {
                    items.push(ListItem::new(Line::from("")));
                    current_line += 1;
                }
                items.push(ListItem::new(Line::from(Span::styled(
                    group.clone(),
                    header_style,
                ))));
                current_line += 1;
                current_group = Some(group);
            }

            let is_selected = item_index + self.scroll_offset == self.selected;
            let style = if is_selected {
                theme.sidebar_selected_style().add_modifier(Modifier::BOLD)
            } else {
                theme.sidebar_style()
            };

            // Check if this session has a pending action
            let is_confirming_delete = confirm_delete == Some(session.id.as_str());
            let is_renaming = renaming.is_some_and(|(id, _)| id == session.id);

            // Track the Y position of the renaming item
            if is_renaming {
                rename_line_y = Some(current_line);
            }

            let (title_line, preview_line) = if is_confirming_delete {
                // Delete confirmation
                (
                    Line::from(vec![
                        Span::styled(
                            "Delete? ",
                            Style::default()
                                .fg(theme.error)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled("y/n", Style::default().fg(theme.warning)),
                    ]),
                    Line::from(Span::styled(
                        format!(" {}", session.title),
                        Style::default().fg(theme.thinking_fg),
                    )),
                )
            } else if let Some((_, buf)) = renaming.filter(|_| is_renaming) {
                (
                    Line::from(vec![
                        Span::styled("Rename: ", Style::default().fg(theme.accent)),
                        Span::styled(
                            buf.to_string(),
                            Style::default()
                                .fg(theme.fg)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                        Span::styled("_", Style::default().fg(theme.accent)),
                    ]),
                    Line::from(Span::styled(
                        " Enter:save  Esc:cancel",
                        Style::default().fg(theme.thinking_fg),
                    )),
                )
            } else {
                let search_q = &self.search_input;
                let hl_style = Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD);

                let mut title_spans: Vec<Span> =
                    highlight_matches(&session.title, search_q, style, hl_style);
                for tag in &session.tags {
                    title_spans.push(Span::raw(" "));
                    title_spans.push(Span::styled(
                        format!("[{}]", tag),
                        Style::default().fg(theme.tag),
                    ));
                }

                let preview_text = session.preview.as_deref().unwrap_or("").replace('\n', " ");
                let max_w = inner.width.saturating_sub(4) as usize;
                let preview_truncated =
                    tuillem_markdown::width::truncate_with_ellipsis(&preview_text, max_w);

                let preview_style = Style::default().fg(theme.thinking_fg);
                let preview_spans =
                    highlight_matches(&format!(" {}", preview_truncated), search_q, preview_style, hl_style);

                (
                    Line::from(title_spans),
                    Line::from(preview_spans),
                )
            };

            let mut item_lines = vec![title_line, preview_line];
            if is_loose {
                item_lines.push(Line::from(""));
            }

            let item_height = if is_loose { 3u16 } else { 2u16 };
            items.push(ListItem::new(item_lines).style(if is_selected {
                Style::default().bg(theme.sidebar_selected_bg)
            } else {
                Style::default()
            }));

            current_line += item_height;
            item_index += 1;

            // Stop if we've filled the visible area (rough estimate)
            let total_lines: usize = items.iter().map(|i| i.height()).sum();
            if total_lines >= list_area.height as usize {
                break;
            }
        }

        // Track how many session items fit on screen for scroll calculations
        self.visible_count = item_index;

        let list = List::new(items);
        frame.render_widget(list, list_area);

        // Show terminal cursor when renaming
        if let Some((_, buf)) = renaming
            && let Some(ry) = rename_line_y
        {
            let prefix_text = "Rename: ";
            let cursor_x = list_area.x + prefix_text.len() as u16 + buf.chars().count() as u16;
            let cursor_y = list_area.y + ry;
            if cursor_x < list_area.x + list_area.width && cursor_y < list_area.y + list_area.height
            {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }

        // Render scrollbar if there are more sessions than visible
        let total_sessions = filtered.len();
        if total_sessions > self.visible_count {
            let mut scrollbar_state =
                ScrollbarState::new(total_sessions.saturating_sub(self.visible_count))
                    .position(self.scroll_offset);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .style(Style::default().fg(theme.thinking_fg));
            frame.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
        }
    }

    pub fn move_up(&mut self, count: usize) {
        self.selected = self.selected.saturating_sub(count);
        // Keep selection visible
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
    }

    pub fn move_down(&mut self, session_count: usize, count: usize) {
        if session_count > 0 {
            self.selected = (self.selected + count).min(session_count - 1);
            // Keep selection visible — use visible_count from last render
            let visible = if self.visible_count > 0 {
                self.visible_count
            } else {
                10
            };
            if self.selected >= self.scroll_offset + visible {
                self.scroll_offset = self.selected + 1 - visible;
            }
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine the date group label for a session based on its updated_at timestamp.
/// Recent dates use friendly labels; older dates use the configured format.
fn date_group_label(updated_at: &str, today: NaiveDate, date_format: &str) -> String {
    let date = DateTime::parse_from_rfc3339(updated_at)
        .map(|dt| dt.with_timezone(&Local).date_naive())
        .or_else(|_| {
            updated_at
                .parse::<DateTime<Utc>>()
                .map(|dt| dt.with_timezone(&Local).date_naive())
        })
        .unwrap_or(today);

    let days_ago = (today - date).num_days();

    match days_ago {
        0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        2..=6 => date.format("%A").to_string(), // e.g. "Monday"
        7..=13 => "Last Week".to_string(),
        14..=29 => "This Month".to_string(),
        _ => {
            // Use configured date format for older entries
            let chrono_fmt = match date_format {
                "yyyy-mm-dd" => "%Y-%m-%d",
                "mm/dd/yyyy" => "%m/%d/%Y",
                "dd.mm.yyyy" => "%d.%m.%Y",
                "dd/mm/yyyy" => "%d/%m/%Y",
                _ => "%d/%m/%Y",
            };
            date.format(chrono_fmt).to_string()
        }
    }
}
