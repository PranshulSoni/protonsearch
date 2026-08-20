use crate::system;
use crate::xdg::XdgPaths;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct HyprlandInfo {
    pub confirmed: bool,
    pub reason: String,
    pub monitors: Option<Value>,
    pub workspaces: Option<Value>,
    pub active_workspace: Option<Value>,
    pub config_files: Vec<PathBuf>,
}

pub fn discover() -> HyprlandInfo {
    let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE");
    let Some(signature) = signature else {
        return HyprlandInfo {
            confirmed: false,
            reason: "Hyprland instance signature is not present".to_string(),
            monitors: None,
            workspaces: None,
            active_workspace: None,
            config_files: Vec::new(),
        };
    };
    if signature.is_empty() {
        return HyprlandInfo {
            confirmed: false,
            reason: "Hyprland instance signature is empty".to_string(),
            monitors: None,
            workspaces: None,
            active_workspace: None,
            config_files: Vec::new(),
        };
    }
    let monitors = query_json("monitors");
    let confirmed = monitors.is_some();
    let config_files = if confirmed {
        XdgPaths::discover()
            .ok()
            .map(|paths| discover_config_files(&paths))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    HyprlandInfo {
        confirmed,
        reason: if confirmed {
            "Hyprland query IPC responded".to_string()
        } else {
            "Hyprland signature exists but query IPC is unavailable".to_string()
        },
        monitors,
        workspaces: query_json("workspaces"),
        active_workspace: query_json("activeworkspace"),
        config_files,
    }
}

pub fn query_json(query: &str) -> Option<Value> {
    if !matches!(
        query,
        "monitors" | "workspaces" | "activeworkspace" | "version" | "clients" | "devices" | "binds"
    ) {
        return None;
    }
    let result = system::run("hyprctl", &["-j", query]).ok()?;
    if result.status != Some(0) || result.timed_out {
        return None;
    }
    serde_json::from_str(&result.stdout).ok()
}

pub fn focus_window(address: &str) -> anyhow::Result<()> {
    let address = address.trim();
    if !address.starts_with("0x")
        || address.len() < 3
        || !address[2..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        anyhow::bail!("invalid Hyprland window address");
    }
    let selector = format!("address:{address}");
    let result = system::run("hyprctl", &["dispatch", "focuswindow", &selector])?;
    if result.status != Some(0) || result.timed_out {
        anyhow::bail!("Hyprland could not focus the requested window");
    }
    Ok(())
}

fn discover_config_files(paths: &XdgPaths) -> Vec<PathBuf> {
    let primary = std::env::var_os("HYPRLAND_CONFIG")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            [
                paths.config.join("hypr/hyprland.conf"),
                paths.config.join("hypr/hyprland.lua"),
            ]
            .into_iter()
            .find(|path| path.is_file())
        });
    let Some(primary) = primary else {
        return Vec::new();
    };
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    collect_config(&primary, &mut visited, &mut result, 0);
    result
}

fn collect_config(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    result: &mut Vec<PathBuf>,
    depth: usize,
) {
    if depth > 32 || !path.is_file() {
        return;
    }
    let Ok(path) = path.canonicalize() else {
        return;
    };
    if !visited.insert(path.clone()) {
        return;
    }
    result.push(path.clone());
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("source") else {
            continue;
        };
        let include = rest.trim().trim_matches('"');
        if include.is_empty() || include.contains('$') || include.contains('*') {
            continue;
        }
        let include = PathBuf::from(include);
        let include = if include.is_absolute() {
            include
        } else {
            path.parent().unwrap_or(path.as_path()).join(include)
        };
        collect_config(&include, visited, result, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mutating_or_unlisted_queries() {
        assert!(query_json("reload").is_none());
        assert!(query_json("dispatch").is_none());
    }
}
