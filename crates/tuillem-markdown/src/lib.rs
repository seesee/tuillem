pub mod highlight;
pub mod parser;
pub mod renderer;

use ratatui::text::Text;

pub fn render_markdown(markdown: &str) -> Text<'static> {
    let elements = parser::parse(markdown);
    let renderer = renderer::MdRenderer::new();
    renderer.render(&elements)
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

        let all_text: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(all_text.contains("Welcome"));
        assert!(all_text.contains("bold"));
        assert!(all_text.contains("item one"));
        assert!(all_text.contains("foo"));
        assert!(all_text.contains("─"));
    }
}
