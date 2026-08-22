//! Local, low-overhead resource detection and adaptive search budgets.
//!
//! Linux ProtonSearch deliberately uses bounded on-demand providers instead of
//! a permanent indexing thread.  This module keeps those bounds appropriate to
//! the machine without polling: static hardware facts are read once and the
//! current load/battery state is sampled only when a search starts.

use crate::settings::LinuxSettings;
use crate::xdg::XdgPaths;
use serde::Serialize;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::OnceLock;

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceProfile {
    LowResource,
    Balanced,
    HighPerformance,
}

impl ResourceProfile {
    pub fn label(self) -> &'static str {
        match self {
            Self::LowResource => "Low resource",
            Self::Balanced => "Balanced",
            Self::HighPerformance => "High performance",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HardwareSnapshot {
    pub logical_cpus: usize,
    pub memory_bytes: u64,
    pub disk_free_bytes: u64,
    pub rotational_home_storage: Option<bool>,
    pub on_battery: Option<bool>,
    pub load_one: f64,
    pub detected_profile: ResourceProfile,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchBudget {
    pub profile: ResourceProfile,
    pub max_results: usize,
    pub file_results: usize,
    pub file_entries: usize,
    pub file_depth: usize,
    pub content_results: usize,
    pub content_entries: usize,
    pub content_depth: usize,
    pub image_results: usize,
    pub image_entries: usize,
    pub image_depth: usize,
    pub git_repositories: usize,
    pub git_entries: usize,
    pub application_results: usize,
}

static HARDWARE: OnceLock<HardwareSnapshot> = OnceLock::new();

pub fn hardware(paths: &XdgPaths) -> HardwareSnapshot {
    HARDWARE
        .get_or_init(|| detect_hardware(&paths.home))
        .clone()
}

pub fn budget(paths: &XdgPaths, settings: &LinuxSettings) -> SearchBudget {
    let snapshot = hardware(paths);
    let mut profile = match settings
        .performance_mode
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "low-resource" | "low" | "power-saver" => ResourceProfile::LowResource,
        "high-performance" | "high" | "performance" => ResourceProfile::HighPerformance,
        _ => snapshot.detected_profile,
    };

    // These are intentionally conservative.  On-demand search is the only
    // work affected; the resident daemon never spins a background indexer.
    if settings.pause_search_on_battery && snapshot.on_battery == Some(true) {
        profile = ResourceProfile::LowResource;
    }
    if settings.reduce_search_when_busy && snapshot.load_one > busy_load(snapshot.logical_cpus) {
        profile = ResourceProfile::LowResource;
    }

    match profile {
        ResourceProfile::LowResource => SearchBudget {
            profile,
            max_results: 60,
            file_results: 36,
            file_entries: 12_000,
            file_depth: 18,
            content_results: 12,
            content_entries: 3_000,
            content_depth: 12,
            image_results: 48,
            image_entries: 10_000,
            image_depth: 16,
            git_repositories: 5,
            git_entries: 2_500,
            application_results: 24,
        },
        ResourceProfile::Balanced => SearchBudget {
            profile,
            max_results: 100,
            file_results: 55,
            file_entries: 50_000,
            file_depth: 32,
            content_results: 30,
            content_entries: 12_000,
            content_depth: 20,
            image_results: 100,
            image_entries: 30_000,
            image_depth: 24,
            git_repositories: 12,
            git_entries: 6_000,
            application_results: 35,
        },
        ResourceProfile::HighPerformance => SearchBudget {
            profile,
            max_results: 140,
            file_results: 80,
            file_entries: 100_000,
            file_depth: 48,
            content_results: 45,
            content_entries: 24_000,
            content_depth: 28,
            image_results: 140,
            image_entries: 60_000,
            image_depth: 32,
            git_repositories: 20,
            git_entries: 10_000,
            application_results: 50,
        },
    }
}

pub fn summary(paths: &XdgPaths, settings: &LinuxSettings) -> String {
    let snapshot = hardware(paths);
    let budget = budget(paths, settings);
    let memory = if snapshot.memory_bytes == 0 {
        "unknown".to_string()
    } else {
        format!("{:.1} GiB", snapshot.memory_bytes as f64 / GIB as f64)
    };
    let storage = match snapshot.rotational_home_storage {
        Some(true) => "rotational disk",
        Some(false) => "SSD/NVMe",
        None => "storage type unknown",
    };
    let disk = if snapshot.disk_free_bytes == 0 {
        "free disk space unknown".to_string()
    } else {
        format!(
            "{:.1} GiB free disk",
            snapshot.disk_free_bytes as f64 / GIB as f64
        )
    };
    let power = match snapshot.on_battery {
        Some(true) => "on battery",
        Some(false) => "AC power",
        None => "power state unknown",
    };
    format!(
        "Detected {} · {} logical CPUs · {} RAM · {} · {} · {} · current load {:.2}",
        budget.profile.label(),
        snapshot.logical_cpus,
        memory,
        storage,
        power,
        disk,
        snapshot.load_one
    )
}

fn detect_hardware(home: &Path) -> HardwareSnapshot {
    let logical_cpus = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    let memory_bytes = total_memory_bytes();
    let disk_free_bytes = free_disk_space(home);
    let rotational_home_storage = rotational_storage(home);
    let profile = if logical_cpus <= 4
        || memory_bytes > 0 && memory_bytes <= 6 * GIB
        || rotational_home_storage == Some(true)
    {
        ResourceProfile::LowResource
    } else if logical_cpus >= 12
        && memory_bytes >= 16 * GIB
        && rotational_home_storage != Some(true)
    {
        ResourceProfile::HighPerformance
    } else {
        ResourceProfile::Balanced
    };
    HardwareSnapshot {
        logical_cpus,
        memory_bytes,
        disk_free_bytes,
        rotational_home_storage,
        on_battery: power_state(),
        load_one: load_one(),
        detected_profile: profile,
    }
}

fn busy_load(cpus: usize) -> f64 {
    (cpus as f64 * 0.8).max(1.0)
}

fn total_memory_bytes() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let mut parts = line.split_whitespace();
                if parts.next() != Some("MemTotal:") {
                    return None;
                }
                Some(parts.next()?.parse::<u64>().ok()?.saturating_mul(1024))
            })
        })
        .unwrap_or(0)
}

fn load_one() -> f64 {
    fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|value| value.split_whitespace().next()?.parse().ok())
        .unwrap_or(0.0)
}

fn power_state() -> Option<bool> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut found_battery = false;
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = fs::read_to_string(path.join("type")).ok()?;
        if kind.trim() != "Battery" {
            continue;
        }
        found_battery = true;
        let status = fs::read_to_string(path.join("status")).ok()?;
        if matches!(status.trim(), "Charging" | "Full") {
            return Some(false);
        }
        if status.trim() == "Discharging" {
            return Some(true);
        }
    }
    found_battery.then_some(false)
}

fn rotational_storage(path: &Path) -> Option<bool> {
    let device = fs::metadata(path).ok()?.dev();
    let major = libc::major(device);
    let minor = libc::minor(device);
    fs::read_to_string(format!("/sys/dev/block/{major}:{minor}/queue/rotational"))
        .ok()
        .and_then(|value| match value.trim() {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        })
}

fn free_disk_space(path: &Path) -> u64 {
    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return 0;
    };
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // `statvfs` writes the complete structure on success. It is a local,
    // read-only filesystem query and does not invoke a subprocess.
    let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
    if result != 0 {
        return 0;
    }
    let stats = unsafe { stats.assume_init() };
    (stats.f_bavail as u64).saturating_mul(stats.f_frsize as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::LinuxSettings;
    use std::path::PathBuf;

    fn paths() -> XdgPaths {
        XdgPaths {
            home: PathBuf::from("/home/test"),
            config: PathBuf::from("/home/test/.config"),
            data: PathBuf::from("/home/test/.local/share"),
            state: PathBuf::from("/home/test/.local/state"),
            cache: PathBuf::from("/home/test/.cache"),
            runtime: None,
        }
    }

    #[test]
    fn explicit_low_profile_is_bounded() {
        let mut settings = LinuxSettings::default();
        settings.performance_mode = "low-resource".to_string();
        let budget = budget(&paths(), &settings);
        assert_eq!(budget.profile, ResourceProfile::LowResource);
        assert!(budget.file_entries < 50_000);
        assert!(budget.image_entries < 30_000);
    }

    #[test]
    fn unknown_mode_uses_detected_profile() {
        let mut settings = LinuxSettings::default();
        settings.performance_mode = "adaptive".to_string();
        let budget = budget(&paths(), &settings);
        assert!(matches!(
            budget.profile,
            ResourceProfile::LowResource
                | ResourceProfile::Balanced
                | ResourceProfile::HighPerformance
        ));
    }
}
