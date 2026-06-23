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
            accent: Color::Rgb(0x74, 0xad, 0xe8),
            accent_selection: Color::Rgb(0x3a, 0x4b, 0x5f),
            success: Color::Rgb(0xa6, 0xd1, 0x89),
            warning: Color::Rgb(0xf0, 0xca, 0x83),
            error: Color::Rgb(0xea, 0x75, 0x80),
            text_primary: Color::Rgb(0xe5, 0xe9, 0xf2),
            text_secondary: Color::Rgb(0xc4, 0xcb, 0xd6),
            text_muted: Color::Rgb(0x5e, 0x65, 0x75),
            text_disabled: Color::Rgb(0x5e, 0x65, 0x75),
            text_placeholder: Color::Rgb(0x5e, 0x65, 0x75),
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

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "one-dark" => Some(Self::one_dark()),
            "catppuccin-mocha" => Some(Self::catppuccin_mocha()),
            "dracula" => Some(Self::dracula()),
            "nord" => Some(Self::nord()),
            "gruvbox" => Some(Self::gruvbox()),
            _ => None,
        }
    }

    pub const fn theme_names() -> &'static [&'static str] {
        &["catppuccin-mocha", "dracula", "gruvbox", "nord", "one-dark"]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::one_dark()
    }
}
