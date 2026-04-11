use std::collections::{HashMap, HashSet};

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use tuillem_core::actions::MessageView;

use crate::theme::Theme;

/// Scroll state machine
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollState {
    /// Following the bottom (new session load, short content)
    FollowBottom,
    /// Streaming: follow bottom until one viewport of response, then freeze
    Streaming { start_offset: u16 },
    /// Frozen: user reads at their own pace (Enter to advance)
    Frozen,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub scroll_offset: u16,
    pub expanded_thinking: HashSet<usize>,
    pub total_lines: u16,
    pub visible_height: u16,
    pub scroll_state: ScrollState,
    pub highlight_line: Option<u16>,
    pub highlight_set_at: Option<std::time::Instant>,
    /// Cache of rendered lines per message. Key is (message_id, thinking_expanded).
    /// Invalidated when content_width or layout changes.
    render_cache: HashMap<(String, bool), Vec<Line<'static>>>,
    cached_width: usize,
    cached_layout: String,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            expanded_thinking: HashSet::new(),
            total_lines: 0,
            visible_height: 0,
            scroll_state: ScrollState::FollowBottom,
            highlight_line: None,
            highlight_set_at: None,
            render_cache: HashMap::new(),
            cached_width: 0,
            cached_layout: String::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        messages: &[MessageView],
        streaming_text: &str,
        streaming_thinking: &str,
        is_streaming: bool,
        current_model: &str,
        error: Option<&str>,
        status_message: Option<&str>,
        focused: bool,
        theme: &Theme,
        layout: &str,
        _nerd_fonts: bool,
    ) {
        let is_loose = layout == "loose";
        let margin: usize = if is_loose { 2 } else { 0 };
        let margin_str: &str = if is_loose { "  " } else { "" };
        let content_width = area.width.saturating_sub(2).saturating_sub(margin as u16) as usize;
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Model indicator at top with focus hint
        let focus_hint = if focused {
            " [j/k:scroll  t:thinking  Tab:switch]"
        } else {
            ""
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} Model: {} ", margin_str, current_model),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                focus_hint.to_string(),
                Style::default().fg(theme.thinking_fg),
            ),
        ]));
        lines.push(Line::from(""));

        // Invalidate cache when width or layout changes
        if self.cached_width != content_width || self.cached_layout != layout {
            self.render_cache.clear();
            self.cached_width = content_width;
            self.cached_layout = layout.to_string();
        }

        // Render each message (with caching)
        for (idx, msg) in messages.iter().enumerate() {
            let thinking_expanded = self.expanded_thinking.contains(&idx);
            let cache_key = (msg.id.clone(), thinking_expanded);

            if let Some(cached) = self.render_cache.get(&cache_key) {
                lines.extend(cached.iter().cloned());
                continue;
            }

            let mut msg_lines: Vec<Line<'static>> = Vec::new();

            let is_user = msg.role == "user";

            // Role label
            let role_label = if is_user {
                "You".to_string()
            } else {
                let model = msg.model_id.as_deref().unwrap_or(current_model);
                format!("Assistant ({})", model)
            };

            let role_style = if is_user {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD)
            };

            if is_user {
                msg_lines.push(
                    Line::from(Span::styled(role_label, role_style)).alignment(Alignment::Right),
                );
            } else {
                msg_lines.push(Line::from(Span::styled(
                    format!("{}{}", margin_str, role_label),
                    role_style,
                )));
            }

            // Thinking blocks
            for block in &msg.blocks {
                if block.block_type == "thinking" {
                    if thinking_expanded {
                        let content = block.content.as_deref().unwrap_or("");
                        msg_lines.push(Line::from(Span::styled(
                            format!("{} [thinking] (press t to collapse)", margin_str),
                            theme.thinking_style(),
                        )));
                        for line in content.lines() {
                            msg_lines.push(Line::from(Span::styled(
                                format!("{}  {}", margin_str, line),
                                theme.thinking_style(),
                            )));
                        }
                    } else {
                        let preview = block
                            .content
                            .as_deref()
                            .unwrap_or("")
                            .chars()
                            .take(40)
                            .collect::<String>();
                        msg_lines.push(Line::from(Span::styled(
                            format!(
                                "{} [thinking] {}... (press t to expand)",
                                margin_str, preview
                            ),
                            theme.thinking_style(),
                        )));
                    }
                }
            }

            // Message content
            if let Some(ref content) = msg.content {
                if is_user {
                    // User messages: right-aligned with distinct background
                    let user_style = Style::default().fg(theme.fg).bg(theme.user_msg_bg);
                    if is_loose {
                        // Loose mode: bubble effect with bg-colored blank lines above/below
                        let mut wrapped_lines: Vec<String> = Vec::new();
                        for text_line in content.lines() {
                            if text_line.is_empty() {
                                wrapped_lines.push(String::new());
                            } else {
                                for wrapped in
                                    tuillem_markdown::width::wrap_to_width(text_line, content_width)
                                {
                                    wrapped_lines.push(wrapped);
                                }
                            }
                        }
                        let max_line_w = wrapped_lines
                            .iter()
                            .map(|l| tuillem_markdown::width::terminal_width(l))
                            .max()
                            .unwrap_or(0);
                        let text_style = Style::default().fg(theme.fg).bg(theme.user_msg_bg);
                        let bubble_w = max_line_w + 4; // 2 space padding each side
                        // Top edge: ▄ with fg=bubble draws lower-half block = curved top
                        let top_style = Style::default().fg(theme.user_msg_bg).bg(theme.bg);
                        // Bottom edge: ▀ with fg=bubble draws upper-half block = curved bottom
                        let bottom_style = Style::default().fg(theme.user_msg_bg).bg(theme.bg);

                        // Top edge
                        let top_corner = "▄";
                        msg_lines.push(
                            Line::from(vec![
                                Span::styled(top_corner.to_string(), top_style),
                                Span::styled("▄".repeat(bubble_w - 1), top_style),
                            ])
                            .alignment(Alignment::Right),
                        );

                        // Message content lines (solid background, no side chars)
                        for ml in &wrapped_lines {
                            let padded = format!("  {:width$}  ", ml, width = max_line_w);
                            msg_lines.push(
                                Line::from(Span::styled(padded, text_style))
                                    .alignment(Alignment::Right),
                            );
                        }

                        // Bottom edge
                        let bottom_corner = "▀";
                        msg_lines.push(
                            Line::from(vec![
                                Span::styled(bottom_corner.to_string(), bottom_style),
                                Span::styled("▀".repeat(bubble_w - 1), bottom_style),
                            ])
                            .alignment(Alignment::Right),
                        );
                    } else {
                        // Tight mode: original behavior
                        for text_line in content.lines() {
                            if text_line.is_empty() {
                                msg_lines.push(Line::from(""));
                            } else {
                                for wrapped in
                                    tuillem_markdown::width::wrap_to_width(text_line, content_width)
                                {
                                    msg_lines.push(
                                        Line::from(Span::styled(
                                            format!(" {} ", wrapped),
                                            user_style,
                                        ))
                                        .alignment(Alignment::Right),
                                    );
                                }
                            }
                        }
                    }
                } else {
                    // Assistant messages: left-aligned, rendered as markdown
                    let rendered = tuillem_markdown::render_markdown_width(content, content_width);
                    for line in rendered.lines {
                        // Skip wrapping for table/border lines — renderer handles those
                        let first_char = line.spans.first().map(|s| s.content.chars().next());
                        let is_table =
                            matches!(first_char, Some(Some('│' | '┌' | '├' | '└' | '─')));
                        if !is_table && content_width > 0 {
                            let line_w: usize = line
                                .spans
                                .iter()
                                .map(|s| tuillem_markdown::width::terminal_width(&s.content))
                                .sum();
                            if line_w > content_width {
                                let full_text: String =
                                    line.spans.iter().map(|s| s.content.to_string()).collect();
                                let style = if line.spans.is_empty() {
                                    Style::default()
                                } else {
                                    line.spans[0].style
                                };
                                for wrapped in tuillem_markdown::width::wrap_to_width(
                                    &full_text,
                                    content_width,
                                ) {
                                    msg_lines.push(Line::from(Span::styled(
                                        format!("{}{}", margin_str, wrapped),
                                        style,
                                    )));
                                }
                                continue;
                            }
                        }
                        if is_loose {
                            // Prepend margin to assistant lines
                            let mut new_spans = vec![Span::raw(margin_str.to_string())];
                            new_spans.extend(line.spans);
                            msg_lines.push(Line::from(new_spans));
                        } else {
                            msg_lines.push(line);
                        }
                    }
                }
            }

            // Separator between messages
            msg_lines.push(Line::from(""));
            if is_loose {
                msg_lines.push(Line::from(""));
            }

            // Store in cache and extend output
            self.render_cache.insert(cache_key, msg_lines.clone());
            lines.extend(msg_lines);
        }

        // Streaming content
        if is_streaming {
            // Always show a throbber when streaming
            let throbber_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let tick = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                / 100) as usize;
            let throbber = throbber_chars[tick % throbber_chars.len()];

            if !streaming_thinking.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    format!("{} {} Thinking... ", margin_str, throbber),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )]));
            }

            if !streaming_text.is_empty() {
                // Render and wrap streaming text (handles incomplete tables/code blocks)
                let rendered =
                    tuillem_markdown::render_markdown_streaming(streaming_text, content_width);
                for line in rendered.lines {
                    let first_char = line.spans.first().map(|s| s.content.chars().next());
                    let is_table = matches!(first_char, Some(Some('│' | '┌' | '├' | '└' | '─')));
                    if !is_table && content_width > 0 {
                        let line_w: usize = line
                            .spans
                            .iter()
                            .map(|s| tuillem_markdown::width::terminal_width(&s.content))
                            .sum();
                        if line_w > content_width {
                            let full_text: String =
                                line.spans.iter().map(|s| s.content.to_string()).collect();
                            let style = if line.spans.is_empty() {
                                Style::default()
                            } else {
                                line.spans[0].style
                            };
                            for wrapped in
                                tuillem_markdown::width::wrap_to_width(&full_text, content_width)
                            {
                                lines.push(Line::from(Span::styled(
                                    format!("{}{}", margin_str, wrapped),
                                    style,
                                )));
                            }
                            continue;
                        }
                    }
                    if is_loose {
                        let mut new_spans = vec![Span::raw(margin_str.to_string())];
                        new_spans.extend(line.spans);
                        lines.push(Line::from(new_spans));
                    } else {
                        lines.push(line);
                    }
                }
            }

            if streaming_text.is_empty() && streaming_thinking.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    format!("{} {} Waiting for response...", margin_str, throbber),
                    Style::default()
                        .fg(theme.thinking_fg)
                        .add_modifier(Modifier::ITALIC),
                )]));
            }

            // Streaming indicator when content is below viewport
            let total_so_far = lines.len() as u16;
            if total_so_far > area.height.saturating_add(self.scroll_offset) {
                lines.push(Line::from(Span::styled(
                    format!("{}streaming...", margin_str),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::ITALIC),
                )));
            }
        }

        // Error display
        if let Some(err) = error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{} Error: {}", margin_str, err),
                theme.error_style().add_modifier(Modifier::BOLD),
            )));
        }

        // Status message (non-error feedback)
        if let Some(msg) = status_message {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{} {}", margin_str, msg),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::ITALIC),
            )));
        }

        self.total_lines = lines.len() as u16;
        self.visible_height = area.height;

        // Auto-expire highlight after 2 seconds
        if let Some(set_at) = self.highlight_set_at
            && set_at.elapsed() > std::time::Duration::from_secs(2)
        {
            self.highlight_line = None;
            self.highlight_set_at = None;
        }

        // Scroll state machine
        let max_offset = self.total_lines.saturating_sub(self.visible_height);
        match self.scroll_state {
            ScrollState::FollowBottom => {
                self.scroll_offset = max_offset;
            }
            ScrollState::Streaming { start_offset } => {
                // Follow the bottom until response fills one visible page,
                // then freeze so user sees the top of the response.
                let content_since_start = max_offset.saturating_sub(start_offset);
                if content_since_start <= self.visible_height {
                    // Still filling first page — follow bottom
                    self.scroll_offset = max_offset;
                } else {
                    // First page filled — freeze at the start of the response
                    self.scroll_offset = start_offset;
                    self.scroll_state = ScrollState::Frozen;
                }
            }
            ScrollState::Frozen => {
                // Don't touch scroll_offset — user controls it
                self.scroll_offset = self.scroll_offset.min(max_offset);
            }
        }

        // Apply highlight to the target line (full width)
        if let Some(hl) = self.highlight_line {
            let hl_idx = hl as usize;
            if hl_idx < lines.len() {
                let highlight_bg = theme.user_msg_bg;
                let content_w = area.width as usize;
                let line = &mut lines[hl_idx];
                // Calculate current visible text width
                let text_w: usize = line
                    .spans
                    .iter()
                    .map(|s| tuillem_markdown::width::terminal_width(&s.content))
                    .sum();
                let pad = content_w.saturating_sub(text_w);
                let mut new_spans: Vec<Span<'static>> = line
                    .spans
                    .iter()
                    .map(|span| Span::styled(span.content.to_string(), span.style.bg(highlight_bg)))
                    .collect();
                if pad > 0 {
                    new_spans.push(Span::styled(
                        " ".repeat(pad),
                        Style::default().bg(highlight_bg),
                    ));
                }
                *line = Line::from(new_spans);
            }
        }

        let text = Text::from(lines);

        // Reserve 2 columns on the right for the scrollbar so right-aligned
        // text doesn't render under it
        let has_scrollbar = self.total_lines > self.visible_height;
        let paragraph_area = if has_scrollbar {
            Rect::new(area.x, area.y, area.width.saturating_sub(2), area.height)
        } else {
            area
        };

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg).bg(theme.bg))
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, paragraph_area);

        // Scrollbar on the right edge when content exceeds viewport
        if has_scrollbar {
            let max_scroll = self.total_lines.saturating_sub(self.visible_height) as usize;
            let mut scrollbar_state =
                ScrollbarState::new(max_scroll).position(self.scroll_offset as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .track_style(Style::default().fg(theme.border))
                .thumb_style(Style::default().fg(theme.accent));
            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }

        // "More content" indicator at bottom-right when not at the end
        let max_offset = self.total_lines.saturating_sub(self.visible_height);
        if self.scroll_offset < max_offset && self.total_lines > self.visible_height {
            let indicator = " ... ";
            let x = area.x + area.width.saturating_sub(indicator.len() as u16 + 1);
            let y = area.y + area.height.saturating_sub(1);
            if y >= area.y && x >= area.x {
                let indicator_area = Rect::new(x, y, indicator.len() as u16, 1);
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        indicator,
                        Style::default().fg(theme.thinking_fg).bg(theme.bg),
                    )),
                    indicator_area,
                );
            }
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        // Any manual scroll freezes — break out of FollowBottom/Streaming
        self.scroll_state = ScrollState::Frozen;
    }

    pub fn scroll_down(&mut self, amount: u16) {
        let max_offset = self.total_lines.saturating_sub(self.visible_height);
        self.scroll_offset = self.scroll_offset.saturating_add(amount).min(max_offset);
        // Any manual scroll freezes
        self.scroll_state = ScrollState::Frozen;
    }

    pub fn scroll_to_bottom(&mut self) {
        let max_offset = self.total_lines.saturating_sub(self.visible_height);
        self.scroll_offset = max_offset;
        self.scroll_state = ScrollState::FollowBottom;
    }

    /// Clear the render cache entirely (e.g. on session switch).
    pub fn clear_render_cache(&mut self) {
        self.render_cache.clear();
    }

    /// Remove cache entries for messages no longer in the list.
    /// Keeps existing valid entries for performance.
    pub fn prune_render_cache(&mut self, messages: &[tuillem_core::actions::MessageView]) {
        let valid_ids: HashSet<&str> = messages.iter().map(|m| m.id.as_str()).collect();
        self.render_cache
            .retain(|(id, _), _| valid_ids.contains(id.as_str()));
    }

    pub fn toggle_thinking(&mut self, message_index: usize) {
        if self.expanded_thinking.contains(&message_index) {
            self.expanded_thinking.remove(&message_index);
        } else {
            self.expanded_thinking.insert(message_index);
        }
    }
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}
