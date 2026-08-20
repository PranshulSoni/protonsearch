use crate::xdg::XdgPaths;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LinuxSettings {
    pub schema_version: u32,
    pub run_on_startup: bool,
    pub hide_on_lose_focus: bool,
    pub theme_mode: String,
    pub show_taskbar: bool,
    pub window_width: u32,
    pub item_height: u32,
    pub search_bar_height: u32,
    pub show_placeholder: bool,
    pub include_hidden: bool,
    pub show_terminal_apps: bool,
    pub enable_system_actions: bool,
    pub enable_hyprland: bool,
    pub enable_calculator: bool,
    pub enable_git_commits: bool,
    pub enable_clipboard_history: bool,
    pub enable_ocr: bool,
    pub enable_browser_history: bool,
    pub confirm_power_actions: bool,
    pub log_level: String,
    /// Compositor-facing shortcut notation, e.g. `ALT,SPACE` for Hyprland.
    pub hotkey: String,
    pub search_roots: Vec<String>,
    pub ignored_names: Vec<String>,
}

impl Default for LinuxSettings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            run_on_startup: true,
            hide_on_lose_focus: true,
            theme_mode: "Dark".to_string(),
            show_taskbar: false,
            window_width: 720,
            item_height: 76,
            search_bar_height: 60,
            show_placeholder: true,
            include_hidden: false,
            show_terminal_apps: true,
            enable_system_actions: true,
            enable_hyprland: true,
            enable_calculator: true,
            enable_git_commits: true,
            enable_clipboard_history: true,
            enable_ocr: true,
            enable_browser_history: true,
            confirm_power_actions: true,
            log_level: "info".to_string(),
            hotkey: "ALT,SPACE".to_string(),
            search_roots: Vec::new(),
            ignored_names: Vec::new(),
        }
    }
}

pub fn load(paths: &XdgPaths) -> LinuxSettings {
    let path = paths.settings_file();
    let Ok(contents) = fs::read_to_string(&path) else {
        return LinuxSettings::default();
    };

    match serde_json::from_str(&contents) {
        Ok(settings) => settings,
        Err(_) => {
            let backup = path.with_extension("json.bak");
            let _ = fs::rename(&path, backup);
            LinuxSettings::default()
        }
    }
}

pub fn save(paths: &XdgPaths, settings: &LinuxSettings) -> Result<()> {
    fs::create_dir_all(paths.config_dir())?;
    let content = serde_json::to_vec_pretty(settings)?;
    let temporary = paths.settings_file().with_extension("json.new");
    fs::write(&temporary, content)?;
    fs::rename(temporary, paths.settings_file())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct LinuxSettingItem {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub action: &'static str,
}

pub fn catalogue() -> Vec<LinuxSettingItem> {
    vec![
        LinuxSettingItem {
            id: "search.roots",
            name: "Search roots",
            description: "Search HOME, XDG user directories, and configured paths",
            action: "settings",
        },
        LinuxSettingItem {
            id: "search.hidden",
            name: "Include hidden files",
            description: "Include hidden entries only when explicitly enabled",
            action: "settings",
        },
        LinuxSettingItem {
            id: "launcher.hotkey",
            name: "Launcher hotkey",
            description: "Configure the Hyprland shortcut used to toggle ProtonSearch",
            action: "settings",
        },
        LinuxSettingItem {
            id: "apps.desktop",
            name: "Installed applications",
            description: "Search launchable XDG desktop entries",
            action: "apps",
        },
        LinuxSettingItem {
            id: "network.wifi",
            name: "Wi-Fi controls",
            description: "Read or change NetworkManager Wi-Fi radio state",
            action: "wifi-status",
        },
        LinuxSettingItem {
            id: "network.wifi.settings",
            name: "Open Wi-Fi settings",
            description: "Open the native Linux network settings panel",
            action: "open-network-settings",
        },
        LinuxSettingItem {
            id: "bluetooth",
            name: "Bluetooth controls",
            description: "Read or change BlueZ adapter power",
            action: "bluetooth-status",
        },
        LinuxSettingItem {
            id: "bluetooth.settings",
            name: "Open Bluetooth settings",
            description: "Open the native Linux Bluetooth settings panel",
            action: "open-bluetooth-settings",
        },
        LinuxSettingItem {
            id: "audio",
            name: "Audio controls",
            description: "PipeWire volume and mute controls",
            action: "audio-status",
        },
        LinuxSettingItem {
            id: "audio.settings",
            name: "Open audio settings",
            description: "Open the native Linux sound mixer or settings panel",
            action: "open-audio-settings",
        },
        LinuxSettingItem {
            id: "brightness",
            name: "Brightness controls",
            description: "Backlight brightness when hardware and permissions allow",
            action: "brightness-status",
        },
        LinuxSettingItem {
            id: "display.settings",
            name: "Open display settings",
            description: "Open the native Linux display settings panel",
            action: "open-display-settings",
        },
        LinuxSettingItem {
            id: "power",
            name: "Power and session",
            description: "Battery, power profile, lock, and confirmed session actions",
            action: "power-lock",
        },
        LinuxSettingItem {
            id: "power.profile",
            name: "Power profile status",
            description: "Show the active Linux power profile",
            action: "power-profile-status",
        },
        LinuxSettingItem {
            id: "power.settings",
            name: "Open power settings",
            description: "Open the native Linux power settings panel",
            action: "open-power-settings",
        },
        LinuxSettingItem {
            id: "keyboard.settings",
            name: "Open keyboard settings",
            description: "Open the native Linux keyboard settings panel",
            action: "open-keyboard-settings",
        },
        LinuxSettingItem {
            id: "mouse.settings",
            name: "Open mouse settings",
            description: "Open the native Linux mouse and touchpad settings panel",
            action: "open-mouse-settings",
        },
        LinuxSettingItem {
            id: "appearance.settings",
            name: "Open appearance settings",
            description: "Open the native Linux theme and appearance settings panel",
            action: "open-appearance-settings",
        },
        LinuxSettingItem {
            id: "notifications.settings",
            name: "Open notification settings",
            description: "Open the native Linux notification settings panel",
            action: "open-notification-settings",
        },
        LinuxSettingItem {
            id: "privacy.settings",
            name: "Open privacy settings",
            description: "Open the native Linux privacy settings panel",
            action: "open-privacy-settings",
        },
        LinuxSettingItem {
            id: "date.settings",
            name: "Open date and time settings",
            description: "Open the native Linux date and time settings panel",
            action: "open-date-settings",
        },
        LinuxSettingItem {
            id: "users.settings",
            name: "Open user settings",
            description: "Open the native Linux user account settings panel",
            action: "open-users-settings",
        },
        LinuxSettingItem {
            id: "region.settings",
            name: "Open language and region settings",
            description: "Open the native Linux language and regional settings panel",
            action: "open-region-settings",
        },
        LinuxSettingItem {
            id: "software.settings",
            name: "Open software manager",
            description: "Open the installed Linux graphical software manager",
            action: "open-software-settings",
        },
        LinuxSettingItem {
            id: "media",
            name: "Media controls",
            description: "MPRIS player controls through playerctl",
            action: "media-status",
        },
        LinuxSettingItem {
            id: "hyprland",
            name: "Hyprland status",
            description: "Read-only monitors, workspaces, and config includes",
            action: "hyprland-status",
        },
        LinuxSettingItem {
            id: "diagnostics",
            name: "Capability diagnostics",
            description: "Show available providers and package guidance",
            action: "doctor",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(root: &std::path::Path) -> XdgPaths {
        XdgPaths {
            config: root.join("config"),
            data: root.join("data"),
            state: root.join("state"),
            cache: root.join("cache"),
            home: root.join("home"),
            runtime: None,
        }
    }

    #[test]
    fn defaults_cover_linux_parity_fields() {
        let settings = LinuxSettings::default();
        assert_eq!(settings.schema_version, 1);
        assert!(settings.run_on_startup);
        assert_eq!(settings.theme_mode, "Dark");
        assert!(settings.enable_calculator);
        assert!(settings.enable_git_commits);
    }

    #[test]
    fn legacy_json_receives_new_defaults() {
        let root = std::env::temp_dir().join(format!(
            "protonsearch-settings-legacy-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let paths = paths(&root);
        fs::create_dir_all(paths.config_dir()).unwrap();
        fs::write(
            paths.settings_file(),
            r#"{"include_hidden":true,"hotkey":"SUPER,SPACE"}"#,
        )
        .unwrap();

        let settings = load(&paths);
        assert!(settings.include_hidden);
        assert_eq!(settings.hotkey, "SUPER,SPACE");
        assert!(settings.run_on_startup);
        assert_eq!(settings.schema_version, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_json_is_backed_up_before_defaults() {
        let root = std::env::temp_dir().join(format!(
            "protonsearch-settings-malformed-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let paths = paths(&root);
        fs::create_dir_all(paths.config_dir()).unwrap();
        fs::write(paths.settings_file(), b"not-json").unwrap();

        let settings = load(&paths);
        assert_eq!(settings, LinuxSettings::default());
        assert!(!paths.settings_file().exists());
        assert!(paths.settings_file().with_extension("json.bak").exists());
        let _ = fs::remove_dir_all(root);
    }
}
