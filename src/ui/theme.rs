use ratatui::style::Color;

pub const BG_PRIMARY: Color = Color::Rgb(0x1e, 0x1e, 0x1e);
pub const BG_SECONDARY: Color = Color::Rgb(0x25, 0x25, 0x26);
pub const BG_TERTIARY: Color = Color::Rgb(0x2d, 0x2d, 0x2d);

pub const ACCENT: Color = Color::Rgb(0x00, 0x7a, 0xcc);
pub const ACCENT_SELECTION: Color = Color::Rgb(0x09, 0x47, 0x71);
pub const ACCENT_HOVER: Color = Color::Rgb(0x11, 0x77, 0xbb);

pub const SUCCESS: Color = Color::Rgb(0x4a, 0xde, 0x80);
pub const WARNING: Color = Color::Rgb(0xfb, 0xbf, 0x24);
pub const ERROR: Color = Color::Rgb(0xf8, 0x71, 0x71);

pub const TEXT_PRIMARY: Color = Color::Rgb(0xff, 0xff, 0xff);
pub const TEXT_SECONDARY: Color = Color::Rgb(0xcc, 0xcc, 0xcc);
pub const TEXT_MUTED: Color = Color::Rgb(0x88, 0x88, 0x88);
pub const TEXT_DISABLED: Color = Color::Rgb(0x6e, 0x6e, 0x6e);
pub const TEXT_PLACEHOLDER: Color = Color::Rgb(0x6e, 0x6e, 0x6e);

pub const BORDER_DEFAULT: Color = Color::Rgb(0x3c, 0x3c, 0x3c);

pub const TODO_ICON: &str = "\u{f012c}";
pub const DONE_ICON: &str = "\u{2713}";
pub const DELETE_ICON: &str = "\u{d7}";
pub const PRIORITY_ICON: &str = "\u{25c6}";
pub const ACTIVE_DOT: &str = "\u{25cf}";
pub const INACTIVE_DOT: &str = "\u{25cb}";
