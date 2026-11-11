//! Installed-app discovery (Start Menu shortcuts + packaged/UWP apps).
//! OWNER: sources-content. Bounded, no elevation, no shell strings.
//!
//! Identity rules (task-14):
//! - A Start-Menu .lnk identifies an app by its canonical target path plus its
//!   shortcut arguments (so "Firefox Private Browsing" stays distinct from
//!   "Firefox" while duplicate user/common-menu shortcuts collapse).
//! - A packaged app identifies by its AUMID.
//! - Start-Menu entries that point at documents/URLs (.rtf/.txt/.url/.html/...)
//!   are file shortcuts, not applications, and are excluded.
//! - AppsFolder shell-namespace entries that merely re-expose classic shortcuts
//!   (path/URL-shaped "AUMIDs") are excluded; genuine packaged apps are kept.
//! - When a classic Start-Menu app and a packaged entry share a display name,
//!   the classic entry wins (it is the one the user pinned/expects).
//! - Icons are keyed by icon-source identity (canonical target / AUMID + size),
//!   never by display name.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::AppEntry;

const MAX_APPS: usize = 3000;
const MAX_LNK_DEPTH: usize = 7;
const MAX_ICONS: usize = 400;

/// Extensions that can actually be launched as applications/tools. Anything
/// else from the Start Menu is a documentation shortcut and belongs to file
/// search, not the application index.
const APPLICATION_EXTENSIONS: &[&str] = &[
    "exe",
    "com",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "msc",
    "cpl",
    "scr",
    "appref-ms",
    "jar",
    "hta",
    "lnk",
];

/// Internal row used for identity-aware dedupe before icons are attached.
struct RawApp {
    entry: AppEntry,
    identity: String,
    packaged: bool,
    display_name: String,
    /// Classic shortcut with launch arguments (e.g. Firefox Private Browsing).
    has_args: bool,
}

/// Enumerate installed applications. Bounded, no elevation.
pub fn scan() -> (Vec<AppEntry>, Vec<String>) {
    scan_with_icons(&crate::default_icon_cache_dir())
}

/// Enumerate applications and cache their real icons under icon_cache_dir.
pub fn scan_with_icons(icon_cache_dir: &Path) -> (Vec<AppEntry>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut raw: Vec<RawApp> = Vec::new();

    for root in start_menu_roots() {
        collect_start_menu(&root, &mut raw, &mut warnings);
    }

    match ps_windows::list_appsfolder() {
        Ok(entries) => {
            for app in entries {
                if !is_packaged_aumid(&app.aumid) {
                    // Shell-namespace entry for a classic shortcut/file/URL: the
                    // Start Menu scan above already owns those identities.
                    continue;
                }
                let target = format!("shell:AppsFolder\\{}", app.aumid);
                raw.push(RawApp {
                    has_args: false,
                    identity: format!("aumid:{}", app.aumid.to_lowercase()),
                    entry: AppEntry {
                        id: app.aumid.clone(),
                        name: app.name.clone(),
                        target,
                        icon_path: None,
                        keywords: vec!["uwp".to_string(), "packaged".to_string()],
                    },
                    packaged: true,
                    display_name: app.name,
                });
            }
        }
        Err(error) => warnings.push(format!("AppsFolder enumeration unavailable: {error}")),
    }

    let deduped = dedupe_apps(raw);

    // Attach real icons (bounded), then sort for a stable UI order.
    let mut out: Vec<AppEntry> = Vec::with_capacity(deduped.len());
    let mut icons_written = 0usize;
    let mut icon_warnings = 0usize;
    for mut app in deduped {
        if out.len() >= MAX_APPS {
            break;
        }
        if icons_written < MAX_ICONS {
            let source = PathBuf::from(&app.entry.target);
            match ps_windows::extract_icon_png(&source, 32, icon_cache_dir) {
                Ok(icon) if icon.is_file() => {
                    app.entry.icon_path = Some(icon.to_string_lossy().to_string());
                    icons_written += 1;
                }
                Ok(_) => {
                    icon_warnings += 1;
                    if warnings.len() < 20 {
                        warnings.push(format!(
                            "icon {}: extraction reported a missing file",
                            app.entry.name
                        ));
                    }
                }
                Err(error) => {
                    // A single bad target must never break app discovery or
                    // produce an icon path that does not exist.
                    icon_warnings += 1;
                    if warnings.len() < 20 {
                        warnings.push(format!("icon {}: {error}", app.entry.name));
                    }
                }
            }
        }
        out.push(app.entry);
    }
    if icon_warnings > 0 && warnings.len() < 20 {
        warnings.push(format!(
            "icon extraction failed for {icon_warnings} app(s); remaining apps show monograms"
        ));
    }
    out.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.target.cmp(&b.target))
    });
    (out, warnings)
}

/// Collapse shortcuts that launch the same program (see the passes below).
fn dedupe_apps(raw: Vec<RawApp>) -> Vec<RawApp> {
    // Pass 1: dedupe by normalized identity (target+args / AUMID).
    let mut seen_classic: HashSet<String> = HashSet::new();
    let mut seen_packaged: HashSet<String> = HashSet::new();
    let mut by_identity: Vec<RawApp> = Vec::new();
    for app in raw {
        if app.packaged {
            if !seen_packaged.insert(app.identity.clone()) {
                continue;
            }
        } else if !seen_classic.insert(app.identity.clone()) {
            continue;
        }
        by_identity.push(app);
    }

    // Pass 2: one row per program and display name. Preference order:
    // classic app without args > classic app with args > packaged app, so the
    // plain Start Menu shortcut wins over an argument variant of the same
    // program and over its AppsFolder twin. Classic shortcuts with the same name
    // that launch *different* programs are separate installs and both stay; the
    // engine shows each one's location.
    by_identity.sort_by(|a, b| {
        dedupe_preference(a)
            .cmp(&dedupe_preference(b))
            .then_with(|| {
                normalize_display_name(&a.display_name).cmp(&normalize_display_name(&b.display_name))
            })
            .then_with(|| a.entry.target.cmp(&b.entry.target))
    });
    let mut seen_names: HashSet<String> = HashSet::new();
    let mut seen_programs: HashSet<(String, String)> = HashSet::new();
    let mut deduped: Vec<RawApp> = Vec::new();
    for app in by_identity {
        let name = normalize_display_name(&app.display_name);
        if name.is_empty() {
            continue;
        }
        let keep = if app.packaged {
            !seen_names.contains(&name)
        } else {
            let program = app.identity.split('#').next().unwrap_or(&app.identity).to_string();
            seen_programs.insert((name.clone(), program))
        };
        if keep {
            seen_names.insert(name);
            deduped.push(app);
        }
    }
    deduped
}

fn collect_start_menu(root: &Path, apps: &mut Vec<RawApp>, warnings: &mut Vec<String>) {
    if !root.is_dir() {
        return;
    }
    for entry in WalkDir::new(root)
        .max_depth(MAX_LNK_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if apps.len() >= MAX_APPS * 2 {
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
                if !is_application_target(&target) {
                    // Documentation/file shortcut: not an application.
                    continue;
                }
                let target_text = target.to_string_lossy().to_string();
                let identity = classic_identity(&target, shortcut.arguments.as_deref());
                let keywords = path
                    .parent()
                    .and_then(|parent| parent.file_name())
                    .map(|folder| vec![folder.to_string_lossy().to_lowercase()])
                    .unwrap_or_default();
                apps.push(RawApp {
                    has_args: shortcut
                        .arguments
                        .as_deref()
                        .map(|args| !args.trim().is_empty())
                        .unwrap_or(false),
                    identity,
                    entry: AppEntry {
                        id: target_text.clone(),
                        name: name.clone(),
                        target: target_text,
                        icon_path: None,
                        keywords,
                    },
                    packaged: false,
                    display_name: name,
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

/// A Start-Menu item is an application if its target is a launchable file.
fn is_application_target(path: &Path) -> bool {
    match path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
    {
        Some(extension) => APPLICATION_EXTENSIONS.contains(&extension.as_str()),
        None => false,
    }
}

/// Canonical identity for a classic app: canonical target + normalized args.
fn classic_identity(target: &Path, arguments: Option<&str>) -> String {
    let canonical = std::fs::canonicalize(target)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| target.to_string_lossy().to_string());
    let canonical = canonical.strip_prefix(r"\\?\").unwrap_or(&canonical);
    let canonical = canonical.replace('/', "\\").to_lowercase();
    let arguments = arguments.unwrap_or("").trim().to_lowercase();
    if arguments.is_empty() {
        format!("exe:{canonical}")
    } else {
        format!("exe:{canonical}#{arguments}")
    }
}

/// True for genuine packaged/UWP AUMIDs; false for shell-namespace entries that
/// merely re-expose a classic shortcut, file or URL.
fn is_packaged_aumid(aumid: &str) -> bool {
    let aumid = aumid.trim();
    if aumid.is_empty() {
        return false;
    }
    // Shell-generated identifiers for classic Start-Menu items.
    if aumid.starts_with("Microsoft.AutoGenerated.") {
        return false;
    }
    // Path/URL-shaped entries: "shell:AppsFolder\C:\...", "{GUID}\file.exe",
    // "https://..." - these duplicate the classic scan or are not applications.
    if aumid.contains('\\') || aumid.contains('/') {
        return false;
    }
    // Known-folder GUID entries are namespaces, not apps.
    if aumid.starts_with('{') && aumid.ends_with('}') {
        return false;
    }
    true
}

/// Preference used when two sources expose the same display name.
fn dedupe_preference(app: &RawApp) -> u8 {
    match (app.packaged, app.has_args) {
        (false, false) => 0,
        (false, true) => 1,
        (true, _) => 2,
    }
}

/// Display-name normalization used for cross-source dedupe.
fn normalize_display_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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

#[cfg(test)]
mod dedupe_tests {
    use super::*;

    fn classic(name: &str, target: &str, args: Option<&str>) -> RawApp {
        RawApp {
            identity: classic_identity(Path::new(target), args),
            entry: AppEntry {
                id: target.into(),
                name: name.into(),
                target: target.into(),
                icon_path: None,
                keywords: Vec::new(),
            },
            packaged: false,
            display_name: name.into(),
            has_args: args.is_some(),
        }
    }

    fn packaged(name: &str, aumid: &str) -> RawApp {
        RawApp {
            identity: format!("aumid:{}", aumid.to_lowercase()),
            entry: AppEntry {
                id: aumid.into(),
                name: name.into(),
                target: format!("shell:AppsFolder\\{aumid}"),
                icon_path: None,
                keywords: Vec::new(),
            },
            packaged: true,
            display_name: name.into(),
            has_args: false,
        }
    }

    #[test]
    fn same_program_collapses_but_separate_installs_stay() {
        let apps = dedupe_apps(vec![
            // The same shortcut in the machine and the user Start Menu.
            classic("Google Chrome", r"C:\Chrome\chrome.exe", None),
            classic("Google Chrome", r"C:\Chrome\chrome.exe", None),
            // Same program and name with arguments: the plain shortcut wins.
            classic("Google Chrome", r"C:\Chrome\chrome.exe", Some("--profile-directory=Default")),
            // A differently named variant of the same program stays.
            classic("Chrome (Work)", r"C:\Chrome\chrome.exe", Some("--profile-directory=Work")),
            // The Store twin of a Start Menu shortcut yields to it.
            packaged("Google Chrome", "Chrome.App"),
            // Two installs of the same product in different folders both stay.
            classic("Python 3.12", r"C:\Python312\python.exe", None),
            classic("Python 3.12", r"C:\Users\X\Python312\python.exe", None),
        ]);
        let mut rows: Vec<(String, String)> = apps
            .iter()
            .map(|app| (app.entry.name.clone(), app.entry.target.clone()))
            .collect();
        rows.sort();
        assert_eq!(
            rows,
            vec![
                ("Chrome (Work)".to_string(), r"C:\Chrome\chrome.exe".to_string()),
                ("Google Chrome".to_string(), r"C:\Chrome\chrome.exe".to_string()),
                ("Python 3.12".to_string(), r"C:\Python312\python.exe".to_string()),
                ("Python 3.12".to_string(), r"C:\Users\X\Python312\python.exe".to_string()),
            ]
        );
        assert!(apps.iter().all(|app| !app.packaged));
    }
}
