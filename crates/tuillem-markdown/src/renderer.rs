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
        let overhead = 3 * num_cols + 1; // │ + " " + content + " " per col, plus leading │

        // If we can't fit even minimal columns, use card layout
        if self.max_width > 0 && self.max_width < overhead + num_cols * 4 {
            self.render_table_compact(lines, headers, rows);
            return;
        }

        // Calculate column widths: short columns keep natural width,
        // long columns share the remaining space and wrap content.
        let content_budget = if self.max_width > 0 {
            self.max_width.saturating_sub(overhead)
        } else {
            usize::MAX
        };

        // Natural widths (by char count)
        let mut natural: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
        for row in rows {
            for (i, cell) in row.iter().enumerate() {
                if i < natural.len() {
                    natural[i] = natural[i].max(cell.chars().count());
                }
            }
        }

        let widths = fit_columns(&natural, content_budget);
        let border_style = Style::default().fg(self.border_color);

        // Top border
        lines.push(Line::from(Span::styled(
            build_border("┌", "┬", "┐", &widths),
            border_style,
        )));

        // Header row (may wrap)
        let header_cells: Vec<&str> = headers.iter().map(|h| h.as_str()).collect();
        self.render_table_row(lines, &header_cells, &widths, border_style, true);

        // Separator
        lines.push(Line::from(Span::styled(
            build_border("├", "┼", "┤", &widths),
            border_style,
        )));

        // Data rows
        for row in rows {
            let cells: Vec<&str> = row.iter().map(|c| c.as_str()).collect();
            self.render_table_row(lines, &cells, &widths, border_style, false);
        }

        // Bottom border
        lines.push(Line::from(Span::styled(
            build_border("└", "┴", "┘", &widths),
            border_style,
        )));
    }

    /// Render a single table row, wrapping cell content across multiple terminal lines.
    fn render_table_row(
        &self,
        lines: &mut Vec<Line<'static>>,
        cells: &[&str],
        widths: &[usize],
        border_style: Style,
        bold: bool,
    ) {
        let num_cols = widths.len();
        // Wrap each cell into lines
        let wrapped: Vec<Vec<String>> = (0..num_cols)
            .map(|i| {
                let text = cells.get(i).copied().unwrap_or("");
                wrap_cell(text, widths[i])
            })
            .collect();

        let max_lines = wrapped.iter().map(|w| w.len()).max().unwrap_or(1);

        let cell_style = if bold {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        for line_idx in 0..max_lines {
            let mut spans: Vec<Span<'static>> = Vec::new();
            spans.push(Span::styled("│".to_string(), border_style));
            for (col, w) in widths.iter().enumerate() {
                let text = wrapped[col]
                    .get(line_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let padded = format!(" {:<width$} ", text, width = w);
                spans.push(Span::styled(padded, cell_style));
                spans.push(Span::styled("│".to_string(), border_style));
            }
            lines.push(Line::from(spans));
        }
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

/// Build a horizontal border line like ┌───┬───┐
fn build_border(left: &str, mid: &str, right: &str, widths: &[usize]) -> String {
    let mut s = left.to_string();
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&"─".repeat(w + 2));
        if i < widths.len() - 1 {
            s.push_str(mid);
        }
    }
    s.push_str(right);
    s
}

/// Fit columns into a budget. Short columns keep natural width, long ones share the rest.
fn fit_columns(natural: &[usize], budget: usize) -> Vec<usize> {
    if budget == usize::MAX {
        return natural.to_vec();
    }

    let total: usize = natural.iter().sum();
    if total <= budget {
        return natural.to_vec();
    }

    let num_cols = natural.len();
    // Threshold: columns at or below this width keep their natural size
    // Start with the median and iterate
    let mut sorted = natural.to_vec();
    sorted.sort();
    let median = sorted.get(num_cols / 2).copied().unwrap_or(10);
    let threshold = median.min(budget / num_cols);

    let mut widths = vec![0usize; num_cols];
    let mut remaining_budget = budget;
    let mut long_cols = Vec::new();

    for (i, &nat) in natural.iter().enumerate() {
        if nat <= threshold {
            widths[i] = nat;
            remaining_budget = remaining_budget.saturating_sub(nat);
        } else {
            long_cols.push(i);
        }
    }

    // Distribute remaining budget equally among long columns
    if !long_cols.is_empty() {
        let per_col = remaining_budget / long_cols.len();
        let mut leftover = remaining_budget % long_cols.len();
        for &i in &long_cols {
            widths[i] = per_col.max(4);
            if leftover > 0 {
                widths[i] += 1;
                leftover -= 1;
            }
        }
    }

    // Final clamp
    while widths.iter().sum::<usize>() > budget {
        if let Some(max_idx) = widths.iter().enumerate().max_by_key(|(_, w)| *w).map(|(i, _)| i) {
            widths[max_idx] = widths[max_idx].saturating_sub(1);
        } else {
            break;
        }
    }

    widths
}

/// Wrap text into lines that fit within max_width characters.
fn wrap_cell(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    if text.chars().count() <= max_width {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_len = 0;

    for word in text.split_whitespace() {
        let wlen = word.chars().count();
        if current_len == 0 {
            // Force-break very long words
            if wlen > max_width {
                let mut chars = word.chars();
                while chars.clone().count() > 0 {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if chunk.is_empty() {
                        break;
                    }
                    result.push(chunk);
                }
            } else {
                current = word.to_string();
                current_len = wlen;
            }
        } else if current_len + 1 + wlen <= max_width {
            current.push(' ');
            current.push_str(word);
            current_len += 1 + wlen;
        } else {
            result.push(current);
            if wlen > max_width {
                let mut chars = word.chars();
                while chars.clone().count() > 0 {
                    let chunk: String = chars.by_ref().take(max_width).collect();
                    if chunk.is_empty() {
                        break;
                    }
                    result.push(chunk);
                }
                current = String::new();
                current_len = 0;
            } else {
                current = word.to_string();
                current_len = wlen;
            }
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

fn truncate_str(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len <= 1 {
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
