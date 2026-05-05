use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::theme::Theme;

pub fn render_help(frame: &mut Frame, area: Rect, theme: &Theme, scroll: u16) {
    let popup_width = 60u16.min(area.width.saturating_sub(6));
    let popup_height = 38u16.min(area.height.saturating_sub(4));
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
            Span::styled("  Alt+1/2/3 ", normal),
            Span::styled("Focus sidebar/conversation/input directly", dim),
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
            Span::styled("  Ctrl+P    ", normal),
            Span::styled("Switch provider", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+T    ", normal),
            Span::styled("Toggle thinking mode", dim),
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
        Line::from(vec![
            Span::styled("  Ctrl+G    ", normal),
            Span::styled("Redraw screen", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled("Input", accent)),
        Line::from(vec![
            Span::styled("  Enter     ", normal),
            Span::styled("Send message (or advance N lines if empty)", dim),
        ]),
        Line::from(vec![
            Span::styled("  Alt+Enter ", normal),
            Span::styled("Insert newline", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+U    ", normal),
            Span::styled("Clear message box", dim),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+X    ", normal),
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
        Line::from(""),
        Line::from(Span::styled("Slash Commands", accent)),
        Line::from(vec![
            Span::styled("  /help     ", normal),
            Span::styled("See /help for all slash commands", dim),
        ]),
    ];

    let total_lines = lines.len() as u16;
    // Inner height = popup_height - 2 (for top/bottom border)
    let inner_height = popup_height.saturating_sub(2);

    let scroll_hint = if total_lines > inner_height {
        " j/k:scroll  Esc:close "
    } else {
        " Esc:close "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(Line::from(Span::styled(" Keyboard Shortcuts ", accent)))
        .title_bottom(Line::from(Span::styled(scroll_hint, dim)))
        .style(Style::default().bg(theme.bg));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, popup_area);

    // Scrollbar if content exceeds popup
    if total_lines > inner_height {
        // Scrollbar area is the inner area of the popup (excluding borders)
        let inner_area = Rect::new(
            popup_area.x + 1,
            popup_area.y + 1,
            popup_area.width.saturating_sub(2),
            inner_height,
        );
        let max_scroll = (total_lines as usize).saturating_sub(inner_height as usize);
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(theme.border))
            .thumb_style(Style::default().fg(theme.accent));
        frame.render_stateful_widget(scrollbar, inner_area, &mut scrollbar_state);
    }
}

/// Returns the maximum scroll offset for the help overlay at the given area size.
pub fn help_max_scroll(area: Rect) -> u16 {
    let popup_height = 38u16.min(area.height.saturating_sub(4));
    let inner_height = popup_height.saturating_sub(2);
    let total_lines: u16 = 41; // number of lines in the help content
    total_lines.saturating_sub(inner_height)
}
