use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct Input {
    pub content: String,
    pub cursor_pos: usize,
    pub focused: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: 0,
            focused: true,
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        current_model: &str,
        is_streaming: bool,
        theme: &Theme,
    ) {
        let status = if is_streaming { " streaming... " } else { "" };

        let title_line = Line::from(vec![
            Span::styled(
                " tuillem ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(status, Style::default().fg(theme.warning)),
        ]);

        let bottom_line = Line::from(vec![
            Span::styled(
                format!(" {} ", current_model),
                Style::default().fg(theme.thinking_fg),
            ),
            Span::styled(
                " Enter:send | S-Ent:newline | C-e:editor | C-k:commands | C-h:help ",
                Style::default().fg(theme.thinking_fg),
            ),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focused {
                Style::default().fg(theme.accent)
            } else {
                theme.border_style()
            })
            .title_top(title_line)
            .title_bottom(bottom_line);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let display = if self.content.is_empty() {
            Paragraph::new(Span::styled(
                "Type a message...",
                Style::default().fg(theme.thinking_fg),
            ))
        } else {
            Paragraph::new(self.content.as_str().to_owned())
                .style(Style::default().fg(theme.fg))
                .wrap(Wrap { trim: false })
        };
        frame.render_widget(display, inner);

        // Show cursor — compute position accounting for wrapping
        if self.focused && inner.width > 0 && inner.height > 0 {
            let text_before_cursor = &self.content[..self.cursor_pos];
            let wrap_width = inner.width as usize;
            if wrap_width > 0 {
                // Count how many visual lines the text before cursor spans
                let mut x = 0usize;
                let mut y = 0u16;
                for ch in text_before_cursor.chars() {
                    if ch == '\n' {
                        x = 0;
                        y += 1;
                    } else {
                        x += 1;
                        if x > wrap_width {
                            x = 1;
                            y += 1;
                        }
                    }
                }
                let cursor_x = inner.x + x as u16;
                let cursor_y = inner.y + y;
                if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                    frame.set_cursor_position((cursor_x, cursor_y));
                }
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos < self.content.len() {
            let next_char = self.content[self.cursor_pos..].chars().next();
            if let Some(c) = next_char {
                self.content.remove(self.cursor_pos);
                let _ = c;
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.content[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i);
            if let Some(pos) = prev {
                self.content.remove(pos);
                self.cursor_pos = pos;
            }
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.content[..self.cursor_pos]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor_pos = prev;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor_pos < self.content.len() {
            let next = self.content[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| self.cursor_pos + c.len_utf8())
                .unwrap_or(self.content.len());
            self.cursor_pos = next;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.content.len();
    }

    /// Take the content out, resetting the input. Returns the taken content.
    pub fn take_content(&mut self) -> String {
        let content = std::mem::take(&mut self.content);
        self.cursor_pos = 0;
        content
    }

    /// Set content and move cursor to end.
    pub fn set_content(&mut self, content: String) {
        self.cursor_pos = content.len();
        self.content = content;
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_backspace() {
        let mut input = Input::new();
        input.insert_char('H');
        input.insert_char('i');
        assert_eq!(input.content, "Hi");
        assert_eq!(input.cursor_pos, 2);

        input.backspace();
        assert_eq!(input.content, "H");
        assert_eq!(input.cursor_pos, 1);

        input.backspace();
        assert_eq!(input.content, "");
        assert_eq!(input.cursor_pos, 0);

        // Backspace on empty does nothing
        input.backspace();
        assert_eq!(input.content, "");
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = Input::new();
        input.set_content("Hello".to_string());
        assert_eq!(input.cursor_pos, 5);

        input.move_home();
        assert_eq!(input.cursor_pos, 0);

        input.move_right();
        assert_eq!(input.cursor_pos, 1);

        input.move_end();
        assert_eq!(input.cursor_pos, 5);

        input.move_left();
        assert_eq!(input.cursor_pos, 4);

        input.move_home();
        input.move_left();
        assert_eq!(input.cursor_pos, 0);

        input.move_end();
        input.move_right();
        assert_eq!(input.cursor_pos, 5);
    }

    #[test]
    fn test_take_content() {
        let mut input = Input::new();
        input.set_content("Hello world".to_string());
        let taken = input.take_content();
        assert_eq!(taken, "Hello world");
        assert_eq!(input.content, "");
        assert_eq!(input.cursor_pos, 0);
    }
}
