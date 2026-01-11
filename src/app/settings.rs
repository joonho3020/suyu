use crate::model;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub(super) struct ColorTheme {
    pub name: String,
    #[serde(default)]
    pub colors: HashMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct AppSettings {
    pub file_path: String,
    pub svg_path: String,
    pub snap_to_grid: bool,
    pub grid_size: f32,
    pub move_step: f32,
    pub move_step_fast: f32,
    pub apply_style_to_selection: bool,
    #[serde(default)]
    pub color_themes: Vec<ColorTheme>,
    #[serde(default)]
    pub active_color_theme: Option<usize>,
    #[serde(default)]
    pub font_directory: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            file_path: "diagram.json".to_string(),
            svg_path: "diagram.svg".to_string(),
            snap_to_grid: true,
            grid_size: 64.0,
            move_step: 1.0,
            move_step_fast: 10.0,
            apply_style_to_selection: true,
            color_themes: Vec::new(),
            active_color_theme: None,
            font_directory: None,
        }
    }
}

impl ColorTheme {
    pub fn get_color(&self, name: &str) -> Option<model::Rgba> {
        let hex = self.colors.get(name)?;
        parse_hex_color(hex)
    }
}

pub(super) fn parse_hex_color(s: &str) -> Option<model::Rgba> {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        return Some(model::Rgba { r, g, b, a: 255 });
    }
    if s.len() == 3 {
        let r = u8::from_str_radix(&s[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&s[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&s[2..3], 16).ok()? * 17;
        return Some(model::Rgba { r, g, b, a: 255 });
    }
    None
}

pub(super) fn load_settings(path: &str) -> Option<AppSettings> {
    let s = std::fs::read_to_string(path).ok()?;
    if path.ends_with(".toml") {
        toml::from_str::<AppSettings>(&s)
            .ok()
            .or_else(|| serde_json::from_str::<AppSettings>(&s).ok())
    } else {
        serde_json::from_str::<AppSettings>(&s)
            .ok()
            .or_else(|| toml::from_str::<AppSettings>(&s).ok())
    }
}

pub(super) fn save_settings(path: &str, settings: &AppSettings) -> Result<(), String> {
    if path.ends_with(".toml") {
        let toml = toml::to_string_pretty(settings).map_err(|e| e.to_string())?;
        std::fs::write(path, toml).map_err(|e| e.to_string())
    } else {
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}
