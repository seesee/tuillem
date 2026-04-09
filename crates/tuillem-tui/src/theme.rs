use std::collections::HashMap;

use ratatui::style::{Color, Style};
use tuillem_config::ThemeColors;

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub sidebar_bg: Color,
    pub sidebar_fg: Color,
    pub sidebar_selected: Color,
    pub user_msg_bg: Color,
    pub assistant_msg_bg: Color,
    pub thinking_fg: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub warning: Color,
    pub border: Color,
    pub code_bg: Color,
    pub code_fg: Color,
    pub heading: Color,
    pub link: Color,
    pub tag: Color,
}

impl Theme {
    /// Catppuccin Mocha palette.
    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),                  // #1e1e2e
            fg: Color::Rgb(205, 214, 244),               // #cdd6f4
            sidebar_bg: Color::Rgb(24, 24, 37),          // #181825
            sidebar_fg: Color::Rgb(186, 194, 222),       // #bac2de
            sidebar_selected: Color::Rgb(137, 180, 250), // #89b4fa
            user_msg_bg: Color::Rgb(49, 50, 68),         // #313244
            assistant_msg_bg: Color::Rgb(30, 30, 46),    // #1e1e2e
            thinking_fg: Color::Rgb(127, 132, 156),      // #7f849c
            accent: Color::Rgb(137, 180, 250),           // #89b4fa
            error: Color::Rgb(243, 139, 168),            // #f38ba8
            success: Color::Rgb(166, 227, 161),          // #a6e3a1
            warning: Color::Rgb(249, 226, 175),          // #f9e2af
            border: Color::Rgb(88, 91, 112),             // #585b70
            code_bg: Color::Rgb(24, 24, 37),             // #181825
            code_fg: Color::Rgb(166, 227, 161),          // #a6e3a1
            heading: Color::Rgb(180, 190, 254),          // #b4befe
            link: Color::Rgb(116, 199, 236),             // #74c7ec
            tag: Color::Rgb(203, 166, 247),              // #cba6f7
        }
    }

    /// Catppuccin Latte palette.
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),               // #eff1f5
            fg: Color::Rgb(76, 79, 105),                 // #4c4f69
            sidebar_bg: Color::Rgb(230, 233, 239),       // #e6e9ef
            sidebar_fg: Color::Rgb(92, 95, 119),         // #5c5f77
            sidebar_selected: Color::Rgb(30, 102, 245),  // #1e66f5
            user_msg_bg: Color::Rgb(204, 208, 218),      // #ccd0da
            assistant_msg_bg: Color::Rgb(239, 241, 245), // #eff1f5
            thinking_fg: Color::Rgb(140, 143, 161),      // #8c8fa1
            accent: Color::Rgb(30, 102, 245),            // #1e66f5
            error: Color::Rgb(210, 15, 57),              // #d20f39
            success: Color::Rgb(64, 160, 43),            // #40a02b
            warning: Color::Rgb(223, 142, 29),           // #df8e1d
            border: Color::Rgb(172, 176, 190),           // #acb0be
            code_bg: Color::Rgb(230, 233, 239),          // #e6e9ef
            code_fg: Color::Rgb(64, 160, 43),            // #40a02b
            heading: Color::Rgb(114, 135, 253),          // #7287fd
            link: Color::Rgb(4, 165, 229),               // #04a5e5
            tag: Color::Rgb(136, 57, 239),               // #8839ef
        }
    }

    /// Get a theme by name, applying custom overrides if available.
    pub fn from_config(name: &str, custom_themes: &HashMap<String, ThemeColors>) -> Self {
        let base = match name {
            "light" => Self::light(),
            _ => Self::dark(),
        };
        match custom_themes.get(name) {
            Some(colors) => base.apply_overrides(colors),
            None => base,
        }
    }

    /// Apply overrides from a ThemeColors config. Each `Some` field overrides the corresponding color.
    pub fn apply_overrides(mut self, colors: &ThemeColors) -> Self {
        if let Some(ref c) = colors.bg {
            self.bg = parse_hex(c);
        }
        if let Some(ref c) = colors.fg {
            self.fg = parse_hex(c);
        }
        if let Some(ref c) = colors.sidebar_bg {
            self.sidebar_bg = parse_hex(c);
        }
        if let Some(ref c) = colors.sidebar_fg {
            self.sidebar_fg = parse_hex(c);
        }
        if let Some(ref c) = colors.sidebar_selected {
            self.sidebar_selected = parse_hex(c);
        }
        if let Some(ref c) = colors.user_msg_bg {
            self.user_msg_bg = parse_hex(c);
        }
        if let Some(ref c) = colors.assistant_msg_bg {
            self.assistant_msg_bg = parse_hex(c);
        }
        if let Some(ref c) = colors.thinking_fg {
            self.thinking_fg = parse_hex(c);
        }
        if let Some(ref c) = colors.accent {
            self.accent = parse_hex(c);
        }
        if let Some(ref c) = colors.error {
            self.error = parse_hex(c);
        }
        if let Some(ref c) = colors.success {
            self.success = parse_hex(c);
        }
        if let Some(ref c) = colors.warning {
            self.warning = parse_hex(c);
        }
        if let Some(ref c) = colors.border {
            self.border = parse_hex(c);
        }
        if let Some(ref c) = colors.code_bg {
            self.code_bg = parse_hex(c);
        }
        if let Some(ref c) = colors.code_fg {
            self.code_fg = parse_hex(c);
        }
        if let Some(ref c) = colors.heading {
            self.heading = parse_hex(c);
        }
        if let Some(ref c) = colors.link {
            self.link = parse_hex(c);
        }
        if let Some(ref c) = colors.tag {
            self.tag = parse_hex(c);
        }
        self
    }

    // Style convenience methods

    pub fn sidebar_style(&self) -> Style {
        Style::default().fg(self.sidebar_fg).bg(self.sidebar_bg)
    }

    pub fn sidebar_selected_style(&self) -> Style {
        Style::default()
            .fg(self.sidebar_selected)
            .bg(self.sidebar_bg)
    }

    pub fn user_message_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.user_msg_bg)
    }

    pub fn assistant_message_style(&self) -> Style {
        Style::default().fg(self.fg).bg(self.assistant_msg_bg)
    }

    pub fn thinking_style(&self) -> Style {
        Style::default().fg(self.thinking_fg)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error)
    }
}

/// Parse a hex color string ("#rrggbb" or "rrggbb") to Color::Rgb.
/// Falls back to Color::White on invalid input.
pub fn parse_hex(hex: &str) -> Color {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return Color::White;
    }
    let r = u8::from_str_radix(&hex[0..2], 16);
    let g = u8::from_str_radix(&hex[2..4], 16);
    let b = u8::from_str_radix(&hex[4..6], 16);
    match (r, g, b) {
        (Ok(r), Ok(g), Ok(b)) => Color::Rgb(r, g, b),
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("#ff0000"), Color::Rgb(255, 0, 0));
        assert_eq!(parse_hex("00ff00"), Color::Rgb(0, 255, 0));
        assert_eq!(parse_hex("#1e1e2e"), Color::Rgb(30, 30, 46));
        // Invalid inputs fall back to White
        assert_eq!(parse_hex("zzzzzz"), Color::White);
        assert_eq!(parse_hex("short"), Color::White);
        assert_eq!(parse_hex(""), Color::White);
    }

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.bg, Color::Rgb(30, 30, 46));
        assert_eq!(theme.fg, Color::Rgb(205, 214, 244));
        assert_eq!(theme.sidebar_bg, Color::Rgb(24, 24, 37));
    }

    #[test]
    fn test_custom_theme_override() {
        let mut custom = HashMap::new();
        custom.insert(
            "dark".to_string(),
            ThemeColors {
                bg: Some("#000000".to_string()),
                fg: Some("#ffffff".to_string()),
                ..Default::default()
            },
        );

        let theme = Theme::from_config("dark", &custom);
        assert_eq!(theme.bg, Color::Rgb(0, 0, 0));
        assert_eq!(theme.fg, Color::Rgb(255, 255, 255));
        // Non-overridden fields retain defaults
        assert_eq!(theme.sidebar_bg, Color::Rgb(24, 24, 37));
    }
}
