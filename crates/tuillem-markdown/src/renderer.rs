use std::sync::LazyLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

use crate::highlight::Highlighter;
use crate::parser::{InlineElement, MdElement};
use crate::width;

static HIGHLIGHTER: LazyLock<Highlighter> = LazyLock::new(Highlighter::new);

pub struct MdRenderer {
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
            heading_color: Color::Rgb(137, 180, 250),
            link_color: Color::Rgb(116, 199, 236),
            code_bg: Color::Rgb(17, 17, 27),
            code_fg: Color::Rgb(205, 214, 244),
            blockquote_color: Color::Rgb(108, 112, 134),
            border_color: Color::Rgb(69, 71, 90),
            max_width: 0,
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
                MdElement::Heading(level, inlines) => {
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
                    let text = inlines_to_plain_text(inlines);
                    lines.push(Line::from(vec![
                        Span::styled(prefix.to_string(), Style::default().fg(self.border_color)),
                        Span::styled(text, style),
                    ]));
                    lines.push(Line::from(""));
                }
                MdElement::Paragraph(inlines) => {
                    lines.push(Line::from(self.inline_to_spans(inlines)));
                    lines.push(Line::from(""));
                }
                MdElement::CodeBlock { language, code } => {
                    let lang_display = if language.is_empty() {
                        "code"
                    } else {
                        language
                    };
                    lines.push(Line::from(Span::styled(
                        format!("┌─ {}", lang_display),
                        Style::default().fg(self.border_color),
                    )));
                    let highlighted = HIGHLIGHTER.highlight(code, language);
                    for hl_spans in highlighted {
                        let mut line_spans = vec![Span::styled(
                            "│ ".to_string(),
                            Style::default().fg(self.border_color),
                        )];
                        line_spans.extend(hl_spans);
                        lines.push(Line::from(line_spans));
                    }
                    lines.push(Line::from(Span::styled(
                        "└─".to_string(),
                        Style::default().fg(self.border_color),
                    )));
                    lines.push(Line::from(""));
                }
                MdElement::List(items) => {
                    for item in items {
                        let mut spans = vec![Span::raw("  • ")];
                        spans.extend(self.inline_to_spans(&item.content));
                        lines.push(Line::from(spans));
                    }
                    lines.push(Line::from(""));
                }
                MdElement::OrderedList(items) => {
                    for (idx, item) in items.iter().enumerate() {
                        let mut spans = vec![Span::raw(format!("  {}. ", idx + 1))];
                        spans.extend(self.inline_to_spans(&item.content));
                        lines.push(Line::from(spans));
                    }
                    lines.push(Line::from(""));
                }
                MdElement::BlockQuote(inner) => {
                    let inner_text = self.render(inner);
                    for line in inner_text.lines {
                        let mut spans = vec![Span::styled(
                            "▎ ".to_string(),
                            Style::default().fg(self.blockquote_color),
                        )];
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

    fn render_table(
        &self,
        lines: &mut Vec<Line<'static>>,
        headers: &[Vec<InlineElement>],
        rows: &[Vec<Vec<InlineElement>>],
    ) {
        if headers.is_empty() {
            return;
        }
        let num_cols = headers.len();
        let border_style = Style::default().fg(self.border_color);

        // Convert to plain text for width measurement and wrapping
        let header_texts: Vec<String> = headers.iter().map(|h| inlines_to_plain_text(h)).collect();
        let row_texts: Vec<Vec<String>> = rows
            .iter()
            .map(|row| row.iter().map(|cell| inlines_to_plain_text(cell)).collect())
            .collect();

        let widths = compute_column_widths(&header_texts, &row_texts, num_cols, self.max_width);

        // Verify: total rendered width must not exceed max_width
        let total_rendered = widths.iter().sum::<usize>() + 3 * num_cols + 1;
        if self.max_width > 0 && total_rendered > self.max_width {
            tracing::warn!(
                "Table width {} exceeds max {}! widths={:?} num_cols={} overhead={}",
                total_rendered,
                self.max_width,
                widths,
                num_cols,
                3 * num_cols + 1
            );
        }

        lines.push(Line::from(Span::styled(
            build_border("┌", "┬", "┐", &widths),
            border_style,
        )));
        render_wrapped_row(lines, &header_texts, &widths, border_style, true);
        lines.push(Line::from(Span::styled(
            build_border("├", "┼", "┤", &widths),
            border_style,
        )));

        for (i, row) in row_texts.iter().enumerate() {
            render_wrapped_row(lines, row, &widths, border_style, false);
            if i < row_texts.len() - 1 {
                lines.push(Line::from(Span::styled(
                    build_border("├", "┼", "┤", &widths),
                    border_style,
                )));
            }
        }

        lines.push(Line::from(Span::styled(
            build_border("└", "┴", "┘", &widths),
            border_style,
        )));
    }

    fn inline_to_spans(&self, inlines: &[InlineElement]) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        self.collect_spans(inlines, Style::default(), &mut spans);
        spans
    }

    fn collect_spans(
        &self,
        inlines: &[InlineElement],
        base_style: Style,
        out: &mut Vec<Span<'static>>,
    ) {
        for inline in inlines {
            match inline {
                InlineElement::Text(t) => out.push(Span::styled(t.clone(), base_style)),
                InlineElement::Bold(inner) => {
                    self.collect_spans(inner, base_style.add_modifier(Modifier::BOLD), out)
                }
                InlineElement::Italic(inner) => {
                    self.collect_spans(inner, base_style.add_modifier(Modifier::ITALIC), out)
                }
                InlineElement::Strikethrough(inner) => {
                    self.collect_spans(inner, base_style.add_modifier(Modifier::CROSSED_OUT), out)
                }
                InlineElement::Code(t) => out.push(Span::styled(
                    format!(" {} ", t),
                    Style::default().bg(self.code_bg).fg(self.code_fg),
                )),
                InlineElement::Link { text, url } => {
                    out.push(Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(self.link_color)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                    out.push(Span::styled(
                        format!(" ({})", url),
                        Style::default().fg(self.link_color),
                    ));
                }
                InlineElement::SoftBreak => out.push(Span::raw(" ")),
            }
        }
    }
}

impl Default for MdRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// --- Table helpers ---

fn inlines_to_plain_text(inlines: &[InlineElement]) -> String {
    let mut s = String::new();
    for inline in inlines {
        match inline {
            InlineElement::Text(t) => s.push_str(t),
            InlineElement::Bold(inner)
            | InlineElement::Italic(inner)
            | InlineElement::Strikethrough(inner) => {
                s.push_str(&inlines_to_plain_text(inner));
            }
            InlineElement::Code(t) => s.push_str(t),
            InlineElement::Link { text, .. } => s.push_str(text),
            InlineElement::SoftBreak => s.push(' '),
        }
    }
    s
}

fn display_width(s: &str) -> usize {
    width::terminal_width(s)
}

/// Compute column widths to fit within max_width.
/// Short columns keep natural width. Long columns share remaining space and wrap.
fn compute_column_widths(
    headers: &[String],
    rows: &[Vec<String>],
    num_cols: usize,
    max_width: usize,
) -> Vec<usize> {
    let mut natural = vec![0usize; num_cols];
    for (i, h) in headers.iter().enumerate() {
        natural[i] = display_width(h);
    }
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                natural[i] = natural[i].max(display_width(cell));
            }
        }
    }

    let overhead = 3 * num_cols + 1;
    if max_width == 0 || max_width <= overhead {
        return natural;
    }
    let budget = max_width - overhead;

    let total_natural: usize = natural.iter().sum();
    tracing::debug!(
        "Table: {} cols, natural={:?} (sum={}), max_width={}, overhead={}, budget={}",
        num_cols,
        natural,
        total_natural,
        max_width,
        overhead,
        budget
    );
    if total_natural <= budget {
        tracing::debug!("Table fits naturally");
        return natural;
    }

    // Give each column at least its header width or fair_share, whichever is smaller
    let fair_share = budget / num_cols;
    let mut widths = vec![0usize; num_cols];
    let mut remaining = budget;
    let mut wide_indices = Vec::new();

    // Header widths as minimum for short columns
    let header_widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();

    for (i, &nat) in natural.iter().enumerate() {
        let min_w = header_widths.get(i).copied().unwrap_or(4).min(fair_share);
        if nat <= fair_share {
            widths[i] = nat;
            remaining = remaining.saturating_sub(nat);
        } else {
            widths[i] = min_w.max(4);
            remaining = remaining.saturating_sub(widths[i]);
            wide_indices.push(i);
        }
    }

    // Redistribute remaining to wide columns
    if !wide_indices.is_empty() && remaining > 0 {
        let extra_per = remaining / wide_indices.len();
        let mut leftover = remaining % wide_indices.len();
        for &i in &wide_indices {
            widths[i] += extra_per;
            if leftover > 0 {
                widths[i] += 1;
                leftover -= 1;
            }
            widths[i] = widths[i].min(natural[i]);
        }
    }

    // Final safety clamp
    while widths.iter().sum::<usize>() > budget {
        if let Some(max_idx) = widths
            .iter()
            .enumerate()
            .max_by_key(|(_, w)| *w)
            .map(|(i, _)| i)
        {
            if widths[max_idx] > 4 {
                widths[max_idx] -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    widths
}

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

/// Render a table row with cell content wrapped using textwrap.
fn render_wrapped_row(
    lines: &mut Vec<Line<'static>>,
    cells: &[String],
    widths: &[usize],
    border_style: Style,
    bold: bool,
) {
    let num_cols = widths.len();
    let cell_style = if bold {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let wrapped: Vec<Vec<String>> = (0..num_cols)
        .map(|i| {
            let text = cells.get(i).map(|s| s.as_str()).unwrap_or("");
            let w = widths[i];
            if w == 0 {
                return vec![String::new()];
            }
            width::wrap_to_width(text, w)
        })
        .collect();

    let max_lines = wrapped.iter().map(|w| w.len()).max().unwrap_or(1);

    for line_idx in 0..max_lines {
        let mut spans: Vec<Span<'static>> = Vec::new();
        spans.push(Span::styled("│".to_string(), border_style));
        for (col, w) in widths.iter().enumerate() {
            let text = wrapped[col].get(line_idx).map(|s| s.as_str()).unwrap_or("");
            let padded = width::pad_cell(text, *w);
            spans.push(Span::styled(padded, cell_style));
            spans.push(Span::styled("│".to_string(), border_style));
        }
        lines.push(Line::from(spans));
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
        let content: String = text.lines[0]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(content.contains("Hello"));
        assert!(content.contains("═══"));
    }

    #[test]
    fn test_render_table_fits() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let elements = parser::parse(md);
        let renderer = MdRenderer::new();
        let text = renderer.render(&elements);
        let all: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(all.contains("┌"));
        assert!(all.contains("┘"));
    }

    #[test]
    fn test_table_wraps_wide_content() {
        let md = "| Short | A very long column that should wrap |\n|---|---|\n| x | This cell has lots of text that needs to wrap within the column width |";
        let elements = parser::parse(md);
        let renderer = MdRenderer::new().with_max_width(50);
        let text = renderer.render(&elements);
        for line in &text.lines {
            let w: usize = line.spans.iter().map(|s| display_width(&s.content)).sum();
            assert!(w <= 52, "Line too wide: {} display cols", w);
        }
    }

    #[test]
    fn test_render_list() {
        let elements = parser::parse("- one\n- two");
        let renderer = MdRenderer::new();
        let text = renderer.render(&elements);
        let all: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();
        assert!(all.contains("•"));
        assert!(all.contains("one"));
    }
}
