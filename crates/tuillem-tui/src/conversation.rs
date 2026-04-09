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
pub struct Conversation {
    pub scroll_offset: u16,
    pub expanded_thinking: HashSet<usize>,
    pub total_lines: u16,
    pub visible_height: u16,
    pub auto_scroll: bool,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            expanded_thinking: HashSet::new(),
            total_lines: 0,
            visible_height: 0,
            auto_scroll: true,
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
        focused: bool,
        theme: &Theme,
    ) {
        let content_width = area.width.saturating_sub(2) as usize;
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Model indicator at top with focus hint
        let focus_hint = if focused { " [j/k:scroll  t:thinking  Tab:switch]" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(
                format!(" Model: {} ", current_model),
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
                lines.push(Line::from(Span::styled(role_label, role_style)).alignment(Alignment::Right));
            } else {
                lines.push(Line::from(Span::styled(role_label, role_style)));
            }

            // Thinking blocks
            for block in &msg.blocks {
                if block.block_type == "thinking" {
                    let is_expanded = self.expanded_thinking.contains(&idx);
                    if is_expanded {
                        let content = block.content.as_deref().unwrap_or("");
                        lines.push(Line::from(Span::styled(
                            " [thinking] (press t to collapse)",
                            theme.thinking_style(),
                        )));
                        for line in content.lines() {
                            lines.push(Line::from(Span::styled(
                                format!("  {}", line),
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
                            format!(" [thinking] {}... (press t to expand)", preview),
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
                    for text_line in content.lines() {
                        if text_line.is_empty() {
                            lines.push(Line::from(""));
                        } else {
                            for wrapped in wrap_text(text_line, content_width) {
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
                } else {
                    // Assistant messages: left-aligned, rendered as markdown
                    // The markdown renderer handles table widths and wrapping internally
                    let rendered = tuillem_markdown::render_markdown_width(content, content_width);
                    for line in rendered.lines {
                        lines.push(line);
                    }
                }
            }

            // Token usage
            if msg.token_usage_in.is_some() || msg.token_usage_out.is_some() {
                let usage_in = msg.token_usage_in.unwrap_or(0);
                let usage_out = msg.token_usage_out.unwrap_or(0);
                lines.push(Line::from(Span::styled(
                    format!(" [tokens: {} in / {} out]", usage_in, usage_out),
                    theme.thinking_style(),
                )));
            }

            lines.push(Line::from(""));
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
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} Thinking... ", throbber),
                        Style::default()
                            .fg(theme.warning)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }

            if !streaming_text.is_empty() {
                // Render and wrap streaming text (handles incomplete tables/code blocks)
                let rendered = tuillem_markdown::render_markdown_streaming(streaming_text, content_width);
                for line in rendered.lines {
                    lines.push(line);
                }
            }

            if streaming_text.is_empty() && streaming_thinking.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} Waiting for response...", throbber),
                        Style::default()
                            .fg(theme.thinking_fg)
                            .add_modifier(Modifier::ITALIC),
                    ),
                ]));
            }
        }

        // Error display
        if let Some(err) = error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" Error: {}", err),
                theme.error_style().add_modifier(Modifier::BOLD),
            )));
        }

        self.total_lines = lines.len() as u16;
        self.visible_height = area.height;

        // Auto-scroll to bottom when new content arrives
        if self.auto_scroll {
            self.scroll_offset = self.total_lines.saturating_sub(self.visible_height);
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

/// Word-wrap text to fit within `max_width` characters.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in text.split_whitespace() {
        let word_len = word.len();
        if current_len == 0 {
            current_line = word.to_string();
            current_len = word_len;
        } else if current_len + 1 + word_len <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
            current_len += 1 + word_len;
        } else {
            result.push(current_line);
            current_line = word.to_string();
            current_len = word_len;
        }
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}
