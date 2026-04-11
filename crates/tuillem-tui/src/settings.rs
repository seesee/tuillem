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
    /// Model selector: shows configured models + "Add new..." option
    ModelSelect {
        models: Vec<String>,
        selected: usize,
        adding: bool,
    },
    /// Action button (e.g. "Edit Config YAML")
    Action(String),
}

impl SettingValue {
    pub fn display(&self) -> String {
        match self {
            SettingValue::Text(s) => {
                if s.is_empty() {
                    "(empty)".to_string()
                } else if s.chars().count() > 30 {
                    format!("{}...", s.chars().take(27).collect::<String>())
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
            SettingValue::ModelSelect {
                models, selected, ..
            } => models.get(*selected).cloned().unwrap_or_default(),
            SettingValue::Action(label) => label.clone(),
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
    /// All available models by provider, for refreshing model list on provider change
    available_models: Vec<(String, Vec<String>)>,
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
        scroll_lines: u16,
        command_prefix: &str,
        nerd_fonts: bool,
        stream_visible_lines: u16,
        color_mode: &str,
        available_models: &[(String, Vec<String>)],
    ) -> Self {
        // Build model list for current provider
        let provider_models: Vec<String> = available_models
            .iter()
            .find(|(name, _)| name == default_provider)
            .map(|(_, models)| models.clone())
            .unwrap_or_default();
        let model_idx = provider_models
            .iter()
            .position(|m| m == default_model)
            .unwrap_or(0);

        // Build provider list from available_models
        let provider_names: Vec<String> = available_models.iter().map(|(n, _)| n.clone()).collect();
        let provider_idx = provider_names
            .iter()
            .position(|p| p == default_provider)
            .unwrap_or(0);

        let items = vec![
            SettingItem {
                label: "Default Provider".to_string(),
                key: "defaults.provider".to_string(),
                value: if provider_names.is_empty() {
                    SettingValue::Text(default_provider.to_string())
                } else {
                    SettingValue::Enum {
                        options: provider_names,
                        selected: provider_idx,
                    }
                },
            },
            SettingItem {
                label: "Default Model".to_string(),
                key: "defaults.model".to_string(),
                value: if provider_models.is_empty() {
                    SettingValue::Text(default_model.to_string())
                } else {
                    SettingValue::ModelSelect {
                        models: provider_models,
                        selected: model_idx,
                        adding: false,
                    }
                },
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
                label: "Scroll Lines".to_string(),
                key: "ui.scroll_lines".to_string(),
                value: SettingValue::Text(scroll_lines.to_string()),
            },
            SettingItem {
                label: "Stream Visible Lines".to_string(),
                key: "ui.stream_visible_lines".to_string(),
                value: SettingValue::Text(stream_visible_lines.to_string()),
            },
            SettingItem {
                label: "Command Prefix".to_string(),
                key: "ui.command_prefix".to_string(),
                value: SettingValue::Text(command_prefix.to_string()),
            },
            SettingItem {
                label: "Nerd Fonts".to_string(),
                key: "ui.nerd_fonts".to_string(),
                value: SettingValue::Bool(nerd_fonts),
            },
            SettingItem {
                label: "Color Mode".to_string(),
                key: "ui.color_mode".to_string(),
                value: SettingValue::Enum {
                    options: vec![
                        "auto".to_string(),
                        "truecolor".to_string(),
                        "256".to_string(),
                        "basic".to_string(),
                    ],
                    selected: match color_mode {
                        "truecolor" => 1,
                        "256" => 2,
                        "basic" => 3,
                        _ => 0,
                    },
                },
            },
            SettingItem {
                label: "System Prompt".to_string(),
                key: "defaults.system_prompt".to_string(),
                value: SettingValue::Text(system_prompt.to_string()),
            },
            SettingItem {
                label: "".to_string(),
                key: "action.edit_yaml".to_string(),
                value: SettingValue::Action("Open config in editor...".to_string()),
            },
        ];

        Self {
            items,
            selected: 0,
            editing: false,
            edit_buffer: String::new(),
            scroll_offset: 0,
            dirty: false,
            available_models: available_models.to_vec(),
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

    /// Refresh the model list to match the current provider selection.
    fn refresh_model_list(&mut self) {
        // Find the current provider value
        let provider = self
            .items
            .iter()
            .find(|i| i.key == "defaults.provider")
            .map(|i| i.value.display())
            .unwrap_or_default();

        // Find models for this provider
        let models = self
            .available_models
            .iter()
            .find(|(name, _)| *name == provider)
            .map(|(_, m)| m.clone())
            .unwrap_or_default();

        // Update the model select item
        if let Some(item) = self.items.iter_mut().find(|i| i.key == "defaults.model") {
            item.value = if models.is_empty() {
                SettingValue::Text(String::new())
            } else {
                SettingValue::ModelSelect {
                    models,
                    selected: 0,
                    adding: false,
                }
            };
        }
    }

    /// Check if the selected item is the "Edit YAML" action.
    pub fn is_edit_yaml_action(&self) -> bool {
        self.items
            .get(self.selected)
            .is_some_and(|i| i.key == "action.edit_yaml")
    }

    /// Navigate right within the selected item (Enum cycles forward, ModelSelect forward).
    pub fn nav_right(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            match &mut item.value {
                SettingValue::Enum { options, selected } => {
                    *selected = (*selected + 1) % options.len();
                    self.dirty = true;
                    let is_provider = item.key == "defaults.provider";
                    if is_provider {
                        self.refresh_model_list();
                    }
                }
                SettingValue::ModelSelect {
                    models, selected, ..
                } => {
                    if *selected < models.len() {
                        *selected += 1;
                    }
                    self.dirty = true;
                }
                SettingValue::Bool(b) => {
                    *b = !*b;
                    self.dirty = true;
                }
                _ => {}
            }
        }
    }

    /// Navigate left within the selected item (Enum cycles backward, ModelSelect backward).
    pub fn nav_left(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            match &mut item.value {
                SettingValue::Enum { options, selected } => {
                    *selected = if *selected == 0 {
                        options.len() - 1
                    } else {
                        *selected - 1
                    };
                    self.dirty = true;
                    let is_provider = item.key == "defaults.provider";
                    if is_provider {
                        self.refresh_model_list();
                    }
                }
                SettingValue::ModelSelect { selected, .. } => {
                    *selected = selected.saturating_sub(1);
                    self.dirty = true;
                }
                SettingValue::Bool(b) => {
                    *b = !*b;
                    self.dirty = true;
                }
                _ => {}
            }
        }
    }

    /// Toggle or start editing the selected item.
    pub fn enter(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            match &mut item.value {
                SettingValue::Bool(b) => {
                    *b = !*b;
                    self.dirty = true;
                }
                SettingValue::Enum { .. } => {
                    // Left/Right to cycle; Enter does nothing extra for enums
                }
                SettingValue::Text(s) => {
                    self.edit_buffer = s.clone();
                    self.editing = true;
                }
                SettingValue::Action(_) => {
                    // Handled by the app — signals to open YAML editor
                }
                SettingValue::ModelSelect {
                    models,
                    selected,
                    adding,
                } => {
                    if *adding {
                        // Already in add mode — handled by edit keys
                    } else if *selected >= models.len() {
                        // "Add new..." selected — enter add mode
                        *adding = true;
                        self.edit_buffer.clear();
                        self.editing = true;
                    } else {
                        // Select this model as default
                        self.dirty = true;
                    }
                }
            }
        }
    }

    /// Accept the edit buffer into the value.
    pub fn confirm_edit(&mut self) {
        if self.editing {
            if let Some(item) = self.items.get_mut(self.selected) {
                match &mut item.value {
                    SettingValue::Text(s) => {
                        *s = self.edit_buffer.clone();
                        self.dirty = true;
                    }
                    SettingValue::ModelSelect {
                        models,
                        selected,
                        adding,
                    } => {
                        if *adding && !self.edit_buffer.trim().is_empty() {
                            let new_model = self.edit_buffer.trim().to_string();
                            models.push(new_model);
                            *selected = models.len() - 1;
                            *adding = false;
                            self.dirty = true;
                        } else {
                            *adding = false;
                        }
                    }
                    _ => {}
                }
            }
            self.editing = false;
            self.edit_buffer.clear();
        }
    }

    pub fn cancel_edit(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected)
            && let SettingValue::ModelSelect { adding, .. } = &mut item.value
        {
            *adding = false;
        }
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

    /// Get the full model list (if this key is a ModelSelect).
    pub fn get_model_list(&self, key: &str) -> Option<Vec<String>> {
        self.items.iter().find(|i| i.key == key).and_then(|i| {
            if let SettingValue::ModelSelect { models, .. } = &i.value {
                Some(models.clone())
            } else {
                None
            }
        })
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
                match &item.value {
                    SettingValue::ModelSelect { adding: true, .. } => {
                        format!("+ {}|", self.edit_buffer)
                    }
                    _ => format!("{}|", self.edit_buffer),
                }
            } else {
                match &item.value {
                    SettingValue::ModelSelect {
                        models,
                        selected: sel,
                        ..
                    } => {
                        if *sel >= models.len() {
                            "[+ Add new...] ←→".to_string()
                        } else {
                            let name = &models[*sel];
                            let max_w = 25;
                            let truncated = if name.chars().count() > max_w {
                                let t: String = name.chars().take(max_w - 1).collect();
                                format!("{}…", t)
                            } else {
                                name.clone()
                            };
                            format!("{} ({}/{}) ←→", truncated, sel + 1, models.len())
                        }
                    }
                    SettingValue::Enum {
                        options,
                        selected: sel,
                    } => {
                        format!(
                            "{} ({}/{}) ←→",
                            options.get(*sel).cloned().unwrap_or_default(),
                            sel + 1,
                            options.len()
                        )
                    }
                    SettingValue::Action(label) => format!("▸ {}", label),
                    _ => item.value.display(),
                }
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
