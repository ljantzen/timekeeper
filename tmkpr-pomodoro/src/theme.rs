use std::collections::HashMap;

use ratatui::style::Color;
use tmkpr_lib::config::ThemeConfig;

/// Semantic color roles used throughout the UI.
pub struct Theme {
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
            active: Color::Green,
            accent: Color::Cyan,
            dim: Color::DarkGray,
            error: Color::Red,
            warning: Color::Yellow,
        }
    }

    /// Rose Pinė — https://github.com/rose-pine/helix
    fn rose_pine() -> Self {
        Self {
            active: Color::Rgb(0x31, 0x74, 0x8f),  // pine
            accent: Color::Rgb(0x9c, 0xcf, 0xd8),  // foam
            dim: Color::Rgb(0x6e, 0x6a, 0x86),     // muted
            error: Color::Rgb(0xeb, 0x6f, 0x92),   // love
            warning: Color::Rgb(0xf6, 0xc1, 0x77), // gold
        }
    }

    /// Catppuccin Mocha — https://github.com/catppuccin/helix
    fn catppuccin_mocha() -> Self {
        Self {
            active: Color::Rgb(0xa6, 0xe3, 0xa1),  // green
            accent: Color::Rgb(0xcb, 0xa6, 0xf7),  // mauve
            dim: Color::Rgb(0x7f, 0x84, 0x9c),     // overlay1
            error: Color::Rgb(0xf3, 0x8b, 0xa8),   // red
            warning: Color::Rgb(0xf9, 0xe2, 0xaf), // yellow
        }
    }

    /// Catppuccin Macchiato — https://github.com/catppuccin/helix
    fn catppuccin_macchiato() -> Self {
        Self {
            active: Color::Rgb(0xa6, 0xda, 0x95),  // green
            accent: Color::Rgb(0xc6, 0xa0, 0xf6),  // mauve
            dim: Color::Rgb(0x80, 0x87, 0xa2),     // overlay1
            error: Color::Rgb(0xed, 0x87, 0x96),   // red
            warning: Color::Rgb(0xee, 0xd4, 0x9f), // yellow
        }
    }

    /// Catppuccin Frappé — https://github.com/catppuccin/helix
    fn catppuccin_frappe() -> Self {
        Self {
            active: Color::Rgb(0xa6, 0xd1, 0x89),  // green
            accent: Color::Rgb(0xca, 0x9e, 0xe6),  // mauve
            dim: Color::Rgb(0x83, 0x8b, 0xa7),     // overlay1
            error: Color::Rgb(0xe7, 0x82, 0x84),   // red
            warning: Color::Rgb(0xe5, 0xc8, 0x90), // yellow
        }
    }

    /// Nord — https://github.com/arcticicestudio/nord
    fn nord() -> Self {
        Self {
            active: Color::Rgb(0xa3, 0xbe, 0x8c),  // nord14 sage green
            accent: Color::Rgb(0x88, 0xc0, 0xd0),  // nord8 light blue
            dim: Color::Rgb(0x61, 0x6e, 0x87),     // lightened nord3
            error: Color::Rgb(0xbf, 0x61, 0x6a),   // nord11 red
            warning: Color::Rgb(0xd0, 0x87, 0x70), // nord12 orange
        }
    }

    /// Gruvbox Dark — https://github.com/morhetz/gruvbox
    fn gruvbox_dark() -> Self {
        Self {
            active: Color::Rgb(0xb8, 0xbb, 0x26),  // bright green
            accent: Color::Rgb(0x83, 0xa5, 0x98),  // bright aqua/blue
            dim: Color::Rgb(0x92, 0x83, 0x74),     // gray fg4
            error: Color::Rgb(0xfb, 0x49, 0x34),   // bright red
            warning: Color::Rgb(0xfa, 0xbd, 0x2f), // bright yellow
        }
    }

    /// Monokai — https://monokai.pro
    fn monokai() -> Self {
        Self {
            active: Color::Rgb(0xa6, 0xe2, 0x2e),  // green
            accent: Color::Rgb(0x66, 0xd9, 0xe8),  // cyan
            dim: Color::Rgb(0x75, 0x71, 0x5e),     // comment gray
            error: Color::Rgb(0xf9, 0x26, 0x72),   // red
            warning: Color::Rgb(0xe6, 0xdb, 0x74), // yellow
        }
    }

    /// Dracula — https://draculatheme.com
    fn dracula() -> Self {
        Self {
            active: Color::Rgb(0x50, 0xfa, 0x7b),  // green
            accent: Color::Rgb(0x8b, 0xe9, 0xfd),  // cyan
            dim: Color::Rgb(0x62, 0x72, 0xa4),     // comment blue-gray
            error: Color::Rgb(0xff, 0x55, 0x55),   // red
            warning: Color::Rgb(0xff, 0xb8, 0x6c), // orange
        }
    }

    /// Tokyo Night — https://github.com/enkia/tokyo-night-vscode-theme
    fn tokyonight() -> Self {
        Self {
            active: Color::Rgb(0x9e, 0xce, 0x6a),  // green
            accent: Color::Rgb(0x7a, 0xa2, 0xf7),  // blue
            dim: Color::Rgb(0x56, 0x5f, 0x89),     // comment
            error: Color::Rgb(0xf7, 0x76, 0x8e),   // red
            warning: Color::Rgb(0xe0, 0xaf, 0x68), // yellow
        }
    }

    /// One Dark — Atom One Dark (helix onedark)
    fn onedark() -> Self {
        Self {
            active: Color::Rgb(0x98, 0xc3, 0x79),  // green
            accent: Color::Rgb(0x61, 0xaf, 0xef),  // blue
            dim: Color::Rgb(0x5c, 0x63, 0x70),     // comment
            error: Color::Rgb(0xe0, 0x6c, 0x75),   // red
            warning: Color::Rgb(0xe5, 0xc0, 0x7b), // yellow
        }
    }

    /// Solarized Dark — https://ethanschoonover.com/solarized
    fn solarized_dark() -> Self {
        Self {
            active: Color::Rgb(0x85, 0x99, 0x00),  // green
            accent: Color::Rgb(0x26, 0x8b, 0xd2),  // blue
            dim: Color::Rgb(0x58, 0x6e, 0x75),     // base01
            error: Color::Rgb(0xdc, 0x32, 0x2f),   // red
            warning: Color::Rgb(0xb5, 0x89, 0x00), // yellow
        }
    }

    /// GitHub Dark — https://github.com/primer/github-vscode-theme
    fn github_dark() -> Self {
        Self {
            active: Color::Rgb(0x7e, 0xe7, 0x87),  // green
            accent: Color::Rgb(0x79, 0xc0, 0xff),  // blue
            dim: Color::Rgb(0x8b, 0x94, 0x9e),     // comment gray
            error: Color::Rgb(0xff, 0x7b, 0x72),   // red
            warning: Color::Rgb(0xe3, 0xb3, 0x41), // yellow
        }
    }

    /// Kanagawa — https://github.com/rebelot/kanagawa.nvim
    fn kanagawa() -> Self {
        Self {
            active: Color::Rgb(0x98, 0xbb, 0x6c),  // springGreen
            accent: Color::Rgb(0x7e, 0x9c, 0xd8),  // crystalBlue
            dim: Color::Rgb(0x72, 0x71, 0x69),     // fujiGray
            error: Color::Rgb(0xc3, 0x40, 0x43),   // autumnRed
            warning: Color::Rgb(0xdc, 0xa5, 0x61), // carpYellow
        }
    }

    /// Everforest — https://github.com/sainnhe/everforest
    fn everforest() -> Self {
        Self {
            active: Color::Rgb(0xa7, 0xc0, 0x80),  // green
            accent: Color::Rgb(0x7f, 0xbb, 0xb3),  // blue
            dim: Color::Rgb(0x7a, 0x84, 0x78),     // grey0
            error: Color::Rgb(0xe6, 0x7e, 0x80),   // red
            warning: Color::Rgb(0xdb, 0xbc, 0x7f), // yellow
        }
    }

    /// Ayu Dark — https://github.com/dempfi/ayu
    fn ayu_dark() -> Self {
        Self {
            active: Color::Rgb(0xaa, 0xd9, 0x4c),  // green
            accent: Color::Rgb(0x59, 0xc2, 0xff),  // blue
            dim: Color::Rgb(0x62, 0x6a, 0x73),     // comment
            error: Color::Rgb(0xf0, 0x71, 0x78),   // red
            warning: Color::Rgb(0xe6, 0xb4, 0x50), // yellow
        }
    }
}
