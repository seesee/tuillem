//! Terminal markdown rendering for tuillem.
//! Thin wrapper around `tui-markdown` for ratatui-native markdown rendering.

use ratatui::text::{Line, Span, Text};

/// Render markdown to ratatui Text (owned, 'static lifetime).
pub fn render_markdown(markdown: &str) -> Text<'static> {
    to_owned_text(tui_markdown::from_str(markdown))
}

/// Render markdown with a max width hint (currently unused by tui-markdown,
/// but kept for API compatibility).
pub fn render_markdown_width(markdown: &str, _max_width: usize) -> Text<'static> {
    render_markdown(markdown)
}

/// Render streaming markdown safely. For incomplete code fences, render
/// the complete portion normally and the tail as plain text.
pub fn render_markdown_streaming(markdown: &str, _max_width: usize) -> Text<'static> {
    // Check for unclosed code fences
    let fence_count = markdown.matches("```").count();
    if fence_count % 2 != 0 {
        if let Some(pos) = markdown.rfind("```") {
            let complete = &markdown[..pos];
            let incomplete = &markdown[pos..];
            let mut text = to_owned_text(tui_markdown::from_str(complete));
            for line in incomplete.lines() {
                text.lines.push(Line::from(line.to_string()));
            }
            return text;
        }
    }
    render_markdown(markdown)
}

/// Convert a borrowed Text<'a> to an owned Text<'static> by cloning all string content.
fn to_owned_text(text: Text<'_>) -> Text<'static> {
    let lines: Vec<Line<'static>> = text
        .lines
        .into_iter()
        .map(|line| {
            let spans: Vec<Span<'static>> = line
                .spans
                .into_iter()
                .map(|span| Span::styled(span.content.to_string(), span.style))
                .collect();
            Line::from(spans).alignment(line.alignment.unwrap_or_default())
        })
        .collect();
    Text::from(lines).style(text.style)
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown_e2e() {
        let md = r#"# Welcome

This is a **bold** paragraph with *italic* and `code`.

```rust
fn main() {
    println!("hello");
}
```

- item one
- item two

| Name | Value |
|------|-------|
| foo  | bar   |

---
"#;
        let text = render_markdown(md);
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_streaming_unclosed_fence() {
        let md = "Some text\n\n```python\nprint('hello')\n";
        let text = render_markdown_streaming(md, 80);
        assert!(!text.lines.is_empty());
    }
}
