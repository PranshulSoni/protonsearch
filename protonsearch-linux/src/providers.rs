//! Search providers used by the GTK launcher.
//!
//! Providers intentionally return launchable, bounded results. They do not
//! execute shell strings and optional data sources are simply skipped when
//! their files or utilities are unavailable.

use crate::actions;
use crate::calculator;
use crate::desktop::{self, DesktopEntry};
use crate::search::{self, FileResult, SearchOptions};
use crate::settings::{self, LinuxSettings};
use crate::system;
use crate::xdg::XdgPaths;
use gdk_pixbuf::{Colorspace, InterpType, Pixbuf};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MAX_GIT_REPOSITORIES: usize = 12;
const MAX_GIT_SCAN_ENTRIES: usize = 6_000;

pub const MAX_RESULTS: usize = 100;

#[derive(Clone)]
pub enum Target {
    Application(DesktopEntry),
    Path(PathBuf),
    Url(String),
    Action {
        id: String,
        args: Vec<String>,
        confirmed: bool,
    },
    Copy(String),
    Cliphist(String),
    /// Change the launcher query without spawning another process.
    Query(String),
    /// A safe informational home-card action.
    Notice(String),
}

#[derive(Clone)]
pub struct Item {
    pub title: String,
    pub subtitle: String,
    pub source: String,
    pub kind: String,
    pub target: Target,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    All,
    Apps,
    Files,
    Folders,
    Settings,
    Commands,
    Browser,
    Bookmarks,
    History,
    Recent,
    Git,
    Content,
    Clipboard,
    Images,
    Notes,
    Snippets,
    Quicklinks,
    Windows,
}

pub fn collect(paths: &XdgPaths, linux_settings: &LinuxSettings, raw_query: &str) -> Vec<Item> {
    let (scope, query) = parse_scope(raw_query);
    let query_lower = query.to_ascii_lowercase();
    let mut results = Vec::new();

    if query.is_empty() {
        match scope {
            Scope::All => {
                add_home_sources(&mut results);
            }
            Scope::Folders => add_files(paths, linux_settings, "", scope, &mut results),
            Scope::Files => add_files(paths, linux_settings, "", scope, &mut results),
            Scope::Apps => add_applications(paths, "", scope, &mut results),
            Scope::Settings => add_settings(paths, "", &mut results),
            Scope::Commands => add_commands(paths, "", &mut results),
            Scope::Recent => add_recent(paths, "", &mut results),
            Scope::Browser | Scope::Bookmarks | Scope::History => {
                add_browser("", scope, &mut results)
            }
            Scope::Notes | Scope::Snippets | Scope::Quicklinks => {
                add_local_workflows(paths, "", scope, &mut results)
            }
            Scope::Images => add_content(paths, linux_settings, "", Scope::Images, &mut results),
            Scope::Clipboard => add_clipboard("", &mut results),
            Scope::Git => add_git(paths, "", &mut results),
            Scope::Content | Scope::Windows => {}
        }
        return trim(results);
    }

    if (scope == Scope::All || scope == Scope::Commands) && linux_settings.enable_system_actions {
        add_commands(paths, &query_lower, &mut results);
    }
    if scope == Scope::All || scope == Scope::Settings {
        add_settings(paths, &query_lower, &mut results);
    }
    if matches!(
        scope,
        Scope::All | Scope::Notes | Scope::Snippets | Scope::Quicklinks
    ) {
        add_local_workflows(paths, &query_lower, scope, &mut results);
    }
    if scope == Scope::All || scope == Scope::Apps {
        add_applications(paths, &query, scope, &mut results);
    }
    if linux_settings.enable_hyprland && (scope == Scope::All || scope == Scope::Windows) {
        add_windows(&query_lower, &mut results);
    }
    if scope == Scope::All || scope == Scope::Files || scope == Scope::Folders {
        add_files(paths, linux_settings, &query, scope, &mut results);
    }
    if scope == Scope::All {
        add_recent(paths, &query_lower, &mut results);
        add_browser(&query_lower, Scope::Browser, &mut results);
    }
    if calculator::evaluate(&query).is_some() && (scope == Scope::All || scope == Scope::Commands) {
        add_calculator(&query, &mut results);
    }
    if scope == Scope::Recent {
        add_recent(paths, &query_lower, &mut results);
    }
    if matches!(scope, Scope::Browser | Scope::Bookmarks | Scope::History) {
        add_browser(&query_lower, scope, &mut results);
    }
    if scope == Scope::Git {
        add_git(paths, &query_lower, &mut results);
    }
    if matches!(scope, Scope::Content | Scope::Images) {
        add_content(paths, linux_settings, &query_lower, scope, &mut results);
    }
    if scope == Scope::Clipboard {
        add_clipboard(&query_lower, &mut results);
    }

    trim(results)
}

fn add_home_sources(results: &mut Vec<Item>) {
    let sources = [
        (
            "Browser Bookmarks",
            "Browser > Bookmarks",
            "Browser",
            "bookmarks:",
        ),
        (
            "Browser History",
            "Browser > History",
            "Browser",
            "history:",
        ),
        ("Git Commits", "Git > Commits", "Git", "git:"),
        (
            "Clipboard History",
            "Clipboard > History",
            "Clipboard",
            "clipboard:",
        ),
        ("Local Files", "Local > Files", "Local", "files:"),
        ("Agents", "AI > Agents", "AI", "agents:"),
        (
            "Agent History",
            "AI > Agent History",
            "AI",
            "agent-history:",
        ),
    ];
    results.extend(
        sources
            .into_iter()
            .map(|(title, subtitle, source, query)| Item {
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                source: source.to_string(),
                kind: "SOURCE".to_string(),
                target: if matches!(query, "agents:" | "agent-history:") {
                    Target::Notice(
                        "AI providers are not configured on this Linux installation yet."
                            .to_string(),
                    )
                } else {
                    Target::Query(query.to_string())
                },
            }),
    );
}

fn parse_scope(raw_query: &str) -> (Scope, String) {
    let trimmed = raw_query.trim();
    let Some((prefix, rest)) = trimmed.split_once(':') else {
        return (Scope::All, trimmed.to_string());
    };
    let scope = match prefix.to_ascii_lowercase().as_str() {
        "app" | "apps" => Scope::Apps,
        "file" | "files" => Scope::Files,
        "folder" | "folders" => Scope::Folders,
        "setting" | "settings" => Scope::Settings,
        "command" | "commands" | "action" => Scope::Commands,
        "browser" => Scope::Browser,
        "bookmark" | "bookmarks" => Scope::Bookmarks,
        "history" => Scope::History,
        "recent" => Scope::Recent,
        "git" | "commit" | "commits" => Scope::Git,
        "content" | "code" => Scope::Content,
        "ocr" | "image" | "images" => Scope::Images,
        "clip" | "clipboard" => Scope::Clipboard,
        "note" | "notes" => Scope::Notes,
        "snippet" | "snippets" | "snip" => Scope::Snippets,
        "quicklink" | "quicklinks" | "ql" => Scope::Quicklinks,
        "window" | "windows" | "switch" => Scope::Windows,
        "agents" | "agent" | "agent-history" => Scope::Notes,
        _ => return (Scope::All, trimmed.to_string()),
    };
    (scope, rest.trim().to_string())
}

fn trim(mut results: Vec<Item>) -> Vec<Item> {
    let mut seen = HashSet::new();
    results.retain(|item| seen.insert(format!("{}\n{}", item.title, item.subtitle)));
    results.truncate(MAX_RESULTS);
    results
}

fn add_applications(paths: &XdgPaths, query: &str, scope: Scope, results: &mut Vec<Item>) {
    if scope != Scope::All && scope != Scope::Apps {
        return;
    }
    let entries = if query.trim().is_empty() {
        desktop::discover_applications(paths)
    } else {
        desktop::matching_applications(paths, query)
    };
    results.extend(entries.into_iter().take(35).map(|entry| {
        Item {
            title: entry.name.clone(),
            subtitle: entry
                .generic_name
                .clone()
                .or_else(|| entry.comment.clone())
                .unwrap_or_else(|| "Application".to_string()),
            source: "Applications".to_string(),
            kind: "APP".to_string(),
            target: Target::Application(entry),
        }
    }));
}

fn add_files(
    paths: &XdgPaths,
    linux_settings: &LinuxSettings,
    query: &str,
    scope: Scope,
    results: &mut Vec<Item>,
) {
    let options = SearchOptions {
        include_hidden: linux_settings.include_hidden,
        max_results: 55,
        max_entries: 50_000,
        max_depth: 32,
        extra_roots: linux_settings
            .search_roots
            .iter()
            .map(PathBuf::from)
            .collect(),
        ignored_names: linux_settings.ignored_names.clone(),
    };
    let kind_filter = match scope {
        Scope::Folders => Some("directory"),
        Scope::Files => Some("file"),
        _ => None,
    };
    results.extend(
        search::search_files_filtered(paths, query, &options, kind_filter)
            .into_iter()
            .map(file_item),
    );
}

fn file_item(file: FileResult) -> Item {
    let is_folder = file.kind == "directory";
    Item {
        title: file.name,
        subtitle: file.path.display().to_string(),
        source: "Local".to_string(),
        kind: if is_folder { "FOLDER" } else { "FILE" }.to_string(),
        target: Target::Path(file.path),
    }
}

fn add_calculator(query: &str, results: &mut Vec<Item>) {
    let Some(value) = calculator::evaluate(query) else {
        return;
    };
    let output = if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.8}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    };
    results.insert(
        0,
        Item {
            title: output.clone(),
            subtitle: format!("Calculator · {query} · Enter copies result"),
            source: "Calculator".to_string(),
            kind: "CALC".to_string(),
            target: Target::Copy(output),
        },
    );
}

fn add_settings(_paths: &XdgPaths, query: &str, results: &mut Vec<Item>) {
    for setting in settings::catalogue().into_iter().filter(|setting| {
        setting.name.to_ascii_lowercase().contains(query)
            || setting.description.to_ascii_lowercase().contains(query)
            || setting.id.to_ascii_lowercase().contains(query)
    }) {
        let target = if setting.action == "settings" {
            Target::Action {
                id: "open-settings".to_string(),
                args: Vec::new(),
                confirmed: false,
            }
        } else if setting.action == "apps" {
            Target::Query("app:".to_string())
        } else if setting.action == "doctor" {
            Target::Action {
                id: "doctor".to_string(),
                args: Vec::new(),
                confirmed: false,
            }
        } else {
            Target::Action {
                id: setting.action.to_string(),
                args: Vec::new(),
                confirmed: false,
            }
        };
        results.push(Item {
            title: setting.name.to_string(),
            subtitle: setting.description.to_string(),
            source: "Settings".to_string(),
            kind: "SETTING".to_string(),
            target,
        });
    }
}

fn add_commands(paths: &XdgPaths, query: &str, results: &mut Vec<Item>) {
    let commands = [
        ("Wi-Fi status", "network wifi", "wifi-status", ""),
        (
            "Open Wi-Fi settings",
            "Native network settings panel",
            "open-network-settings",
            "",
        ),
        ("Turn Wi-Fi on", "Network controls", "wifi-on", ""),
        ("Turn Wi-Fi off", "Network controls", "wifi-off", ""),
        (
            "Bluetooth status",
            "Bluetooth controls",
            "bluetooth-status",
            "",
        ),
        (
            "Open Bluetooth settings",
            "Native Bluetooth settings panel",
            "open-bluetooth-settings",
            "",
        ),
        (
            "Turn Bluetooth on",
            "Bluetooth controls",
            "bluetooth-on",
            "",
        ),
        (
            "Turn Bluetooth off",
            "Bluetooth controls",
            "bluetooth-off",
            "",
        ),
        (
            "Audio status",
            "PipeWire volume and mute",
            "audio-status",
            "",
        ),
        (
            "Open audio settings",
            "Native sound mixer or settings panel",
            "open-audio-settings",
            "",
        ),
        ("Volume up", "PipeWire audio controls", "audio-up", ""),
        ("Volume down", "PipeWire audio controls", "audio-down", ""),
        ("Mute audio", "PipeWire audio controls", "audio-mute", ""),
        (
            "Unmute audio",
            "PipeWire audio controls",
            "audio-unmute",
            "",
        ),
        ("Brightness up", "Backlight brightness", "brightness-up", ""),
        (
            "Brightness status",
            "Current backlight brightness",
            "brightness-status",
            "",
        ),
        (
            "Brightness down",
            "Backlight brightness",
            "brightness-down",
            "",
        ),
        (
            "Open display settings",
            "Native display and monitor settings panel",
            "open-display-settings",
            "",
        ),
        (
            "Open power settings",
            "Native power and battery settings panel",
            "open-power-settings",
            "",
        ),
        ("Media play/pause", "Media controls", "media-play-pause", ""),
        ("Media next track", "Media controls", "media-next", ""),
        (
            "Media previous track",
            "Media controls",
            "media-previous",
            "",
        ),
        ("Battery status", "Power and battery", "battery-status", ""),
        (
            "Power profile status",
            "Show the active power-saver, balanced, or performance profile",
            "power-profile-status",
            "",
        ),
        ("Lock screen", "Session", "power-lock", ""),
        (
            "Suspend computer",
            "Session · confirmation required",
            "power-suspend",
            "",
        ),
        ("Open Home", "Open folder", "open-folder", "home"),
        ("Open Downloads", "Open folder", "open-folder", "downloads"),
        ("Open Documents", "Open folder", "open-folder", "documents"),
        (
            "Hyprland status",
            "Read-only compositor information",
            "hyprland-status",
            "",
        ),
        (
            "Open ProtonSearch settings",
            "Linux configuration directory",
            "open-settings",
            "",
        ),
        (
            "Open keyboard settings",
            "Native keyboard settings panel",
            "open-keyboard-settings",
            "",
        ),
        (
            "Open mouse settings",
            "Native mouse and touchpad settings panel",
            "open-mouse-settings",
            "",
        ),
        (
            "Open appearance settings",
            "Native theme and appearance settings panel",
            "open-appearance-settings",
            "",
        ),
        (
            "Open notification settings",
            "Native notification settings panel",
            "open-notification-settings",
            "",
        ),
        (
            "Open privacy settings",
            "Native privacy settings panel",
            "open-privacy-settings",
            "",
        ),
        (
            "Open date and time settings",
            "Native date and time settings panel",
            "open-date-settings",
            "",
        ),
        (
            "Open user settings",
            "Native user account settings panel",
            "open-users-settings",
            "",
        ),
        (
            "Open language and region settings",
            "Native language and regional settings panel",
            "open-region-settings",
            "",
        ),
        (
            "Open software manager",
            "Installed graphical Linux software manager",
            "open-software-settings",
            "",
        ),
        (
            "Capture screenshot",
            "Select an area with slurp and save it",
            "capture-screen",
            "",
        ),
    ];
    for (title, subtitle, id, arg) in commands {
        if title.to_ascii_lowercase().contains(query)
            || subtitle.to_ascii_lowercase().contains(query)
            || id.contains(query)
        {
            let args = if arg.is_empty() {
                Vec::new()
            } else {
                vec![arg.to_string()]
            };
            results.push(Item {
                title: title.to_string(),
                subtitle: subtitle.to_string(),
                source: "Commands".to_string(),
                kind: "COMMAND".to_string(),
                target: Target::Action {
                    id: id.to_string(),
                    args,
                    confirmed: false,
                },
            });
        }
    }
    if query.contains("setting") || query.contains("config") || query.contains("preference") {
        add_settings(paths, query, results);
    }
}

fn add_local_workflows(paths: &XdgPaths, query: &str, scope: Scope, results: &mut Vec<Item>) {
    let data = paths.data_dir();
    if matches!(scope, Scope::All | Scope::Notes) {
        let notes_dir = data.join("notes");
        if let Ok(entries) = fs::read_dir(notes_dir) {
            for entry in entries.flatten().take(100) {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                let name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                let preview = fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| {
                        text.lines()
                            .find(|line| !line.trim().is_empty())
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                if name.to_ascii_lowercase().contains(query)
                    || preview.to_ascii_lowercase().contains(query)
                {
                    results.push(Item {
                        title: name.to_string(),
                        subtitle: if preview.is_empty() {
                            path.display().to_string()
                        } else {
                            preview
                        },
                        source: "Notes".to_string(),
                        kind: "NOTE".to_string(),
                        target: Target::Path(path),
                    });
                }
            }
        }
    }

    if matches!(scope, Scope::All | Scope::Snippets) {
        add_json_workflow_items(&data.join("snippets.json"), query, "snippet", results);
    }
    if matches!(scope, Scope::All | Scope::Quicklinks) {
        add_json_workflow_items(&data.join("quicklinks.json"), query, "quicklink", results);
    }
}

fn add_windows(query: &str, results: &mut Vec<Item>) {
    let Some(value) = crate::hyprland::query_json("clients") else {
        return;
    };
    let Some(clients) = value.as_array() else {
        return;
    };
    for client in clients.iter().take(100) {
        let address = client.get("address").and_then(Value::as_str).unwrap_or("");
        let title = client
            .get("title")
            .and_then(Value::as_str)
            .filter(|title| !title.trim().is_empty())
            .or_else(|| client.get("class").and_then(Value::as_str))
            .unwrap_or("");
        let class = client.get("class").and_then(Value::as_str).unwrap_or("");
        if address.is_empty()
            || (!query.is_empty()
                && !title.to_ascii_lowercase().contains(query)
                && !class.to_ascii_lowercase().contains(query))
        {
            continue;
        }
        results.push(Item {
            title: title.to_string(),
            subtitle: format!("{} · Hyprland window", class),
            source: "Windows".to_string(),
            kind: "WINDOW".to_string(),
            target: Target::Action {
                id: "focus-window".to_string(),
                args: vec![address.to_string()],
                confirmed: true,
            },
        });
    }
}

fn add_json_workflow_items(path: &Path, query: &str, kind: &str, results: &mut Vec<Item>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&contents) else {
        return;
    };
    let values = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    for item in values.into_iter().take(100) {
        let Some(object) = item.as_object() else {
            continue;
        };
        let name = object
            .get("name")
            .or_else(|| object.get("title"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let keyword = object
            .get("keyword")
            .or_else(|| object.get("trigger"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let content = object
            .get(if kind == "snippet" { "content" } else { "url" })
            .and_then(Value::as_str)
            .unwrap_or("");
        if name.is_empty()
            || (!query.is_empty()
                && !name.to_ascii_lowercase().contains(query)
                && !keyword.to_ascii_lowercase().contains(query)
                && !content.to_ascii_lowercase().contains(query))
        {
            continue;
        }
        let (subtitle, target) = if kind == "snippet" {
            (
                format!("{keyword} · Enter copies snippet"),
                Target::Copy(content.to_string()),
            )
        } else {
            let url = content.replace("{query}", query);
            (format!("{keyword} · {content}"), Target::Url(url))
        };
        results.push(Item {
            title: name.to_string(),
            subtitle,
            source: if kind == "snippet" {
                "Snippets"
            } else {
                "Quicklinks"
            }
            .to_string(),
            kind: kind.to_ascii_uppercase(),
            target,
        });
    }
}

fn add_recent(paths: &XdgPaths, query: &str, results: &mut Vec<Item>) {
    let path = paths.data.join("recently-used.xbel");
    let Ok(contents) = fs::read_to_string(&path) else {
        return;
    };
    for entry in contents.match_indices("href=\"").take(100) {
        let start = entry.0 + entry.1.len();
        let Some(end) = contents[start..].find('"') else {
            continue;
        };
        let href = &contents[start..start + end];
        let Some(path) = file_uri_to_path(href) else {
            continue;
        };
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if query.is_empty() || name.to_ascii_lowercase().contains(query) || href.contains(query) {
            results.push(Item {
                title: name.to_string(),
                subtitle: path.display().to_string(),
                source: "Recent".to_string(),
                kind: "RECENT".to_string(),
                target: Target::Path(path),
            });
        }
    }
}

fn file_uri_to_path(href: &str) -> Option<PathBuf> {
    let value = href.strip_prefix("file://")?;
    let decoded = percent_decode(value);
    Some(PathBuf::from(decoded)).filter(|path| path.is_absolute() && path.exists())
}

fn percent_decode(value: &str) -> String {
    let mut output = String::new();
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                output.push(byte as char);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index] as char);
        index += 1;
    }
    output
}

fn add_browser(query: &str, scope: Scope, results: &mut Vec<Item>) {
    let include_bookmarks = matches!(scope, Scope::Browser | Scope::Bookmarks);
    let include_history = matches!(scope, Scope::Browser | Scope::History);
    for (browser, path) in browser_files() {
        if include_bookmarks && path.file_name().is_some_and(|name| name == "Bookmarks") {
            add_bookmarks(&browser, &path, query, results);
        }
        if include_bookmarks
            && browser == "Firefox"
            && path.file_name().is_some_and(|name| name == "places.sqlite")
        {
            add_firefox_bookmarks(&path, query, results);
        }
        if include_history
            && path.file_name().is_some_and(|name| {
                name == "History" || (browser == "Firefox" && name == "places.sqlite")
            })
        {
            add_history(&browser, &path, query, results);
        }
    }
}

fn browser_files() -> Vec<(String, PathBuf)> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let configs = [
        ("Chrome", ".config/google-chrome/Default/Bookmarks"),
        ("Chromium", ".config/chromium/Default/Bookmarks"),
        (
            "Brave",
            ".config/BraveSoftware/Brave-Browser/Default/Bookmarks",
        ),
        ("Vivaldi", ".config/vivaldi/Default/Bookmarks"),
        ("Chrome", ".config/google-chrome/Default/History"),
        ("Chromium", ".config/chromium/Default/History"),
        (
            "Brave",
            ".config/BraveSoftware/Brave-Browser/Default/History",
        ),
        ("Firefox", ".mozilla/firefox/places.sqlite"),
    ];
    let mut result = configs
        .into_iter()
        .map(|(browser, suffix)| (browser.to_string(), home.join(suffix)))
        .filter(|(_, path)| path.exists())
        .collect::<Vec<_>>();
    let firefox_root = home.join(".mozilla/firefox");
    if let Ok(entries) = fs::read_dir(firefox_root) {
        for entry in entries.flatten() {
            let path = entry.path().join("places.sqlite");
            if path.exists() {
                result.push(("Firefox".to_string(), path));
            }
        }
    }
    result
}

fn add_bookmarks(browser: &str, path: &Path, query: &str, results: &mut Vec<Item>) {
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<Value>(&contents) else {
        return;
    };
    let mut bookmarks = Vec::new();
    collect_bookmarks(&json, &mut bookmarks, 0);
    for (title, url) in bookmarks.into_iter().take(200) {
        if title.to_ascii_lowercase().contains(query) || url.to_ascii_lowercase().contains(query) {
            results.push(Item {
                title,
                subtitle: url.clone(),
                source: format!("{browser} bookmarks"),
                kind: "BOOKMARK".to_string(),
                target: Target::Url(url),
            });
        }
    }
}

fn collect_bookmarks(value: &Value, output: &mut Vec<(String, String)>, depth: usize) {
    if depth > 32 {
        return;
    }
    let Some(object) = value.as_object() else {
        return;
    };
    if object.get("type").and_then(Value::as_str) == Some("url") {
        if let (Some(title), Some(url)) = (
            object.get("name").and_then(Value::as_str),
            object.get("url").and_then(Value::as_str),
        ) {
            output.push((title.to_string(), url.to_string()));
        }
    }
    if let Some(children) = object.get("children").and_then(Value::as_array) {
        for child in children {
            collect_bookmarks(child, output, depth + 1);
        }
    }
    for child in object.values() {
        if child.is_object() {
            collect_bookmarks(child, output, depth + 1);
        }
    }
}

fn add_history(browser: &str, path: &Path, query: &str, results: &mut Vec<Item>) {
    if !system::command_available("sqlite3") {
        return;
    }
    let temporary = std::env::temp_dir().join(format!(
        "protonsearch-history-{}-{}.sqlite",
        std::process::id(),
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    if fs::copy(path, &temporary).is_err() {
        return;
    }
    let temporary = temporary.to_string_lossy().into_owned();
    let output = if browser == "Firefox" {
        system::run(
            "sqlite3",
            &[
                "-readonly",
                "-separator",
                "\t",
                temporary.as_str(),
                "SELECT coalesce(title,''), url FROM moz_places WHERE url LIKE 'http%' ORDER BY last_visit_date DESC LIMIT 200;",
            ],
        )
    } else {
        system::run(
            "sqlite3",
            &[
                "-readonly",
                "-separator",
                "\t",
                temporary.as_str(),
                "SELECT coalesce(title,''), url FROM urls WHERE url LIKE 'http%' ORDER BY last_visit_time DESC LIMIT 200;",
            ],
        )
    };
    let _ = fs::remove_file(&temporary);
    let Ok(output) = output else { return };
    if output.timed_out || output.status != Some(0) {
        return;
    }
    for line in output.stdout.lines() {
        let Some((title, url)) = line.split_once('\t') else {
            continue;
        };
        if title.to_ascii_lowercase().contains(query) || url.to_ascii_lowercase().contains(query) {
            results.push(Item {
                title: if title.trim().is_empty() {
                    url.to_string()
                } else {
                    title.to_string()
                },
                subtitle: url.to_string(),
                source: format!("{browser} history"),
                kind: "HISTORY".to_string(),
                target: Target::Url(url.to_string()),
            });
        }
    }
}

fn add_firefox_bookmarks(path: &Path, query: &str, results: &mut Vec<Item>) {
    if !system::command_available("sqlite3") {
        return;
    }
    let temporary = std::env::temp_dir().join(format!(
        "protonsearch-bookmarks-{}-{}.sqlite",
        std::process::id(),
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    if fs::copy(path, &temporary).is_err() {
        return;
    }
    let temporary = temporary.to_string_lossy().into_owned();
    let output = system::run(
        "sqlite3",
        &[
            "-readonly",
            "-separator",
            "\t",
            temporary.as_str(),
            "SELECT coalesce(b.title,''), p.url FROM moz_bookmarks b JOIN moz_places p ON p.id=b.fk WHERE b.type=1 AND p.url LIKE 'http%' LIMIT 300;",
        ],
    );
    let _ = fs::remove_file(&temporary);
    let Ok(output) = output else { return };
    if output.timed_out || output.status != Some(0) {
        return;
    }
    for line in output.stdout.lines() {
        let Some((title, url)) = line.split_once('\t') else {
            continue;
        };
        if title.to_ascii_lowercase().contains(query) || url.to_ascii_lowercase().contains(query) {
            results.push(Item {
                title: if title.trim().is_empty() {
                    url.to_string()
                } else {
                    title.to_string()
                },
                subtitle: url.to_string(),
                source: "Firefox bookmarks".to_string(),
                kind: "BOOKMARK".to_string(),
                target: Target::Url(url.to_string()),
            });
        }
    }
}

fn add_git(paths: &XdgPaths, query: &str, results: &mut Vec<Item>) {
    if !system::command_available("git") {
        return;
    }
    for repo in find_repositories(paths) {
        let name = repo
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if !query.is_empty() && name.to_ascii_lowercase().contains(query) {
            results.push(Item {
                title: name.to_string(),
                subtitle: repo.display().to_string(),
                source: "Git".to_string(),
                kind: "REPO".to_string(),
                target: Target::Path(repo.clone()),
            });
        }
        let repo_path = repo.to_string_lossy().into_owned();
        let Ok(output) = system::run(
            "git",
            &[
                "-C",
                repo_path.as_str(),
                "log",
                "-n",
                "40",
                "--date=short",
                "--pretty=format:%h\t%ad\t%s",
            ],
        ) else {
            continue;
        };
        if output.timed_out || output.status != Some(0) {
            continue;
        }
        for line in output.stdout.lines() {
            let Some((hash, rest)) = line.split_once('\t') else {
                continue;
            };
            let Some((date, message)) = rest.split_once('\t') else {
                continue;
            };
            if message.to_ascii_lowercase().contains(query)
                || hash.to_ascii_lowercase().contains(query)
            {
                results.push(Item {
                    title: message.to_string(),
                    subtitle: format!("{repo:?} · {hash} · {date}"),
                    source: "Git".to_string(),
                    kind: "COMMIT".to_string(),
                    target: Target::Path(repo.clone()),
                });
            }
        }
    }
}

fn find_repositories(paths: &XdgPaths) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut roots = Vec::new();
    for mount_root in [
        Path::new("/mnt"),
        Path::new("/media"),
        Path::new("/run/media"),
    ] {
        roots.extend(mounted_user_roots(mount_root));
    }
    roots.extend(paths.search_roots(&[]));
    if let Some(extra) = std::env::var_os("PROTONSEARCH_GIT_ROOTS") {
        roots.extend(std::env::split_paths(&extra).filter(|path| path.is_absolute()));
    }
    let mut seen_roots = HashSet::new();
    let mut visited = 0;
    for root in roots {
        if seen_roots.insert(root.clone()) {
            find_repositories_inner(&root, &mut found, 0, &mut visited);
        }
        if found.len() >= MAX_GIT_REPOSITORIES {
            break;
        }
    }
    found
}

fn mounted_user_roots(mount_root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let Ok(mounts) = fs::read_dir(mount_root) else {
        return roots;
    };
    for mount in mounts.flatten().take(12) {
        let mount = mount.path();
        if !mount.is_dir() || mount.is_symlink() {
            continue;
        }
        for users_dir in [mount.join("Users"), mount.join("home")] {
            let Ok(users) = fs::read_dir(&users_dir) else {
                continue;
            };
            for user in users
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir() && !path.is_symlink())
                .take(24)
            {
                for relative in [
                    Path::new("Documents/Projects"),
                    Path::new("Documents"),
                    Path::new("Projects"),
                    Path::new("Code"),
                    Path::new("src"),
                ] {
                    let candidate = user.join(relative);
                    if candidate.is_dir() {
                        roots.push(candidate);
                    }
                }
                roots.push(user);
            }
        }
    }
    roots
}

fn find_repositories_inner(
    path: &Path,
    found: &mut Vec<PathBuf>,
    depth: usize,
    visited: &mut usize,
) {
    if depth > 8 || found.len() >= MAX_GIT_REPOSITORIES || *visited >= MAX_GIT_SCAN_ENTRIES {
        return;
    }
    *visited += 1;
    if path.join(".git").exists() {
        found.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    let mut entries = entries.flatten().collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.contains("project") {
            0
        } else if name.contains("code") || name == "src" || name == "backend" {
            1
        } else {
            2
        }
    });
    for entry in entries {
        let child = entry.path();
        if !child.is_dir() || child.is_symlink() {
            continue;
        }
        let name = child
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if matches!(
            name,
            ".cache" | ".config" | ".local" | "node_modules" | "target" | "build" | "dist"
        ) {
            continue;
        }
        find_repositories_inner(&child, found, depth + 1, visited);
    }
}

fn add_content(
    paths: &XdgPaths,
    linux_settings: &LinuxSettings,
    query: &str,
    scope: Scope,
    results: &mut Vec<Item>,
) {
    let options = SearchOptions {
        include_hidden: linux_settings.include_hidden,
        max_results: if scope == Scope::Images { 100 } else { 30 },
        max_entries: 12_000,
        max_depth: 20,
        extra_roots: linux_settings
            .search_roots
            .iter()
            .map(PathBuf::from)
            .collect(),
        ignored_names: linux_settings.ignored_names.clone(),
    };
    let mut image_paths = HashSet::new();
    if scope == Scope::Images {
        // Filename matches remain useful even when the optional OCR provider
        // is not installed. An empty query enumerates image folders and XDG
        // user directories so opening Images is useful immediately.
        for file in search::search_images(paths, query, &options) {
            image_paths.insert(file.path.clone());
            results.push(Item {
                title: file.name,
                subtitle: format!("image · {}", file.path.display()),
                source: "Images".to_string(),
                kind: "IMAGE".to_string(),
                target: Target::Path(file.path),
            });
        }
    }
    for file in search::search_content(paths, query, &options) {
        let is_image = matches!(file.kind.as_str(), "image" | "ocr");
        if scope == Scope::Images && !is_image {
            continue;
        }
        if scope == Scope::Images && !image_paths.insert(file.path.clone()) {
            continue;
        }
        results.push(Item {
            title: file.name,
            subtitle: format!("{} · {}", file.kind, file.path.display()),
            source: if is_image { "OCR" } else { "Content" }.to_string(),
            kind: if is_image { "OCR" } else { "CONTENT" }.to_string(),
            target: Target::Path(file.path),
        });
    }
    if scope == Scope::Images && !system::command_available("tesseract") {
        results.push(Item {
            title: "OCR provider unavailable".to_string(),
            subtitle: "Install tesseract to search text inside images".to_string(),
            source: "OCR".to_string(),
            kind: "INFO".to_string(),
            target: Target::Notice(
                "Image filename search works now; OCR needs the tesseract package.".to_string(),
            ),
        });
    }
}

fn add_clipboard(query: &str, results: &mut Vec<Item>) {
    if system::command_available("cliphist") {
        let Ok(output) = system::run("cliphist", &["list"]) else {
            return;
        };
        for line in output.stdout.lines().take(100) {
            let Some((id, preview)) = line.split_once('\t') else {
                continue;
            };
            if preview.to_ascii_lowercase().contains(query) {
                let is_image = clipboard_preview_is_image(preview);
                results.push(Item {
                    title: if is_image {
                        "Clipboard image".to_string()
                    } else {
                        preview.to_string()
                    },
                    subtitle: if is_image {
                        format!("Clipboard image #{id} · Enter copies")
                    } else {
                        format!("Clipboard history #{id} · Enter copies")
                    },
                    source: "Clipboard".to_string(),
                    kind: if is_image { "IMAGE" } else { "CLIP" }.to_string(),
                    target: Target::Cliphist(line.to_string()),
                });
            }
        }
        return;
    }
    let Ok(text) = read_clipboard() else { return };
    if text.trim().is_empty() || !text.to_ascii_lowercase().contains(query) {
        return;
    }
    results.push(Item {
        title: text
            .lines()
            .next()
            .unwrap_or("Clipboard")
            .chars()
            .take(80)
            .collect(),
        subtitle: "Current Wayland clipboard · Enter copies it again".to_string(),
        source: "Clipboard".to_string(),
        kind: "CLIP".to_string(),
        target: Target::Copy(text),
    });
}

fn clipboard_preview_is_image(preview: &str) -> bool {
    let preview = preview.to_ascii_lowercase();
    preview.contains("binary data")
        && [
            "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "image/",
        ]
        .iter()
        .any(|format| preview.contains(format))
}

fn clipboard_image_mime(preview: &str) -> &'static str {
    let preview = preview.to_ascii_lowercase();
    if preview.contains("jpg") || preview.contains("jpeg") {
        "image/jpeg"
    } else if preview.contains("gif") {
        "image/gif"
    } else if preview.contains("webp") {
        "image/webp"
    } else if preview.contains("bmp") {
        "image/bmp"
    } else {
        "image/png"
    }
}

fn read_clipboard() -> anyhow::Result<String> {
    if system::command_available("wl-paste") {
        let output = system::run("wl-paste", &["--no-newline"])?;
        if output.status == Some(0) {
            return Ok(output.stdout);
        }
    }
    anyhow::bail!("no clipboard provider available")
}

pub fn activate(paths: &XdgPaths, target: &Target) -> anyhow::Result<Option<String>> {
    match target {
        Target::Application(entry) => {
            desktop::launch(entry)?;
            Ok(None)
        }
        Target::Path(path) => {
            system::open_target(&path.to_string_lossy())?;
            Ok(None)
        }
        Target::Url(url) => {
            system::open_target(url)?;
            Ok(None)
        }
        Target::Copy(text) => {
            copy_clipboard(text)?;
            Ok(None)
        }
        Target::Cliphist(line) => {
            if clipboard_preview_is_image(line) {
                let bytes = decode_cliphist_image(line)?;
                copy_clipboard_bytes(&bytes, clipboard_image_mime(line))?;
                return Ok(None);
            }
            copy_clipboard(&decode_cliphist_text(line)?)?;
            Ok(None)
        }
        Target::Query(_) | Target::Notice(_) => {
            // These targets are handled by the GTK layer because they do not
            // represent an external process to launch.
            Ok(None)
        }
        Target::Action {
            id,
            args,
            confirmed,
        } => {
            if matches!(
                id.as_str(),
                "power-suspend" | "power-reboot" | "poweroff" | "power-logout"
            ) && !confirmed
            {
                anyhow::bail!("confirmation required before running {id}");
            }
            match id.as_str() {
                "focus-window" => {
                    let address = args
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("window address missing"))?;
                    crate::hyprland::focus_window(address)?;
                    Ok(None)
                }
                "open-settings" => {
                    let executable = std::env::current_exe()?;
                    system::spawn_detached(&executable, &["settings-ui"])?;
                    Ok(None)
                }
                "capture-screen" => {
                    capture_screen(paths)?;
                    Ok(None)
                }
                _ => {
                    let result = actions::execute(paths, id, args, *confirmed)?;
                    if matches!(
                        id.as_str(),
                        "wifi-status"
                            | "bluetooth-status"
                            | "audio-status"
                            | "brightness-status"
                            | "battery-status"
                            | "power-profile-status"
                            | "media-status"
                            | "hyprland-status"
                            | "doctor"
                    ) {
                        Ok(Some(result.message))
                    } else {
                        Ok(None)
                    }
                }
            }
        }
    }
}

/// Copy several selected clipboard rows as one text payload. Images cannot be
/// represented together on the single Wayland clipboard, so the last selected
/// image is restored after any selected text and the caller receives counts for
/// user feedback.
pub fn activate_clipboard_batch(items: &[Item]) -> anyhow::Result<(usize, usize)> {
    let mut text_items = Vec::new();
    let mut image_lines = Vec::new();
    for item in items {
        if item.source != "Clipboard" {
            continue;
        }
        match &item.target {
            Target::Copy(text) => text_items.push(text.clone()),
            Target::Cliphist(line) if clipboard_preview_is_image(line) => {
                image_lines.push(line.clone());
            }
            Target::Cliphist(line) => text_items.push(decode_cliphist_text(line)?),
            _ => {}
        }
    }
    if !text_items.is_empty() {
        copy_clipboard(&text_items.join("\n"))?;
    }
    let image_count = image_lines.len();
    if image_count == 1 {
        let line = &image_lines[0];
        let bytes = decode_cliphist_image(line)?;
        copy_clipboard_bytes(&bytes, clipboard_image_mime(line))?;
    } else if image_count > 1 {
        let sheet = compose_clipboard_images(&image_lines)?;
        copy_clipboard_bytes(&sheet, "image/png")?;
    }
    Ok((text_items.len(), image_count))
}

fn compose_clipboard_images(lines: &[String]) -> anyhow::Result<Vec<u8>> {
    const MAX_IMAGES: usize = 16;
    const COLUMNS: i32 = 2;
    const CELL: i32 = 480;
    const GAP: i32 = 12;
    let lines = &lines[..lines.len().min(MAX_IMAGES)];
    let mut images = Vec::new();
    for line in lines {
        let bytes = decode_cliphist_image(line)?;
        let image = Pixbuf::from_read(Cursor::new(bytes))?;
        let scale = (CELL as f64 / image.width() as f64)
            .min(CELL as f64 / image.height() as f64)
            .min(1.0);
        let width = (image.width() as f64 * scale).round().max(1.0) as i32;
        let height = (image.height() as f64 * scale).round().max(1.0) as i32;
        images.push(
            image
                .scale_simple(width, height, InterpType::Bilinear)
                .ok_or_else(|| anyhow::anyhow!("could not scale clipboard image"))?,
        );
    }
    let rows = (images.len() as i32 + COLUMNS - 1) / COLUMNS;
    let width = COLUMNS * CELL + (COLUMNS + 1) * GAP;
    let height = rows * CELL + (rows + 1) * GAP;
    let sheet = Pixbuf::new(Colorspace::Rgb, true, 8, width, height)
        .ok_or_else(|| anyhow::anyhow!("could not allocate clipboard image sheet"))?;
    sheet.fill(0x202124ff);
    for (index, image) in images.iter().enumerate() {
        let index = index as i32;
        let column = index % COLUMNS;
        let row = index / COLUMNS;
        let x = GAP + column * (CELL + GAP) + (CELL - image.width()) / 2;
        let y = GAP + row * (CELL + GAP) + (CELL - image.height()) / 2;
        image.copy_area(0, 0, image.width(), image.height(), &sheet, x, y);
    }
    Ok(sheet.save_to_bufferv("png", &[])?)
}

fn decode_cliphist_text(line: &str) -> anyhow::Result<String> {
    let input = format!("{line}\n");
    let output = system::run_with_input("cliphist", &["decode"], &input)?;
    if output.timed_out || output.status != Some(0) {
        anyhow::bail!("cliphist could not decode the clipboard item");
    }
    Ok(output.stdout)
}

fn capture_screen(paths: &XdgPaths) -> anyhow::Result<()> {
    if !system::command_available("grim") {
        anyhow::bail!("grim is unavailable; install grim");
    }
    let directory = paths.data_dir().join("screenshots");
    fs::create_dir_all(&directory)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let output_path = directory.join(format!("screenshot-{timestamp}.png"));
    let geometry = if system::command_available("slurp") {
        system::run("slurp", &[])
            .ok()
            .filter(|output| output.status == Some(0) && !output.timed_out)
            .map(|output| output.stdout)
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    let output_path_string = output_path.to_string_lossy().into_owned();
    let mut args = Vec::<String>::new();
    if let Some(geometry) = geometry {
        args.push("-g".to_string());
        args.push(geometry);
    }
    args.push(output_path_string.clone());
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let result = system::run("grim", &args)?;
    if result.timed_out || result.status != Some(0) {
        anyhow::bail!("grim could not capture the screen");
    }
    if system::command_available("wl-copy") {
        let image = fs::read(&output_path)?;
        copy_clipboard_bytes(&image, "image/png")?;
    }
    system::open_target(&output_path_string)?;
    Ok(())
}

fn copy_clipboard(text: &str) -> anyhow::Result<()> {
    if system::command_available("wl-copy") {
        let output = system::run_with_input("wl-copy", &[], text)?;
        if output.timed_out || output.status != Some(0) {
            anyhow::bail!("wl-copy could not update the clipboard");
        }
        return Ok(());
    }
    anyhow::bail!("wl-copy is unavailable; install wl-clipboard")
}

fn decode_cliphist_image(line: &str) -> anyhow::Result<Vec<u8>> {
    let mut child = Command::new("cliphist")
        .args(["decode"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(format!("{line}\n").as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        anyhow::bail!("cliphist could not decode the clipboard image");
    }
    if output.stdout.len() > 16 * 1024 * 1024 {
        anyhow::bail!("clipboard image is too large");
    }
    Ok(output.stdout)
}

fn copy_clipboard_bytes(bytes: &[u8], mime_type: &str) -> anyhow::Result<()> {
    if !system::command_available("wl-copy") {
        anyhow::bail!("wl-copy is unavailable; install wl-clipboard");
    }
    let mut child = Command::new("wl-copy")
        .args(["--type", mime_type])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(bytes)?;
    }
    if !child.wait()?.success() {
        anyhow::bail!("wl-copy could not update the image clipboard");
    }
    Ok(())
}
