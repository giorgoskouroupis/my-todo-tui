use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ui::theme::Theme;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub theme: Option<ThemeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidebar_width: Option<u16>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ThemeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_secondary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_tertiary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_selection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_primary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_secondary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_muted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_disabled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_default: Option<String>,
}

fn hex_to_color(hex: &str) -> Option<ratatui::style::Color> {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            Some(ratatui::style::Color::Rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(ratatui::style::Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

fn apply_theme_overrides(theme: &mut Theme, overrides: &ThemeConfig) {
    if let Some(v) = &overrides.bg_primary {
        if let Some(c) = hex_to_color(v) {
            theme.bg_primary = c;
        }
    }
    if let Some(v) = &overrides.bg_secondary {
        if let Some(c) = hex_to_color(v) {
            theme.bg_secondary = c;
        }
    }
    if let Some(v) = &overrides.bg_tertiary {
        if let Some(c) = hex_to_color(v) {
            theme.bg_tertiary = c;
        }
    }
    if let Some(v) = &overrides.accent {
        if let Some(c) = hex_to_color(v) {
            theme.accent = c;
        }
    }
    if let Some(v) = &overrides.accent_selection {
        if let Some(c) = hex_to_color(v) {
            theme.accent_selection = c;
        }
    }
    if let Some(v) = &overrides.success {
        if let Some(c) = hex_to_color(v) {
            theme.success = c;
        }
    }
    if let Some(v) = &overrides.warning {
        if let Some(c) = hex_to_color(v) {
            theme.warning = c;
        }
    }
    if let Some(v) = &overrides.error {
        if let Some(c) = hex_to_color(v) {
            theme.error = c;
        }
    }
    if let Some(v) = &overrides.text_primary {
        if let Some(c) = hex_to_color(v) {
            theme.text_primary = c;
        }
    }
    if let Some(v) = &overrides.text_secondary {
        if let Some(c) = hex_to_color(v) {
            theme.text_secondary = c;
        }
    }
    if let Some(v) = &overrides.text_muted {
        if let Some(c) = hex_to_color(v) {
            theme.text_muted = c;
        }
    }
    if let Some(v) = &overrides.text_disabled {
        if let Some(c) = hex_to_color(v) {
            theme.text_disabled = c;
        }
    }
    if let Some(v) = &overrides.text_placeholder {
        if let Some(c) = hex_to_color(v) {
            theme.text_placeholder = c;
        }
    }
    if let Some(v) = &overrides.border_default {
        if let Some(c) = hex_to_color(v) {
            theme.border_default = c;
        }
    }
}

fn config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("todo-tui").join("config.json")
}

pub fn load_theme() -> Theme {
    let path = config_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str::<Config>(&raw) {
            if let Some(tc) = cfg.theme {
                if let Some(ref name) = tc.name {
                    if let Some(preset) = Theme::by_name(name) {
                        let mut theme = preset;
                        apply_theme_overrides(&mut theme, &tc);
                        return theme;
                    }
                }
                let mut theme = Theme::one_dark();
                apply_theme_overrides(&mut theme, &tc);
                return theme;
            }
        }
    }
    Theme::one_dark()
}

pub fn save_theme_name(name: &str) {
    let path = config_path();
    let mut cfg = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Config>(&raw).ok())
        .unwrap_or_default();

    let tc = cfg.theme.get_or_insert_with(ThemeConfig::default);
    tc.name = Some(name.to_string());

    write_config(&path, &cfg);
}

pub fn load_sidebar_width() -> Option<u16> {
    let path = config_path();
    let raw = std::fs::read_to_string(&path).ok()?;
    let cfg: Config = serde_json::from_str(&raw).ok()?;
    cfg.sidebar_width
}

pub fn save_sidebar_width(width: u16) {
    let path = config_path();
    let mut cfg = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Config>(&raw).ok())
        .unwrap_or_default();
    cfg.sidebar_width = Some(width);
    write_config(&path, &cfg);
}

fn write_config(path: &std::path::Path, cfg: &Config) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(cfg) {
        let tmp = path.with_extension("json.tmp");
        let _ = std::fs::write(&tmp, &raw);
        let _ = std::fs::rename(&tmp, path);
    }
}
