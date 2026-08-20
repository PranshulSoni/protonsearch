//! Distribution- and desktop-aware Linux platform services.
//!
//! The launcher providers consume capabilities, not distribution names. This
//! module is the single place that translates `/etc/os-release`, desktop
//! environment variables, display-server state, and package-manager behavior
//! into stable Linux platform information.

use crate::system;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionFamily {
    Arch,
    Debian,
    Fedora,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageManagerKind {
    Pacman,
    Apt,
    Dnf,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DesktopEnvironment {
    Gnome,
    Kde,
    Cinnamon,
    Xfce,
    Hyprland,
    Sway,
    I3,
    Other,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayServer {
    Wayland,
    X11,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DistributionInfo {
    pub id: String,
    pub id_like: Vec<String>,
    pub name: String,
    pub version_id: String,
    pub pretty_name: String,
    pub family: DistributionFamily,
    pub package_manager: PackageManagerKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformInfo {
    pub distribution: DistributionInfo,
    pub desktop: DesktopEnvironment,
    pub display_server: DisplayServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencySpec {
    pub feature: &'static str,
    pub command: &'static str,
    pub arch: Option<&'static str>,
    pub debian: Option<&'static str>,
    pub fedora: Option<&'static str>,
}

impl DependencySpec {
    pub fn package(self, family: DistributionFamily) -> Option<&'static str> {
        match family {
            DistributionFamily::Arch => self.arch,
            DistributionFamily::Debian => self.debian,
            DistributionFamily::Fedora => self.fedora,
            DistributionFamily::Other => self.debian.or(self.fedora).or(self.arch),
        }
    }
}

const DEPENDENCIES: &[DependencySpec] = &[
    DependencySpec {
        feature: "desktop opener",
        command: "gio",
        arch: Some("xdg-utils"),
        debian: Some("xdg-utils"),
        fedora: Some("xdg-utils"),
    },
    DependencySpec {
        feature: "Wayland clipboard",
        command: "wl-paste",
        arch: Some("wl-clipboard"),
        debian: Some("wl-clipboard"),
        fedora: Some("wl-clipboard"),
    },
    DependencySpec {
        feature: "clipboard history",
        command: "cliphist",
        arch: Some("cliphist"),
        debian: Some("cliphist"),
        fedora: Some("cliphist"),
    },
    DependencySpec {
        feature: "browser history",
        command: "sqlite3",
        arch: Some("sqlite"),
        debian: Some("sqlite3"),
        fedora: Some("sqlite"),
    },
    DependencySpec {
        feature: "PDF content search",
        command: "pdftotext",
        arch: Some("poppler"),
        debian: Some("poppler-utils"),
        fedora: Some("poppler-utils"),
    },
    DependencySpec {
        feature: "area screenshots",
        command: "grim",
        arch: Some("grim"),
        debian: Some("grim"),
        fedora: Some("grim"),
    },
    DependencySpec {
        feature: "screenshot selection",
        command: "slurp",
        arch: Some("slurp"),
        debian: Some("slurp"),
        fedora: Some("slurp"),
    },
    DependencySpec {
        feature: "Wi-Fi controls",
        command: "nmcli",
        arch: Some("networkmanager"),
        debian: Some("network-manager"),
        fedora: Some("NetworkManager"),
    },
    DependencySpec {
        feature: "Bluetooth controls",
        command: "bluetoothctl",
        arch: Some("bluez-utils"),
        debian: Some("bluez"),
        fedora: Some("bluez"),
    },
    DependencySpec {
        feature: "PipeWire audio controls",
        command: "wpctl",
        arch: Some("wireplumber"),
        debian: Some("wireplumber"),
        fedora: Some("wireplumber"),
    },
    DependencySpec {
        feature: "brightness controls",
        command: "brightnessctl",
        arch: Some("brightnessctl"),
        debian: Some("brightnessctl"),
        fedora: Some("brightnessctl"),
    },
    DependencySpec {
        feature: "media controls",
        command: "playerctl",
        arch: Some("playerctl"),
        debian: Some("playerctl"),
        fedora: Some("playerctl"),
    },
    DependencySpec {
        feature: "power profiles",
        command: "powerprofilesctl",
        arch: Some("power-profiles-daemon"),
        debian: Some("power-profiles-daemon"),
        fedora: Some("power-profiles-daemon"),
    },
    DependencySpec {
        feature: "battery status",
        command: "upower",
        arch: Some("upower"),
        debian: Some("upower"),
        fedora: Some("upower"),
    },
    DependencySpec {
        feature: "OCR (deferred)",
        command: "tesseract",
        arch: Some("tesseract"),
        debian: Some("tesseract-ocr"),
        fedora: Some("tesseract"),
    },
];

pub fn dependency_specs() -> &'static [DependencySpec] {
    DEPENDENCIES
}

pub fn dependency_for(command: &str) -> Option<DependencySpec> {
    DEPENDENCIES
        .iter()
        .copied()
        .find(|dependency| dependency.command == command)
}

pub fn detect() -> PlatformInfo {
    static PLATFORM: OnceLock<PlatformInfo> = OnceLock::new();
    PLATFORM.get_or_init(detect_uncached).clone()
}

fn detect_uncached() -> PlatformInfo {
    let distribution = DistributionInfo::detect();
    PlatformInfo {
        distribution,
        desktop: detect_desktop(),
        display_server: detect_display_server(),
    }
}

impl DistributionInfo {
    pub fn detect() -> Self {
        let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        Self::from_os_release(&content)
    }

    pub fn from_os_release(content: &str) -> Self {
        let values = parse_os_release(content);
        let id = values
            .get("ID")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let id_like = values
            .get("ID_LIKE")
            .map(|value| {
                value
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let family = family_for(&id, &id_like);
        let package_manager = PackageManager::detect_for(family).kind;
        Self {
            name: values
                .get("NAME")
                .cloned()
                .unwrap_or_else(|| "Linux".to_string()),
            version_id: values.get("VERSION_ID").cloned().unwrap_or_default(),
            pretty_name: values
                .get("PRETTY_NAME")
                .cloned()
                .unwrap_or_else(|| "Linux".to_string()),
            id,
            id_like,
            family,
            package_manager,
        }
    }
}

fn parse_os_release(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), unquote(value.trim())))
        })
        .collect()
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].replace("\\\"", "\"")
    } else {
        trimmed.to_string()
    }
}

fn family_for(id: &str, id_like: &[String]) -> DistributionFamily {
    let mut ids = vec![id.to_ascii_lowercase()];
    ids.extend(id_like.iter().map(|value| value.to_ascii_lowercase()));
    if ids.iter().any(|value| {
        matches!(
            value.as_str(),
            "arch" | "endeavouros" | "manjaro" | "garuda" | "artix"
        )
    }) {
        DistributionFamily::Arch
    } else if ids.iter().any(|value| {
        matches!(
            value.as_str(),
            "debian" | "ubuntu" | "linuxmint" | "mint" | "pop" | "elementary"
        )
    }) {
        DistributionFamily::Debian
    } else if ids.iter().any(|value| {
        matches!(
            value.as_str(),
            "fedora" | "rhel" | "centos" | "rocky" | "almalinux"
        )
    }) {
        DistributionFamily::Fedora
    } else {
        DistributionFamily::Other
    }
}

fn detect_desktop() -> DesktopEnvironment {
    let values = [
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        std::env::var("XDG_SESSION_DESKTOP").unwrap_or_default(),
        std::env::var("DESKTOP_SESSION").unwrap_or_default(),
    ]
    .join(":")
    .to_ascii_lowercase();
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some() || values.contains("hyprland") {
        DesktopEnvironment::Hyprland
    } else if std::env::var_os("SWAYSOCK").is_some() || values.contains("sway") {
        DesktopEnvironment::Sway
    } else if std::env::var_os("I3SOCK").is_some() || values.contains("i3") {
        DesktopEnvironment::I3
    } else if values.contains("gnome") {
        DesktopEnvironment::Gnome
    } else if values.contains("kde") || values.contains("plasma") {
        DesktopEnvironment::Kde
    } else if values.contains("cinnamon") {
        DesktopEnvironment::Cinnamon
    } else if values.contains("xfce") {
        DesktopEnvironment::Xfce
    } else if values.trim_matches(':').is_empty() {
        DesktopEnvironment::Unknown
    } else {
        DesktopEnvironment::Other
    }
}

fn detect_display_server() -> DisplayServer {
    match std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "wayland" => DisplayServer::Wayland,
        "x11" => DisplayServer::X11,
        _ if std::env::var_os("WAYLAND_DISPLAY").is_some() => DisplayServer::Wayland,
        _ if std::env::var_os("DISPLAY").is_some() => DisplayServer::X11,
        _ => DisplayServer::Unknown,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PackageManager {
    pub kind: PackageManagerKind,
    pub executable: &'static str,
}

impl PackageManager {
    pub fn detect() -> Self {
        Self::detect_for(DistributionInfo::detect().family)
    }

    fn detect_for(family: DistributionFamily) -> Self {
        let preferred: &[(PackageManagerKind, &str)] = match family {
            DistributionFamily::Arch => &[(PackageManagerKind::Pacman, "pacman")],
            DistributionFamily::Debian => &[(PackageManagerKind::Apt, "apt-get")],
            DistributionFamily::Fedora => &[(PackageManagerKind::Dnf, "dnf")],
            DistributionFamily::Other => &[],
        };
        for &(kind, executable) in preferred {
            if system::command_available(executable) {
                return Self { kind, executable };
            }
        }
        for (kind, executable) in [
            (PackageManagerKind::Pacman, "pacman"),
            (PackageManagerKind::Apt, "apt-get"),
            (PackageManagerKind::Dnf, "dnf"),
        ] {
            if system::command_available(executable) {
                return Self { kind, executable };
            }
        }
        Self {
            kind: PackageManagerKind::Unknown,
            executable: "",
        }
    }

    pub fn is_installed(&self, package: &str) -> bool {
        let args: Vec<String> = match self.kind {
            PackageManagerKind::Pacman => vec!["-Q".into(), package.into()],
            PackageManagerKind::Apt => vec!["-W".into(), "-f=${Status}".into(), package.into()],
            PackageManagerKind::Dnf => vec!["-q".into(), package.into()],
            PackageManagerKind::Unknown => return false,
        };
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        system::run(self.executable, &refs)
            .map(|result| {
                result.status == Some(0)
                    && (self.kind != PackageManagerKind::Apt
                        || result.stdout.contains("install ok installed"))
            })
            .unwrap_or(false)
    }

    pub fn version(&self, package: &str) -> Option<String> {
        let args: Vec<String> = match self.kind {
            PackageManagerKind::Pacman => vec!["-Q".into(), package.into()],
            PackageManagerKind::Apt => vec!["-W".into(), "-f=${Version}".into(), package.into()],
            PackageManagerKind::Dnf => vec![
                "-q".into(),
                "--qf".into(),
                "%{VERSION}-%{RELEASE}".into(),
                package.into(),
            ],
            PackageManagerKind::Unknown => return None,
        };
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        system::run(self.executable, &refs)
            .ok()
            .filter(|result| result.status == Some(0))
            .map(|result| result.stdout.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    pub fn search(&self, query: &str) -> Result<String> {
        let query = query.trim();
        if query.is_empty() {
            anyhow::bail!("package search requires a query");
        }
        let args: Vec<String> = match self.kind {
            PackageManagerKind::Pacman => vec!["-Ss".into(), query.into()],
            PackageManagerKind::Apt => vec!["search".into(), query.into()],
            PackageManagerKind::Dnf => vec!["search".into(), query.into()],
            PackageManagerKind::Unknown => anyhow::bail!("no supported package manager detected"),
        };
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        let result = system::run_with_timeout(self.executable, &refs, Duration::from_secs(30))
            .context("search package metadata")?;
        if result.timed_out || result.status != Some(0) {
            anyhow::bail!("package search failed: {}", result.stderr.trim());
        }
        Ok(result.stdout)
    }

    pub fn install(&self, packages: &[String]) -> Result<system::CommandResult> {
        self.run_privileged("install", packages)
    }

    pub fn remove(&self, packages: &[String]) -> Result<system::CommandResult> {
        self.run_privileged("remove", packages)
    }

    pub fn update_metadata(&self) -> Result<system::CommandResult> {
        let args: Vec<&str> = match self.kind {
            PackageManagerKind::Pacman => vec!["-Sy"],
            PackageManagerKind::Apt => vec!["update"],
            PackageManagerKind::Dnf => vec!["makecache"],
            PackageManagerKind::Unknown => anyhow::bail!("no supported package manager detected"),
        };
        self.run_privileged_args(&args)
    }

    fn run_privileged(
        &self,
        operation: &str,
        packages: &[String],
    ) -> Result<system::CommandResult> {
        if packages.is_empty() {
            anyhow::bail!("no packages were specified");
        }
        let mut args = vec![operation.to_string()];
        args.extend(packages.iter().cloned());
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        self.run_privileged_args(&refs)
    }

    fn run_privileged_args(&self, args: &[&str]) -> Result<system::CommandResult> {
        if self.kind == PackageManagerKind::Unknown {
            anyhow::bail!("no supported package manager detected");
        }
        let mut sudo_args = vec![self.executable];
        sudo_args.extend_from_slice(args);
        system::run_with_timeout("sudo", &sudo_args, Duration::from_secs(300))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_arch_derivatives() {
        let info = DistributionInfo::from_os_release(
            "ID=manjaro\nID_LIKE=arch\nNAME=Manjaro\nVERSION_ID=24.0\n",
        );
        assert_eq!(info.family, DistributionFamily::Arch);
        assert_eq!(info.id, "manjaro");
    }

    #[test]
    fn recognizes_mint_as_debian_family() {
        let info = DistributionInfo::from_os_release(
            "ID=linuxmint\nID_LIKE=ubuntu debian\nPRETTY_NAME=\"Linux Mint 22\"\n",
        );
        assert_eq!(info.family, DistributionFamily::Debian);
        assert_eq!(info.pretty_name, "Linux Mint 22");
    }

    #[test]
    fn recognizes_fedora_derivatives() {
        let info = DistributionInfo::from_os_release(
            "ID=rocky\nID_LIKE=\"rhel centos fedora\"\nNAME=Rocky Linux\n",
        );
        assert_eq!(info.family, DistributionFamily::Fedora);
    }

    #[test]
    fn dependency_names_follow_distribution_family() {
        let pdf = dependency_for("pdftotext").expect("PDF dependency");
        assert_eq!(pdf.package(DistributionFamily::Arch), Some("poppler"));
        assert_eq!(
            pdf.package(DistributionFamily::Debian),
            Some("poppler-utils")
        );
        assert_eq!(
            pdf.package(DistributionFamily::Fedora),
            Some("poppler-utils")
        );
    }

    #[test]
    fn desktop_and_display_detection_have_stable_fallbacks() {
        assert!(matches!(
            detect_display_server(),
            DisplayServer::Wayland | DisplayServer::X11 | DisplayServer::Unknown
        ));
        assert!(matches!(
            detect_desktop(),
            DesktopEnvironment::Gnome
                | DesktopEnvironment::Kde
                | DesktopEnvironment::Cinnamon
                | DesktopEnvironment::Xfce
                | DesktopEnvironment::Hyprland
                | DesktopEnvironment::Sway
                | DesktopEnvironment::I3
                | DesktopEnvironment::Other
                | DesktopEnvironment::Unknown
        ));
    }
}
