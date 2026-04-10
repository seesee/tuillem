use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::theme::Theme;

pub fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let popup_width = 60u16.min(area.width.saturating_sub(6));
    let popup_height = 34u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let accent = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme.thinking_fg);
    let normal = Style::default().fg(theme.fg);

    let lines = vec![
        Line::from(Span::styled("Global", accent)),
        Line::from(vec![
            Span::styled("  Tab       ", normal),
            Span::styled("Cycle focus (sidebar/conversation/input)", dim),
        ]),
        Line::from(vec![
            Span::styled("  Esc       ", normal),
            Span::styled("Close overlay / cancel stream / back to input", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C    ", normal),
            Span::styled("Quit", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+N    ", normal),
            Span::styled("New conversation", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+K    ", normal),
            Span::styled("Command palette", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+O    ", normal),
            Span::styled("Switch model", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+T    ", normal),
            Span::styled("Switch provider", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+S    ", normal),
            Span::styled("Settings", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Y    ", normal),
            Span::styled("Copy last response to clipboard", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+B    ", normal),
            Span::styled("Copy code blocks from last response", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+H    ", normal),
            Span::styled("This help screen", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+L    ", normal),
            Span::styled("Toggle sidebar", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled("Input", accent)),
        Line::from(vec![
            Span::styled("  Enter     ", normal),
            Span::styled("Send message (or advance N lines if empty)", dim),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Ent ", normal),
            Span::styled("Insert newline", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+E    ", normal),
            Span::styled("Open in external editor", dim),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down   ", normal),
            Span::styled("Browse input history", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+R    ", normal),
            Span::styled("Regenerate last response", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled("Sidebar", accent)),
        Line::from(vec![
            Span::styled("  j/k       ", normal),
            Span::styled("Navigate sessions", dim),
        ]),
        Line::from(vec![
            Span::styled("  Enter     ", normal),
            Span::styled("Select session", dim),
        ]),
        Line::from(vec![
            Span::styled("  /         ", normal),
            Span::styled("Search sessions", dim),
        ]),
        Line::from(vec![
            Span::styled("  d         ", normal),
            Span::styled("Delete session (y/n confirm)", dim),
        ]),
        Line::from(vec![
            Span::styled("  r         ", normal),
            Span::styled("Rename session", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled("Conversation", accent)),
        Line::from(vec![
            Span::styled("  j/k       ", normal),
            Span::styled("Scroll up/down", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+D/U  ", normal),
            Span::styled("Page down/up", dim),
        ]),
        Line::from(vec![
            Span::styled("  g/G       ", normal),
            Span::styled("Jump to top/bottom", dim),
        ]),
        Line::from(vec![
            Span::styled("  t         ", normal),
            Span::styled("Toggle thinking block", dim),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Line::from(Span::styled(" Keyboard Shortcuts ", accent)))
        .title_bottom(Line::from(Span::styled(" Esc:close ", dim)))
        .style(Style::default().bg(theme.bg));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}
