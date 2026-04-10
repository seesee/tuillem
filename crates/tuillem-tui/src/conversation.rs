use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
};
use tuillem_core::actions::MessageView;

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct ReadingMode {
    pub active: bool,
    pub paused: bool,
    pub last_scroll: std::time::Instant,
    pub lines_per_second: f64,
    pub target_offset: u16,
    pub wpm: u16,
    pub content_width: u16,
}

impl ReadingMode {
    /// Update WPM and recalculate scroll speed.
    pub fn update_wpm(&mut self, wpm: u16) {
        self.wpm = wpm;
        let words_per_line = (self.content_width as f64) / 5.0;
        self.lines_per_second = if words_per_line > 0.0 {
            (wpm as f64) / 60.0 / words_per_line
        } else {
            0.5
        };
    }
}

impl Default for ReadingMode {
    fn default() -> Self {
        Self {
            active: false,
            paused: false,
            last_scroll: std::time::Instant::now(),
            lines_per_second: 0.0,
            target_offset: 0,
            wpm: 250,
            content_width: 80,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub scroll_offset: u16,
    pub expanded_thinking: HashSet<usize>,
    pub total_lines: u16,
    pub visible_height: u16,
    pub auto_scroll: bool,
    pub reading_mode: ReadingMode,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            expanded_thinking: HashSet::new(),
            total_lines: 0,
            visible_height: 0,
            auto_scroll: true,
            reading_mode: ReadingMode::default(),
        }
    }

    pub fn start_reading(&mut self, wpm: u16, content_width: u16) {
        let words_per_line = (content_width as f64) / 5.0;
        let lines_per_second = if words_per_line > 0.0 {
            (wpm as f64) / 60.0 / words_per_line
        } else {
            0.5
        };
        let target = self.total_lines.saturating_sub(self.visible_height);
        if self.scroll_offset < target {
            self.reading_mode = ReadingMode {
                active: true,
                paused: false,
                last_scroll: std::time::Instant::now(),
                lines_per_second,
                target_offset: target,
                wpm,
                content_width,
            };
        }
    }

    pub fn stop_reading(&mut self) {
        self.reading_mode.active = false;
        self.reading_mode.paused = false;
    }

    pub fn toggle_pause(&mut self) {
        if self.reading_mode.active {
            self.reading_mode.paused = !self.reading_mode.paused;
            if !self.reading_mode.paused {
                // Reset the clock so we don't jump ahead
                self.reading_mode.last_scroll = std::time::Instant::now();
            }
        }
    }

    pub fn nudge_forward(&mut self, lines: u16) {
        if self.reading_mode.active {
            self.scroll_offset = self
                .scroll_offset
                .saturating_add(lines)
                .min(self.reading_mode.target_offset);
            self.reading_mode.last_scroll = std::time::Instant::now();
            if self.scroll_offset >= self.reading_mode.target_offset {
                self.reading_mode.active = false;
            }
        }
    }

    pub fn nudge_backward(&mut self, lines: u16) {
        if self.reading_mode.active {
            self.scroll_offset = self.scroll_offset.saturating_sub(lines);
            self.reading_mode.last_scroll = std::time::Instant::now();
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

        // Render each message
        for (idx, msg) in messages.iter().enumerate() {
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
                lines.push(
                    Line::from(Span::styled(role_label, role_style)).alignment(Alignment::Right),
                );
            } else {
                lines.push(Line::from(Span::styled(
                    format!("{}{}", margin_str, role_label),
                    role_style,
                )));
            }

            // Thinking blocks
            for block in &msg.blocks {
                if block.block_type == "thinking" {
                    let is_expanded = self.expanded_thinking.contains(&idx);
                    if is_expanded {
                        let content = block.content.as_deref().unwrap_or("");
                        lines.push(Line::from(Span::styled(
                            format!("{} [thinking] (press t to collapse)", margin_str),
                            theme.thinking_style(),
                        )));
                        for line in content.lines() {
                            lines.push(Line::from(Span::styled(
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
                        lines.push(Line::from(Span::styled(
                            format!("{} [thinking] {}... (press t to expand)", margin_str, preview),
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
                        // First, collect all wrapped lines to find max width for uniform bubble
                        let mut msg_lines: Vec<String> = Vec::new();
                        for text_line in content.lines() {
                            if text_line.is_empty() {
                                msg_lines.push(String::new());
                            } else {
                                for wrapped in
                                    tuillem_markdown::width::wrap_to_width(text_line, content_width)
                                {
                                    msg_lines.push(wrapped);
                                }
                            }
                        }
                        let max_line_w = msg_lines
                            .iter()
                            .map(|l| tuillem_markdown::width::terminal_width(l))
                            .max()
                            .unwrap_or(0);
                        let bubble_w = max_line_w + 2; // 1 space padding each side

                        // Top blank line (bg-colored)
                        let blank = format!("{:width$}", "", width = bubble_w);
                        lines.push(
                            Line::from(Span::styled(blank.clone(), user_style))
                                .alignment(Alignment::Right),
                        );
                        // Message lines, padded to uniform width
                        for ml in &msg_lines {
                            let padded = format!(" {:width$} ", ml, width = max_line_w);
                            lines.push(
                                Line::from(Span::styled(padded, user_style))
                                    .alignment(Alignment::Right),
                            );
                        }
                        // Bottom blank line (bg-colored)
                        lines.push(
                            Line::from(Span::styled(blank, user_style))
                                .alignment(Alignment::Right),
                        );
                    } else {
                        // Tight mode: original behavior
                        for text_line in content.lines() {
                            if text_line.is_empty() {
                                lines.push(Line::from(""));
                            } else {
                                for wrapped in
                                    tuillem_markdown::width::wrap_to_width(text_line, content_width)
                                {
                                    lines.push(
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
                                    lines.push(Line::from(Span::styled(
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
                            lines.push(Line::from(new_spans));
                        } else {
                            lines.push(line);
                        }
                    }
                }
            }

            // Separator between messages
            lines.push(Line::from(""));
            if is_loose {
                lines.push(Line::from(""));
            }
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

        // Reading mode auto-scroll advancement
        if self.reading_mode.active && !self.reading_mode.paused {
            let elapsed = self.reading_mode.last_scroll.elapsed().as_secs_f64();
            let lines_to_advance = elapsed * self.reading_mode.lines_per_second;
            if lines_to_advance >= 1.0 {
                self.scroll_offset = self
                    .scroll_offset
                    .saturating_add(lines_to_advance as u16)
                    .min(self.reading_mode.target_offset);
                self.reading_mode.last_scroll = std::time::Instant::now();
                if self.scroll_offset >= self.reading_mode.target_offset {
                    self.reading_mode.active = false;
                }
            }
        } else if self.auto_scroll {
            // Auto-scroll to bottom when new content arrives (non-reading mode)
            self.scroll_offset = self.total_lines.saturating_sub(self.visible_height);
        }

        // Reading mode indicator bar (replaces last visible line)
        if self.reading_mode.active {
            let indicator = if self.reading_mode.paused {
                format!(
                    "{}\u{23f8} Paused {}wpm [\u{2190}\u{2192}:speed Enter:resume G:end]",
                    margin_str,
                    self.reading_mode.wpm
                )
            } else {
                format!(
                    "{}\u{25b6} Reading {}wpm [Space:nudge \u{2190}\u{2192}:speed Enter:pause G:end]",
                    margin_str,
                    self.reading_mode.wpm
                )
            };
            let indicator_style = if self.reading_mode.paused {
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            };
            lines.push(Line::from(Span::styled(indicator, indicator_style)));
            // Update total_lines to include the indicator
            self.total_lines = lines.len() as u16;
        }

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg).bg(theme.bg))
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
        // Re-enable auto-scroll if we're at or near the bottom
        if self.scroll_offset >= self.total_lines.saturating_sub(self.visible_height) {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
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
