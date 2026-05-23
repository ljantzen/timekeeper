use std::collections::HashMap;

use ratatui::style::Color;
use tmkpr_lib::config::ThemeConfig;

/// Semantic color roles used throughout the UI.
#[derive(Clone)]
pub struct Theme {
    /// Terminal background. `Color::Reset` means "use the terminal's own bg".
    pub bg: Color,
    /// Primary text color. `None` means "use the terminal's own fg" (suitable for dark themes).
    pub fg: Option<Color>,
    /// Active / running entry, success state.
    pub active: Color,
    /// Focused borders, accents, status-ok messages.
    pub accent: Color,
    /// Secondary text, hints, timestamps, notes.
    pub dim: Color,
    /// Errors and destructive-action dialogs.
    pub error: Color,
    /// Warning / create-confirmation dialogs.
    pub warning: Color,
    /// Background highlight for the selected list row.
    pub selection: Color,
    /// Panel and modal border color.
    pub border: Color,
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
    pub fn builtin_names() -> &'static [&'static str] {
        &[
            "default",
            "ayu_dark",
            "catppuccin_frappe",
            "catppuccin_latte",
            "catppuccin_macchiato",
            "catppuccin_mocha",
            "dracula",
            "everforest",
            "github_dark",
            "github_light",
            "gruvbox_dark",
            "gruvbox_light",
            "high_contrast",
            "kanagawa",
            "monokai",
            "nord",
            "onedark",
            "rose_pine",
            "solarized_dark",
            "solarized_light",
            "tokyonight",
        ]
    }

    /// Resolve a theme by name, checking user-defined themes before built-ins.
    /// Invalid hex values in a user theme fall back to the default palette colour.
    pub fn resolve(name: &str, custom: &HashMap<String, ThemeConfig>) -> Self {
        if let Some(cfg) = custom.get(name) {
            let fb = Self::default_theme();
            return Self {
                bg: parse_hex(&cfg.bg).unwrap_or(Color::Reset),
                fg: parse_hex(&cfg.fg),
                active: parse_hex(&cfg.active).unwrap_or(fb.active),
                accent: parse_hex(&cfg.accent).unwrap_or(fb.accent),
                dim: parse_hex(&cfg.dim).unwrap_or(fb.dim),
                error: parse_hex(&cfg.error).unwrap_or(fb.error),
                warning: parse_hex(&cfg.warning).unwrap_or(fb.warning),
                selection: parse_hex(&cfg.selection).unwrap_or(fb.selection),
                border: parse_hex(&cfg.border).unwrap_or(fb.border),
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
            "catppuccin_latte" => Self::catppuccin_latte(),
            "nord" => Self::nord(),
            "gruvbox_dark" => Self::gruvbox_dark(),
            "gruvbox_light" => Self::gruvbox_light(),
            "monokai" => Self::monokai(),
            "dracula" => Self::dracula(),
            "tokyonight" => Self::tokyonight(),
            "onedark" => Self::onedark(),
            "solarized_dark" => Self::solarized_dark(),
            "solarized_light" => Self::solarized_light(),
            "github_dark" => Self::github_dark(),
            "github_light" => Self::github_light(),
            "kanagawa" => Self::kanagawa(),
            "everforest" => Self::everforest(),
            "ayu_dark" => Self::ayu_dark(),
            "high_contrast" => Self::high_contrast(),
            _ => Self::default_theme(),
        }
    }

    /// Terminal-palette colours — adapts to the user's own terminal theme.
    fn default_theme() -> Self {
        Self {
            bg: Color::Reset,
            fg: None,
            active: Color::Green,
            accent: Color::Cyan,
            dim: Color::DarkGray,
            error: Color::Red,
            warning: Color::Yellow,
            selection: Color::Rgb(0x3a, 0x3a, 0x3a),
            border: Color::DarkGray,
        }
    }

    /// Rose Pinė — https://github.com/rose-pine/helix
    fn rose_pine() -> Self {
        Self {
            bg: Color::Rgb(0x19, 0x17, 0x24),
            fg: None,
            active: Color::Rgb(0x31, 0x74, 0x8f),
            accent: Color::Rgb(0x9c, 0xcf, 0xd8),
            dim: Color::Rgb(0x6e, 0x6a, 0x86),
            error: Color::Rgb(0xeb, 0x6f, 0x92),
            warning: Color::Rgb(0xf6, 0xc1, 0x77),
            selection: Color::Rgb(0x26, 0x23, 0x3a),
            border: Color::Rgb(0x40, 0x3d, 0x52),
        }
    }

    /// Catppuccin Mocha — https://github.com/catppuccin/helix
    fn catppuccin_mocha() -> Self {
        Self {
            bg: Color::Rgb(0x1e, 0x1e, 0x2e),
            fg: None,
            active: Color::Rgb(0xa6, 0xe3, 0xa1),
            accent: Color::Rgb(0xcb, 0xa6, 0xf7),
            dim: Color::Rgb(0x7f, 0x84, 0x9c),
            error: Color::Rgb(0xf3, 0x8b, 0xa8),
            warning: Color::Rgb(0xf9, 0xe2, 0xaf),
            selection: Color::Rgb(0x31, 0x32, 0x44),
            border: Color::Rgb(0x45, 0x47, 0x5a),
        }
    }

    /// Catppuccin Macchiato — https://github.com/catppuccin/helix
    fn catppuccin_macchiato() -> Self {
        Self {
            bg: Color::Rgb(0x24, 0x27, 0x3a),
            fg: None,
            active: Color::Rgb(0xa6, 0xda, 0x95),
            accent: Color::Rgb(0xc6, 0xa0, 0xf6),
            dim: Color::Rgb(0x80, 0x87, 0xa2),
            error: Color::Rgb(0xed, 0x87, 0x96),
            warning: Color::Rgb(0xee, 0xd4, 0x9f),
            selection: Color::Rgb(0x36, 0x3a, 0x4f),
            border: Color::Rgb(0x49, 0x4d, 0x64),
        }
    }

    /// Catppuccin Frappé — https://github.com/catppuccin/helix
    fn catppuccin_frappe() -> Self {
        Self {
            bg: Color::Rgb(0x30, 0x34, 0x46),
            fg: None,
            active: Color::Rgb(0xa6, 0xd1, 0x89),
            accent: Color::Rgb(0xca, 0x9e, 0xe6),
            dim: Color::Rgb(0x83, 0x8b, 0xa7),
            error: Color::Rgb(0xe7, 0x82, 0x84),
            warning: Color::Rgb(0xe5, 0xc8, 0x90),
            selection: Color::Rgb(0x41, 0x45, 0x59),
            border: Color::Rgb(0x51, 0x57, 0x6d),
        }
    }

    /// Catppuccin Latte — the light Catppuccin variant
    fn catppuccin_latte() -> Self {
        Self {
            bg: Color::Rgb(0xef, 0xf1, 0xf5),
            fg: Some(Color::Rgb(0x4c, 0x4f, 0x69)),
            active: Color::Rgb(0x40, 0xa0, 0x2b),
            accent: Color::Rgb(0x88, 0x39, 0xef),
            dim: Color::Rgb(0x9c, 0xa0, 0xb0),
            error: Color::Rgb(0xd2, 0x0f, 0x39),
            warning: Color::Rgb(0xdf, 0x8e, 0x1d),
            selection: Color::Rgb(0xcc, 0xd0, 0xda),
            border: Color::Rgb(0xbc, 0xc0, 0xcc),
        }
    }

    /// Nord — https://github.com/arcticicestudio/nord
    fn nord() -> Self {
        Self {
            bg: Color::Rgb(0x2e, 0x34, 0x40),
            fg: None,
            active: Color::Rgb(0xa3, 0xbe, 0x8c),
            accent: Color::Rgb(0x88, 0xc0, 0xd0),
            dim: Color::Rgb(0x61, 0x6e, 0x87),
            error: Color::Rgb(0xbf, 0x61, 0x6a),
            warning: Color::Rgb(0xd0, 0x87, 0x70),
            selection: Color::Rgb(0x3b, 0x42, 0x52),
            border: Color::Rgb(0x4c, 0x56, 0x6a),
        }
    }

    /// Gruvbox Dark — https://github.com/morhetz/gruvbox
    fn gruvbox_dark() -> Self {
        Self {
            bg: Color::Rgb(0x28, 0x28, 0x28),
            fg: None,
            active: Color::Rgb(0xb8, 0xbb, 0x26),
            accent: Color::Rgb(0x83, 0xa5, 0x98),
            dim: Color::Rgb(0x92, 0x83, 0x74),
            error: Color::Rgb(0xfb, 0x49, 0x34),
            warning: Color::Rgb(0xfa, 0xbd, 0x2f),
            selection: Color::Rgb(0x3c, 0x38, 0x36),
            border: Color::Rgb(0x50, 0x49, 0x45),
        }
    }

    /// Gruvbox Light — https://github.com/morhetz/gruvbox
    fn gruvbox_light() -> Self {
        Self {
            bg: Color::Rgb(0xfb, 0xf1, 0xc7),
            fg: Some(Color::Rgb(0x3c, 0x38, 0x36)),
            active: Color::Rgb(0x79, 0x74, 0x0e),
            accent: Color::Rgb(0x07, 0x66, 0x78),
            dim: Color::Rgb(0x7c, 0x6f, 0x64),
            error: Color::Rgb(0x9d, 0x00, 0x06),
            warning: Color::Rgb(0xb5, 0x76, 0x14),
            selection: Color::Rgb(0xeb, 0xdb, 0xb2),
            border: Color::Rgb(0xbd, 0xae, 0x93),
        }
    }

    /// Monokai — https://monokai.pro
    fn monokai() -> Self {
        Self {
            bg: Color::Rgb(0x27, 0x28, 0x22),
            fg: None,
            active: Color::Rgb(0xa6, 0xe2, 0x2e),
            accent: Color::Rgb(0x66, 0xd9, 0xe8),
            dim: Color::Rgb(0x75, 0x71, 0x5e),
            error: Color::Rgb(0xf9, 0x26, 0x72),
            warning: Color::Rgb(0xe6, 0xdb, 0x74),
            selection: Color::Rgb(0x38, 0x38, 0x30),
            border: Color::Rgb(0x49, 0x48, 0x3e),
        }
    }

    /// Dracula — https://draculatheme.com
    fn dracula() -> Self {
        Self {
            bg: Color::Rgb(0x28, 0x2a, 0x36),
            fg: None,
            active: Color::Rgb(0x50, 0xfa, 0x7b),
            accent: Color::Rgb(0x8b, 0xe9, 0xfd),
            dim: Color::Rgb(0x62, 0x72, 0xa4),
            error: Color::Rgb(0xff, 0x55, 0x55),
            warning: Color::Rgb(0xff, 0xb8, 0x6c),
            selection: Color::Rgb(0x44, 0x47, 0x5a),
            border: Color::Rgb(0x44, 0x47, 0x5a),
        }
    }

    /// Tokyo Night — https://github.com/enkia/tokyo-night-vscode-theme
    fn tokyonight() -> Self {
        Self {
            bg: Color::Rgb(0x1a, 0x1b, 0x26),
            fg: None,
            active: Color::Rgb(0x9e, 0xce, 0x6a),
            accent: Color::Rgb(0x7a, 0xa2, 0xf7),
            dim: Color::Rgb(0x56, 0x5f, 0x89),
            error: Color::Rgb(0xf7, 0x76, 0x8e),
            warning: Color::Rgb(0xe0, 0xaf, 0x68),
            selection: Color::Rgb(0x28, 0x34, 0x57),
            border: Color::Rgb(0x29, 0x2e, 0x42),
        }
    }

    /// One Dark — Atom One Dark (helix onedark)
    fn onedark() -> Self {
        Self {
            bg: Color::Rgb(0x28, 0x2c, 0x34),
            fg: None,
            active: Color::Rgb(0x98, 0xc3, 0x79),
            accent: Color::Rgb(0x61, 0xaf, 0xef),
            dim: Color::Rgb(0x5c, 0x63, 0x70),
            error: Color::Rgb(0xe0, 0x6c, 0x75),
            warning: Color::Rgb(0xe5, 0xc0, 0x7b),
            selection: Color::Rgb(0x3e, 0x44, 0x52),
            border: Color::Rgb(0x3e, 0x44, 0x52),
        }
    }

    /// Solarized Dark — https://ethanschoonover.com/solarized
    fn solarized_dark() -> Self {
        Self {
            bg: Color::Rgb(0x00, 0x2b, 0x36),
            fg: None,
            active: Color::Rgb(0x85, 0x99, 0x00),
            accent: Color::Rgb(0x26, 0x8b, 0xd2),
            dim: Color::Rgb(0x58, 0x6e, 0x75),
            error: Color::Rgb(0xdc, 0x32, 0x2f),
            warning: Color::Rgb(0xb5, 0x89, 0x00),
            selection: Color::Rgb(0x07, 0x36, 0x42),
            border: Color::Rgb(0x07, 0x36, 0x42),
        }
    }

    /// Solarized Light — https://ethanschoonover.com/solarized
    fn solarized_light() -> Self {
        Self {
            bg: Color::Rgb(0xfd, 0xf6, 0xe3),
            fg: Some(Color::Rgb(0x65, 0x7b, 0x83)),
            active: Color::Rgb(0x85, 0x99, 0x00),
            accent: Color::Rgb(0x26, 0x8b, 0xd2),
            dim: Color::Rgb(0x93, 0xa1, 0xa1),
            error: Color::Rgb(0xdc, 0x32, 0x2f),
            warning: Color::Rgb(0xb5, 0x89, 0x00),
            selection: Color::Rgb(0xee, 0xe8, 0xd5),
            border: Color::Rgb(0x93, 0xa1, 0xa1),
        }
    }

    /// GitHub Dark — https://github.com/primer/github-vscode-theme
    fn github_dark() -> Self {
        Self {
            bg: Color::Rgb(0x0d, 0x11, 0x17),
            fg: None,
            active: Color::Rgb(0x7e, 0xe7, 0x87),
            accent: Color::Rgb(0x79, 0xc0, 0xff),
            dim: Color::Rgb(0x8b, 0x94, 0x9e),
            error: Color::Rgb(0xff, 0x7b, 0x72),
            warning: Color::Rgb(0xe3, 0xb3, 0x41),
            selection: Color::Rgb(0x16, 0x1b, 0x22),
            border: Color::Rgb(0x30, 0x36, 0x3d),
        }
    }

    /// GitHub Light — https://github.com/primer/github-vscode-theme
    fn github_light() -> Self {
        Self {
            bg: Color::Rgb(0xff, 0xff, 0xff),
            fg: Some(Color::Rgb(0x1f, 0x23, 0x28)),
            active: Color::Rgb(0x1a, 0x7f, 0x37),
            accent: Color::Rgb(0x09, 0x69, 0xda),
            dim: Color::Rgb(0x6e, 0x77, 0x81),
            error: Color::Rgb(0xcf, 0x22, 0x2e),
            warning: Color::Rgb(0x9a, 0x67, 0x00),
            selection: Color::Rgb(0xf6, 0xf8, 0xfa),
            border: Color::Rgb(0xd0, 0xd7, 0xde),
        }
    }

    /// Kanagawa — https://github.com/rebelot/kanagawa.nvim
    fn kanagawa() -> Self {
        Self {
            bg: Color::Rgb(0x1f, 0x1f, 0x28),
            fg: None,
            active: Color::Rgb(0x98, 0xbb, 0x6c),
            accent: Color::Rgb(0x7e, 0x9c, 0xd8),
            dim: Color::Rgb(0x72, 0x71, 0x69),
            error: Color::Rgb(0xc3, 0x40, 0x43),
            warning: Color::Rgb(0xdc, 0xa5, 0x61),
            selection: Color::Rgb(0x2a, 0x2a, 0x37),
            border: Color::Rgb(0x36, 0x36, 0x46),
        }
    }

    /// Everforest — https://github.com/sainnhe/everforest
    fn everforest() -> Self {
        Self {
            bg: Color::Rgb(0x2d, 0x35, 0x3b),
            fg: None,
            active: Color::Rgb(0xa7, 0xc0, 0x80),
            accent: Color::Rgb(0x7f, 0xbb, 0xb3),
            dim: Color::Rgb(0x7a, 0x84, 0x78),
            error: Color::Rgb(0xe6, 0x7e, 0x80),
            warning: Color::Rgb(0xdb, 0xbc, 0x7f),
            selection: Color::Rgb(0x37, 0x41, 0x45),
            border: Color::Rgb(0x4a, 0x55, 0x5b),
        }
    }

    /// Ayu Dark — https://github.com/dempfi/ayu
    fn ayu_dark() -> Self {
        Self {
            bg: Color::Rgb(0x0a, 0x0e, 0x14),
            fg: None,
            active: Color::Rgb(0xaa, 0xd9, 0x4c),
            accent: Color::Rgb(0x59, 0xc2, 0xff),
            dim: Color::Rgb(0x62, 0x6a, 0x73),
            error: Color::Rgb(0xf0, 0x71, 0x78),
            warning: Color::Rgb(0xe6, 0xb4, 0x50),
            selection: Color::Rgb(0x0d, 0x10, 0x17),
            border: Color::Rgb(0x1a, 0x1f, 0x29),
        }
    }

    /// High Contrast — true black background with vivid saturated colors
    fn high_contrast() -> Self {
        Self {
            bg: Color::Rgb(0x00, 0x00, 0x00),
            fg: None,
            active: Color::Rgb(0x00, 0xff, 0x00),
            accent: Color::Rgb(0x00, 0xd7, 0xff),
            dim: Color::Rgb(0x80, 0x80, 0x80),
            error: Color::Rgb(0xff, 0x33, 0x33),
            warning: Color::Rgb(0xff, 0xcc, 0x00),
            selection: Color::Rgb(0x1c, 0x1c, 0x1c),
            border: Color::Rgb(0x4a, 0x4a, 0x4a),
        }
    }
}
