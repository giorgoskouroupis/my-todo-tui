use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_tertiary: Color,
    pub accent: Color,
    pub accent_selection: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_disabled: Color,
    pub text_placeholder: Color,
    pub border_default: Color,
}

impl Theme {
    pub const fn one_dark() -> Self {
        Self {
            name: "one-dark",
            bg_primary: Color::Rgb(0x28, 0x2c, 0x34),
            bg_secondary: Color::Rgb(0x2f, 0x34, 0x3e),
            bg_tertiary: Color::Rgb(0x3a, 0x4b, 0x5f),
            accent: Color::Rgb(0x6b, 0xbc, 0xff),
            accent_selection: Color::Rgb(0x3a, 0x4b, 0x5f),
            success: Color::Rgb(0xa6, 0xd1, 0x89),
            warning: Color::Rgb(0xf0, 0xca, 0x83),
            error: Color::Rgb(0xea, 0x75, 0x80),
            text_primary: Color::Rgb(0xe5, 0xe9, 0xf2),
            text_secondary: Color::Rgb(0xc4, 0xcb, 0xd6),
            text_muted: Color::Rgb(0x74, 0x7f, 0x96),
            text_disabled: Color::Rgb(0x74, 0x7f, 0x96),
            text_placeholder: Color::Rgb(0x74, 0x7f, 0x96),
            border_default: Color::Rgb(0x3b, 0x41, 0x4d),
        }
    }

    pub const fn catppuccin_mocha() -> Self {
        Self {
            name: "catppuccin-mocha",
            bg_primary: Color::Rgb(0x1e, 0x1e, 0x2e),
            bg_secondary: Color::Rgb(0x25, 0x25, 0x3a),
            bg_tertiary: Color::Rgb(0x30, 0x30, 0x4a),
            accent: Color::Rgb(0x89, 0xb4, 0xfa),
            accent_selection: Color::Rgb(0x30, 0x30, 0x4a),
            success: Color::Rgb(0xa6, 0xe3, 0xa1),
            warning: Color::Rgb(0xf9, 0xe2, 0xaf),
            error: Color::Rgb(0xf3, 0x8b, 0xa8),
            text_primary: Color::Rgb(0xcd, 0xd6, 0xf4),
            text_secondary: Color::Rgb(0xba, 0xc2, 0xde),
            text_muted: Color::Rgb(0x6c, 0x70, 0x86),
            text_disabled: Color::Rgb(0x6c, 0x70, 0x86),
            text_placeholder: Color::Rgb(0x6c, 0x70, 0x86),
            border_default: Color::Rgb(0x45, 0x45, 0x64),
        }
    }

    pub const fn dracula() -> Self {
        Self {
            name: "dracula",
            bg_primary: Color::Rgb(0x28, 0x28, 0x28),
            bg_secondary: Color::Rgb(0x32, 0x32, 0x32),
            bg_tertiary: Color::Rgb(0x3c, 0x3c, 0x3c),
            accent: Color::Rgb(0xbd, 0x93, 0xf9),
            accent_selection: Color::Rgb(0x3c, 0x3c, 0x3c),
            success: Color::Rgb(0x50, 0xfa, 0x7b),
            warning: Color::Rgb(0xf1, 0xfa, 0x8c),
            error: Color::Rgb(0xff, 0x55, 0x55),
            text_primary: Color::Rgb(0xf8, 0xf8, 0xf2),
            text_secondary: Color::Rgb(0xbf, 0xbf, 0xbf),
            text_muted: Color::Rgb(0x62, 0x62, 0x62),
            text_disabled: Color::Rgb(0x62, 0x62, 0x62),
            text_placeholder: Color::Rgb(0x62, 0x62, 0x62),
            border_default: Color::Rgb(0x44, 0x44, 0x44),
        }
    }

    pub const fn nord() -> Self {
        Self {
            name: "nord",
            bg_primary: Color::Rgb(0x2e, 0x34, 0x40),
            bg_secondary: Color::Rgb(0x3b, 0x42, 0x52),
            bg_tertiary: Color::Rgb(0x43, 0x4c, 0x5e),
            accent: Color::Rgb(0x88, 0xc0, 0xd0),
            accent_selection: Color::Rgb(0x43, 0x4c, 0x5e),
            success: Color::Rgb(0xa3, 0xbe, 0x8c),
            warning: Color::Rgb(0xeb, 0xcb, 0x8b),
            error: Color::Rgb(0xbf, 0x61, 0x6a),
            text_primary: Color::Rgb(0xd8, 0xde, 0xe9),
            text_secondary: Color::Rgb(0xe5, 0xe9, 0xf0),
            text_muted: Color::Rgb(0x61, 0x6e, 0x88),
            text_disabled: Color::Rgb(0x61, 0x6e, 0x88),
            text_placeholder: Color::Rgb(0x61, 0x6e, 0x88),
            border_default: Color::Rgb(0x4c, 0x56, 0x6a),
        }
    }

    pub const fn gruvbox() -> Self {
        Self {
            name: "gruvbox",
            bg_primary: Color::Rgb(0x28, 0x28, 0x28),
            bg_secondary: Color::Rgb(0x32, 0x30, 0x2f),
            bg_tertiary: Color::Rgb(0x3c, 0x38, 0x36),
            accent: Color::Rgb(0xd7, 0x99, 0x21),
            accent_selection: Color::Rgb(0x3c, 0x38, 0x36),
            success: Color::Rgb(0x98, 0x97, 0x1a),
            warning: Color::Rgb(0xd7, 0x99, 0x21),
            error: Color::Rgb(0xfb, 0x49, 0x34),
            text_primary: Color::Rgb(0xeb, 0xdb, 0xb2),
            text_secondary: Color::Rgb(0xd5, 0xc4, 0xa1),
            text_muted: Color::Rgb(0x92, 0x83, 0x74),
            text_disabled: Color::Rgb(0x92, 0x83, 0x74),
            text_placeholder: Color::Rgb(0x92, 0x83, 0x74),
            border_default: Color::Rgb(0x50, 0x49, 0x45),
        }
    }

    pub const fn one_light() -> Self {
        Self {
            name: "one-light",
            bg_primary: Color::Rgb(0xfa, 0xfa, 0xfa),
            bg_secondary: Color::Rgb(0xef, 0xef, 0xef),
            bg_tertiary: Color::Rgb(0xdc, 0xe2, 0xea),
            accent: Color::Rgb(0x40, 0x78, 0xf2),
            accent_selection: Color::Rgb(0xdc, 0xe2, 0xea),
            success: Color::Rgb(0x50, 0xa1, 0x4f),
            warning: Color::Rgb(0xc1, 0x84, 0x01),
            error: Color::Rgb(0xe4, 0x56, 0x49),
            text_primary: Color::Rgb(0x38, 0x3a, 0x42),
            text_secondary: Color::Rgb(0x4f, 0x52, 0x5c),
            text_muted: Color::Rgb(0xa0, 0xa1, 0xa7),
            text_disabled: Color::Rgb(0xa0, 0xa1, 0xa7),
            text_placeholder: Color::Rgb(0xa0, 0xa1, 0xa7),
            border_default: Color::Rgb(0xd0, 0xd0, 0xd0),
        }
    }

    pub const fn catppuccin_latte() -> Self {
        Self {
            name: "catppuccin-latte",
            bg_primary: Color::Rgb(0xef, 0xf1, 0xf5),
            bg_secondary: Color::Rgb(0xe6, 0xe9, 0xef),
            bg_tertiary: Color::Rgb(0xcc, 0xd0, 0xda),
            accent: Color::Rgb(0x1e, 0x66, 0xf5),
            accent_selection: Color::Rgb(0xcc, 0xd0, 0xda),
            success: Color::Rgb(0x40, 0xa0, 0x2b),
            warning: Color::Rgb(0xdf, 0x8e, 0x1d),
            error: Color::Rgb(0xd2, 0x0f, 0x39),
            text_primary: Color::Rgb(0x4c, 0x4f, 0x69),
            text_secondary: Color::Rgb(0x6c, 0x6f, 0x85),
            text_muted: Color::Rgb(0x9c, 0xa0, 0xb0),
            text_disabled: Color::Rgb(0x9c, 0xa0, 0xb0),
            text_placeholder: Color::Rgb(0x9c, 0xa0, 0xb0),
            border_default: Color::Rgb(0xbc, 0xc0, 0xcc),
        }
    }

    pub const fn solarized_light() -> Self {
        Self {
            name: "solarized-light",
            bg_primary: Color::Rgb(0xfd, 0xf6, 0xe3),
            bg_secondary: Color::Rgb(0xee, 0xe8, 0xd5),
            bg_tertiary: Color::Rgb(0xe3, 0xdb, 0xb8),
            accent: Color::Rgb(0x26, 0x8b, 0xd2),
            accent_selection: Color::Rgb(0xe3, 0xdb, 0xb8),
            success: Color::Rgb(0x85, 0x99, 0x00),
            warning: Color::Rgb(0xb5, 0x89, 0x00),
            error: Color::Rgb(0xdc, 0x32, 0x2f),
            text_primary: Color::Rgb(0x58, 0x6e, 0x75),
            text_secondary: Color::Rgb(0x65, 0x7b, 0x83),
            text_muted: Color::Rgb(0x93, 0xa1, 0xa1),
            text_disabled: Color::Rgb(0x93, 0xa1, 0xa1),
            text_placeholder: Color::Rgb(0x93, 0xa1, 0xa1),
            border_default: Color::Rgb(0xd6, 0xce, 0xac),
        }
    }

    pub const fn gruvbox_light() -> Self {
        Self {
            name: "gruvbox-light",
            bg_primary: Color::Rgb(0xfb, 0xf1, 0xc7),
            bg_secondary: Color::Rgb(0xf2, 0xe5, 0xbc),
            bg_tertiary: Color::Rgb(0xeb, 0xdb, 0xb2),
            accent: Color::Rgb(0x07, 0x66, 0x78),
            accent_selection: Color::Rgb(0xeb, 0xdb, 0xb2),
            success: Color::Rgb(0x79, 0x74, 0x0e),
            warning: Color::Rgb(0xb5, 0x76, 0x14),
            error: Color::Rgb(0xcc, 0x24, 0x1d),
            text_primary: Color::Rgb(0x3c, 0x38, 0x36),
            text_secondary: Color::Rgb(0x50, 0x49, 0x45),
            text_muted: Color::Rgb(0x7c, 0x6f, 0x64),
            text_disabled: Color::Rgb(0x7c, 0x6f, 0x64),
            text_placeholder: Color::Rgb(0x7c, 0x6f, 0x64),
            border_default: Color::Rgb(0xd5, 0xc4, 0xa1),
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "one-dark" => Some(Self::one_dark()),
            "catppuccin-mocha" => Some(Self::catppuccin_mocha()),
            "dracula" => Some(Self::dracula()),
            "nord" => Some(Self::nord()),
            "gruvbox" => Some(Self::gruvbox()),
            "one-light" => Some(Self::one_light()),
            "catppuccin-latte" => Some(Self::catppuccin_latte()),
            "solarized-light" => Some(Self::solarized_light()),
            "gruvbox-light" => Some(Self::gruvbox_light()),
            _ => None,
        }
    }

    /// `(name, is_light)`, ordered so that all light themes appear before dark ones.
    pub const fn theme_registry() -> &'static [(&'static str, bool)] {
        &[
            ("catppuccin-latte", true),
            ("gruvbox-light", true),
            ("one-light", true),
            ("solarized-light", true),
            ("catppuccin-mocha", false),
            ("dracula", false),
            ("gruvbox", false),
            ("nord", false),
            ("one-dark", false),
        ]
    }

    pub fn theme_names() -> Vec<&'static str> {
        Self::theme_registry()
            .iter()
            .map(|(name, _)| *name)
            .collect()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::one_dark()
    }
}
