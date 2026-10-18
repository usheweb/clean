// 设置持久化：data_dir/settings.json
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

fn default_theme() -> String {
    "light".into()
}

fn default_root_name() -> String {
    "clean_c_migrated".into()
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_root_name")]
    pub target_root_name: String,
    #[serde(default)]
    pub target_drive: String,
    #[serde(default)]
    pub include_candidates: bool,
    #[serde(default)]
    pub last_promo_day: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            target_root_name: default_root_name(),
            target_drive: String::new(),
            include_candidates: false,
            last_promo_day: String::new(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    settings.save(&state.data_dir.join("settings.json"))?;
    state.update_settings(settings);
    Ok(())
}
