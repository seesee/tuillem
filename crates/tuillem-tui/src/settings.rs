use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum SettingValue {
    Text(String),
    Bool(bool),
    Enum {
        options: Vec<String>,
        selected: usize,
    },
}

impl SettingValue {
    pub fn display(&self) -> String {
        match self {
            SettingValue::Text(s) => {
                if s.is_empty() {
                    "(empty)".to_string()
                } else if s.len() > 30 {
                    format!("{}...", &s[..27])
                } else {
                    s.clone()
                }
            }
            SettingValue::Bool(b) => {
                if *b {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
            SettingValue::Enum { options, selected } => {
                options.get(*selected).cloned().unwrap_or_default()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingItem {
    pub label: String,
    pub key: String,
    pub value: SettingValue,
}

#[derive(Debug, Clone)]
pub struct SettingsPanel {
    pub items: Vec<SettingItem>,
    pub selected: usize,
    pub editing: bool,
    pub edit_buffer: String,
    pub scroll_offset: usize,
    pub dirty: bool,
}

impl SettingsPanel {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        default_provider: &str,
        default_model: &str,
        editor: &str,
        theme_name: &str,
        keybindings: &str,
        show_thinking: bool,
        show_token_usage: bool,
        mouse_enabled: bool,
        system_prompt: &str,
        show_stats: bool,
        layout: &str,
        date_format: &str,
        reading_wpm: u16,
        reading_nudge_lines: u16,
    ) -> Self {
        let items = vec![
            SettingItem {
                label: "Default Provider".to_string(),
                key: "defaults.provider".to_string(),
                value: SettingValue::Text(default_provider.to_string()),
            },
            SettingItem {
                label: "Default Model".to_string(),
                key: "defaults.model".to_string(),
                value: SettingValue::Text(default_model.to_string()),
            },
            SettingItem {
                label: "Editor Command".to_string(),
                key: "editor".to_string(),
                value: SettingValue::Text(editor.to_string()),
            },
            SettingItem {
                label: "Theme".to_string(),
                key: "theme".to_string(),
                value: SettingValue::Enum {
                    options: vec![
                        "dark".to_string(),
                        "light".to_string(),
                        "dracula".to_string(),
                        "nord".to_string(),
                        "gruvbox".to_string(),
                        "tokyo_night".to_string(),
                        "solarized".to_string(),
                        "solarized_light".to_string(),
                        "github_light".to_string(),
                        "rose_pine_dawn".to_string(),
                    ],
                    selected: match theme_name {
                        "light" => 1,
                        "dracula" => 2,
                        "nord" => 3,
                        "gruvbox" => 4,
                        "tokyo_night" => 5,
                        "solarized" => 6,
                        "solarized_light" => 7,
                        "github_light" => 8,
                        "rose_pine_dawn" => 9,
                        _ => 0,
                    },
                },
            },
            SettingItem {
                label: "Keybindings".to_string(),
                key: "keybindings".to_string(),
                value: SettingValue::Enum {
                    options: vec![
                        "default".to_string(),
                        "vim".to_string(),
                        "emacs".to_string(),
                    ],
                    selected: match keybindings {
                        "vim" => 1,
                        "emacs" => 2,
                        _ => 0,
                    },
                },
            },
            SettingItem {
                label: "Show Thinking".to_string(),
                key: "ui.show_thinking".to_string(),
                value: SettingValue::Bool(show_thinking),
            },
            SettingItem {
                label: "Show Token Usage".to_string(),
                key: "ui.show_token_usage".to_string(),
                value: SettingValue::Bool(show_token_usage),
            },
            SettingItem {
                label: "Mouse Enabled".to_string(),
                key: "ui.mouse".to_string(),
                value: SettingValue::Bool(mouse_enabled),
            },
            SettingItem {
                label: "Stats for Nerds".to_string(),
                key: "ui.show_stats".to_string(),
                value: SettingValue::Bool(show_stats),
            },
            SettingItem {
                label: "Layout".to_string(),
                key: "ui.layout".to_string(),
                value: SettingValue::Enum {
                    options: vec!["loose".to_string(), "tight".to_string()],
                    selected: if layout == "tight" { 1 } else { 0 },
                },
            },
            SettingItem {
                label: "Date Format".to_string(),
                key: "ui.date_format".to_string(),
                value: SettingValue::Enum {
                    options: vec![
                        "dd/mm/yyyy".to_string(),
                        "mm/dd/yyyy".to_string(),
                        "yyyy-mm-dd".to_string(),
                        "dd.mm.yyyy".to_string(),
                    ],
                    selected: match date_format {
                        "mm/dd/yyyy" => 1,
                        "yyyy-mm-dd" => 2,
                        "dd.mm.yyyy" => 3,
                        _ => 0,
                    },
                },
            },
            SettingItem {
                label: "Reading Speed (WPM)".to_string(),
                key: "ui.reading_wpm".to_string(),
                value: SettingValue::Text(reading_wpm.to_string()),
            },
            SettingItem {
                label: "Nudge Lines".to_string(),
                key: "ui.reading_nudge_lines".to_string(),
                value: SettingValue::Text(reading_nudge_lines.to_string()),
            },
            SettingItem {
                label: "System Prompt".to_string(),
                key: "defaults.system_prompt".to_string(),
                value: SettingValue::Text(system_prompt.to_string()),
            },
        ];

        Self {
            items,
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            scroll_offset: 0,
            dirty: false,
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Toggle or start editing the selected item.
    pub fn enter(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            match &mut item.value {
                SettingValue::Bool(b) => {
                    *b = !*b;
                    self.dirty = true;
                }
                SettingValue::Enum { options, selected } => {
                    *selected = (*selected + 1) % options.len();
                    self.dirty = true;
                }
                SettingValue::Text(s) => {
                    self.edit_buffer = s.clone();
                    self.editing = true;
                }
            }
        }
    }

    /// Accept the edit buffer into the value.
    pub fn confirm_edit(&mut self) {
        if self.editing {
            if let Some(item) = self.items.get_mut(self.selected)
                && let SettingValue::Text(ref mut s) = item.value
            {
                *s = self.edit_buffer.clone();
                self.dirty = true;
            }
            self.editing = false;
            self.edit_buffer.clear();
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn edit_insert(&mut self, c: char) {
        self.edit_buffer.push(c);
    }

    pub fn edit_backspace(&mut self) {
        self.edit_buffer.pop();
    }

    /// Get the value for a given key.
    pub fn get_value(&self, key: &str) -> Option<String> {
        self.items
            .iter()
            .find(|i| i.key == key)
            .map(|i| i.value.display())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_width = 60u16.min(area.width.saturating_sub(6));
        let popup_height = (self.items.len() as u16 * 2 + 3).min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let accent = Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD);
        let dim = Style::default().fg(theme.thinking_fg);
        let normal = Style::default().fg(theme.fg);

        let mut lines: Vec<Line> = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            let selected = i == self.selected;
            let marker = if selected { "▸ " } else { "  " };
            let label_style = if selected { accent } else { normal };

            let value_display = if self.editing && selected {
                format!("{}|", self.edit_buffer)
            } else {
                item.value.display()
            };

            let value_style = if selected {
                match &item.value {
                    SettingValue::Bool(b) => {
                        if *b {
                            Style::default().fg(theme.success)
                        } else {
                            dim
                        }
                    }
                    _ => Style::default().fg(theme.fg),
                }
            } else {
                match &item.value {
                    SettingValue::Bool(b) => {
                        if *b {
                            Style::default().fg(theme.success)
                        } else {
                            dim
                        }
                    }
                    _ => dim,
                }
            };

            let inner_width = popup_width.saturating_sub(2) as usize;
            let label_part = format!("{}{}", marker, item.label);
            let pad = inner_width.saturating_sub(label_part.len() + value_display.len());

            lines.push(Line::from(vec![
                Span::styled(label_part, label_style),
                Span::styled(
                    format!(
                        "{:>width$}",
                        value_display,
                        width = pad + value_display.len()
                    ),
                    value_style,
                ),
            ]));

            // Add a blank line between items (except after the last)
            if i + 1 < self.items.len() {
                lines.push(Line::from(""));
            }
        }

        let dirty_marker = if self.dirty { " [modified] " } else { "" };
        let title = format!(" Settings{} ", dirty_marker);

        let bottom_hint = if self.editing {
            " Enter:confirm  Esc:cancel "
        } else {
            " j/k:select  Enter:edit/toggle  Ctrl+S:save  Esc:close "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Line::from(Span::styled(title, accent)))
            .title_bottom(Line::from(Span::styled(bottom_hint, dim)))
            .style(Style::default().bg(theme.bg));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, popup_area);
    }
}
