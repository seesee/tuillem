use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    SwitchModel,
    SwitchProvider,
    NewConversation,
    RegenerateResponse,
    SaveTranscript,
    OpenInEditor,
    ToggleThinking,
}

impl ControlAction {
    pub const ALL: &'static [ControlAction] = &[
        ControlAction::SwitchModel,
        ControlAction::SwitchProvider,
        ControlAction::NewConversation,
        ControlAction::RegenerateResponse,
        ControlAction::SaveTranscript,
        ControlAction::OpenInEditor,
        ControlAction::ToggleThinking,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ControlAction::SwitchModel => "Switch Model",
            ControlAction::SwitchProvider => "Switch Provider",
            ControlAction::NewConversation => "New Conversation",
            ControlAction::RegenerateResponse => "Regenerate Response",
            ControlAction::SaveTranscript => "Save Transcript",
            ControlAction::OpenInEditor => "Open in Editor",
            ControlAction::ToggleThinking => "Toggle Thinking",
        }
    }

    pub fn hint(&self) -> &'static str {
        match self {
            ControlAction::SwitchModel => "Ctrl+K > m",
            ControlAction::SwitchProvider => "Ctrl+K > p",
            ControlAction::NewConversation => "Ctrl+N",
            ControlAction::RegenerateResponse => "Ctrl+R",
            ControlAction::SaveTranscript => "",
            ControlAction::OpenInEditor => "Ctrl+E",
            ControlAction::ToggleThinking => "",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControlPanel {
    pub selected: usize,
}

impl ControlPanel {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < ControlAction::ALL.len() {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn selected_action(&self) -> ControlAction {
        ControlAction::ALL[self.selected]
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_width = 45u16.min(area.width.saturating_sub(10));
        let popup_height =
            (ControlAction::ALL.len() as u16 + 2).min(area.height.saturating_sub(6));
        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = ControlAction::ALL
            .iter()
            .enumerate()
            .map(|(i, action)| {
                let selected = i == self.selected;
                let style = if selected {
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.fg)
                };
                let marker = if selected { "▸ " } else { "  " };
                let hint = action.hint();
                let label = action.label();
                let inner_width = popup_width.saturating_sub(2) as usize;
                let pad = inner_width
                    .saturating_sub(marker.len() + label.len() + hint.len());
                let line = if hint.is_empty() {
                    Line::from(Span::styled(format!("{}{}", marker, label), style))
                } else {
                    Line::from(vec![
                        Span::styled(format!("{}{}", marker, label), style),
                        Span::styled(
                            format!("{:>width$}", hint, width = pad + hint.len()),
                            Style::default().fg(theme.thinking_fg),
                        ),
                    ])
                };
                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Line::from(Span::styled(
                " Commands ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )))
            .title_bottom(Line::from(Span::styled(
                " j/k:select  Enter:run  Esc:cancel ",
                Style::default().fg(theme.thinking_fg),
            )))
            .style(Style::default().bg(theme.bg));

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected));

        let list = List::new(items).block(block);
        frame.render_stateful_widget(list, popup_area, &mut list_state);
    }
}
