use crate::system;
use crate::xdg::XdgPaths;
use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub action: String,
    pub changed: bool,
    pub message: String,
    pub status: Option<StatusPanel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusProperty {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusPanel {
    pub title: String,
    pub icon_name: String,
    pub value: Option<String>,
    pub summary: Option<String>,
    pub progress: Option<f64>,
    pub properties: Vec<StatusProperty>,
}

impl ActionResult {
    fn new(action: impl Into<String>, changed: bool, message: String) -> Self {
        Self {
            action: action.into(),
            changed,
            message,
            status: None,
        }
    }
}

pub fn execute(
    paths: &XdgPaths,
    action: &str,
    args: &[String],
    confirmed: bool,
) -> Result<ActionResult> {
    let action = action.trim().to_ascii_lowercase();
    match action.as_str() {
        "wifi-status" => command_message(&action, "nmcli", &["radio", "wifi"]),
        "open-network-settings" => open_native_settings(
            &action,
            &[
                ("nm-connection-editor", &[]),
                ("gnome-control-center", &["network"]),
                ("systemsettings6", &["kcm_networkmanagement"]),
                ("systemsettings", &["kcm_networkmanagement"]),
            ],
        ),
        "wifi-on" | "wifi-off" => {
            let state = if action.ends_with("on") { "on" } else { "off" };
            command_changed(&action, "nmcli", &["radio", "wifi", state])
        }
        "bluetooth-status" => command_message(&action, "bluetoothctl", &["show"]),
        "open-bluetooth-settings" => open_native_settings(
            &action,
            &[
                ("blueman-manager", &[]),
                ("gnome-control-center", &["bluetooth"]),
                ("systemsettings6", &["kcm_bluetooth"]),
                ("systemsettings", &["kcm_bluetooth"]),
            ],
        ),
        "bluetooth-on" | "bluetooth-off" => {
            let state = if action.ends_with("on") { "on" } else { "off" };
            command_changed(&action, "bluetoothctl", &["power", state])
        }
        "audio-status" => {
            command_message(&action, "wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])
        }
        "open-audio-settings" => open_native_settings(
            &action,
            &[
                ("pavucontrol", &[]),
                ("gnome-control-center", &["sound"]),
                ("helvum", &[]),
                ("qpwgraph", &[]),
                ("systemsettings6", &["kcm_pulseaudio"]),
                ("systemsettings", &["kcm_pulseaudio"]),
            ],
        ),
        "audio-mute" | "audio-unmute" => {
            let value = audio_mute_value(&action);
            command_changed(
                &action,
                "wpctl",
                &["set-mute", "@DEFAULT_AUDIO_SINK@", value],
            )
        }
        "audio-volume" => {
            let percent = args
                .first()
                .context("audio-volume requires a percentage")?
                .parse::<u8>()?;
            if percent > 100 {
                anyhow::bail!("audio percentage must be between 0 and 100");
            }
            let value = format!("{:.2}", f32::from(percent) / 100.0);
            command_changed(
                &action,
                "wpctl",
                &["set-volume", "@DEFAULT_AUDIO_SINK@", &value],
            )
        }
        "audio-up" => command_changed(
            &action,
            "wpctl",
            &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"],
        ),
        "audio-down" => command_changed(
            &action,
            "wpctl",
            &["set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"],
        ),
        "brightness-up" => command_changed(&action, "brightnessctl", &["set", "+5%"]),
        "brightness-down" => command_changed(&action, "brightnessctl", &["set", "5%-"]),
        "brightness-status" => command_message(&action, "brightnessctl", &["info"]),
        "open-display-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["display"]),
                ("wdisplays", &[]),
                ("systemsettings6", &["kcm_kscreen"]),
                ("systemsettings", &["kcm_kscreen"]),
                ("xfce4-display-settings", &[]),
            ],
        ),
        "open-power-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["power"]),
                ("systemsettings6", &["kcm_energyinfo"]),
                ("systemsettings", &["kcm_energyinfo"]),
                ("xfce4-power-manager-settings", &[]),
            ],
        ),
        "open-keyboard-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["keyboard"]),
                ("systemsettings6", &["kcm_keyboard"]),
                ("systemsettings", &["kcm_keyboard"]),
                ("xfce4-keyboard-settings", &[]),
            ],
        ),
        "open-mouse-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["mouse"]),
                ("systemsettings6", &["kcm_mouse"]),
                ("systemsettings", &["kcm_mouse"]),
                ("xfce4-mouse-settings", &[]),
            ],
        ),
        "open-appearance-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["appearance"]),
                ("gnome-control-center", &["background"]),
                ("systemsettings6", &["kcm_style"]),
                ("systemsettings", &["kcm_style"]),
                ("xfce4-appearance-settings", &[]),
            ],
        ),
        "open-notification-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["notifications"]),
                ("systemsettings6", &["kcm_notifications"]),
                ("systemsettings", &["kcm_notifications"]),
            ],
        ),
        "open-privacy-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["privacy"]),
                ("systemsettings6", &["kcm_privacy"]),
                ("systemsettings", &["kcm_privacy"]),
            ],
        ),
        "open-date-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["datetime"]),
                ("systemsettings6", &["kcm_clock"]),
                ("systemsettings", &["kcm_clock"]),
            ],
        ),
        "open-users-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["user-accounts"]),
                ("systemsettings6", &["kcm_users"]),
                ("systemsettings", &["kcm_users"]),
            ],
        ),
        "open-region-settings" => open_native_settings(
            &action,
            &[
                ("gnome-control-center", &["region"]),
                ("systemsettings6", &["kcm_regionandlang"]),
                ("systemsettings", &["kcm_regionandlang"]),
            ],
        ),
        "open-software-settings" => open_native_settings(
            &action,
            &[
                ("gnome-software", &[]),
                ("discover", &[]),
                ("pamac-manager", &[]),
                ("pamac", &[]),
                ("octopi", &[]),
            ],
        ),
        "brightness" => {
            let percent = args
                .first()
                .context("brightness requires a percentage")?
                .parse::<u8>()?;
            if percent > 100 {
                anyhow::bail!("brightness percentage must be between 0 and 100");
            }
            command_changed(&action, "brightnessctl", &["set", &format!("{percent}%")])
        }
        "media-play-pause" => command_changed(&action, "playerctl", &["play-pause"]),
        "media-next" => command_changed(&action, "playerctl", &["next"]),
        "media-previous" => command_changed(&action, "playerctl", &["previous"]),
        "media-status" => command_message(&action, "playerctl", &["metadata"]),
        "battery-status" => {
            let battery = system::battery_status();
            let message = if !battery.present {
                "No battery detected on this device".to_string()
            } else {
                let percentage = battery
                    .percentage
                    .as_deref()
                    .unwrap_or("percentage unavailable");
                let state = battery.state.as_deref().unwrap_or("state unavailable");
                format!("Battery {percentage} · {state}")
            };
            Ok(ActionResult {
                action: action.to_string(),
                changed: false,
                message,
                status: Some(battery_panel(&battery)),
            })
        }
        "power-profile-status" => command_message(&action, "powerprofilesctl", &["get"]),
        "power-profile" => {
            let profile = args
                .first()
                .context("power-profile requires balanced, power-saver, or performance")?;
            if !matches!(profile.as_str(), "balanced" | "power-saver" | "performance") {
                anyhow::bail!("unsupported power profile: {profile}");
            }
            command_changed(&action, "powerprofilesctl", &["set", profile])
        }
        "hyprland-status" => {
            let info = crate::hyprland::discover();
            let message = serde_json::to_string_pretty(&info)?;
            Ok(ActionResult {
                action: action.to_string(),
                changed: false,
                message,
                status: Some(StatusPanel {
                    title: "Hyprland".to_string(),
                    icon_name: "preferences-desktop-display-symbolic".to_string(),
                    value: Some(
                        if info.confirmed {
                            "Connected"
                        } else {
                            "Unavailable"
                        }
                        .to_string(),
                    ),
                    summary: Some(info.reason),
                    progress: None,
                    properties: vec![
                        property("Monitors", json_count(info.monitors.as_ref())),
                        property("Workspaces", json_count(info.workspaces.as_ref())),
                        property("Config files", info.config_files.len().to_string()),
                    ],
                }),
            })
        }
        "doctor" => {
            let capabilities = crate::capabilities::detect();
            let available = capabilities
                .iter()
                .filter(|capability| {
                    matches!(
                        capability.state,
                        crate::capabilities::CapabilityState::Available
                    )
                })
                .count();
            let unavailable = capabilities.len().saturating_sub(available);
            let properties = capabilities
                .iter()
                .filter(|capability| {
                    !matches!(
                        capability.state,
                        crate::capabilities::CapabilityState::Available
                    )
                })
                .take(8)
                .map(|capability| property(capability.name.clone(), capability.reason.clone()))
                .collect();
            let message = serde_json::to_string_pretty(&capabilities)?;
            Ok(ActionResult {
                action: action.to_string(),
                changed: false,
                message,
                status: Some(StatusPanel {
                    title: "System capabilities".to_string(),
                    icon_name: "applications-system-symbolic".to_string(),
                    value: Some(format!("{available} available")),
                    summary: Some(format!("{unavailable} optional provider(s) need attention")),
                    progress: None,
                    properties,
                }),
            })
        }
        "power-lock" => command_changed(&action, "loginctl", &["lock-session"]),
        "power-suspend" => confirmed_system_action(&action, "systemctl", &["suspend"], confirmed),
        "power-reboot" => confirmed_system_action(&action, "systemctl", &["reboot"], confirmed),
        "poweroff" => confirmed_system_action(&action, "systemctl", &["poweroff"], confirmed),
        "power-logout" => confirmed_system_action(
            &action,
            "loginctl",
            &["terminate-session", "self"],
            confirmed,
        ),
        "open-folder" => {
            let folder = args
                .first()
                .context("open-folder requires a user directory name")?;
            let path = match folder.as_str() {
                "home" => paths.home.clone(),
                "desktop" | "documents" | "downloads" | "music" | "pictures" | "public"
                | "templates" | "videos" => paths
                    .user_dirs()
                    .into_iter()
                    .find(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.eq_ignore_ascii_case(folder))
                    })
                    .context("requested XDG user directory is unavailable")?,
                _ => anyhow::bail!("folder is not an allowlisted XDG user directory"),
            };
            let result = system::open_target(&path.to_string_lossy())?;
            Ok(ActionResult::new(action, false, result.stdout))
        }
        _ => anyhow::bail!("unsupported Linux action: {action}"),
    }
}

fn command_message(action: &str, program: &str, args: &[&str]) -> Result<ActionResult> {
    let result = system::run(program, args)?;
    if result.timed_out || result.status != Some(0) {
        anyhow::bail!(
            "{program} did not complete successfully: {}",
            output_message(&result)
        );
    }
    let message = output_message(&result);
    let mut action_result = ActionResult::new(action, false, message.clone());
    action_result.status = Some(command_status_panel(action, &message));
    Ok(action_result)
}

fn command_changed(action: &str, program: &str, args: &[&str]) -> Result<ActionResult> {
    let result = system::run(program, args)?;
    if result.timed_out || result.status != Some(0) {
        anyhow::bail!(
            "{program} did not complete successfully: {}",
            output_message(&result)
        );
    }
    Ok(ActionResult::new(action, true, output_message(&result)))
}

fn open_native_settings(action: &str, candidates: &[(&str, &[&str])]) -> Result<ActionResult> {
    for (program, args) in candidates {
        if !system::command_available(program) {
            continue;
        }
        system::spawn_detached_command(program, args)?;
        return Ok(ActionResult::new(
            action,
            false,
            format!("opened Linux settings with {program}"),
        ));
    }
    anyhow::bail!(
        "no supported native settings application is installed; install a desktop settings app"
    )
}

fn confirmed_system_action(
    action: &str,
    program: &str,
    args: &[&str],
    confirmed: bool,
) -> Result<ActionResult> {
    if !confirmed {
        return Ok(ActionResult::new(
            action,
            false,
            "confirmation required; repeat with --confirm for this session action".to_string(),
        ));
    }
    command_changed(action, program, args)
}

fn audio_mute_value(action: &str) -> &'static str {
    match action {
        "audio-mute" => "1",
        "audio-unmute" => "0",
        _ => unreachable!("audio_mute_value called for unsupported action: {action}"),
    }
}

fn output_message(result: &system::CommandResult) -> String {
    if !result.stdout.is_empty() {
        result.stdout.clone()
    } else if !result.stderr.is_empty() {
        result.stderr.clone()
    } else {
        "provider completed without output".to_string()
    }
}

fn property(label: impl Into<String>, value: impl Into<String>) -> StatusProperty {
    StatusProperty {
        label: label.into(),
        value: value.into(),
    }
}

fn json_count(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| items.len().to_string())
        .unwrap_or_else(|| "Not available".to_string())
}

fn battery_panel(status: &system::BatteryStatus) -> StatusPanel {
    if !status.present {
        return StatusPanel {
            title: "Battery".to_string(),
            icon_name: "battery-missing-symbolic".to_string(),
            value: Some("—".to_string()),
            summary: Some("No battery detected".to_string()),
            progress: None,
            properties: vec![property("Availability", "Not available on this device")],
        };
    }
    let percentage = status
        .percentage
        .clone()
        .unwrap_or_else(|| "Not available".to_string());
    let percentage = if percentage.ends_with('%') {
        percentage
    } else if percentage.parse::<f64>().is_ok() {
        format!("{percentage}%")
    } else {
        percentage
    };
    let progress = percentage
        .trim_end_matches('%')
        .parse::<f64>()
        .ok()
        .filter(|value| (0.0..=100.0).contains(value))
        .map(|value| value / 100.0);
    let state = status
        .state
        .as_deref()
        .map(humanize_state)
        .unwrap_or_else(|| "Status unavailable".to_string());
    let icon_name = match status
        .state
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "charging" | "pending-charge" => "battery-good-charging-symbolic",
        "full" => "battery-full-symbolic",
        "discharging" => "battery-good-symbolic",
        _ => "battery-symbolic",
    };
    StatusPanel {
        title: "Battery".to_string(),
        icon_name: icon_name.to_string(),
        value: Some(percentage),
        summary: Some(state.clone()),
        progress,
        properties: vec![property("Status", state)],
    }
}

fn command_status_panel(action: &str, output: &str) -> StatusPanel {
    let first_line = output.lines().next().unwrap_or_default().trim();
    match action {
        "wifi-status" => {
            let enabled = !first_line.to_ascii_lowercase().contains("disabled");
            StatusPanel {
                title: "Wi-Fi".to_string(),
                icon_name: "network-wireless-symbolic".to_string(),
                value: Some(if enabled { "Enabled" } else { "Disabled" }.to_string()),
                summary: Some("NetworkManager radio state".to_string()),
                progress: None,
                properties: vec![property("Radio", if enabled { "On" } else { "Off" })],
            }
        }
        "bluetooth-status" => {
            let powered = output
                .lines()
                .find(|line| {
                    line.trim_start()
                        .to_ascii_lowercase()
                        .starts_with("powered:")
                })
                .map(|line| line.to_ascii_lowercase().contains("yes"))
                .unwrap_or(false);
            StatusPanel {
                title: "Bluetooth".to_string(),
                icon_name: "bluetooth-symbolic".to_string(),
                value: Some(if powered { "Enabled" } else { "Disabled" }.to_string()),
                summary: Some("Bluetooth adapter state".to_string()),
                progress: None,
                properties: vec![property(
                    "Adapter",
                    if powered { "Powered on" } else { "Powered off" },
                )],
            }
        }
        "audio-status" => {
            let volume = output
                .split_whitespace()
                .find_map(|token| token.parse::<f64>().ok())
                .map(|value| format!("{}%", (value * 100.0).round() as u8))
                .unwrap_or_else(|| "Not available".to_string());
            let muted = output.to_ascii_lowercase().contains("muted");
            let progress = volume
                .trim_end_matches('%')
                .parse::<f64>()
                .ok()
                .map(|value| value / 100.0);
            StatusPanel {
                title: "Audio".to_string(),
                icon_name: "audio-volume-high-symbolic".to_string(),
                value: Some(volume.clone()),
                summary: Some(
                    if muted {
                        "Output muted"
                    } else {
                        "Output active"
                    }
                    .to_string(),
                ),
                progress,
                properties: vec![
                    property("Volume", volume),
                    property("Mute", if muted { "On" } else { "Off" }),
                ],
            }
        }
        "brightness-status" => {
            let normalized_output = output.replace(['(', ')'], " ");
            let percentage = normalized_output
                .split_whitespace()
                .find(|token| token.ends_with('%'))
                .unwrap_or("Not available")
                .to_string();
            let progress = percentage
                .trim_end_matches('%')
                .parse::<f64>()
                .ok()
                .map(|value| value / 100.0);
            StatusPanel {
                title: "Brightness".to_string(),
                icon_name: "display-brightness-symbolic".to_string(),
                value: Some(percentage.clone()),
                summary: Some("Current display brightness".to_string()),
                progress,
                properties: vec![property("Level", percentage)],
            }
        }
        "power-profile-status" => StatusPanel {
            title: "Power profile".to_string(),
            icon_name: "power-profile-balanced-symbolic".to_string(),
            value: Some(humanize_state(first_line)),
            summary: Some("Active system power profile".to_string()),
            progress: None,
            properties: Vec::new(),
        },
        "media-status" => StatusPanel {
            title: "Media".to_string(),
            icon_name: "multimedia-player-symbolic".to_string(),
            value: Some(
                if first_line.is_empty() {
                    "No active player"
                } else {
                    "Active player"
                }
                .to_string(),
            ),
            summary: Some("MPRIS playback status".to_string()),
            progress: None,
            properties: if first_line.is_empty() {
                Vec::new()
            } else {
                vec![property("Player", first_line)]
            },
        },
        _ => StatusPanel {
            title: "System status".to_string(),
            icon_name: "applications-system-symbolic".to_string(),
            value: None,
            summary: Some("Information is available".to_string()),
            progress: None,
            properties: Vec::new(),
        },
    }
}

fn humanize_state(value: &str) -> String {
    value
        .split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{audio_mute_value, command_status_panel, confirmed_system_action};

    #[test]
    fn audio_mute_actions_map_to_expected_wpctl_values() {
        assert_eq!(audio_mute_value("audio-mute"), "1");
        assert_eq!(audio_mute_value("audio-unmute"), "0");
    }

    #[test]
    fn destructive_actions_require_confirmation_without_running_command() {
        let result =
            confirmed_system_action("poweroff", "a-command-that-is-not-installed", &[], false)
                .expect("unconfirmed actions should return a result");

        assert!(!result.changed);
        assert!(result.message.contains("confirmation required"));
    }

    #[test]
    fn status_panels_translate_provider_output_into_readable_values() {
        let wifi = command_status_panel("wifi-status", "enabled\n");
        assert_eq!(wifi.title, "Wi-Fi");
        assert_eq!(wifi.value.as_deref(), Some("Enabled"));
        assert!(wifi
            .properties
            .iter()
            .all(|property| !property.value.contains('{')));

        let brightness = command_status_panel("brightness-status", "Current brightness: 1 (75%)");
        assert_eq!(brightness.title, "Brightness");
        assert_eq!(brightness.value.as_deref(), Some("75%"));
    }
}
