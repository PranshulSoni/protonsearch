//! Installed-app discovery (Start Menu shortcuts + packaged/UWP apps).
//! OWNER: sources-content. Bounded, no elevation, no shell strings.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::AppEntry;

const MAX_APPS: usize = 3000;
const MAX_LNK_DEPTH: usize = 7;
const MAX_ICONS: usize = 400;

/// Enumerate installed applications. Bounded, no elevation.
pub fn scan() -> (Vec<AppEntry>, Vec<String>) {
    scan_with_icons(&crate::default_icon_cache_dir())
}

/// Enumerate applications and cache their real icons under icon_cache_dir.
pub fn scan_with_icons(icon_cache_dir: &Path) -> (Vec<AppEntry>, Vec<String>) {
    let mut raw: Vec<AppEntry> = Vec::new();
    let mut warnings = Vec::new();

    for root in start_menu_roots() {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .max_depth(MAX_LNK_DEPTH)
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if raw.len() >= MAX_APPS {
                break;
            }
            let path = entry.path();
            if !path.is_file()
                || !path
                    .extension()
                    .map(|ext| ext.eq_ignore_ascii_case("lnk"))
                    .unwrap_or(false)
            {
                continue;
            }
            let Some(name) = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().trim().to_string())
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            // Skip uninstallers/help shims that only add noise to app search.
            let lower = name.to_ascii_lowercase();
            if lower.starts_with("uninstall") || lower.contains("uninstall ") {
                continue;
            }
            match ps_windows::resolve_shortcut(path) {
                Ok(shortcut) => {
                    let Some(target) = shortcut.target.filter(|target| target.is_file()) else {
                        continue;
                    };
                    let keywords = path
                        .parent()
                        .and_then(|parent| parent.file_name())
                        .map(|folder| vec![folder.to_string_lossy().to_lowercase()])
                        .unwrap_or_default();
                    raw.push(AppEntry {
                        id: target.to_string_lossy().to_string(),
                        name,
                        target: target.to_string_lossy().to_string(),
                        icon_path: None,
                        keywords,
                    });
                }
                Err(error) => {
                    if warnings.len() < 10 {
                        warnings.push(format!("shortcut {}: {error}", path.display()));
                    }
                }
            }
        }
    }

    // Packaged / UWP apps from the shell AppsFolder namespace.
    match ps_windows::list_appsfolder() {
        Ok(apps) => {
            for app in apps {
                let target = format!("shell:AppsFolder\\{}", app.aumid);
                raw.push(AppEntry {
                    id: app.aumid,
                    name: app.name,
                    target,
                    icon_path: None,
                    keywords: vec!["uwp".to_string(), "packaged".to_string()],
                });
            }
        }
        Err(error) => warnings.push(format!("AppsFolder enumeration unavailable: {error}")),
    }

    // Dedupe (name, target) and resolve icons for the surviving entries.
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut icons_written = 0usize;
    let mut icon_warnings = 0usize;
    for mut entry in raw {
        let key = format!(
            "{}\u{1}{}",
            entry.name.to_lowercase(),
            entry.target.to_lowercase()
        );
        if !seen.insert(key) {
            continue;
        }
        if icons_written < MAX_ICONS {
            let source = PathBuf::from(&entry.target);
            match ps_windows::extract_icon_png(&source, 32, icon_cache_dir) {
                Ok(icon) => {
                    // Only report a path the shell/UI can actually read.
                    if icon.is_file() {
                        entry.icon_path = Some(icon.to_string_lossy().to_string());
                        icons_written += 1;
                    } else {
                        icon_warnings += 1;
                        if warnings.len() < 20 {
                            warnings.push(format!(
                                "icon {}: extraction reported a missing file",
                                entry.name
                            ));
                        }
                    }
                }
                Err(error) => {
                    // A single bad target must never break app discovery or
                    // produce a path that does not exist.
                    icon_warnings += 1;
                    if warnings.len() < 20 {
                        warnings.push(format!("icon {}: {error}", entry.name));
                    }
                }
            }
        }
        out.push(entry);
        if out.len() >= MAX_APPS {
            break;
        }
    }
    out.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.target.cmp(&b.target))
    });
    if icon_warnings > warnings.iter().filter(|w| w.starts_with("icon ")).count() {
        warnings.push(format!(
            "icon extraction failed for {icon_warnings} app(s); remaining apps show monograms"
        ));
    }
    (out, warnings)
}

fn start_menu_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(
            PathBuf::from(program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    roots
}