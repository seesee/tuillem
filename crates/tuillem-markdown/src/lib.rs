pub mod highlight;
pub mod parser;
pub mod renderer;

use ratatui::text::Text;

pub fn render_markdown(markdown: &str) -> Text<'static> {
    let elements = parser::parse(markdown);
    let renderer = renderer::MdRenderer::new();
    renderer.render(&elements)
}

/// Render streaming markdown safely. Detects incomplete tables/code blocks
/// and renders the incomplete tail as plain text to avoid broken output.
pub fn render_markdown_streaming(markdown: &str) -> Text<'static> {
    // Find the last complete section and render incomplete parts as plain text
    let (complete, incomplete) = split_incomplete_blocks(markdown);
    let mut text = render_markdown(complete);
    if !incomplete.is_empty() {
        for line in incomplete.lines() {
            text.lines.push(ratatui::text::Line::from(line.to_string()));
        }
    }
    text
}

/// Split markdown into complete and possibly-incomplete trailing sections.
/// An incomplete table is one where we see `|` rows without a closing blank line.
/// An incomplete code block has an opening ``` without a closing ```.
fn split_incomplete_blocks(markdown: &str) -> (&str, &str) {
    // Check for unclosed code fences
    let fence_count = markdown.matches("```").count();
    if fence_count % 2 != 0 {
        // Find the last opening fence
        if let Some(pos) = markdown.rfind("```") {
            return (&markdown[..pos], &markdown[pos..]);
        }
    }

    // Check for incomplete table at the end — a table line starts with |
    // Find the last blank line, check if everything after it looks like a table
    let trimmed = markdown.trim_end();
    if let Some(last_line) = trimmed.lines().last() {
        if last_line.trim_start().starts_with('|') {
            // Walk backward to find where this table block starts
            let lines: Vec<&str> = trimmed.lines().collect();
            let mut table_start = lines.len();
            for (i, line) in lines.iter().enumerate().rev() {
                let lt = line.trim();
                if lt.starts_with('|') || (lt.contains("---") && lt.contains('|')) {
                    table_start = i;
                } else {
                    break;
                }
            }
            // Check if table has header + separator (minimum for a valid table)
            let table_lines = &lines[table_start..];
            let has_separator = table_lines.iter().any(|l| {
                let t = l.trim();
                t.contains("---") && t.contains('|')
            });
            if !has_separator || table_lines.len() < 3 {
                // Incomplete table — find byte offset of table_start
                let mut byte_offset = 0;
                for (i, line) in trimmed.lines().enumerate() {
                    if i == table_start {
                        break;
                    }
                    byte_offset += line.len() + 1; // +1 for \n
                }
                let byte_offset = byte_offset.min(markdown.len());
                return (&markdown[..byte_offset], &markdown[byte_offset..]);
            }
        }
    }

    (markdown, "")
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
