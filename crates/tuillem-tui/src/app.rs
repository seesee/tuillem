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
    pub config_theme: String,
    pub config_keybindings: String,
    pub config_show_thinking: bool,
    pub config_show_token_usage: bool,
    pub config_mouse: bool,
    pub config_system_prompt: String,
    pub show_stats: bool,
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
            config_theme: "dark".to_string(),
            config_keybindings: "default".to_string(),
            config_show_thinking: false,
            config_show_token_usage: true,
            config_mouse: true,
            config_system_prompt: String::new(),
            show_stats: false,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Horizontal split: sidebar | right
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(1)])
            .split(size);

        // Right panel: conversation | [stats bar] | input (5 lines)
        let show_stats_bar = self.show_stats
            && !self.state.is_streaming
            && self.state.last_response_stats.is_some();

        let v_constraints = if show_stats_bar {
            vec![
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(5),
            ]
        } else {
            vec![Constraint::Min(1), Constraint::Length(5)]
        };
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(v_constraints)
            .split(h_chunks[1]);

        self.sidebar.render(
            frame,
            h_chunks[0],
            &self.state.sessions,
            self.focus == Focus::Sidebar,
            &self.theme,
        );

        self.conversation.render(
            frame,
            v_chunks[0],
            &self.state.messages,
            &self.state.streaming_text,
            &self.state.streaming_thinking,
            self.state.is_streaming,
            &self.state.current_model,
            self.state.error.as_deref(),
            self.state.status_message.as_deref(),
            self.focus == Focus::Conversation,
            &self.theme,
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
                render_help(frame, size, &self.theme);
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
                approx, stats.tokens_in, stats.tokens_out,
                toks_per_sec, ctx_pct
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

        // Rebuild input history when messages are loaded
        if let Event::MessagesLoaded { messages } = event {
            self.input_history = messages
                .iter()
                .filter(|m| m.role == "user")
                .filter_map(|m| m.content.clone())
                .collect();
            self.history_index = None;
        }

        // Auto-scroll on streaming and message events
        match event {
            Event::StreamStarted
            | Event::StreamDelta { .. }
            | Event::ThinkingDelta { .. }
            | Event::MessagesLoaded { .. }
            | Event::ResponseError { .. } => {
                self.conversation.scroll_to_bottom();
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
                    self.overlay = Overlay::Help;
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
            Overlay::Help => {
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Char('q')
                    || (key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('h'))
                {
                    self.overlay = Overlay::None;
                }
            }
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
                        KeyCode::Char('j') | KeyCode::Down => {
                            panel.move_down();
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            panel.move_up();
                        }
                        KeyCode::Enter => {
                            panel.enter();
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
                        self.state.status_message =
                            Some("Copied code block to clipboard".to_string());
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
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sidebar.move_up(1);
            }
            KeyCode::Char('g') => {
                self.sidebar.selected = 0;
                self.sidebar.scroll_offset = 0;
            }
            KeyCode::Char('G') => {
                if session_count > 0 {
                    self.sidebar.selected = session_count - 1;
                }
            }
            KeyCode::Enter => {
                let filtered = self.sidebar.filtered_sessions(&self.state.sessions);
                if let Some(session) = filtered.get(self.sidebar.selected) {
                    let _ = self.action_tx.send(Action::SelectSession {
                        id: session.id.clone(),
                    });
                }
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let filtered = self.sidebar.filtered_sessions(&self.state.sessions);
                if let Some(session) = filtered.get(self.sidebar.selected) {
                    let _ = self.action_tx.send(Action::DeleteSession {
                        id: session.id.clone(),
                    });
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
                } else {
                    let content = self.input.take_content();
                    if !content.trim().is_empty() {
                        self.state.error = None;
                        self.state.status_message = None;
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
            &self.state.current_provider,
            &self.state.current_model,
            &self.editor_command,
            &self.config_theme,
            &self.config_keybindings,
            self.config_show_thinking,
            self.config_show_token_usage,
            self.config_mouse,
            &self.config_system_prompt,
            self.show_stats,
        );
        self.overlay = Overlay::Settings(panel);
    }

    fn save_settings(&mut self) {
        // Extract values from the settings panel before closing
        if let Overlay::Settings(ref panel) = self.overlay {
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
                self.state.status_message = Some("Copied response to clipboard".to_string());
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
                self.state.status_message = Some("Copied code block to clipboard".to_string());
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
            Focus::Input => Focus::Sidebar,
            Focus::Sidebar => Focus::Conversation,
            Focus::Conversation => Focus::Input,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focus = match self.focus {
            Focus::Input => Focus::Conversation,
            Focus::Conversation => Focus::Sidebar,
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
}
