use crate::model;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct StylePalette {
    pub name: String,
    pub style: model::Style,
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
    pub palettes: Vec<StylePalette>,
    pub active_palette: Option<usize>,
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
            palettes: Vec::new(),
            active_palette: None,
        }
    }
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
