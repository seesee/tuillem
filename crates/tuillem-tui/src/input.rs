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
            .title_bottom(bottom_line)
            .style(Style::default().fg(theme.fg).bg(theme.bg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let display = if self.content.is_empty() {
            Paragraph::new(Span::styled(
                "Type a message...",
                Style::default().fg(theme.thinking_fg).bg(theme.bg),
            ))
        } else {
            Paragraph::new(self.content.as_str().to_owned())
                .style(Style::default().fg(theme.fg).bg(theme.bg))
                .wrap(Wrap { trim: false })
        };
        frame.render_widget(display, inner);

        // Show cursor — simulate ratatui's word wrapping to find cursor position
        if self.focused && inner.width > 0 && inner.height > 0 {
            let wrap_width = inner.width as usize;
            let (cx, cy) = compute_cursor_pos(&self.content, self.cursor_pos, wrap_width);
            let cursor_x = inner.x + cx as u16;
            let cursor_y = inner.y + cy as u16;
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
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

/// Compute visual (x, y) cursor position by simulating word wrapping.
/// Matches ratatui's `Wrap { trim: false }` behavior.
fn compute_cursor_pos(text: &str, byte_pos: usize, wrap_width: usize) -> (usize, usize) {
    if wrap_width == 0 {
        return (0, 0);
    }

    let text_before = &text[..byte_pos];
    let mut x = 0usize;
    let mut y = 0usize;

    // Process line by line (hard breaks from \n)
    for (line_idx, line) in text_before.split('\n').enumerate() {
        if line_idx > 0 {
            y += 1;
        }

        // Simulate word wrapping within this line
        let mut col = 0usize;
        let mut chars = line.chars().peekable();

        while chars.peek().is_some() {
            // Find next word and trailing spaces
            let mut word = String::new();
            // Consume spaces first
            while let Some(&c) = chars.peek() {
                if c == ' ' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            // Consume non-space chars
            let space_len = word.len();
            while let Some(&c) = chars.peek() {
                if c == ' ' {
                    break;
                }
                word.push(c);
                chars.next();
            }

            let word_len = word.len();

            if col == 0 {
                // Start of line — always place the word
                col = word_len;
            } else if col + word_len <= wrap_width {
                // Fits on current line
                col += word_len;
            } else {
                // Doesn't fit — wrap to next line
                y += 1;
                // In wrap mode, leading spaces on wrapped line are kept (trim: false)
                col = word_len - space_len; // just the non-space part on new line
                if col == 0 && space_len > 0 {
                    col = word_len; // all spaces — keep them
                }
            }
        }

        x = col;
    }

    (x, y)
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
