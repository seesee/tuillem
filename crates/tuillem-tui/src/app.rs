use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tokio::sync::mpsc;
use tracing::{debug, warn};
use tuillem_core::{
    actions::{Action, Event},
    state::AppState,
};

use crate::{
    commands::{self, CommandContext, render_commands_help},
    control::{ControlAction, ControlPanel},
    conversation::Conversation,
    help::render_help,
    input::Input,
    settings::SettingsPanel,
    sidebar::Sidebar,
    theme::Theme,
};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Conversation,
    Input,
}

/// A simple selection popup for models or providers.
#[derive(Debug, Clone)]
pub struct SelectionPopup {
    pub title: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub kind: PopupKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PopupKind {
    Model,
    Provider,
}

#[derive(Debug, Clone)]
pub enum Overlay {
    None,
    Help,
    CommandsHelp,
    Control(ControlPanel),
    Settings(SettingsPanel),
    Selection(SelectionPopup),
    CodeBlockSelect {
        items: Vec<String>,
        blocks: Vec<String>,
        selected: usize,
    },
}

pub struct App {
    pub state: AppState,
    pub theme: Theme,
    pub sidebar: Sidebar,
    pub conversation: Conversation,
    pub input: Input,
    pub focus: Focus,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub should_quit: bool,
    pub editor_command: String,
    pub overlay: Overlay,
    pub available_models: Vec<(String, Vec<String>)>, // (provider_name, [model_ids])
    pub needs_redraw: bool,
    pub cancel_flag: Arc<AtomicBool>,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>, // None = not browsing history
    // Settings-related config values (used to populate settings panel)
    pub config_themes: std::collections::HashMap<String, tuillem_config::ThemeColors>,
    pub config_theme: String,
    pub config_keybindings: String,
    pub config_show_thinking: bool,
    pub config_show_token_usage: bool,
    pub config_mouse: bool,
    pub config_system_prompt: String,
    pub show_stats: bool,
    pub layout: String,
    pub date_format: String,
    pub scroll_lines: u16,
    pub default_provider: String,
    pub default_model: String,
    /// Sidebar interaction state
    pub sidebar_confirm_delete: Option<String>, // session_id pending delete
    pub sidebar_renaming: Option<(String, String)>, // (session_id, edit_buffer)
    pub sidebar_collapsed: bool,
    /// Prefix for slash commands (default "/"). Empty string disables commands.
    pub command_prefix: String,
    /// Use Nerd Font / Powerline glyphs for bubble corners (default true).
    pub nerd_fonts: bool,
    /// Colour mode: "truecolor", "256", "basic", or "auto" (default "auto").
    pub color_mode: String,
    /// Set when /clear is issued; next Enter confirms.
    pub pending_clear: bool,
    /// Scroll offset for the keyboard help overlay.
    pub help_scroll: u16,
    /// Scroll offset for the commands help overlay.
    pub commands_help_scroll: u16,
    /// Last known terminal area (for overlay scroll calculations in key handlers).
    pub last_area: Rect,
}

impl App {
    pub fn new(
        state: AppState,
        theme: Theme,
        action_tx: mpsc::UnboundedSender<Action>,
        editor_command: String,
        available_models: Vec<(String, Vec<String>)>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            state,
            theme,
            sidebar: Sidebar::new(),
            conversation: Conversation::new(),
            input: Input::new(),
            focus: Focus::Input,
            action_tx,
            should_quit: false,
            editor_command,
            overlay: Overlay::None,
            available_models,
            needs_redraw: false,
            cancel_flag,
            input_history: Vec::new(),
            history_index: None,
            config_themes: std::collections::HashMap::new(),
            config_theme: "dark".to_string(),
            config_keybindings: "default".to_string(),
            config_show_thinking: false,
            config_show_token_usage: true,
            config_mouse: true,
            config_system_prompt: String::new(),
            show_stats: false,
            layout: "loose".to_string(),
            date_format: "dd/mm/yyyy".to_string(),
            scroll_lines: 5,
            default_provider: String::new(),
            default_model: String::new(),
            sidebar_confirm_delete: None,
            sidebar_renaming: None,
            sidebar_collapsed: false,
            command_prefix: "/".to_string(),
            nerd_fonts: true,
            color_mode: "auto".to_string(),
            pending_clear: false,
            help_scroll: 0,
            commands_help_scroll: 0,
            last_area: Rect::default(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        // Auto-expire status messages after 5 seconds
        if let Some((_, created)) = &self.state.status_message
            && created.elapsed() > std::time::Duration::from_secs(5)
        {
            self.state.status_message = None;
        }

        let size = frame.area();
        self.last_area = size;

        // Horizontal split: sidebar | right
        let sidebar_width = if self.sidebar_collapsed { 0 } else { 30 };
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(sidebar_width), Constraint::Min(1)])
            .split(size);

        // Right panel: conversation | [stats bar] | input
        let input_height: u16 = if self.layout == "tight" { 5 } else { 7 };
        let show_stats_bar =
            self.show_stats && !self.state.is_streaming && self.state.last_response_stats.is_some();

        let v_constraints = if show_stats_bar {
            vec![
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(input_height),
            ]
        } else {
            vec![Constraint::Min(1), Constraint::Length(input_height)]
        };
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(v_constraints)
            .split(h_chunks[1]);

        if !self.sidebar_collapsed {
            self.sidebar.render(
                frame,
                h_chunks[0],
                &self.state.sessions,
                self.focus == Focus::Sidebar,
                &self.theme,
                &self.layout,
                &self.date_format,
                self.sidebar_confirm_delete.as_deref(),
                self.sidebar_renaming
                    .as_ref()
                    .map(|(id, buf)| (id.as_str(), buf.as_str())),
            );
        }

        self.conversation.render(
            frame,
            v_chunks[0],
            &self.state.messages,
            &self.state.streaming_text,
            &self.state.streaming_thinking,
            self.state.is_streaming,
            &self.state.current_model,
            self.state.error.as_deref(),
            self.state
                .status_message
                .as_ref()
                .map(|(msg, _)| msg.as_str()),
            self.focus == Focus::Conversation,
            &self.theme,
            &self.layout,
            self.nerd_fonts,
        );

        if show_stats_bar {
            self.render_stats_bar(frame, v_chunks[1]);
        }

        let input_chunk = if show_stats_bar {
            v_chunks[2]
        } else {
            v_chunks[1]
        };
        self.input.render(
            frame,
            input_chunk,
            &self.state.current_model,
            self.state.is_streaming,
            &self.theme,
        );

        // Draw overlay on top if active
        match &self.overlay {
            Overlay::None => {}
            Overlay::Help => {
                render_help(frame, size, &self.theme, self.help_scroll);
            }
            Overlay::CommandsHelp => {
                render_commands_help(
                    frame,
                    size,
                    &self.theme,
                    &self.command_prefix,
                    self.commands_help_scroll,
                );
            }
            Overlay::Control(panel) => {
                panel.render(frame, size, &self.theme);
            }
            Overlay::Settings(panel) => {
                panel.render(frame, size, &self.theme);
            }
            Overlay::Selection(popup) => {
                self.draw_selection_popup(frame, size, popup);
            }
            Overlay::CodeBlockSelect {
                items, selected, ..
            } => {
                self.draw_code_block_popup(frame, size, items, *selected);
            }
        }
    }

    fn render_stats_bar(&self, frame: &mut Frame, area: Rect) {
        if let Some(ref stats) = self.state.last_response_stats {
            let toks_per_sec = if stats.latency_ms > 0 {
                stats.tokens_out as f64 / (stats.latency_ms as f64 / 1000.0)
            } else {
                0.0
            };

            // Estimate context usage based on common context windows
            let context_window: u64 = match stats.model.as_str() {
                m if m.contains("gpt-4o") => 128_000,
                m if m.contains("gpt-4-turbo") => 128_000,
                m if m.contains("gpt-4") => 8_192,
                m if m.contains("gpt-3.5") => 16_385,
                m if m.contains("claude-3-haiku") => 200_000,
                m if m.contains("claude") => 200_000,
                m if m.contains("llama") => 8_192,
                _ => 200_000,
            };
            let total_tokens = stats.tokens_in + stats.tokens_out;
            let ctx_pct = (total_tokens as f64 / context_window as f64) * 100.0;

            let approx = if stats.estimated { "~" } else { "" };
            let stats_text = format!(
                "{}Tokens: {}>{}  {:.1} tok/s  ~{:.0}% ctx",
                approx, stats.tokens_in, stats.tokens_out, toks_per_sec, ctx_pct
            );

            let style = Style::default().fg(self.theme.thinking_fg);
            // Right-align the text within the area
            let padding = (area.width as usize).saturating_sub(stats_text.len());
            let line = Line::from(Span::styled(
                format!("{:>width$}", stats_text, width = padding + stats_text.len()),
                style,
            ));
            let paragraph = ratatui::widgets::Paragraph::new(line);
            frame.render_widget(paragraph, area);
        }
    }

    fn draw_code_block_popup(
        &self,
        frame: &mut Frame,
        area: Rect,
        items: &[String],
        selected: usize,
    ) {
        let popup_width = 60u16.min(area.width.saturating_sub(10));
        let popup_height = (items.len() as u16 + 2).min(area.height.saturating_sub(6));
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == selected {
                    Style::default()
                        .fg(self.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(self.theme.fg)
                };
                let marker = if i == selected { "▸ " } else { "  " };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", marker, item),
                    style,
                )))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.accent))
            .title(Line::from(Span::styled(
                " Copy Code Block ",
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))
            .title_bottom(Line::from(Span::styled(
                " j/k:select  Enter:copy  Esc:cancel ",
                Style::default().fg(self.theme.thinking_fg),
            )))
            .style(Style::default().bg(self.theme.bg));

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        let list = List::new(list_items).block(block);
        frame.render_stateful_widget(list, popup_area, &mut list_state);
    }

    fn draw_selection_popup(&self, frame: &mut Frame, area: Rect, popup: &SelectionPopup) {
        let popup_width = 50u16.min(area.width.saturating_sub(10));
        let popup_height = (popup.items.len() as u16 + 2).min(area.height.saturating_sub(6));
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = popup
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == popup.selected {
                    Style::default()
                        .fg(self.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(self.theme.fg)
                };
                let marker = if i == popup.selected { "▸ " } else { "  " };
                ListItem::new(Line::from(Span::styled(
                    format!("{}{}", marker, item),
                    style,
                )))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.accent))
            .title(Line::from(Span::styled(
                format!(" {} ", popup.title),
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))
            .title_bottom(Line::from(Span::styled(
                " j/k:select  Enter:confirm  Esc:cancel ",
                Style::default().fg(self.theme.thinking_fg),
            )))
            .style(Style::default().bg(self.theme.bg));

        let mut list_state = ListState::default();
        list_state.select(Some(popup.selected));

        let list = List::new(items).block(block);
        frame.render_stateful_widget(list, popup_area, &mut list_state);
    }

    pub fn apply_event(&mut self, event: &Event) {
        self.state.apply_event(event);

        // Update sidebar content matches from search results
        if let Event::SearchResults { results } = event {
            let ids: std::collections::HashSet<String> =
                results.iter().map(|r| r.session_id.clone()).collect();
            self.sidebar.content_match_ids = Some(ids);
            self.sidebar.selected = 0;
        }

        // Rebuild input history when messages are loaded; clear transient status
        if let Event::MessagesLoaded { messages } = event {
            // Prune stale cache entries; keep valid ones for performance
            self.conversation.prune_render_cache(messages);
            self.state.status_message = None;
            self.input_history = messages
                .iter()
                .filter(|m| m.role == "user")
                .filter_map(|m| m.content.clone())
                .collect();
            self.history_index = None;
        }

        // Scroll behavior for streaming and message events
        match event {
            Event::StreamStarted => {
                // Jump to bottom first, then enter streaming mode.
                // FollowBottom ensures the next render snaps to the actual bottom
                // with fresh total_lines. The first StreamDelta will transition
                // to Streaming state with the correct start_offset.
                self.conversation.scroll_to_bottom();
            }
            Event::StreamDelta { .. } | Event::ThinkingDelta { .. } => {
                // On first delta, transition from FollowBottom to Streaming
                // so the render freeze logic kicks in with correct start_offset
                if matches!(
                    self.conversation.scroll_state,
                    crate::conversation::ScrollState::FollowBottom
                ) {
                    let start = self
                        .conversation
                        .total_lines
                        .saturating_sub(self.conversation.visible_height);
                    self.conversation.scroll_state = crate::conversation::ScrollState::Streaming {
                        start_offset: start,
                    };
                }
            }
            Event::StreamDone { .. } => {
                // Freeze when done (whether from Streaming or FollowBottom)
                if !matches!(
                    self.conversation.scroll_state,
                    crate::conversation::ScrollState::Frozen
                ) {
                    self.conversation.scroll_state = crate::conversation::ScrollState::Frozen;
                }
            }
            Event::MessagesLoaded { .. } | Event::ResponseError { .. } => {
                // Only scroll to bottom if not frozen (user is reading)
                if !matches!(
                    self.conversation.scroll_state,
                    crate::conversation::ScrollState::Frozen
                ) {
                    self.conversation.scroll_to_bottom();
                }
            }
            _ => {}
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        // Handle overlay keys first
        if !matches!(self.overlay, Overlay::None) {
            self.handle_overlay_key(key);
            return;
        }

        // If sidebar rename or delete confirmation is active, route ALL keys there
        if self.sidebar_renaming.is_some() || self.sidebar_confirm_delete.is_some() {
            self.handle_sidebar_key(key);
            return;
        }

        // Global bindings
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => {
                    self.should_quit = true;
                    let _ = self.action_tx.send(Action::Quit);
                    return;
                }
                KeyCode::Char('n') => {
                    let _ = self.action_tx.send(Action::CreateSession {
                        title: "New Chat".to_string(),
                    });
                    // Reset to default model/provider
                    if !self.default_provider.is_empty() {
                        let _ = self.action_tx.send(Action::SwitchModel {
                            provider: self.default_provider.clone(),
                            model: self.default_model.clone(),
                        });
                    }
                    self.focus = Focus::Input;
                    self.update_focus_state();
                    return;
                }
                KeyCode::Char('r') => {
                    if !self.state.is_streaming {
                        let _ = self.action_tx.send(Action::RegenerateLastResponse);
                    }
                    return;
                }
                KeyCode::Char('k') => {
                    self.overlay = Overlay::Control(ControlPanel::new());
                    return;
                }
                KeyCode::Char('o') => {
                    self.open_model_popup();
                    return;
                }
                KeyCode::Char('t') => {
                    self.open_provider_popup();
                    return;
                }
                KeyCode::Char('s') => {
                    self.open_settings();
                    return;
                }
                KeyCode::Char('h') => {
                    self.help_scroll = 0;
                    self.overlay = Overlay::Help;
                    return;
                }
                KeyCode::Char('l') => {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                    // If collapsing while focused on sidebar, move to input
                    if self.sidebar_collapsed && self.focus == Focus::Sidebar {
                        self.focus = Focus::Input;
                        self.update_focus_state();
                    }
                    return;
                }
                KeyCode::Char('y') => {
                    self.copy_last_response();
                    return;
                }
                KeyCode::Char('b') => {
                    self.copy_code_blocks();
                    return;
                }
                _ => {}
            }
        }

        // Tab / Shift+Tab / BackTab cycle focus
        match key.code {
            KeyCode::Tab => {
                self.cycle_focus_forward();
                self.update_focus_state();
                return;
            }
            KeyCode::BackTab => {
                self.cycle_focus_backward();
                self.update_focus_state();
                return;
            }
            _ => {}
        }

        // Escape: cancel streaming if active, otherwise return to input
        if key.code == KeyCode::Esc {
            if self.state.is_streaming {
                debug!("Escape pressed — cancelling stream");
                self.cancel_flag.store(true, Ordering::Relaxed);
                return;
            }
            if self.sidebar.search_focused {
                self.sidebar.search_focused = false;
                self.sidebar.search_input.clear();
                return;
            }
            self.focus = Focus::Input;
            self.update_focus_state();
            return;
        }

        // Delegate to focused widget
        match self.focus {
            Focus::Sidebar => self.handle_sidebar_key(key),
            Focus::Conversation => self.handle_conversation_key(key),
            Focus::Input => self.handle_input_key(key),
        }
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.conversation.scroll_up(3);
            }
            MouseEventKind::ScrollDown => {
                self.conversation.scroll_down(3);
            }
            _ => {}
        }
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) {
        match &mut self.overlay {
            Overlay::None => {}
            Overlay::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let max = crate::help::help_max_scroll(self.last_area);
                    self.help_scroll = self.help_scroll.saturating_add(1).min(max);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.help_scroll = self.help_scroll.saturating_sub(1);
                }
                _ => {}
            },
            Overlay::CommandsHelp => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let max = crate::commands::commands_help_max_scroll(self.last_area);
                    self.commands_help_scroll =
                        self.commands_help_scroll.saturating_add(1).min(max);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.commands_help_scroll = self.commands_help_scroll.saturating_sub(1);
                }
                _ => {}
            },
            Overlay::Control(panel) => match key.code {
                KeyCode::Esc => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    panel.move_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    panel.move_up();
                }
                KeyCode::Enter => {
                    let action = panel.selected_action();
                    self.overlay = Overlay::None;
                    self.execute_control_action(action);
                }
                _ => {}
            },
            Overlay::Settings(panel) => {
                if panel.editing {
                    match key.code {
                        KeyCode::Esc => {
                            panel.cancel_edit();
                        }
                        KeyCode::Enter => {
                            panel.confirm_edit();
                        }
                        KeyCode::Backspace => {
                            panel.edit_backspace();
                        }
                        KeyCode::Char(c) => {
                            panel.edit_insert(c);
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            self.overlay = Overlay::None;
                        }
                        KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => {
                            panel.move_down();
                        }
                        KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => {
                            panel.move_up();
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            panel.nav_left();
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            panel.nav_right();
                        }
                        KeyCode::Enter => {
                            if panel.is_edit_yaml_action() {
                                self.overlay = Overlay::None;
                                self.edit_config_yaml();
                            } else {
                                panel.enter();
                            }
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            self.save_settings();
                            self.overlay = Overlay::None;
                        }
                        _ => {}
                    }
                }
            }
            Overlay::Selection(popup) => match key.code {
                KeyCode::Esc => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if popup.selected + 1 < popup.items.len() {
                        popup.selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    popup.selected = popup.selected.saturating_sub(1);
                }
                KeyCode::Enter => {
                    let selected_item = popup.items[popup.selected].clone();
                    let kind = popup.kind.clone();
                    self.overlay = Overlay::None;

                    match kind {
                        PopupKind::Model => {
                            let _ = self.action_tx.send(Action::SwitchModel {
                                provider: self.state.current_provider.clone(),
                                model: selected_item,
                            });
                        }
                        PopupKind::Provider => {
                            if let Some((_, models)) = self
                                .available_models
                                .iter()
                                .find(|(name, _)| *name == selected_item)
                            {
                                let model = models.first().cloned().unwrap_or_default();
                                let _ = self.action_tx.send(Action::SwitchModel {
                                    provider: selected_item,
                                    model,
                                });
                            }
                        }
                    }
                }
                _ => {}
            },
            Overlay::CodeBlockSelect {
                items: _,
                blocks,
                selected,
            } => match key.code {
                KeyCode::Esc => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    let len = blocks.len();
                    if *selected + 1 < len {
                        *selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Enter => {
                    let block_text = blocks[*selected].clone();
                    self.overlay = Overlay::None;
                    if let Ok(mut clipboard) = arboard::Clipboard::new()
                        && clipboard.set_text(&block_text).is_ok()
                    {
                        self.state.status_message = Some((
                            "Copied code block to clipboard".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                }
                _ => {}
            },
        }
    }

    fn execute_control_action(&mut self, action: ControlAction) {
        match action {
            ControlAction::SwitchModel => {
                self.open_model_popup();
            }
            ControlAction::SwitchProvider => {
                self.open_provider_popup();
            }
            ControlAction::NewConversation => {
                let _ = self.action_tx.send(Action::CreateSession {
                    title: "New Chat".to_string(),
                });
                if !self.default_provider.is_empty() {
                    let _ = self.action_tx.send(Action::SwitchModel {
                        provider: self.default_provider.clone(),
                        model: self.default_model.clone(),
                    });
                }
                self.focus = Focus::Input;
                self.update_focus_state();
            }
            ControlAction::RegenerateResponse => {
                if !self.state.is_streaming {
                    let _ = self.action_tx.send(Action::RegenerateLastResponse);
                }
            }
            ControlAction::SaveTranscript => {
                let _ = self.action_tx.send(Action::SaveTranscript);
            }
            ControlAction::OpenInEditor => {
                self.open_external_editor();
            }
            ControlAction::ToggleThinking => {
                if !self.state.messages.is_empty() {
                    let idx = self.state.messages.len() - 1;
                    self.conversation.toggle_thinking(idx);
                }
            }
        }
    }

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        // Handle rename mode (inline text editing)
        if self.sidebar_renaming.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.sidebar_renaming = None;
                }
                KeyCode::Enter => {
                    if let Some((sid, buf)) = self.sidebar_renaming.take()
                        && !buf.trim().is_empty()
                    {
                        let _ = self.action_tx.send(Action::RenameSession {
                            id: sid,
                            title: buf.trim().to_string(),
                        });
                    }
                }
                KeyCode::Backspace => {
                    if let Some((_, ref mut buf)) = self.sidebar_renaming {
                        buf.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some((_, ref mut buf)) = self.sidebar_renaming {
                        buf.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        // Handle delete confirmation
        if let Some(ref session_id) = self.sidebar_confirm_delete {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let sid = session_id.clone();
                    self.sidebar_confirm_delete = None;
                    let _ = self.action_tx.send(Action::DeleteSession { id: sid });
                }
                _ => {
                    // Any other key cancels
                    self.sidebar_confirm_delete = None;
                }
            }
            return;
        }

        // Search mode
        if self.sidebar.search_focused {
            match key.code {
                KeyCode::Esc => {
                    self.sidebar.search_focused = false;
                    self.sidebar.search_input.clear();
                    self.sidebar.content_match_ids = None;
                }
                KeyCode::Enter => {
                    self.sidebar.search_focused = false;
                }
                KeyCode::Backspace => {
                    self.sidebar.search_input.pop();
                    self.trigger_search();
                }
                KeyCode::Char(c) => {
                    self.sidebar.search_input.push(c);
                    self.trigger_search();
                }
                _ => {}
            }
            return;
        }

        let session_count = self.sidebar.filtered_sessions(&self.state.sessions).len();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.sidebar.move_down(session_count, 1);
                self.preview_selected_session();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sidebar.move_up(1);
                self.preview_selected_session();
            }
            KeyCode::Char('g') => {
                self.sidebar.selected = 0;
                self.sidebar.scroll_offset = 0;
                self.preview_selected_session();
            }
            KeyCode::Char('G') => {
                if session_count > 0 {
                    self.sidebar.selected = session_count - 1;
                }
                self.preview_selected_session();
            }
            KeyCode::Enter => {
                let filtered = self.sidebar.filtered_sessions(&self.state.sessions);
                if let Some(session) = filtered.get(self.sidebar.selected) {
                    let _ = self.action_tx.send(Action::SelectSession {
                        id: session.id.clone(),
                    });
                    self.focus = Focus::Input;
                    self.update_focus_state();
                }
            }
            KeyCode::PageUp => {
                self.conversation.scroll_up(self.conversation.visible_height.saturating_sub(2));
            }
            KeyCode::PageDown => {
                self.conversation.scroll_down(self.conversation.visible_height.saturating_sub(2));
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.conversation.scroll_up(self.conversation.visible_height / 2);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.conversation.scroll_down(self.conversation.visible_height / 2);
            }
            KeyCode::Char('d') => {
                // Start delete confirmation
                let filtered = self.sidebar.filtered_sessions(&self.state.sessions);
                if let Some(session) = filtered.get(self.sidebar.selected) {
                    self.sidebar_confirm_delete = Some(session.id.clone());
                }
            }
            KeyCode::Char('r') => {
                // Start rename — clear buffer so user types fresh title
                let filtered = self.sidebar.filtered_sessions(&self.state.sessions);
                if let Some(session) = filtered.get(self.sidebar.selected) {
                    self.sidebar_renaming = Some((session.id.clone(), String::new()));
                    self.needs_redraw = true;
                }
            }
            KeyCode::Char('/') => {
                self.sidebar.search_focused = true;
                self.sidebar.search_input.clear();
            }
            _ => {}
        }
    }

    fn handle_conversation_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.conversation.scroll_down(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.conversation.scroll_up(1);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.conversation.scroll_down(15);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.conversation.scroll_up(15);
            }
            KeyCode::Char('g') => {
                self.conversation.scroll_offset = 0;
            }
            KeyCode::Char('G') => {
                self.conversation.scroll_to_bottom();
            }
            KeyCode::PageUp => {
                self.conversation.scroll_up(20);
            }
            KeyCode::PageDown => {
                self.conversation.scroll_down(20);
            }
            KeyCode::Char('t') => {
                // Toggle thinking for the last message (simple heuristic)
                if !self.state.messages.is_empty() {
                    let idx = self.state.messages.len() - 1;
                    self.conversation.toggle_thinking(idx);
                }
            }
            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.input.insert_char('\n');
                } else if self.input.content.trim().is_empty() {
                    // Empty input: advance scroll (works during and after streaming)
                    let advance = self.scroll_lines;
                    let max_offset = self
                        .conversation
                        .total_lines
                        .saturating_sub(self.conversation.visible_height);
                    self.conversation.scroll_offset = self
                        .conversation
                        .scroll_offset
                        .saturating_add(advance)
                        .min(max_offset);
                    // Freeze so render doesn't override
                    self.conversation.scroll_state = crate::conversation::ScrollState::Frozen;
                    // Set highlight on the first line of newly visible content
                    self.conversation.highlight_line = Some(self.conversation.scroll_offset);
                    self.conversation.highlight_set_at = Some(std::time::Instant::now());
                } else if self.pending_clear {
                    // Confirm clear
                    self.pending_clear = false;
                    let content = self.input.take_content();
                    if content.trim().eq_ignore_ascii_case("y") {
                        // Delete the current session from DB and create a fresh one
                        if let Some(ref session_id) = self.state.active_session_id {
                            let _ = self.action_tx.send(Action::DeleteSession {
                                id: session_id.clone(),
                            });
                        }
                        let _ = self.action_tx.send(Action::CreateSession {
                            title: "New Chat".to_string(),
                        });
                        self.state.messages.clear();
                        self.state.streaming_text.clear();
                        self.state.streaming_thinking.clear();
                        self.conversation.scroll_offset = 0;
                        self.conversation.clear_render_cache();
                        self.state.status_message = Some((
                            "Conversation cleared".to_string(),
                            std::time::Instant::now(),
                        ));
                    } else {
                        self.state.status_message =
                            Some(("Clear cancelled".to_string(), std::time::Instant::now()));
                    }
                } else {
                    let content = self.input.take_content();
                    if !content.trim().is_empty() {
                        // Check for slash command
                        let ctx = self.build_command_context();
                        if let Some(result) =
                            commands::parse_command(&content, &self.command_prefix, &ctx)
                        {
                            self.execute_command_result(result);
                        } else {
                            // Normal message send
                            self.state.error = None;
                            self.state.status_message = None;
                            self.conversation.scroll_to_bottom();
                            self.input_history.push(content.clone());
                            self.history_index = None;
                            debug!(
                                "Sending Action::SendMessage, content length={}",
                                content.len()
                            );
                            if let Err(e) = self.action_tx.send(Action::SendMessage { content }) {
                                warn!("Failed to send action to coordinator: {e}");
                                self.state.error =
                                    Some(format!("Internal error: coordinator disconnected ({e})"));
                            }
                        }
                    }
                }
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_external_editor();
            }
            KeyCode::Up => {
                self.history_prev();
            }
            KeyCode::Down => {
                self.history_next();
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
            }
            KeyCode::Backspace => {
                self.input.backspace();
            }
            KeyCode::Delete => {
                self.input.delete_char();
            }
            KeyCode::Left => {
                self.input.move_left();
            }
            KeyCode::Right => {
                self.input.move_right();
            }
            KeyCode::Home => {
                self.input.move_home();
            }
            KeyCode::End => {
                self.input.move_end();
            }
            KeyCode::PageUp => {
                self.conversation
                    .scroll_up(self.conversation.visible_height.saturating_sub(2));
            }
            KeyCode::PageDown => {
                self.conversation
                    .scroll_down(self.conversation.visible_height.saturating_sub(2));
            }
            _ => {}
        }
    }

    fn open_model_popup(&mut self) {
        // Find models for the current provider
        let models = self
            .available_models
            .iter()
            .find(|(name, _)| *name == self.state.current_provider)
            .map(|(_, models)| models.clone())
            .unwrap_or_default();

        if models.is_empty() {
            return;
        }

        let current_idx = models
            .iter()
            .position(|m| *m == self.state.current_model)
            .unwrap_or(0);

        self.overlay = Overlay::Selection(SelectionPopup {
            title: format!("Switch Model ({})", self.state.current_provider),
            items: models,
            selected: current_idx,
            kind: PopupKind::Model,
        });
    }

    fn open_provider_popup(&mut self) {
        let providers: Vec<String> = self
            .available_models
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        if providers.is_empty() {
            return;
        }

        let current_idx = providers
            .iter()
            .position(|p| *p == self.state.current_provider)
            .unwrap_or(0);

        self.overlay = Overlay::Selection(SelectionPopup {
            title: "Switch Provider".to_string(),
            items: providers,
            selected: current_idx,
            kind: PopupKind::Provider,
        });
    }

    fn open_settings(&mut self) {
        let panel = SettingsPanel::new(
            &self.default_provider,
            &self.default_model,
            &self.editor_command,
            &self.config_theme,
            &self.config_keybindings,
            self.config_show_thinking,
            self.config_show_token_usage,
            self.config_mouse,
            &self.config_system_prompt,
            self.show_stats,
            &self.layout,
            &self.date_format,
            self.scroll_lines,
            &self.command_prefix,
            self.nerd_fonts,
            &self.color_mode,
            &self.available_models,
        );
        self.overlay = Overlay::Settings(panel);
    }

    fn save_settings(&mut self) {
        // Extract values from the settings panel before closing
        if let Overlay::Settings(ref panel) = self.overlay {
            if let Some(v) = panel.get_value("defaults.provider") {
                self.default_provider = v;
            }
            if let Some(v) = panel.get_value("defaults.model") {
                self.default_model = v;
            }
            // Update available_models if models were added
            if let Some(models) = panel.get_model_list("defaults.model")
                && let Some(entry) = self
                    .available_models
                    .iter_mut()
                    .find(|(name, _)| *name == self.default_provider)
            {
                entry.1 = models;
            }
            if let Some(v) = panel.get_value("editor") {
                self.editor_command = v;
            }
            if let Some(v) = panel.get_value("theme") {
                self.config_theme = v;
            }
            if let Some(v) = panel.get_value("keybindings") {
                self.config_keybindings = v;
            }
            if let Some(v) = panel.get_value("ui.show_thinking") {
                self.config_show_thinking = v == "on";
            }
            if let Some(v) = panel.get_value("ui.show_token_usage") {
                self.config_show_token_usage = v == "on";
            }
            if let Some(v) = panel.get_value("ui.mouse") {
                self.config_mouse = v == "on";
            }
            if let Some(v) = panel.get_value("ui.show_stats") {
                self.show_stats = v == "on";
            }
            if let Some(v) = panel.get_value("defaults.system_prompt") {
                self.config_system_prompt = if v == "(empty)" { String::new() } else { v };
            }
            if let Some(v) = panel.get_value("ui.layout") {
                self.layout = v;
            }
            if let Some(v) = panel.get_value("ui.date_format") {
                self.date_format = v;
            }
            if let Some(v) = panel.get_value("ui.scroll_lines")
                && let Ok(lines) = v.parse::<u16>()
            {
                self.scroll_lines = lines.max(1);
            }
            if let Some(v) = panel.get_value("ui.command_prefix") {
                self.command_prefix = if v == "(empty)" { String::new() } else { v };
            }
            if let Some(v) = panel.get_value("ui.nerd_fonts") {
                self.nerd_fonts = v == "on";
            }
            if let Some(v) = panel.get_value("ui.color_mode") {
                self.color_mode = v;
            }
            // Apply theme instantly (with colour degradation)
            let resolved = crate::theme::resolve_color_mode(&self.color_mode);
            self.theme =
                Theme::from_config(&self.config_theme, &self.config_themes)
                    .adapt_to_color_mode(resolved);

            // Write to config file
            self.write_config_file();
        }
    }

    fn write_config_file(&self) {
        let config_path = tuillem_config::Config::default_path();
        // Load existing config or create default
        let mut config = if config_path.exists() {
            tuillem_config::Config::from_file(&config_path).unwrap_or_default()
        } else {
            tuillem_config::Config::default()
        };

        config.editor = self.editor_command.clone();
        config.theme = self.config_theme.clone();
        config.keybindings = match self.config_keybindings.as_str() {
            "vim" => tuillem_config::KeybindingPreset::Vim,
            "emacs" => tuillem_config::KeybindingPreset::Emacs,
            _ => tuillem_config::KeybindingPreset::Default,
        };
        config.ui.show_thinking = self.config_show_thinking;
        config.ui.show_token_usage = self.config_show_token_usage;
        config.ui.mouse = self.config_mouse;
        config.ui.show_stats = self.show_stats;
        config.ui.layout = self.layout.clone();
        config.ui.date_format = self.date_format.clone();
        config.ui.scroll_lines = self.scroll_lines;
        config.ui.command_prefix = self.command_prefix.clone();
        config.ui.nerd_fonts = self.nerd_fonts;
        config.ui.color_mode = self.color_mode.clone();
        if !self.default_provider.is_empty() {
            config.defaults.provider = Some(self.default_provider.clone());
        }
        if !self.default_model.is_empty() {
            config.defaults.model = Some(self.default_model.clone());
        }
        // Sync model lists from available_models to config providers
        for (provider_name, models) in &self.available_models {
            if let Some(pc) = config
                .providers
                .iter_mut()
                .find(|p| &p.name == provider_name)
            {
                pc.models = models.clone();
            }
        }
        if self.config_system_prompt.is_empty() {
            config.defaults.system_prompt = None;
        } else {
            config.defaults.system_prompt = Some(self.config_system_prompt.clone());
        }

        if let Ok(yaml) = serde_yaml::to_string(&config) {
            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&config_path, yaml) {
                warn!("Failed to write config file: {e}");
            } else {
                debug!("Settings saved to {}", config_path.display());
            }
        }
    }

    fn trigger_search(&mut self) {
        if self.sidebar.search_input.is_empty() {
            self.sidebar.content_match_ids = None;
        } else {
            let _ = self.action_tx.send(Action::Search {
                query: self.sidebar.search_input.clone(),
            });
        }
        self.sidebar.selected = 0;
    }

    /// Preview the currently highlighted session in the conversation pane
    /// without fully selecting it (Enter does the full select + focus switch).
    fn preview_selected_session(&mut self) {
        let filtered = self.sidebar.filtered_sessions(&self.state.sessions);
        if let Some(session) = filtered.get(self.sidebar.selected) {
            let _ = self.action_tx.send(Action::SelectSession {
                id: session.id.clone(),
            });
        }
    }

    fn build_command_context(&self) -> CommandContext<'_> {
        let (total_in, total_out) = self.state.messages.iter().fold((0u64, 0u64), |(i, o), m| {
            (
                i + m.token_usage_in.unwrap_or(0) as u64,
                o + m.token_usage_out.unwrap_or(0) as u64,
            )
        });

        CommandContext {
            current_provider: &self.state.current_provider,
            current_model: &self.state.current_model,
            active_session_id: self.state.active_session_id.as_deref(),
            message_count: self.state.messages.len(),
            total_tokens_in: total_in,
            total_tokens_out: total_out,
            available_models: &self.available_models,
        }
    }

    fn execute_command_result(&mut self, result: commands::CommandResult) {
        // Jump to bottom when executing commands
        self.conversation.scroll_to_bottom();

        // Show help overlay
        if result.show_help {
            self.commands_help_scroll = 0;
            self.overlay = Overlay::CommandsHelp;
            return;
        }

        // Apply thinking toggle
        if let Some(thinking) = result.set_thinking {
            self.config_show_thinking = thinking;
        }

        // Apply system prompt
        if let Some(ref prompt) = result.set_system_prompt {
            self.config_system_prompt = prompt.clone();
        }

        // Handle clear confirmation
        if result.request_clear {
            self.pending_clear = true;
            self.state.status_message = Some((
                "Clear all messages? Type y and press Enter to confirm, anything else to cancel."
                    .to_string(),
                std::time::Instant::now(),
            ));
            return;
        }

        // Send action to coordinator
        if let Some(action) = result.action {
            // For CreateSession, also reset to default model
            let is_new_session = matches!(action, Action::CreateSession { .. });
            if let Err(e) = self.action_tx.send(action) {
                warn!("Failed to send command action: {e}");
                self.state.error = Some(format!("Internal error: coordinator disconnected ({e})"));
                return;
            }
            if is_new_session && !self.default_provider.is_empty() {
                let _ = self.action_tx.send(Action::SwitchModel {
                    provider: self.default_provider.clone(),
                    model: self.default_model.clone(),
                });
                self.focus = Focus::Input;
                self.update_focus_state();
            }
            // Send initial message if provided (e.g. /new tell me about vegetables)
            if let Some(msg) = result.initial_message {
                let _ = self.action_tx.send(Action::SendMessage { content: msg });
            }
        }

        // Show status or error
        if let Some(error) = result.error {
            self.state.error = Some(error);
        } else if let Some(message) = result.message
            && !message.is_empty()
        {
            self.state.status_message = Some((message, std::time::Instant::now()));
        }
    }

    fn copy_last_response(&mut self) {
        let last_assistant = self
            .state
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant");
        if let Some(msg) = last_assistant {
            let text = msg.content.as_deref().unwrap_or("");
            if let Ok(mut clipboard) = arboard::Clipboard::new()
                && clipboard.set_text(text).is_ok()
            {
                self.state.status_message = Some((
                    "Copied response to clipboard".to_string(),
                    std::time::Instant::now(),
                ));
            }
        }
    }

    fn copy_code_blocks(&mut self) {
        let last_assistant = self
            .state
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant");
        let content = match last_assistant {
            Some(msg) => msg.content.as_deref().unwrap_or(""),
            None => return,
        };

        // Extract fenced code blocks
        let mut blocks: Vec<String> = Vec::new();
        let mut in_block = false;
        let mut current_block = String::new();

        for line in content.lines() {
            if line.trim_start().starts_with("```") {
                if in_block {
                    // End of block
                    blocks.push(current_block.clone());
                    current_block.clear();
                    in_block = false;
                } else {
                    // Start of block
                    in_block = true;
                    current_block.clear();
                }
            } else if in_block {
                if !current_block.is_empty() {
                    current_block.push('\n');
                }
                current_block.push_str(line);
            }
        }

        if blocks.is_empty() {
            // No code blocks — copy full response
            self.copy_last_response();
            return;
        }

        if blocks.len() == 1 {
            // Single block — copy directly
            if let Ok(mut clipboard) = arboard::Clipboard::new()
                && clipboard.set_text(&blocks[0]).is_ok()
            {
                self.state.status_message = Some((
                    "Copied code block to clipboard".to_string(),
                    std::time::Instant::now(),
                ));
            }
            return;
        }

        // Multiple blocks — show selection popup
        let items: Vec<String> = blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let preview: String = b.lines().next().unwrap_or("").chars().take(40).collect();
                format!("Block {} — {}", i + 1, preview)
            })
            .collect();

        self.overlay = Overlay::CodeBlockSelect {
            items,
            blocks,
            selected: 0,
        };
    }

    fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let new_idx = match self.history_index {
            None => self.input_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_index = Some(new_idx);
        self.input.set_content(self.input_history[new_idx].clone());
    }

    fn history_next(&mut self) {
        match self.history_index {
            None => {}
            Some(i) => {
                if i + 1 < self.input_history.len() {
                    self.history_index = Some(i + 1);
                    self.input.set_content(self.input_history[i + 1].clone());
                } else {
                    // Past the end — clear to empty
                    self.history_index = None;
                    self.input.set_content(String::new());
                }
            }
        }
    }

    fn cycle_focus_forward(&mut self) {
        self.focus = match self.focus {
            Focus::Input => {
                if self.sidebar_collapsed {
                    Focus::Conversation
                } else {
                    Focus::Sidebar
                }
            }
            Focus::Sidebar => Focus::Conversation,
            Focus::Conversation => Focus::Input,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focus = match self.focus {
            Focus::Input => Focus::Conversation,
            Focus::Conversation => {
                if self.sidebar_collapsed {
                    Focus::Input
                } else {
                    Focus::Sidebar
                }
            }
            Focus::Sidebar => Focus::Input,
        };
    }

    fn update_focus_state(&mut self) {
        self.input.focused = self.focus == Focus::Input;
    }

    pub fn open_external_editor(&mut self) {
        use crossterm::{
            execute,
            terminal::{
                EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            },
        };

        // Write current content to temp file
        let tmp = match tempfile::NamedTempFile::new() {
            Ok(t) => t,
            Err(_) => return,
        };
        let path = tmp.path().to_path_buf();
        if std::fs::write(&path, &self.input.content).is_err() {
            return;
        }

        // Suspend terminal
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

        // Spawn editor
        let status = std::process::Command::new(&self.editor_command)
            .arg(&path)
            .status();

        // Restore terminal
        let _ = execute!(std::io::stdout(), EnterAlternateScreen);
        let _ = enable_raw_mode();

        // Force a full redraw on next frame
        self.needs_redraw = true;

        // Read back content if editor succeeded
        if let Ok(exit) = status
            && exit.success()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            self.input.set_content(content);
        }
    }

    /// Apply a parsed config to the running app (live reload).
    fn apply_config(&mut self, config: &tuillem_config::Config) {
        // Theme
        self.config_themes = config.themes.clone();
        self.config_theme = config.theme.clone();
        self.color_mode = config.ui.color_mode.clone();
        let resolved = crate::theme::resolve_color_mode(&self.color_mode);
        self.theme = Theme::from_config(&config.theme, &config.themes)
            .adapt_to_color_mode(resolved);

        // Editor
        self.editor_command = config.editor.clone();

        // Keybindings
        self.config_keybindings = match config.keybindings {
            tuillem_config::KeybindingPreset::Vim => "vim".to_string(),
            tuillem_config::KeybindingPreset::Emacs => "emacs".to_string(),
            tuillem_config::KeybindingPreset::Default => "default".to_string(),
        };

        // UI settings
        self.config_show_thinking = config.ui.show_thinking;
        self.config_show_token_usage = config.ui.show_token_usage;
        self.config_mouse = config.ui.mouse;
        self.show_stats = config.ui.show_stats;
        self.layout = config.ui.layout.clone();
        self.date_format = config.ui.date_format.clone();
        self.scroll_lines = config.ui.scroll_lines;
        self.command_prefix = config.ui.command_prefix.clone();
        self.nerd_fonts = config.ui.nerd_fonts;

        // Defaults
        self.default_provider = config.defaults.provider.clone().unwrap_or_else(|| {
            config
                .providers
                .first()
                .map(|p| p.name.clone())
                .unwrap_or_default()
        });
        self.default_model = config.defaults.model.clone().unwrap_or_else(|| {
            config
                .providers
                .first()
                .and_then(|p| {
                    p.default_model
                        .clone()
                        .or_else(|| p.models.first().cloned())
                })
                .unwrap_or_default()
        });

        // System prompt
        self.config_system_prompt = config.defaults.system_prompt.clone().unwrap_or_default();

        // Available models
        self.available_models = config
            .providers
            .iter()
            .map(|p| (p.name.clone(), p.models.clone()))
            .collect();
    }

    /// Open config YAML in editor with validation loop.
    /// Edits a copy; only replaces the real config if it parses correctly.
    fn edit_config_yaml(&mut self) {
        use crossterm::{
            execute,
            terminal::{
                EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            },
        };

        let config_path = tuillem_config::Config::default_path();

        // Read current config (or create empty)
        let original_yaml = std::fs::read_to_string(&config_path).unwrap_or_default();

        // Write to a temp file for editing
        let tmp = match tempfile::NamedTempFile::new_in(
            config_path.parent().unwrap_or(std::path::Path::new(".")),
        ) {
            Ok(t) => t,
            Err(_) => {
                self.state.error = Some("Failed to create temp file".to_string());
                return;
            }
        };
        let tmp_path = tmp.path().to_path_buf();
        if std::fs::write(&tmp_path, &original_yaml).is_err() {
            self.state.error = Some("Failed to write temp config".to_string());
            return;
        }

        loop {
            // Suspend terminal and open editor
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

            let status = std::process::Command::new(&self.editor_command)
                .arg(&tmp_path)
                .status();

            // Restore terminal
            let _ = execute!(std::io::stdout(), EnterAlternateScreen);
            let _ = enable_raw_mode();
            self.needs_redraw = true;

            // Check if editor exited cleanly
            let editor_ok = status.is_ok_and(|s| s.success());
            if !editor_ok {
                // Editor failed or user quit — discard changes
                let _ = std::fs::remove_file(&tmp_path);
                self.state.status_message = Some((
                    "Config edit cancelled".to_string(),
                    std::time::Instant::now(),
                ));
                return;
            }

            // Read the edited file
            let edited_yaml = match std::fs::read_to_string(&tmp_path) {
                Ok(y) => y,
                Err(_) => {
                    let _ = std::fs::remove_file(&tmp_path);
                    self.state.error = Some("Failed to read edited config".to_string());
                    return;
                }
            };

            // Validate by parsing
            match tuillem_config::Config::from_yaml(&edited_yaml) {
                Ok(new_config) => {
                    // Valid — replace the real config
                    if let Some(parent) = config_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match std::fs::write(&config_path, &edited_yaml) {
                        Ok(_) => {
                            let _ = std::fs::remove_file(&tmp_path);
                            // Reload config into the running app
                            self.apply_config(&new_config);
                            self.state.status_message = Some((
                                "Config saved and applied.".to_string(),
                                std::time::Instant::now(),
                            ));
                            return;
                        }
                        Err(e) => {
                            let _ = std::fs::remove_file(&tmp_path);
                            self.state.error = Some(format!("Failed to write config: {e}"));
                            return;
                        }
                    }
                }
                Err(e) => {
                    // Invalid YAML — show error and prompt
                    let _ = disable_raw_mode();
                    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);

                    eprintln!("\n  Config validation error:\n");
                    eprintln!("  {}\n", e);
                    eprintln!("  Press 'r' to re-edit, or any other key to discard changes.");

                    // Wait for a keypress
                    if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                        let _ = execute!(std::io::stdout(), EnterAlternateScreen);
                        let _ = enable_raw_mode();
                        self.needs_redraw = true;

                        if key.code == crossterm::event::KeyCode::Char('r') {
                            // Loop back to re-edit
                            continue;
                        } else {
                            // Discard
                            let _ = std::fs::remove_file(&tmp_path);
                            self.state.status_message = Some((
                                "Config changes discarded".to_string(),
                                std::time::Instant::now(),
                            ));
                            return;
                        }
                    } else {
                        let _ = execute!(std::io::stdout(), EnterAlternateScreen);
                        let _ = enable_raw_mode();
                        self.needs_redraw = true;
                        let _ = std::fs::remove_file(&tmp_path);
                        return;
                    }
                }
            }
        }
    }
}
