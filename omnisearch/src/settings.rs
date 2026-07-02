use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

fn settings_mutex() -> &'static std::sync::Mutex<()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

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

    #[serde(default = "default_false")]
    pub show_taskbar: bool,

    #[serde(default = "default_window_location")]
    pub window_location: String,

    #[serde(default = "default_zero_i32")]
    pub last_win_x: i32,

    #[serde(default = "default_zero_i32")]
    pub last_win_y: i32,

    #[serde(default = "default_scan_folders")]
    pub scan_folders: Vec<String>,

    #[serde(default = "default_search_bar_height")]
    pub search_bar_height: u32,

    #[serde(default = "default_query_font_family")]
    pub query_font_family: String,

    #[serde(default = "default_query_font_weight")]
    pub query_font_weight: String,

    #[serde(default = "default_query_font_size")]
    pub query_font_size: u32,

    #[serde(default = "default_result_title_font_family")]
    pub result_title_font_family: String,

    #[serde(default = "default_result_title_font_weight")]
    pub result_title_font_weight: String,

    #[serde(default = "default_result_title_font_size")]
    pub result_title_font_size: u32,

    #[serde(default = "default_result_subtitle_font_family")]
    pub result_subtitle_font_family: String,

    #[serde(default = "default_result_subtitle_font_weight")]
    pub result_subtitle_font_weight: String,

    #[serde(default = "default_result_subtitle_font_size")]
    pub result_subtitle_font_size: u32,

    #[serde(default = "default_true")]
    pub show_placeholder: bool,
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
            show_taskbar: default_false(),
            window_location: default_window_location(),
            last_win_x: default_zero_i32(),
            last_win_y: default_zero_i32(),
            scan_folders: default_scan_folders(),
            search_bar_height: default_search_bar_height(),
            query_font_family: default_query_font_family(),
            query_font_weight: default_query_font_weight(),
            query_font_size: default_query_font_size(),
            result_title_font_family: default_result_title_font_family(),
            result_title_font_weight: default_result_title_font_weight(),
            result_title_font_size: default_result_title_font_size(),
            result_subtitle_font_family: default_result_subtitle_font_family(),
            result_subtitle_font_weight: default_result_subtitle_font_weight(),
            result_subtitle_font_size: default_result_subtitle_font_size(),
            show_placeholder: default_true(),
        }
    }
}

fn default_false() -> bool {
    false
}
fn default_window_location() -> String {
    "Monitor with Mouse Cursor".to_string()
}
fn default_zero_i32() -> i32 {
    0
}

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "Dark".to_string()
}
fn default_global_hotkey() -> String {
    "Alt+Space".to_string()
}
fn default_window_width() -> u32 {
    720
}
fn default_item_height() -> u32 {
    76
}
fn default_scan_folders() -> Vec<String> {
    Vec::new()
}
fn default_search_bar_height() -> u32 {
    60
}
fn default_query_font_family() -> String {
    "Segoe UI Variable".to_string()
}
fn default_query_font_weight() -> String {
    "Regular".to_string()
}
fn default_query_font_size() -> u32 {
    24
}
fn default_result_title_font_family() -> String {
    "Segoe UI Variable".to_string()
}
fn default_result_title_font_weight() -> String {
    "Bold".to_string()
}
fn default_result_title_font_size() -> u32 {
    18
}
fn default_result_subtitle_font_family() -> String {
    "Segoe UI Variable".to_string()
}
fn default_result_subtitle_font_weight() -> String {
    "Regular".to_string()
}
fn default_result_subtitle_font_size() -> u32 {
    13
}

impl AppSettings {
    pub fn get_settings_path() -> PathBuf {
        let mut path = std::env::current_exe().unwrap();
        path.pop(); // remove executable
        path.push("settings.json");
        path
    }

    pub fn load() -> Self {
        let _guard = settings_mutex().lock().unwrap();
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
        let _guard = settings_mutex().lock().unwrap();
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
