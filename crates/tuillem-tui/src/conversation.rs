use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::Rect,
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
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            expanded_thinking: HashSet::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        messages: &[MessageView],
        streaming_text: &str,
        streaming_thinking: &str,
        is_streaming: bool,
        current_model: &str,
        error: Option<&str>,
        theme: &Theme,
    ) {
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Model indicator at top
        lines.push(Line::from(Span::styled(
            format!(" Model: {} ", current_model),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
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
                theme.user_message_style().add_modifier(Modifier::BOLD)
            } else {
                theme.assistant_message_style().add_modifier(Modifier::BOLD)
            };

            lines.push(Line::from(Span::styled(role_label, role_style)));

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
                let rendered = tuillem_markdown::render_markdown(content);
                for line in rendered.lines {
                    lines.push(line);
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
            if !streaming_thinking.is_empty() {
                lines.push(Line::from(Span::styled(
                    " [thinking...] ",
                    theme.thinking_style().add_modifier(Modifier::SLOW_BLINK),
                )));
                for line in streaming_thinking.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        theme.thinking_style(),
                    )));
                }
            }

            if !streaming_text.is_empty() {
                let rendered = tuillem_markdown::render_markdown(streaming_text);
                for line in rendered.lines {
                    lines.push(line);
                }
            }

            if streaming_text.is_empty() && streaming_thinking.is_empty() {
                lines.push(Line::from(Span::styled(
                    " Waiting for response...",
                    theme.thinking_style().add_modifier(Modifier::SLOW_BLINK),
                )));
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

        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(theme.fg).bg(theme.bg))
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        // Set to a large value; the widget will clamp it
        self.scroll_offset = u16::MAX / 2;
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
