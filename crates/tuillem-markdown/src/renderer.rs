use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

use crate::highlight::Highlighter;
use crate::parser::{InlineElement, ListItem, MdElement};

pub struct MdRenderer {
    highlighter: Highlighter,
    heading_color: Color,
    link_color: Color,
    code_bg: Color,
    code_fg: Color,
    blockquote_color: Color,
    border_color: Color,
    max_width: usize,
}

impl MdRenderer {
    pub fn new() -> Self {
        Self {
            highlighter: Highlighter::new(),
            heading_color: Color::Rgb(137, 180, 250),
            link_color: Color::Rgb(116, 199, 236),
            code_bg: Color::Rgb(17, 17, 27),
            code_fg: Color::Rgb(205, 214, 244),
            blockquote_color: Color::Rgb(108, 112, 134),
            border_color: Color::Rgb(69, 71, 90),
            max_width: 0, // 0 = no limit
        }
    }

    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }

    pub fn render(&self, elements: &[MdElement]) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        for element in elements {
            match element {
                MdElement::Heading(level, text) => {
                    let (prefix, style) = match level {
                        1 => (
                            "═══ ",
                            Style::default()
                                .fg(self.heading_color)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        ),
                        2 => (
                            "── ",
                            Style::default()
                                .fg(self.heading_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        3 => (
                            "─ ",
                            Style::default()
                                .fg(self.heading_color)
                                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                        ),
                        4 => (
                            "  ",
                            Style::default()
                                .fg(self.heading_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        _ => (
                            "  ",
                            Style::default()
                                .fg(self.heading_color)
                                .add_modifier(Modifier::ITALIC),
                        ),
                    };
                    lines.push(Line::from(vec![
                        Span::styled(
                            prefix.to_string(),
                            Style::default().fg(self.border_color),
                        ),
                        Span::styled(text.clone(), style),
                    ]));
                    lines.push(Line::from(""));
                }

                MdElement::Paragraph(inlines) => {
                    lines.push(Line::from(self.inline_to_spans(inlines)));
                    lines.push(Line::from(""));
                }

                MdElement::CodeBlock { language, code } => {
                    let lang_display = if language.is_empty() {
                        "code".to_string()
                    } else {
                        language.clone()
                    };

                    // Header
                    lines.push(Line::from(Span::styled(
                        format!("┌─ {}", lang_display),
                        Style::default().fg(self.border_color),
                    )));

                    // Syntax highlighted lines
                    let highlighted = self.highlighter.highlight(code, language);
                    for hl_spans in highlighted {
                        let mut line_spans: Vec<Span<'static>> = Vec::new();
                        line_spans.push(Span::styled(
                            "│ ".to_string(),
                            Style::default().fg(self.border_color),
                        ));
                        line_spans.extend(hl_spans);
                        lines.push(Line::from(line_spans));
                    }

                    // Footer
                    lines.push(Line::from(Span::styled(
                        "└─".to_string(),
                        Style::default().fg(self.border_color),
                    )));
                    lines.push(Line::from(""));
                }

                MdElement::InlineCode(text) => {
                    lines.push(Line::from(Span::styled(
                        format!(" {} ", text),
                        Style::default().bg(self.code_bg).fg(self.code_fg),
                    )));
                }

                MdElement::List(items) => {
                    self.render_list_items(&mut lines, items, false);
                    lines.push(Line::from(""));
                }

                MdElement::OrderedList(items) => {
                    self.render_list_items(&mut lines, items, true);
                    lines.push(Line::from(""));
                }

                MdElement::BlockQuote(inner_elements) => {
                    let inner_text = self.render(inner_elements);
                    for line in inner_text.lines {
                        let mut spans: Vec<Span<'static>> = Vec::new();
                        spans.push(Span::styled(
                            "▎ ".to_string(),
                            Style::default().fg(self.blockquote_color),
                        ));
                        spans.extend(line.spans);
                        lines.push(Line::from(spans));
                    }
                    lines.push(Line::from(""));
                }

                MdElement::Table { headers, rows } => {
                    self.render_table(&mut lines, headers, rows);
                    lines.push(Line::from(""));
                }

                MdElement::ThematicBreak => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(self.border_color),
                    )));
                    lines.push(Line::from(""));
                }
            }
        }

        Text::from(lines)
    }

    fn render_list_items(&self, lines: &mut Vec<Line<'static>>, items: &[ListItem], ordered: bool) {
        for (idx, item) in items.iter().enumerate() {
            let prefix = if ordered {
                format!("  {}. ", idx + 1)
            } else {
                "  • ".to_string()
            };
            let mut spans = vec![Span::raw(prefix)];
            spans.extend(self.inline_to_spans(&item.content));
            lines.push(Line::from(spans));
        }
    }

    fn render_table(
        &self,
        lines: &mut Vec<Line<'static>>,
        headers: &[String],
        rows: &[Vec<String>],
    ) {
        if headers.is_empty() {
            return;
        }

        let num_cols = headers.len();

        // Measure natural column widths
        let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        // Rendered width: each col = content_width + 2 (padding " x ") + 1 (border │)
        // Plus leading border │, so total = sum(widths) + 3*num_cols + 1
        let overhead = 3 * num_cols + 1;

        if self.max_width > 0 {
            let content_budget = self.max_width.saturating_sub(overhead);

            if content_budget < num_cols * 3 {
                self.render_table_compact(lines, headers, rows);
                return;
            }

            let total_natural: usize = widths.iter().sum();
            if total_natural > content_budget {
                // Shrink columns proportionally to fit
                widths = widths
                    .iter()
                    .map(|w| {
                        let share = (*w as f64 / total_natural as f64 * content_budget as f64) as usize;
                        share.max(3)
                    })
                    .collect();

                // Hard clamp: if rounding pushed us over, trim the widest column
                while widths.iter().sum::<usize>() > content_budget {
                    if let Some(max_idx) = widths.iter().enumerate().max_by_key(|(_, w)| *w).map(|(i, _)| i) {
                        widths[max_idx] = widths[max_idx].saturating_sub(1);
                    } else {
                        break;
                    }
                }
            }
        }

        let border_style = Style::default().fg(self.border_color);

        // Top border: ┌───┬───┐
        let mut top = String::from("┌");
        for (i, w) in widths.iter().enumerate() {
            top.push_str(&"─".repeat(w + 2));
            if i < num_cols - 1 {
                top.push('┬');
            }
        }
        top.push('┐');
        lines.push(Line::from(Span::styled(top, border_style)));

        // Header row
        let mut header_spans: Vec<Span<'static>> = Vec::new();
        header_spans.push(Span::styled("│".to_string(), border_style));
        for (i, h) in headers.iter().enumerate() {
            let w = widths[i];
            let truncated = truncate_str(h, w);
            let padded = format!(" {:<width$} ", truncated, width = w);
            header_spans.push(Span::styled(
                padded,
                Style::default().add_modifier(Modifier::BOLD),
            ));
            header_spans.push(Span::styled("│".to_string(), border_style));
        }
        lines.push(Line::from(header_spans));

        // Separator: ├───┼───┤
        let mut sep = String::from("├");
        for (i, w) in widths.iter().enumerate() {
            sep.push_str(&"─".repeat(w + 2));
            if i < num_cols - 1 {
                sep.push('┼');
            }
        }
        sep.push('┤');
        lines.push(Line::from(Span::styled(sep, border_style)));

        // Data rows
        for row in rows {
            let mut row_spans: Vec<Span<'static>> = Vec::new();
            row_spans.push(Span::styled("│".to_string(), border_style));
            for (i, cell) in row.iter().enumerate() {
                let w = if i < widths.len() {
                    widths[i]
                } else {
                    cell.len()
                };
                let truncated = truncate_str(cell, w);
                let padded = format!(" {:<width$} ", truncated, width = w);
                row_spans.push(Span::raw(padded));
                row_spans.push(Span::styled("│".to_string(), border_style));
            }
            lines.push(Line::from(row_spans));
        }

        // Bottom border: └───┴───┘
        let mut bottom = String::from("└");
        for (i, w) in widths.iter().enumerate() {
            bottom.push_str(&"─".repeat(w + 2));
            if i < num_cols - 1 {
                bottom.push('┴');
            }
        }
        bottom.push('┘');
        lines.push(Line::from(Span::styled(bottom, border_style)));
    }

    /// Render a table in compact card layout when it's too wide for the terminal.
    /// Each row becomes a card with "header: value" pairs.
    fn render_table_compact(
        &self,
        lines: &mut Vec<Line<'static>>,
        headers: &[String],
        rows: &[Vec<String>],
    ) {
        let border_style = Style::default().fg(self.border_color);
        let label_style = Style::default().add_modifier(Modifier::BOLD);

        for (row_idx, row) in rows.iter().enumerate() {
            // Row separator
            if row_idx > 0 {
                lines.push(Line::from(Span::styled(
                    "  ─ ─ ─".to_string(),
                    border_style,
                )));
            }
            for (i, cell) in row.iter().enumerate() {
                let header = headers.get(i).map(|s| s.as_str()).unwrap_or("?");
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", header), label_style),
                    Span::raw(cell.clone()),
                ]));
            }
        }
    }

    fn inline_to_spans(&self, inlines: &[InlineElement]) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for inline in inlines {
            match inline {
                InlineElement::Text(t) => {
                    spans.push(Span::raw(t.clone()));
                }
                InlineElement::Bold(t) => {
                    spans.push(Span::styled(
                        t.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                }
                InlineElement::Italic(t) => {
                    spans.push(Span::styled(
                        t.clone(),
                        Style::default().add_modifier(Modifier::ITALIC),
                    ));
                }
                InlineElement::Strikethrough(t) => {
                    spans.push(Span::styled(
                        t.clone(),
                        Style::default().add_modifier(Modifier::CROSSED_OUT),
                    ));
                }
                InlineElement::Code(t) => {
                    spans.push(Span::styled(
                        format!(" {} ", t),
                        Style::default().bg(self.code_bg).fg(self.code_fg),
                    ));
                }
                InlineElement::Link { text, url } => {
                    spans.push(Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(self.link_color)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    spans.push(Span::styled(
                        format!(" ({})", url),
                        Style::default().fg(self.link_color),
                    ));
                }
            }
        }
        spans
    }
}

impl Default for MdRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        let truncated: String = s.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_render_heading() {
        let elements = parser::parse("# Hello");
        let renderer = MdRenderer::new();
        let text = renderer.render(&elements);
        let first_line = &text.lines[0];
        let content: String = first_line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("Hello"));
        assert!(content.contains("═══"));
    }

    #[test]
    fn test_render_code_block() {
        let elements = parser::parse("```rust\nfn main() {}\n```");
        let renderer = MdRenderer::new();
        let text = renderer.render(&elements);
        let all_text: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(all_text.contains("rust"));
        assert!(all_text.contains("└─"));
    }

    #[test]
    fn test_render_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let elements = parser::parse(md);
        let renderer = MdRenderer::new();
        let text = renderer.render(&elements);
        let all_text: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(all_text.contains("┌"));
        assert!(all_text.contains("┘"));
    }

    #[test]
    fn test_render_list() {
        let elements = parser::parse("- one\n- two");
        let renderer = MdRenderer::new();
        let text = renderer.render(&elements);
        let all_text: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(all_text.contains("•"));
        assert!(all_text.contains("one"));
    }
}
