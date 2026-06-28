use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub run_on_startup: bool,
    
    #[serde(default = "default_true")]
    pub hide_on_lose_focus: bool,
    
    #[serde(default = "default_theme")]
    pub theme_mode: String,
    
    #[serde(default = "default_global_hotkey")]
    pub global_hotkey: String,
    
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    
    #[serde(default = "default_item_height")]
    pub item_height: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            run_on_startup: default_true(),
            hide_on_lose_focus: default_true(),
            theme_mode: default_theme(),
            global_hotkey: default_global_hotkey(),
            window_width: default_window_width(),
            item_height: default_item_height(),
        }
    }
}

fn default_true() -> bool { true }
fn default_theme() -> String { "Dark".to_string() }
fn default_global_hotkey() -> String { "Alt+Space".to_string() }
fn default_window_width() -> u32 { 720 }
fn default_item_height() -> u32 { 76 }

impl AppSettings {
    pub fn get_settings_path() -> PathBuf {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // remove executable
        path.push("settings.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::get_settings_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        let default_settings = Self::default();
        default_settings.save();
        default_settings
    }

    pub fn save(&self) {
        let path = Self::get_settings_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }

    pub fn normalized_theme_mode(&self) -> String {
        match self.theme_mode.as_str() {
            "Light" => "Light",
            "NordDarker" => "NordDarker",
            _ => "Dark",
        }
        .to_string()
    }
}
