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
    pub sidebar_selected_bg: Color,
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
            sidebar_selected_bg: Color::Rgb(49, 50, 68), // #313244 (surface0)
        }
    }

    /// Catppuccin Latte palette.
    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(239, 241, 245),                  // #eff1f5
            fg: Color::Rgb(76, 79, 105),                    // #4c4f69
            sidebar_bg: Color::Rgb(230, 233, 239),          // #e6e9ef
            sidebar_fg: Color::Rgb(92, 95, 119),            // #5c5f77
            sidebar_selected: Color::Rgb(30, 102, 245),     // #1e66f5
            user_msg_bg: Color::Rgb(204, 208, 218),         // #ccd0da
            assistant_msg_bg: Color::Rgb(239, 241, 245),    // #eff1f5
            thinking_fg: Color::Rgb(140, 143, 161),         // #8c8fa1
            accent: Color::Rgb(30, 102, 245),               // #1e66f5
            error: Color::Rgb(210, 15, 57),                 // #d20f39
            success: Color::Rgb(64, 160, 43),               // #40a02b
            warning: Color::Rgb(223, 142, 29),              // #df8e1d
            border: Color::Rgb(172, 176, 190),              // #acb0be
            code_bg: Color::Rgb(230, 233, 239),             // #e6e9ef
            code_fg: Color::Rgb(64, 160, 43),               // #40a02b
            heading: Color::Rgb(114, 135, 253),             // #7287fd
            link: Color::Rgb(4, 165, 229),                  // #04a5e5
            tag: Color::Rgb(136, 57, 239),                  // #8839ef
            sidebar_selected_bg: Color::Rgb(188, 192, 204), // #bcc0cc (surface1)
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
            sidebar_selected_bg: Color::Rgb(68, 71, 90), // #44475a (selection)
        }
    }

    /// Nord palette.
    pub fn nord() -> Self {
        Self {
            bg: Color::Rgb(46, 52, 64),                   // #2e3440
            fg: Color::Rgb(216, 222, 233),                // #d8dee9
            sidebar_bg: Color::Rgb(59, 66, 82),           // #3b4252
            sidebar_fg: Color::Rgb(216, 222, 233),        // #d8dee9
            sidebar_selected: Color::Rgb(136, 192, 208),  // #88c0d0
            user_msg_bg: Color::Rgb(67, 76, 94),          // #434c5e
            assistant_msg_bg: Color::Rgb(46, 52, 64),     // #2e3440
            thinking_fg: Color::Rgb(76, 86, 106),         // #4c566a
            accent: Color::Rgb(136, 192, 208),            // #88c0d0
            error: Color::Rgb(191, 97, 106),              // #bf616a
            success: Color::Rgb(163, 190, 140),           // #a3be8c
            warning: Color::Rgb(235, 203, 139),           // #ebcb8b
            border: Color::Rgb(76, 86, 106),              // #4c566a
            code_bg: Color::Rgb(59, 66, 82),              // #3b4252
            code_fg: Color::Rgb(163, 190, 140),           // #a3be8c
            heading: Color::Rgb(129, 161, 193),           // #81a1c1
            link: Color::Rgb(136, 192, 208),              // #88c0d0
            tag: Color::Rgb(180, 142, 173),               // #b48ead
            sidebar_selected_bg: Color::Rgb(76, 86, 106), // #4c566a (nord3)
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
            sidebar_selected_bg: Color::Rgb(80, 73, 69), // #504945 (bg2)
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
            sidebar_selected_bg: Color::Rgb(54, 58, 79), // #363a4f
        }
    }

    /// Solarized Dark palette.
    pub fn solarized() -> Self {
        Self {
            bg: Color::Rgb(0, 43, 54),                  // #002b36
            fg: Color::Rgb(131, 148, 150),              // #839496
            sidebar_bg: Color::Rgb(7, 54, 66),          // #073642
            sidebar_fg: Color::Rgb(131, 148, 150),      // #839496
            sidebar_selected: Color::Rgb(38, 139, 210), // #268bd2
            user_msg_bg: Color::Rgb(7, 54, 66),         // #073642
            assistant_msg_bg: Color::Rgb(0, 43, 54),    // #002b36
            thinking_fg: Color::Rgb(88, 110, 117),      // #586e75
            accent: Color::Rgb(38, 139, 210),           // #268bd2
            error: Color::Rgb(220, 50, 47),             // #dc322f
            success: Color::Rgb(133, 153, 0),           // #859900
            warning: Color::Rgb(181, 137, 0),           // #b58900
            border: Color::Rgb(88, 110, 117),           // #586e75
            code_bg: Color::Rgb(7, 54, 66),             // #073642
            code_fg: Color::Rgb(133, 153, 0),           // #859900
            heading: Color::Rgb(108, 113, 196),         // #6c71c4
            link: Color::Rgb(42, 161, 152),             // #2aa198
            tag: Color::Rgb(108, 113, 196),             // #6c71c4
            sidebar_selected_bg: Color::Rgb(0, 54, 66), // #003642 (base02 lighter)
        }
    }

    /// Solarized Light palette.
    pub fn solarized_light() -> Self {
        Self {
            bg: Color::Rgb(253, 246, 227),                  // #fdf6e3
            fg: Color::Rgb(101, 123, 131),                  // #657b83
            sidebar_bg: Color::Rgb(238, 232, 213),          // #eee8d5
            sidebar_fg: Color::Rgb(101, 123, 131),          // #657b83
            sidebar_selected: Color::Rgb(38, 139, 210),     // #268bd2
            user_msg_bg: Color::Rgb(238, 232, 213),         // #eee8d5
            assistant_msg_bg: Color::Rgb(253, 246, 227),    // #fdf6e3
            thinking_fg: Color::Rgb(147, 161, 161),         // #93a1a1
            accent: Color::Rgb(38, 139, 210),               // #268bd2
            error: Color::Rgb(220, 50, 47),                 // #dc322f
            success: Color::Rgb(133, 153, 0),               // #859900
            warning: Color::Rgb(181, 137, 0),               // #b58900
            border: Color::Rgb(147, 161, 161),              // #93a1a1
            code_bg: Color::Rgb(238, 232, 213),             // #eee8d5
            code_fg: Color::Rgb(42, 161, 152),              // #2aa198
            heading: Color::Rgb(108, 113, 196),             // #6c71c4
            link: Color::Rgb(42, 161, 152),                 // #2aa198
            tag: Color::Rgb(211, 54, 130),                  // #d33682
            sidebar_selected_bg: Color::Rgb(253, 246, 227), // #fdf6e3 (base3)
        }
    }

    /// GitHub Light palette.
    pub fn github_light() -> Self {
        Self {
            bg: Color::Rgb(255, 255, 255),                  // #ffffff
            fg: Color::Rgb(31, 35, 40),                     // #1f2328
            sidebar_bg: Color::Rgb(246, 248, 250),          // #f6f8fa
            sidebar_fg: Color::Rgb(31, 35, 40),             // #1f2328
            sidebar_selected: Color::Rgb(9, 105, 218),      // #0969da
            user_msg_bg: Color::Rgb(221, 244, 255),         // #ddf4ff
            assistant_msg_bg: Color::Rgb(255, 255, 255),    // #ffffff
            thinking_fg: Color::Rgb(101, 109, 118),         // #656d76
            accent: Color::Rgb(9, 105, 218),                // #0969da
            error: Color::Rgb(207, 34, 46),                 // #cf222e
            success: Color::Rgb(26, 127, 55),               // #1a7f37
            warning: Color::Rgb(154, 103, 0),               // #9a6700
            border: Color::Rgb(208, 215, 222),              // #d0d7de
            code_bg: Color::Rgb(246, 248, 250),             // #f6f8fa
            code_fg: Color::Rgb(26, 127, 55),               // #1a7f37
            heading: Color::Rgb(9, 105, 218),               // #0969da
            link: Color::Rgb(9, 105, 218),                  // #0969da
            tag: Color::Rgb(130, 80, 223),                  // #8250df
            sidebar_selected_bg: Color::Rgb(221, 244, 255), // #ddf4ff (blue tint)
        }
    }

    /// Rose Pine Dawn palette (light).
    pub fn rose_pine_dawn() -> Self {
        Self {
            bg: Color::Rgb(250, 244, 237),                  // #faf4ed
            fg: Color::Rgb(87, 82, 121),                    // #575279
            sidebar_bg: Color::Rgb(255, 250, 243),          // #fffaf3
            sidebar_fg: Color::Rgb(87, 82, 121),            // #575279
            sidebar_selected: Color::Rgb(40, 105, 131),     // #286983
            user_msg_bg: Color::Rgb(242, 233, 222),         // #f2e9de
            assistant_msg_bg: Color::Rgb(250, 244, 237),    // #faf4ed
            thinking_fg: Color::Rgb(152, 147, 165),         // #9893a5
            accent: Color::Rgb(40, 105, 131),               // #286983
            error: Color::Rgb(180, 99, 122),                // #b4637a
            success: Color::Rgb(40, 105, 131),              // #286983
            warning: Color::Rgb(234, 157, 52),              // #ea9d34
            border: Color::Rgb(206, 202, 205),              // #cecacd
            code_bg: Color::Rgb(242, 233, 222),             // #f2e9de
            code_fg: Color::Rgb(87, 82, 121),               // #575279
            heading: Color::Rgb(144, 122, 169),             // #907aa9
            link: Color::Rgb(40, 105, 131),                 // #286983
            tag: Color::Rgb(144, 122, 169),                 // #907aa9
            sidebar_selected_bg: Color::Rgb(242, 233, 222), // #f2e9de (surface)
        }
    }

    /// All built-in theme names.
    pub fn builtin_names() -> &'static [&'static str] {
        &[
            "dark",
            "light",
            "dracula",
            "nord",
            "gruvbox",
            "tokyo_night",
            "solarized",
            "solarized_light",
            "github_light",
            "rose_pine_dawn",
        ]
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
            "solarized_light" => Self::solarized_light(),
            "github_light" => Self::github_light(),
            "rose_pine_dawn" => Self::rose_pine_dawn(),
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
        if let Some(ref c) = colors.sidebar_selected_bg {
            self.sidebar_selected_bg = parse_hex(c);
        }
        self
    }

    /// Create a new Theme where every colour field has been adapted to the given color mode.
    pub fn adapt_to_color_mode(&self, mode: &str) -> Self {
        Self {
            bg: adapt_color(self.bg, mode),
            fg: adapt_color(self.fg, mode),
            sidebar_bg: adapt_color(self.sidebar_bg, mode),
            sidebar_fg: adapt_color(self.sidebar_fg, mode),
            sidebar_selected: adapt_color(self.sidebar_selected, mode),
            user_msg_bg: adapt_color(self.user_msg_bg, mode),
            assistant_msg_bg: adapt_color(self.assistant_msg_bg, mode),
            thinking_fg: adapt_color(self.thinking_fg, mode),
            accent: adapt_color(self.accent, mode),
            error: adapt_color(self.error, mode),
            success: adapt_color(self.success, mode),
            warning: adapt_color(self.warning, mode),
            border: adapt_color(self.border, mode),
            code_bg: adapt_color(self.code_bg, mode),
            code_fg: adapt_color(self.code_fg, mode),
            heading: adapt_color(self.heading, mode),
            link: adapt_color(self.link, mode),
            tag: adapt_color(self.tag, mode),
            sidebar_selected_bg: adapt_color(self.sidebar_selected_bg, mode),
        }
    }

    // Style convenience methods

    pub fn sidebar_style(&self) -> Style {
        Style::default().fg(self.sidebar_fg).bg(self.sidebar_bg)
    }

    pub fn sidebar_selected_style(&self) -> Style {
        Style::default()
            .fg(self.sidebar_selected)
            .bg(self.sidebar_selected_bg)
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

// ---------------------------------------------------------------------------
// Colour-mode conversion
// ---------------------------------------------------------------------------

/// Detect the best color mode from environment variables.
pub fn detect_color_mode() -> &'static str {
    if let Ok(ct) = std::env::var("COLORTERM")
        && (ct == "truecolor" || ct == "24bit")
    {
        return "truecolor";
    }
    if let Ok(term) = std::env::var("TERM")
        && term.contains("256color")
    {
        return "256";
    }
    "basic"
}

/// Resolve a color_mode config value, handling "auto" via environment detection.
pub fn resolve_color_mode(mode: &str) -> &str {
    match mode {
        "truecolor" | "256" | "basic" => mode,
        _ => detect_color_mode(), // "auto" or any unrecognised value
    }
}

/// Convert a `Color::Rgb` to the appropriate format based on the color mode.
/// Non-RGB colours (Reset, Black, Red, Indexed, etc.) pass through unchanged.
pub fn adapt_color(color: Color, mode: &str) -> Color {
    match mode {
        "truecolor" => color,
        "256" => rgb_to_256(color),
        "basic" => rgb_to_basic(color),
        _ => color, // auto already resolved, but fall back to passthrough
    }
}

/// Map an RGB colour to the nearest xterm-256 palette entry.
fn rgb_to_256(color: Color) -> Color {
    if let Color::Rgb(r, g, b) = color {
        // Check if it's close to greyscale first
        if r == g && g == b {
            if r < 8 {
                return Color::Indexed(16);
            }
            if r > 248 {
                return Color::Indexed(231);
            }
            return Color::Indexed(232 + ((r as u16 - 8) * 24 / 247) as u8);
        }
        // Map to 6x6x6 colour cube
        let ri = (r as u16 * 5 / 255) as u8;
        let gi = (g as u16 * 5 / 255) as u8;
        let bi = (b as u16 * 5 / 255) as u8;
        Color::Indexed(16 + 36 * ri + 6 * gi + bi)
    } else {
        color
    }
}

/// Map an RGB colour to the nearest ANSI basic 16-colour.
fn rgb_to_basic(color: Color) -> Color {
    if let Color::Rgb(r, g, b) = color {
        let luma = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u16;
        let bright = luma > 127;

        // Determine dominant channel(s)
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let chroma = max as i16 - min as i16;

        // Near-grey: use black/darkgrey/lightgrey/white
        if chroma < 30 {
            return match luma {
                0..=63 => Color::Black,
                64..=127 => Color::DarkGray,
                128..=191 => Color::Gray,
                _ => Color::White,
            };
        }

        // Map to basic hue based on dominant channel
        match (r >= g, r >= b, g >= b) {
            (true, true, true) if g as i16 - b as i16 > chroma / 3 => {
                if bright { Color::LightYellow } else { Color::Yellow }
            }
            (true, true, _) => {
                if bright { Color::LightRed } else { Color::Red }
            }
            (false, _, true) if g as i16 - r as i16 > chroma / 3 => {
                if bright { Color::LightGreen } else { Color::Green }
            }
            (false, _, true) => {
                if bright { Color::LightCyan } else { Color::Cyan }
            }
            (_, false, false) if r as i16 - g as i16 > chroma / 3 => {
                if bright { Color::LightMagenta } else { Color::Magenta }
            }
            (_, false, false) => {
                if bright { Color::LightBlue } else { Color::Blue }
            }
            _ => {
                if bright { Color::White } else { Color::Gray }
            }
        }
    } else {
        color
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
    fn test_rgb_to_256_greyscale() {
        // Pure black → index 16
        assert_eq!(rgb_to_256(Color::Rgb(0, 0, 0)), Color::Indexed(16));
        // Pure white → index 231
        assert_eq!(rgb_to_256(Color::Rgb(255, 255, 255)), Color::Indexed(231));
        // Mid grey → greyscale ramp
        let c = rgb_to_256(Color::Rgb(128, 128, 128));
        if let Color::Indexed(i) = c {
            assert!((232..=255).contains(&i), "Expected greyscale index, got {i}");
        } else {
            panic!("Expected Color::Indexed");
        }
    }

    #[test]
    fn test_rgb_to_256_colour() {
        // Pure red → should map to cube entry near red
        let c = rgb_to_256(Color::Rgb(255, 0, 0));
        assert_eq!(c, Color::Indexed(16 + 36 * 5)); // ri=5, gi=0, bi=0 → 196
    }

    #[test]
    fn test_rgb_to_basic() {
        // Pure white → White
        assert_eq!(rgb_to_basic(Color::Rgb(255, 255, 255)), Color::White);
        // Pure black → Black
        assert_eq!(rgb_to_basic(Color::Rgb(0, 0, 0)), Color::Black);
        // Non-RGB passes through
        assert_eq!(rgb_to_basic(Color::Red), Color::Red);
        assert_eq!(rgb_to_basic(Color::Reset), Color::Reset);
    }

    #[test]
    fn test_adapt_color_passthrough() {
        // truecolor mode passes through unchanged
        let c = Color::Rgb(100, 200, 50);
        assert_eq!(adapt_color(c, "truecolor"), c);
        // Non-RGB colours always pass through
        assert_eq!(adapt_color(Color::Red, "256"), Color::Red);
        assert_eq!(adapt_color(Color::Reset, "basic"), Color::Reset);
    }

    #[test]
    fn test_adapt_to_color_mode() {
        let theme = Theme::dark();
        let adapted = theme.adapt_to_color_mode("256");
        // All colours should now be Indexed, not Rgb
        if let Color::Indexed(_) = adapted.bg {
            // good
        } else {
            panic!("Expected bg to be Color::Indexed after 256 adaptation");
        }
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
