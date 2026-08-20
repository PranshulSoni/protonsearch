use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use crate::xdg::XdgPaths;

#[derive(Debug, Clone, Serialize)]
pub struct DesktopEntry {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
    pub keywords: Vec<String>,
}

pub fn discover_applications(paths: &XdgPaths) -> Vec<DesktopEntry> {
    discover_applications_filtered(paths, true)
}

pub fn discover_applications_filtered(
    paths: &XdgPaths,
    show_terminal_apps: bool,
) -> Vec<DesktopEntry> {
    let current_desktop = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let mut entries = HashMap::<String, DesktopEntry>::new();
    for directory in paths.application_dirs() {
        let Ok(read_dir) = fs::read_dir(&directory) else {
            continue;
        };
        let mut files = read_dir
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "desktop"))
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let Some(id) = file.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if entries.contains_key(id) {
                continue;
            }
            if let Ok(entry) = parse_desktop_entry(&file, &current_desktop) {
                entries.insert(id.to_string(), entry);
            }
        }
    }
    let mut entries = entries.into_values().collect::<Vec<_>>();
    filter_terminal_entries(&mut entries, show_terminal_apps);
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    entries
}

pub fn parse_desktop_entry(path: &Path, current_desktop: &str) -> Result<DesktopEntry> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut in_entry = false;
    let mut values = HashMap::<String, String>::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key.trim().to_string(), unescape_value(value.trim()));
    }

    if values.get("Type").map(String::as_str) != Some("Application") {
        bail!("not an application desktop entry")
    }
    if values.get("Hidden").is_some_and(|value| value == "true")
        || values.get("NoDisplay").is_some_and(|value| value == "true")
    {
        bail!("desktop entry is hidden")
    }
    let current = current_desktop
        .split(';')
        .filter(|part| !part.is_empty())
        .collect::<HashSet<_>>();
    if !desktop_visibility_allows(&values, &current) {
        bail!("desktop entry is not visible in this desktop")
    }
    let exec = values
        .get("Exec")
        .filter(|exec| !exec.trim().is_empty())
        .cloned()
        .context("desktop entry has no Exec")?;
    if let Some(try_exec) = values.get("TryExec") {
        if !executable_available(try_exec) {
            bail!("TryExec is unavailable")
        }
    }
    let name = localized_value(&values, "Name")
        .filter(|value| !value.trim().is_empty())
        .context("desktop entry has no Name")?;
    let id = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("desktop entry has invalid filename")?
        .to_string();
    Ok(DesktopEntry {
        id,
        path: path.to_path_buf(),
        name,
        generic_name: localized_value(&values, "GenericName"),
        comment: localized_value(&values, "Comment"),
        exec,
        icon: values.get("Icon").cloned(),
        terminal: values.get("Terminal").is_some_and(|value| value == "true"),
        keywords: values
            .get("Keywords")
            .map(|value| {
                value
                    .split(';')
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn desktop_visibility_allows(values: &HashMap<String, String>, current: &HashSet<&str>) -> bool {
    if let Some(only) = values.get("OnlyShowIn") {
        let allowed = only
            .split(';')
            .filter(|part| !part.is_empty())
            .collect::<HashSet<_>>();
        if !allowed.is_empty() && !current.iter().any(|desktop| allowed.contains(*desktop)) {
            return false;
        }
    }
    if let Some(not) = values.get("NotShowIn") {
        if not
            .split(';')
            .any(|desktop| !desktop.is_empty() && current.contains(desktop))
        {
            return false;
        }
    }
    true
}

fn localized_value(values: &HashMap<String, String>, key: &str) -> Option<String> {
    let lang = env::var("LANG").unwrap_or_default();
    let lang = lang.split('.').next().unwrap_or(&lang);
    values
        .get(&format!("{key}[{lang}]"))
        .cloned()
        .or_else(|| values.get(key).cloned())
}

fn unescape_value(value: &str) -> String {
    let mut result = String::new();
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            result.push(match ch {
                's' => ' ',
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                _ => ch,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            result.push(ch);
        }
    }
    if escaped {
        result.push('\\');
    }
    result
}

fn executable_available(executable: &str) -> bool {
    let path = Path::new(executable);
    if path.is_absolute() {
        return path.is_file();
    }
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(executable))
        .any(|candidate| candidate.is_file())
}

pub fn matching_applications(paths: &XdgPaths, query: &str) -> Vec<DesktopEntry> {
    matching_applications_filtered(paths, query, true)
}

pub fn matching_applications_filtered(
    paths: &XdgPaths,
    query: &str,
    show_terminal_apps: bool,
) -> Vec<DesktopEntry> {
    let query = query.to_lowercase();
    discover_applications_filtered(paths, show_terminal_apps)
        .into_iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&query)
                || entry
                    .generic_name
                    .as_deref()
                    .is_some_and(|name| name.to_lowercase().contains(&query))
                || entry
                    .keywords
                    .iter()
                    .any(|keyword| keyword.to_lowercase().contains(&query))
        })
        .collect()
}

fn filter_terminal_entries(entries: &mut Vec<DesktopEntry>, show_terminal_apps: bool) {
    if !show_terminal_apps {
        entries.retain(|entry| !entry.terminal);
    }
}

pub fn launch(entry: &DesktopEntry) -> Result<Child> {
    let mut argv = expand_exec(&entry.exec, entry, &[])?;
    let program = argv
        .first()
        .cloned()
        .context("desktop Exec has no executable")?;
    argv.remove(0);
    let mut command = if entry.terminal {
        let terminal = [
            "x-terminal-emulator",
            "foot",
            "kitty",
            "alacritty",
            "wezterm",
            "gnome-terminal",
            "konsole",
        ]
        .into_iter()
        .find(|executable: &&str| executable_available(executable))
        .context("Terminal=true but no supported terminal is installed")?;
        let mut command = Command::new(terminal);
        command.arg("-e").arg(&program).args(argv);
        command
    } else {
        let mut command = Command::new(&program);
        command.args(argv);
        command
    };
    if let Some(path) = entry.path.parent().filter(|path| path.is_dir()) {
        command.current_dir(path);
    }
    command
        .spawn()
        .with_context(|| format!("launch desktop entry {}", entry.name))
}

fn expand_exec(exec: &str, entry: &DesktopEntry, files: &[PathBuf]) -> Result<Vec<String>> {
    let tokens = tokenize_exec(exec)?;
    let mut result = Vec::new();
    for token in tokens {
        let mut expanded = String::new();
        let mut chars = token.chars();
        while let Some(ch) = chars.next() {
            if ch != '%' {
                expanded.push(ch);
                continue;
            }
            let code = chars.next().context("desktop Exec ends after %")?;
            match code {
                '%' => expanded.push('%'),
                'c' => expanded.push_str(&entry.name),
                'k' => expanded.push_str(&entry.path.to_string_lossy()),
                'i' => {}
                'f' | 'u' => {
                    if let Some(file) = files.first() {
                        expanded.push_str(&file.to_string_lossy());
                    }
                }
                'F' | 'U' => {
                    if let Some(file) = files.first() {
                        expanded.push_str(&file.to_string_lossy());
                    }
                }
                _ => bail!("unsupported desktop Exec field code %{code}"),
            }
        }
        if !expanded.is_empty() {
            result.push(expanded);
        }
    }
    Ok(result)
}

fn tokenize_exec(value: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            token.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                token.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
        } else if ch.is_whitespace() {
            if !token.is_empty() {
                result.push(std::mem::take(&mut token));
            }
        } else {
            token.push(ch);
        }
    }
    if escaped {
        token.push('\\');
    }
    if quote.is_some() {
        bail!("unterminated quote in desktop Exec")
    }
    if !token.is_empty() {
        result.push(token);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn entry(terminal: bool) -> DesktopEntry {
        DesktopEntry {
            id: "test.desktop".to_string(),
            path: PathBuf::from("/tmp/test.desktop"),
            name: "Test".to_string(),
            generic_name: None,
            comment: None,
            exec: "test".to_string(),
            icon: None,
            terminal,
            keywords: Vec::new(),
        }
    }

    #[test]
    fn parses_desktop_entry_without_shell_execution() {
        let path =
            std::env::temp_dir().join(format!("protonsearch-test-{}.desktop", std::process::id()));
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "[Desktop Entry]\nType=Application\nName=Safe App\nExec=demo --title %c\nKeywords=demo;safe;").unwrap();
        let entry = parse_desktop_entry(&path, "Hyprland").unwrap();
        assert_eq!(entry.name, "Safe App");
        assert_eq!(entry.keywords, vec!["demo", "safe"]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_hidden_entries() {
        let path = std::env::temp_dir().join(format!(
            "protonsearch-hidden-{}.desktop",
            std::process::id()
        ));
        fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Hidden\nHidden=true\nExec=demo",
        )
        .unwrap();
        assert!(parse_desktop_entry(&path, "").is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tokenizer_keeps_arguments_separate() {
        assert_eq!(
            tokenize_exec("demo --title 'hello world'").unwrap(),
            vec!["demo", "--title", "hello world"]
        );
    }

    #[test]
    fn terminal_entries_are_filtered_without_mutating_launch_metadata() {
        let mut entries = vec![entry(true), entry(false)];
        filter_terminal_entries(&mut entries, false);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].terminal);

        let mut entries = vec![entry(true), entry(false)];
        filter_terminal_entries(&mut entries, true);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].terminal);
    }
}
