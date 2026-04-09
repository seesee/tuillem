use ratatui::style::{Color, Style as RatStyle};
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight(&self, code: &str, language: &str) -> Vec<Vec<Span<'static>>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut result = Vec::new();
        for line in code.lines() {
            let ranges = h
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();
            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let fg = style.foreground;
                    Span::styled(
                        text.to_owned(),
                        RatStyle::default().fg(Color::Rgb(fg.r, fg.g, fg.b)),
                    )
                })
                .collect();
            result.push(spans);
        }
        result
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let highlighter = Highlighter::new();
        let code = "fn main() {\n    println!(\"hello\");\n}";
        let lines = highlighter.highlight(code, "rs");
        assert!(!lines.is_empty());
        assert!(!lines[0].is_empty());
    }

    #[test]
    fn test_highlight_unknown_language() {
        let highlighter = Highlighter::new();
        let code = "some random text\nwith lines";
        let lines = highlighter.highlight(code, "nonexistent_lang_xyz");
        assert!(!lines.is_empty());
        // Should fall back to plain text gracefully
        assert!(!lines[0].is_empty());
    }
}
