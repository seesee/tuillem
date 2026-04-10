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

    /// Dracula palette.
    pub fn dracula() -> Self {
        Self {
            bg: Color::Rgb(40, 42, 54),                  // #282a36
            fg: Color::Rgb(248, 248, 242),               // #f8f8f2
            sidebar_bg: Color::Rgb(33, 34, 44),          // #21222c
            sidebar_fg: Color::Rgb(248, 248, 242),       // #f8f8f2
            sidebar_selected: Color::Rgb(189, 147, 249), // #bd93f9
            user_msg_bg: Color::Rgb(68, 71, 90),         // #44475a
            assistant_msg_bg: Color::Rgb(40, 42, 54),    // #282a36
            thinking_fg: Color::Rgb(98, 114, 164),       // #6272a4
            accent: Color::Rgb(189, 147, 249),           // #bd93f9
            error: Color::Rgb(255, 85, 85),              // #ff5555
            success: Color::Rgb(80, 250, 123),           // #50fa7b
            warning: Color::Rgb(241, 250, 140),          // #f1fa8c
            border: Color::Rgb(68, 71, 90),              // #44475a
            code_bg: Color::Rgb(33, 34, 44),             // #21222c
            code_fg: Color::Rgb(80, 250, 123),           // #50fa7b
            heading: Color::Rgb(255, 121, 198),          // #ff79c6
            link: Color::Rgb(139, 233, 253),             // #8be9fd
            tag: Color::Rgb(189, 147, 249),              // #bd93f9
        }
    }

    /// Nord palette.
    pub fn nord() -> Self {
        Self {
            bg: Color::Rgb(46, 52, 64),                  // #2e3440
            fg: Color::Rgb(216, 222, 233),               // #d8dee9
            sidebar_bg: Color::Rgb(59, 66, 82),          // #3b4252
            sidebar_fg: Color::Rgb(216, 222, 233),       // #d8dee9
            sidebar_selected: Color::Rgb(136, 192, 208), // #88c0d0
            user_msg_bg: Color::Rgb(67, 76, 94),         // #434c5e
            assistant_msg_bg: Color::Rgb(46, 52, 64),    // #2e3440
            thinking_fg: Color::Rgb(76, 86, 106),        // #4c566a
            accent: Color::Rgb(136, 192, 208),           // #88c0d0
            error: Color::Rgb(191, 97, 106),             // #bf616a
            success: Color::Rgb(163, 190, 140),          // #a3be8c
            warning: Color::Rgb(235, 203, 139),          // #ebcb8b
            border: Color::Rgb(76, 86, 106),             // #4c566a
            code_bg: Color::Rgb(59, 66, 82),             // #3b4252
            code_fg: Color::Rgb(163, 190, 140),          // #a3be8c
            heading: Color::Rgb(129, 161, 193),          // #81a1c1
            link: Color::Rgb(136, 192, 208),             // #88c0d0
            tag: Color::Rgb(180, 142, 173),              // #b48ead
        }
    }

    /// Gruvbox Dark palette.
    pub fn gruvbox() -> Self {
        Self {
            bg: Color::Rgb(40, 40, 40),                  // #282828
            fg: Color::Rgb(235, 219, 178),               // #ebdbb2
            sidebar_bg: Color::Rgb(29, 32, 33),          // #1d2021
            sidebar_fg: Color::Rgb(235, 219, 178),       // #ebdbb2
            sidebar_selected: Color::Rgb(250, 189, 47),  // #fabd2f
            user_msg_bg: Color::Rgb(60, 56, 54),         // #3c3836
            assistant_msg_bg: Color::Rgb(40, 40, 40),    // #282828
            thinking_fg: Color::Rgb(146, 131, 116),      // #928374
            accent: Color::Rgb(250, 189, 47),            // #fabd2f
            error: Color::Rgb(251, 73, 52),              // #fb4934
            success: Color::Rgb(184, 187, 38),           // #b8bb26
            warning: Color::Rgb(254, 128, 25),           // #fe8019
            border: Color::Rgb(80, 73, 69),              // #504945
            code_bg: Color::Rgb(29, 32, 33),             // #1d2021
            code_fg: Color::Rgb(184, 187, 38),           // #b8bb26
            heading: Color::Rgb(131, 165, 152),          // #83a598
            link: Color::Rgb(131, 165, 152),             // #83a598
            tag: Color::Rgb(211, 134, 155),              // #d3869b
        }
    }

    /// Tokyo Night palette.
    pub fn tokyo_night() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),                  // #1a1b26
            fg: Color::Rgb(169, 177, 214),               // #a9b1d6
            sidebar_bg: Color::Rgb(22, 22, 30),          // #16161e
            sidebar_fg: Color::Rgb(169, 177, 214),       // #a9b1d6
            sidebar_selected: Color::Rgb(122, 162, 247), // #7aa2f7
            user_msg_bg: Color::Rgb(41, 46, 66),         // #292e42
            assistant_msg_bg: Color::Rgb(26, 27, 38),    // #1a1b26
            thinking_fg: Color::Rgb(86, 95, 137),        // #565f89
            accent: Color::Rgb(122, 162, 247),           // #7aa2f7
            error: Color::Rgb(247, 118, 142),            // #f7768e
            success: Color::Rgb(158, 206, 106),          // #9ece6a
            warning: Color::Rgb(224, 175, 104),          // #e0af68
            border: Color::Rgb(41, 46, 66),              // #292e42
            code_bg: Color::Rgb(22, 22, 30),             // #16161e
            code_fg: Color::Rgb(158, 206, 106),          // #9ece6a
            heading: Color::Rgb(187, 154, 247),          // #bb9af7
            link: Color::Rgb(125, 207, 255),             // #7dcfff
            tag: Color::Rgb(187, 154, 247),              // #bb9af7
        }
    }

    /// Solarized Dark palette.
    pub fn solarized() -> Self {
        Self {
            bg: Color::Rgb(0, 43, 54),                   // #002b36
            fg: Color::Rgb(131, 148, 150),               // #839496
            sidebar_bg: Color::Rgb(7, 54, 66),           // #073642
            sidebar_fg: Color::Rgb(131, 148, 150),       // #839496
            sidebar_selected: Color::Rgb(38, 139, 210),  // #268bd2
            user_msg_bg: Color::Rgb(7, 54, 66),          // #073642
            assistant_msg_bg: Color::Rgb(0, 43, 54),     // #002b36
            thinking_fg: Color::Rgb(88, 110, 117),       // #586e75
            accent: Color::Rgb(38, 139, 210),            // #268bd2
            error: Color::Rgb(220, 50, 47),              // #dc322f
            success: Color::Rgb(133, 153, 0),            // #859900
            warning: Color::Rgb(181, 137, 0),            // #b58900
            border: Color::Rgb(88, 110, 117),            // #586e75
            code_bg: Color::Rgb(7, 54, 66),              // #073642
            code_fg: Color::Rgb(133, 153, 0),            // #859900
            heading: Color::Rgb(108, 113, 196),          // #6c71c4
            link: Color::Rgb(42, 161, 152),              // #2aa198
            tag: Color::Rgb(108, 113, 196),              // #6c71c4
        }
    }

    /// All built-in theme names.
    pub fn builtin_names() -> &'static [&'static str] {
        &["dark", "light", "dracula", "nord", "gruvbox", "tokyo_night", "solarized"]
    }

    /// Get a theme by name, applying custom overrides if available.
    pub fn from_config(name: &str, custom_themes: &HashMap<String, ThemeColors>) -> Self {
        let base = match name {
            "light" => Self::light(),
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            "tokyo_night" => Self::tokyo_night(),
            "solarized" => Self::solarized(),
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
