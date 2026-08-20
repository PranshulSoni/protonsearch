use crate::system;
use crate::xdg::XdgPaths;
use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub action: String,
    pub changed: bool,
    pub message: String,
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
        "battery-status" => Ok(ActionResult {
            action: action.to_string(),
            changed: false,
            message: serde_json::to_string(&system::battery_status())?,
        }),
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
        "hyprland-status" => Ok(ActionResult {
            action: action.to_string(),
            changed: false,
            message: serde_json::to_string_pretty(&crate::hyprland::discover())?,
        }),
        "doctor" => Ok(ActionResult {
            action: action.to_string(),
            changed: false,
            message: serde_json::to_string_pretty(&crate::capabilities::detect())?,
        }),
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
            Ok(ActionResult {
                action,
                changed: false,
                message: result.stdout,
            })
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
    Ok(ActionResult {
        action: action.to_string(),
        changed: false,
        message: output_message(&result),
    })
}

fn command_changed(action: &str, program: &str, args: &[&str]) -> Result<ActionResult> {
    let result = system::run(program, args)?;
    if result.timed_out || result.status != Some(0) {
        anyhow::bail!(
            "{program} did not complete successfully: {}",
            output_message(&result)
        );
    }
    Ok(ActionResult {
        action: action.to_string(),
        changed: true,
        message: output_message(&result),
    })
}

fn open_native_settings(action: &str, candidates: &[(&str, &[&str])]) -> Result<ActionResult> {
    for (program, args) in candidates {
        if !system::command_available(program) {
            continue;
        }
        system::spawn_detached_command(program, args)?;
        return Ok(ActionResult {
            action: action.to_string(),
            changed: false,
            message: format!("opened Linux settings with {program}"),
        });
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
        return Ok(ActionResult {
            action: action.to_string(),
            changed: false,
            message: "confirmation required; repeat with --confirm for this session action"
                .to_string(),
        });
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

#[cfg(test)]
mod tests {
    use super::{audio_mute_value, confirmed_system_action};

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
}
