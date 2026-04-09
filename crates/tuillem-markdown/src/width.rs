//! Unicode-aware terminal width measurement.
//!
//! Operates on grapheme clusters, not bytes or chars.
//! Handles emoji sequences, ZWJ combinations, flags, skin tone modifiers.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Returns true if a grapheme cluster looks emoji-like.
fn looks_emojiish(g: &str) -> bool {
    g.chars().any(|c| {
        matches!(
            c as u32,
            0x1F1E6..=0x1F1FF  // regional indicators (flags)
            | 0x1F300..=0x1FAFF // most emoji blocks
            | 0x2600..=0x26FF   // misc symbols
            | 0x2700..=0x27BF   // dingbats
            | 0xFE0F            // variation selector-16 (emoji presentation)
            | 0x200D            // ZWJ
        )
    })
}

/// Compute the terminal display width of a string.
///
/// - Normalises to NFC
/// - Splits into grapheme clusters
/// - Uses unicode-width per cluster
/// - Forces emoji-like clusters to width 2
pub fn terminal_width(s: &str) -> usize {
    let normalised: String = s.nfc().collect();

    UnicodeSegmentation::graphemes(normalised.as_str(), true)
        .map(|g| {
            let w = UnicodeWidthStr::width(g);
            if w == 0 && !g.trim().is_empty() {
                1
            } else if looks_emojiish(g) && w < 2 {
                2
            } else {
                w
            }
        })
        .sum()
}

/// Grapheme-aware word wrapping to fit within `max_width` terminal columns.
pub fn wrap_to_width(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    if terminal_width(text) <= max_width {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;

    for word in text.split_whitespace() {
        let word_w = terminal_width(word);
        if current_w == 0 {
            if word_w > max_width {
                // Force-break long word by grapheme
                let normalised: String = word.nfc().collect();
                let mut line = String::new();
                let mut line_w = 0;
                for g in UnicodeSegmentation::graphemes(normalised.as_str(), true) {
                    let gw = grapheme_width(g);
                    if line_w + gw > max_width && !line.is_empty() {
                        result.push(line);
                        line = String::new();
                        line_w = 0;
                    }
                    line.push_str(g);
                    line_w += gw;
                }
                if !line.is_empty() {
                    current = line;
                    current_w = terminal_width(&current);
                }
            } else {
                current = word.to_string();
                current_w = word_w;
            }
        } else if current_w + 1 + word_w <= max_width {
            current.push(' ');
            current.push_str(word);
            current_w += 1 + word_w;
        } else {
            result.push(current);
            if word_w > max_width {
                let normalised: String = word.nfc().collect();
                let mut line = String::new();
                let mut line_w = 0;
                for g in UnicodeSegmentation::graphemes(normalised.as_str(), true) {
                    let gw = grapheme_width(g);
                    if line_w + gw > max_width && !line.is_empty() {
                        result.push(line);
                        line = String::new();
                        line_w = 0;
                    }
                    line.push_str(g);
                    line_w += gw;
                }
                if !line.is_empty() {
                    current = line;
                    current_w = terminal_width(&current);
                } else {
                    current = String::new();
                    current_w = 0;
                }
            } else {
                current = word.to_string();
                current_w = word_w;
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

/// Truncate a string to fit within `max_width` terminal columns.
/// Cuts on grapheme boundaries, never in the middle of a cluster.
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    if terminal_width(s) <= max_width {
        return s.to_string();
    }
    let normalised: String = s.nfc().collect();
    let mut out = String::new();
    let mut used = 0;
    for g in UnicodeSegmentation::graphemes(normalised.as_str(), true) {
        let w = grapheme_width(g);
        if used + w > max_width {
            break;
        }
        out.push_str(g);
        used += w;
    }
    out
}

/// Truncate with ellipsis.
pub fn truncate_with_ellipsis(s: &str, max_width: usize) -> String {
    if terminal_width(s) <= max_width {
        return s.to_string();
    }
    if max_width <= 1 {
        return truncate_to_width(s, max_width);
    }
    let truncated = truncate_to_width(s, max_width.saturating_sub(1));
    format!("{}…", truncated)
}

/// Pad a string to exactly `width` terminal columns with trailing spaces.
/// Returns `" {text}{spaces} "` with 1-char margins.
pub fn pad_cell(text: &str, width: usize) -> String {
    let text_w = terminal_width(text);
    let pad = width.saturating_sub(text_w);
    let mut s = String::with_capacity(text.len() + pad + 2);
    s.push(' ');
    s.push_str(text);
    for _ in 0..pad {
        s.push(' ');
    }
    s.push(' ');
    s
}

fn grapheme_width(g: &str) -> usize {
    let w = UnicodeWidthStr::width(g);
    if w == 0 && !g.trim().is_empty() {
        1
    } else if looks_emojiish(g) && w < 2 {
        2
    } else {
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_smoke_test() {
        let cases = [
            ("a", 1),
            ("é", 1),
            ("中", 2),
            ("🙂", 2),
            ("hello", 5),
            ("hello world", 11),
        ];
        for (s, expected) in cases {
            let w = terminal_width(s);
            assert_eq!(w, expected, "{s:?} had width {w}, expected {expected}");
        }
    }

    #[test]
    fn emoji_minimum_width() {
        // These should be at least 2
        for s in ["👍🏽", "🇬🇧", "🏳️\u{200D}🌈"] {
            let w = terminal_width(s);
            assert!(w >= 2, "{s:?} had width {w}, expected >= 2");
        }
    }

    #[test]
    fn wrap_basic() {
        let lines = wrap_to_width("hello world foo bar", 11);
        assert_eq!(lines[0], "hello world");
        assert!(lines.len() >= 2);
    }

    #[test]
    fn truncate_basic() {
        assert_eq!(truncate_to_width("hello world", 5), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 6), "hello…");
    }

    #[test]
    fn pad_cell_basic() {
        let padded = pad_cell("hi", 10);
        assert_eq!(padded, " hi         "); // 1 + 2 + 8 + 1 = 12
        assert_eq!(padded.len(), 12); // " " + "hi" + 8 spaces + " "
    }
}
