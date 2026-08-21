use crate::hyprland;
use crate::platform;
use crate::system;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Available,
    PackageMissing,
    ServiceInactive,
    PermissionDenied,
    UnsupportedDesktop,
    UnsupportedHardware,
    ConfigurationMissing,
    ProviderError,
}

#[derive(Debug, Clone, Serialize)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub state: CapabilityState,
    pub provider: String,
    pub package: Option<String>,
    pub reason: String,
}

pub fn detect() -> Vec<Capability> {
    vec![
        core(
            "search",
            "File and directory search",
            "filesystem",
            true,
            "",
        ),
        opener(),
        internal_clipboard(
            "clipboard",
            "Clipboard access",
            "Current desktop clipboard through GTK/GDK",
        ),
        internal_clipboard(
            "clipboard-history",
            "Persistent clipboard history",
            "ProtonSearch-owned clipboard history through GTK/GDK",
        ),
        simple_command(
            "browser-history",
            "Browser bookmarks and history",
            "sqlite3",
            "sqlite",
        ),
        deferred_command("ocr", "Image and screenshot OCR", "tesseract", "tesseract"),
        simple_command("pdf-content", "PDF content search", "pdftotext", "poppler"),
        simple_command("screenshot", "Area screenshot capture", "grim", "grim"),
        network_manager(),
        simple_command(
            "bluetooth",
            "Bluetooth controls",
            "bluetoothctl",
            "bluez-utils",
        ),
        simple_command("audio", "PipeWire audio controls", "wpctl", "wireplumber"),
        simple_command(
            "brightness",
            "Backlight brightness",
            "brightnessctl",
            "brightnessctl",
        ),
        simple_command("media", "MPRIS media controls", "playerctl", "playerctl"),
        simple_command("session", "System session actions", "loginctl", "systemd"),
        simple_command(
            "power-profile",
            "Power profile controls",
            "powerprofilesctl",
            "power-profiles-daemon",
        ),
        simple_command(
            "hermes-agent",
            "Hermes Agent integration",
            "hermes",
            "hermes-agent",
        ),
        battery_capability(),
        hyprland_capability(),
    ]
}

fn battery_capability() -> Capability {
    let available = system::battery_available();
    Capability {
        id: "battery".to_string(),
        name: "Battery status".to_string(),
        description: "Read-only UPower or sysfs battery information".to_string(),
        state: if available {
            CapabilityState::Available
        } else {
            CapabilityState::UnsupportedHardware
        },
        provider: if system::command_available("upower") {
            "upower/sysfs"
        } else {
            "sysfs"
        }
        .to_string(),
        package: Some("upower".to_string()),
        reason: if available {
            "battery provider detected"
        } else {
            "no battery device detected"
        }
        .to_string(),
    }
}

fn core(id: &str, name: &str, provider: &str, available: bool, reason: &str) -> Capability {
    Capability {
        id: id.to_string(),
        name: name.to_string(),
        description: name.to_string(),
        state: if available {
            CapabilityState::Available
        } else {
            CapabilityState::ProviderError
        },
        provider: provider.to_string(),
        package: None,
        reason: reason.to_string(),
    }
}

fn internal_clipboard(id: &str, name: &str, description: &str) -> Capability {
    let available =
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some();
    Capability {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        state: if available {
            CapabilityState::Available
        } else {
            CapabilityState::ConfigurationMissing
        },
        provider: "GTK/GDK".to_string(),
        package: None,
        reason: if available {
            "graphical clipboard backend detected; ProtonSearch captures changes without polling"
                .to_string()
        } else {
            "no Wayland or X11 display is available in this session".to_string()
        },
    }
}

fn simple_command(id: &str, name: &str, command: &str, package: &str) -> Capability {
    let available = system::command_available(command);
    let package = package_for(command, package);
    Capability {
        id: id.to_string(),
        name: name.to_string(),
        description: format!("Optional provider through {command}"),
        state: if available {
            CapabilityState::Available
        } else {
            CapabilityState::PackageMissing
        },
        provider: command.to_string(),
        package: Some(package.clone()),
        reason: if available {
            "executable detected".to_string()
        } else {
            format!(
                "install {package} with the detected package manager if this capability is needed"
            )
        },
    }
}

fn deferred_command(id: &str, name: &str, command: &str, package: &str) -> Capability {
    let package = package_for(command, package);
    Capability {
        id: id.to_string(),
        name: name.to_string(),
        description: format!("Optional provider through {command}"),
        state: CapabilityState::ConfigurationMissing,
        provider: command.to_string(),
        package: Some(package),
        reason: "provider is intentionally deferred; the launcher shows Coming Soon".to_string(),
    }
}

fn opener() -> Capability {
    let provider = if system::command_available("gio") {
        "gio"
    } else {
        "xdg-open"
    };
    let available = system::command_available(provider);
    let package = package_for(provider, "xdg-utils");
    Capability {
        id: "open-target".to_string(),
        name: "Open files and URLs".to_string(),
        description: "Use the desktop's preferred opener".to_string(),
        state: if available {
            CapabilityState::Available
        } else {
            CapabilityState::PackageMissing
        },
        provider: provider.to_string(),
        package: Some(package.clone()),
        reason: if available {
            "desktop opener detected".to_string()
        } else {
            format!("install {package} or a desktop GLib runtime")
        },
    }
}

fn network_manager() -> Capability {
    let executable = system::command_available("nmcli");
    let service = executable
        && system::run(
            "systemctl",
            &["is-active", "--quiet", "NetworkManager.service"],
        )
        .map(|result| result.status == Some(0))
        .unwrap_or(false);
    let package = package_for("nmcli", "networkmanager");
    Capability {
        id: "wifi".to_string(),
        name: "Wi-Fi controls".to_string(),
        description: "NetworkManager Wi-Fi state and radio actions".to_string(),
        state: if !executable {
            CapabilityState::PackageMissing
        } else if !service {
            CapabilityState::ServiceInactive
        } else {
            CapabilityState::Available
        },
        provider: "nmcli".to_string(),
        package: Some(package),
        reason: if !executable {
            "nmcli is not installed".to_string()
        } else if !service {
            "NetworkManager.service is not active; ProtonSearch will not start it".to_string()
        } else {
            "NetworkManager detected".to_string()
        },
    }
}

fn package_for(command: &str, fallback: &str) -> String {
    platform::dependency_for(command)
        .and_then(|dependency| dependency.package(platform::detect().distribution.family))
        .unwrap_or(fallback)
        .to_string()
}

fn hyprland_capability() -> Capability {
    let info = hyprland::discover();
    Capability {
        id: "hyprland".to_string(),
        name: "Hyprland IPC".to_string(),
        description: "Read-only monitor and workspace discovery".to_string(),
        state: if info.confirmed {
            CapabilityState::Available
        } else if std::env::var_os("XDG_CURRENT_DESKTOP")
            .is_some_and(|v| v.to_string_lossy().to_lowercase().contains("hyprland"))
        {
            CapabilityState::UnsupportedDesktop
        } else {
            CapabilityState::ConfigurationMissing
        },
        provider: "hyprctl query IPC".to_string(),
        package: Some("hyprland".to_string()),
        reason: info.reason,
    }
}
