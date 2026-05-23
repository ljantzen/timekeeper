use std::collections::HashMap;

use ratatui::style::Color;
use tmkpr_lib::config::ThemeConfig;

/// Semantic color roles used throughout the UI.
#[derive(Clone)]
pub struct Theme {
    /// Terminal background. `Color::Reset` means "use the terminal's own bg".
    pub bg: Color,
    /// Active / running entry, success state.
    pub active: Color,
    /// Focused borders, accents, status-ok messages.
    pub accent: Color,
    /// Secondary text, hints, empty-state messages.
    pub dim: Color,
    /// Errors and destructive-action dialogs.
    pub error: Color,
    /// Warning / create-confirmation dialogs.
    pub warning: Color,
}

fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

impl Theme {
    /// Resolve a theme by name, checking user-defined themes before built-ins.
    /// Invalid hex values in a user theme fall back to the default palette colour.
    pub fn resolve(name: &str, custom: &HashMap<String, ThemeConfig>) -> Self {
        if let Some(cfg) = custom.get(name) {
            let fb = Self::default_theme();
            return Self {
                bg: parse_hex(&cfg.bg).unwrap_or(Color::Reset),
                active: parse_hex(&cfg.active).unwrap_or(fb.active),
                accent: parse_hex(&cfg.accent).unwrap_or(fb.accent),
                dim: parse_hex(&cfg.dim).unwrap_or(fb.dim),
                error: parse_hex(&cfg.error).unwrap_or(fb.error),
                warning: parse_hex(&cfg.warning).unwrap_or(fb.warning),
            };
        }
        Self::from_name(name)
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "rose_pine" => Self::rose_pine(),
            "catppuccin_mocha" => Self::catppuccin_mocha(),
            "catppuccin_macchiato" => Self::catppuccin_macchiato(),
            "catppuccin_frappe" => Self::catppuccin_frappe(),
            "nord" => Self::nord(),
            "gruvbox_dark" => Self::gruvbox_dark(),
            "monokai" => Self::monokai(),
            "dracula" => Self::dracula(),
            "tokyonight" => Self::tokyonight(),
            "onedark" => Self::onedark(),
            "solarized_dark" => Self::solarized_dark(),
            "github_dark" => Self::github_dark(),
            "kanagawa" => Self::kanagawa(),
            "everforest" => Self::everforest(),
            "ayu_dark" => Self::ayu_dark(),
            _ => Self::default_theme(),
        }
    }

    /// Terminal-palette colours — adapts to the user's own terminal theme.
    fn default_theme() -> Self {
        Self {
            bg: Color::Reset,
            active: Color::Green,
            accent: Color::Cyan,
            dim: Color::DarkGray,
            error: Color::Red,
            warning: Color::Yellow,
        }
    }

    fn rose_pine() -> Self {
        Self {
            bg: Color::Rgb(0x19, 0x17, 0x24),
            active: Color::Rgb(0x31, 0x74, 0x8f),
            accent: Color::Rgb(0x9c, 0xcf, 0xd8),
            dim: Color::Rgb(0x6e, 0x6a, 0x86),
            error: Color::Rgb(0xeb, 0x6f, 0x92),
            warning: Color::Rgb(0xf6, 0xc1, 0x77),
        }
    }

    fn catppuccin_mocha() -> Self {
        Self {
            bg: Color::Rgb(0x1e, 0x1e, 0x2e),
            active: Color::Rgb(0xa6, 0xe3, 0xa1),
            accent: Color::Rgb(0xcb, 0xa6, 0xf7),
            dim: Color::Rgb(0x7f, 0x84, 0x9c),
            error: Color::Rgb(0xf3, 0x8b, 0xa8),
            warning: Color::Rgb(0xf9, 0xe2, 0xaf),
        }
    }

    fn catppuccin_macchiato() -> Self {
        Self {
            bg: Color::Rgb(0x24, 0x27, 0x3a),
            active: Color::Rgb(0xa6, 0xda, 0x95),
            accent: Color::Rgb(0xc6, 0xa0, 0xf6),
            dim: Color::Rgb(0x80, 0x87, 0xa2),
            error: Color::Rgb(0xed, 0x87, 0x96),
            warning: Color::Rgb(0xee, 0xd4, 0x9f),
        }
    }

    fn catppuccin_frappe() -> Self {
        Self {
            bg: Color::Rgb(0x30, 0x34, 0x46),
            active: Color::Rgb(0xa6, 0xd1, 0x89),
            accent: Color::Rgb(0xca, 0x9e, 0xe6),
            dim: Color::Rgb(0x83, 0x8b, 0xa7),
            error: Color::Rgb(0xe7, 0x82, 0x84),
            warning: Color::Rgb(0xe5, 0xc8, 0x90),
        }
    }

    fn nord() -> Self {
        Self {
            bg: Color::Rgb(0x2e, 0x34, 0x40),
            active: Color::Rgb(0xa3, 0xbe, 0x8c),
            accent: Color::Rgb(0x88, 0xc0, 0xd0),
            dim: Color::Rgb(0x61, 0x6e, 0x87),
            error: Color::Rgb(0xbf, 0x61, 0x6a),
            warning: Color::Rgb(0xd0, 0x87, 0x70),
        }
    }

    fn gruvbox_dark() -> Self {
        Self {
            bg: Color::Rgb(0x28, 0x28, 0x28),
            active: Color::Rgb(0xb8, 0xbb, 0x26),
            accent: Color::Rgb(0x83, 0xa5, 0x98),
            dim: Color::Rgb(0x92, 0x83, 0x74),
            error: Color::Rgb(0xfb, 0x49, 0x34),
            warning: Color::Rgb(0xfa, 0xbd, 0x2f),
        }
    }

    fn monokai() -> Self {
        Self {
            bg: Color::Rgb(0x27, 0x28, 0x22),
            active: Color::Rgb(0xa6, 0xe2, 0x2e),
            accent: Color::Rgb(0x66, 0xd9, 0xe8),
            dim: Color::Rgb(0x75, 0x71, 0x5e),
            error: Color::Rgb(0xf9, 0x26, 0x72),
            warning: Color::Rgb(0xe6, 0xdb, 0x74),
        }
    }

    fn dracula() -> Self {
        Self {
            bg: Color::Rgb(0x28, 0x2a, 0x36),
            active: Color::Rgb(0x50, 0xfa, 0x7b),
            accent: Color::Rgb(0x8b, 0xe9, 0xfd),
            dim: Color::Rgb(0x62, 0x72, 0xa4),
            error: Color::Rgb(0xff, 0x55, 0x55),
            warning: Color::Rgb(0xff, 0xb8, 0x6c),
        }
    }

    fn tokyonight() -> Self {
        Self {
            bg: Color::Rgb(0x1a, 0x1b, 0x26),
            active: Color::Rgb(0x9e, 0xce, 0x6a),
            accent: Color::Rgb(0x7a, 0xa2, 0xf7),
            dim: Color::Rgb(0x56, 0x5f, 0x89),
            error: Color::Rgb(0xf7, 0x76, 0x8e),
            warning: Color::Rgb(0xe0, 0xaf, 0x68),
        }
    }

    fn onedark() -> Self {
        Self {
            bg: Color::Rgb(0x28, 0x2c, 0x34),
            active: Color::Rgb(0x98, 0xc3, 0x79),
            accent: Color::Rgb(0x61, 0xaf, 0xef),
            dim: Color::Rgb(0x5c, 0x63, 0x70),
            error: Color::Rgb(0xe0, 0x6c, 0x75),
            warning: Color::Rgb(0xe5, 0xc0, 0x7b),
        }
    }

    fn solarized_dark() -> Self {
        Self {
            bg: Color::Rgb(0x00, 0x2b, 0x36),
            active: Color::Rgb(0x85, 0x99, 0x00),
            accent: Color::Rgb(0x26, 0x8b, 0xd2),
            dim: Color::Rgb(0x58, 0x6e, 0x75),
            error: Color::Rgb(0xdc, 0x32, 0x2f),
            warning: Color::Rgb(0xb5, 0x89, 0x00),
        }
    }

    fn github_dark() -> Self {
        Self {
            bg: Color::Rgb(0x0d, 0x11, 0x17),
            active: Color::Rgb(0x7e, 0xe7, 0x87),
            accent: Color::Rgb(0x79, 0xc0, 0xff),
            dim: Color::Rgb(0x8b, 0x94, 0x9e),
            error: Color::Rgb(0xff, 0x7b, 0x72),
            warning: Color::Rgb(0xe3, 0xb3, 0x41),
        }
    }

    fn kanagawa() -> Self {
        Self {
            bg: Color::Rgb(0x1f, 0x1f, 0x28),
            active: Color::Rgb(0x98, 0xbb, 0x6c),
            accent: Color::Rgb(0x7e, 0x9c, 0xd8),
            dim: Color::Rgb(0x72, 0x71, 0x69),
            error: Color::Rgb(0xc3, 0x40, 0x43),
            warning: Color::Rgb(0xdc, 0xa5, 0x61),
        }
    }

    fn everforest() -> Self {
        Self {
            bg: Color::Rgb(0x2d, 0x35, 0x3b),
            active: Color::Rgb(0xa7, 0xc0, 0x80),
            accent: Color::Rgb(0x7f, 0xbb, 0xb3),
            dim: Color::Rgb(0x7a, 0x84, 0x78),
            error: Color::Rgb(0xe6, 0x7e, 0x80),
            warning: Color::Rgb(0xdb, 0xbc, 0x7f),
        }
    }

    fn ayu_dark() -> Self {
        Self {
            bg: Color::Rgb(0x0a, 0x0e, 0x14),
            active: Color::Rgb(0xaa, 0xd9, 0x4c),
            accent: Color::Rgb(0x59, 0xc2, 0xff),
            dim: Color::Rgb(0x62, 0x6a, 0x73),
            error: Color::Rgb(0xf0, 0x71, 0x78),
            warning: Color::Rgb(0xe6, 0xb4, 0x50),
        }
    }
}
