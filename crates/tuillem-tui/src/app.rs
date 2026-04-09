use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tracing::{debug, warn};
use tokio::sync::mpsc;
use tuillem_core::{
    actions::{Action, Event},
    state::AppState,
};

use crate::{conversation::Conversation, input::Input, sidebar::Sidebar, theme::Theme};

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
    pub popup: Option<SelectionPopup>,
    pub available_models: Vec<(String, Vec<String>)>, // (provider_name, [model_ids])
}

impl App {
    pub fn new(
        state: AppState,
        theme: Theme,
        action_tx: mpsc::UnboundedSender<Action>,
        editor_command: String,
        available_models: Vec<(String, Vec<String>)>,
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
            popup: None,
            available_models,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.area();

        // Horizontal split: sidebar | right
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(1)])
            .split(size);

        // Right panel: conversation | input (5 lines)
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(5)])
            .split(h_chunks[1]);

        self.sidebar
            .render(frame, h_chunks[0], &self.state.sessions, &self.theme);

        self.conversation.render(
            frame,
            v_chunks[0],
            &self.state.messages,
            &self.state.streaming_text,
            &self.state.streaming_thinking,
            self.state.is_streaming,
            &self.state.current_model,
            self.state.error.as_deref(),
            &self.theme,
        );

        self.input.render(
            frame,
            v_chunks[1],
            &self.state.current_model,
            self.state.is_streaming,
            &self.theme,
        );

        // Draw popup on top if active
        if let Some(ref popup) = self.popup {
            self.draw_popup(frame, size, popup);
        }
    }

    fn draw_popup(&self, frame: &mut Frame, area: Rect, popup: &SelectionPopup) {
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
        // Handle popup keys first
        if self.popup.is_some() {
            self.handle_popup_key(key);
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
                _ => {}
            }
        }

        // More Ctrl bindings (these are safe in raw mode)
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('o') => {
                    self.open_model_popup();
                    return;
                }
                KeyCode::Char('t') => {
                    self.open_provider_popup();
                    return;
                }
                _ => {}
            }
        }

        // Tab / Shift+Tab cycle focus
        if key.code == KeyCode::Tab {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                self.cycle_focus_backward();
            } else {
                self.cycle_focus_forward();
            }
            self.update_focus_state();
            return;
        }

        // Escape returns to input
        if key.code == KeyCode::Esc {
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

    fn handle_sidebar_key(&mut self, key: KeyEvent) {
        if self.sidebar.search_focused {
            match key.code {
                KeyCode::Esc => {
                    self.sidebar.search_focused = false;
                    self.sidebar.search_input.clear();
                }
                KeyCode::Enter => {
                    self.sidebar.search_focused = false;
                }
                KeyCode::Backspace => {
                    self.sidebar.search_input.pop();
                }
                KeyCode::Char(c) => {
                    self.sidebar.search_input.push(c);
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
                        debug!("Sending Action::SendMessage, content length={}", content.len());
                        if let Err(e) = self.action_tx.send(Action::SendMessage { content }) {
                            warn!("Failed to send action to coordinator: {e}");
                            self.state.error = Some(format!("Internal error: coordinator disconnected ({e})"));
                        }
                    }
                }
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.open_external_editor();
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

    fn handle_popup_key(&mut self, key: KeyEvent) {
        let popup = match &mut self.popup {
            Some(p) => p,
            None => return,
        };

        match key.code {
            KeyCode::Esc => {
                self.popup = None;
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
                self.popup = None;

                match kind {
                    PopupKind::Model => {
                        let _ = self.action_tx.send(Action::SwitchModel {
                            provider: self.state.current_provider.clone(),
                            model: selected_item,
                        });
                    }
                    PopupKind::Provider => {
                        // Switch provider and use its first model
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

        self.popup = Some(SelectionPopup {
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

        self.popup = Some(SelectionPopup {
            title: "Switch Provider".to_string(),
            items: providers,
            selected: current_idx,
            kind: PopupKind::Provider,
        });
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

        // Read back content if editor succeeded
        if let Ok(exit) = status
            && exit.success()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            self.input.set_content(content);
        }
    }
}
