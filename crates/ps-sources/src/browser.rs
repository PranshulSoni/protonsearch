//! Browser history + bookmarks (Chromium: Chrome/Edge/Brave/Vivaldi/Opera,
//! Gecko: Firefox). OWNER: sources-content.
//!
//! Safety rules:
//! - Live browser databases are never opened: the file (plus -wal/-shm) is
//!   copied to our cache first and only the copy is read.
//! - Only History/Bookmarks/places.sqlite are read. Cookie, Login Data,
//!   Web Data and password tables are never touched.
//! - Fully local; no network access.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::BrowserEntry;

const MAX_PER_PROFILE: usize = 2000;
const MAX_TOTAL: usize = 8000;

pub fn scan() -> (Vec<BrowserEntry>, Vec<String>) {
    scan_with_cache(&crate::default_cache_dir())
}

/// Scan all browsers, storing temporary database snapshots under cache_root.
/// An injected cache root keeps tests (and sandboxed callers) off %APPDATA%.
pub fn scan_with_cache(cache_root: &Path) -> (Vec<BrowserEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for (source, root) in chromium_roots() {
        if !root.is_dir() {
            continue;
        }
        for profile in chromium_profiles(&root) {
            let (mut found, mut profile_warnings) =
                scan_chromium_profile_with_cache(&profile, source, cache_root);
            entries.append(&mut found);
            warnings.append(&mut profile_warnings);
            if entries.len() >= MAX_TOTAL {
                break;
            }
        }
    }

    if let Some(profiles_root) = firefox_profiles_root() {
        for profile in firefox_profiles(&profiles_root) {
            let (mut found, mut profile_warnings) =
                scan_firefox_profile_with_cache(&profile, cache_root);
            entries.append(&mut found);
            warnings.append(&mut profile_warnings);
            if entries.len() >= MAX_TOTAL {
                break;
            }
        }
    }

    merge_entries(entries, warnings)
}

/// Privacy-aware entry point: respects settings.privacy.browser_history.
pub fn scan_configured(privacy: &ps_ipc::PrivacySettings) -> (Vec<BrowserEntry>, Vec<String>) {
    if !privacy.browser_history {
        return (Vec::new(), Vec::new());
    }
    scan()
}

fn merge_entries(
    entries: Vec<BrowserEntry>,
    warnings: Vec<String>,
) -> (Vec<BrowserEntry>, Vec<String>) {
    let mut merged: HashMap<String, BrowserEntry> = HashMap::new();
    for entry in entries {
        match merged.get_mut(&entry.url) {
            Some(existing) => {
                existing.visit_count = existing.visit_count.max(entry.visit_count);
                existing.last_visit_ms = existing.last_visit_ms.max(entry.last_visit_ms);
                existing.bookmark |= entry.bookmark;
                if existing.title.is_empty() {
                    existing.title = entry.title;
                }
            }
            None => {
                merged.insert(entry.url.clone(), entry);
            }
        }
    }
    let mut out: Vec<BrowserEntry> = merged.into_values().collect();
    // Most recent / most used first; bookmarks win ties.
    out.sort_by(|a, b| {
        b.bookmark
            .cmp(&a.bookmark)
            .then_with(|| b.last_visit_ms.cmp(&a.last_visit_ms))
            .then_with(|| b.visit_count.cmp(&a.visit_count))
            .then_with(|| a.url.cmp(&b.url))
    });
    out.truncate(MAX_TOTAL);
    (out, warnings)
}

// ---------------------------------------------------------------------------
// Chromium
// ---------------------------------------------------------------------------

fn chromium_roots() -> Vec<(&'static str, PathBuf)> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    let mut roots = Vec::new();
    if let Some(local) = &local {
        roots.push(("Chrome", local.join("Google").join("Chrome").join("User Data")));
        roots.push(("Edge", local.join("Microsoft").join("Edge").join("User Data")));
        roots.push((
            "Brave",
            local
                .join("BraveSoftware")
                .join("Brave-Browser")
                .join("User Data"),
        ));
        roots.push(("Vivaldi", local.join("Vivaldi").join("User Data")));
    }
    if let Some(roaming) = &roaming {
        roots.push((
            "Opera",
            roaming.join("Opera Software").join("Opera Stable"),
        ));
    }
    roots
}

fn chromium_profiles(root: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    if root.join("History").is_file() {
        profiles.push(root.to_path_buf());
    }
    if let Ok(read_dir) = std::fs::read_dir(root) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("History").is_file() {
                profiles.push(path);
            }
        }
    }
    profiles.sort();
    profiles.truncate(8);
    profiles
}

/// Scan one Chromium-family profile directory (History + Bookmarks).
pub fn scan_chromium_profile(
    profile: &Path,
    source: &str,
) -> (Vec<BrowserEntry>, Vec<String>) {
    scan_chromium_profile_with_cache(profile, source, &crate::default_cache_dir())
}

/// Cache-injectable variant of scan_chromium_profile (used by tests).
pub fn scan_chromium_profile_with_cache(
    profile: &Path,
    source: &str,
    cache_root: &Path,
) -> (Vec<BrowserEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let history = profile.join("History");
    if history.is_file() {
        match read_chromium_history(&history, source, cache_root) {
            Ok(mut found) => entries.append(&mut found),
            Err(error) => warnings.push(format!("{source}: history unavailable: {error}")),
        }
    }
    let bookmarks = profile.join("Bookmarks");
    if bookmarks.is_file() {
        match parse_chromium_bookmarks(&bookmarks, source) {
            Ok(mut found) => entries.append(&mut found),
            Err(error) => warnings.push(format!("{source}: bookmarks unavailable: {error}")),
        }
    }
    (entries, warnings)
}

fn read_chromium_history(
    history: &Path,
    source: &str,
    cache_root: &Path,
) -> anyhow::Result<Vec<BrowserEntry>> {
    let snapshot = copy_database(history, cache_root)?;
    let result = (|| -> anyhow::Result<Vec<BrowserEntry>> {
        let conn = rusqlite::Connection::open_with_flags(
            snapshot.path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
        let mut statement = conn.prepare(
            "SELECT url, COALESCE(title, ''), COALESCE(visit_count, 0), \
                    COALESCE(last_visit_time, 0) \
             FROM urls WHERE hidden = 0 AND url NOT LIKE 'chrome-extension://%' \
             ORDER BY last_visit_time DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([MAX_PER_PROFILE as i64], |row| {
            let url: String = row.get(0)?;
            let title: String = row.get(1)?;
            let visit_count: i64 = row.get(2)?;
            let last_visit: i64 = row.get(3)?;
            Ok(BrowserEntry {
                url,
                title,
                source: source.to_string(),
                visit_count: visit_count.max(0) as u32,
                last_visit_ms: chromium_time_to_unix_ms(last_visit),
                bookmark: false,
            })
        })?;
        Ok(rows.flatten().collect())
    })();
    result
}

fn parse_chromium_bookmarks(
    bookmarks: &Path,
    source: &str,
) -> anyhow::Result<Vec<BrowserEntry>> {
    let raw = std::fs::read_to_string(bookmarks)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let mut out = Vec::new();
    if let Some(roots) = value.get("roots") {
        for key in ["bookmark_bar", "other", "synced"] {
            if let Some(node) = roots.get(key) {
                collect_chromium_bookmarks(node, source, &mut out);
            }
        }
    }
    out.truncate(MAX_PER_PROFILE);
    Ok(out)
}

fn collect_chromium_bookmarks(
    node: &serde_json::Value,
    source: &str,
    out: &mut Vec<BrowserEntry>,
) {
    if out.len() >= MAX_PER_PROFILE {
        return;
    }
    if let Some(children) = node.get("children").and_then(|children| children.as_array()) {
        for child in children {
            let node_type = child.get("type").and_then(|value| value.as_str());
            if node_type == Some("url") {
                if let (Some(url), name) = (
                    child.get("url").and_then(|value| value.as_str()),
                    child.get("name").and_then(|value| value.as_str()),
                ) {
                    if !url.is_empty() {
                        out.push(BrowserEntry {
                            url: url.to_string(),
                            title: name.unwrap_or_default().to_string(),
                            source: source.to_string(),
                            visit_count: 0,
                            last_visit_ms: 0,
                            bookmark: true,
                        });
                    }
                }
            } else {
                collect_chromium_bookmarks(child, source, out);
            }
        }
    }
}

/// Chromium timestamps are microseconds since 1601-01-01 UTC.
pub fn chromium_time_to_unix_ms(chromium_us: i64) -> i64 {
    if chromium_us <= 0 {
        return 0;
    }
    const EPOCH_DELTA_MS: i64 = 11_644_473_600_000;
    (chromium_us / 1000 - EPOCH_DELTA_MS).max(0)
}

// ---------------------------------------------------------------------------
// Firefox
// ---------------------------------------------------------------------------

fn firefox_profiles_root() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| {
        PathBuf::from(base)
            .join("Mozilla")
            .join("Firefox")
            .join("Profiles")
    })
}

fn firefox_profiles(root: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(root) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("places.sqlite").is_file() {
                profiles.push(path);
            }
        }
    }
    profiles.sort();
    profiles.truncate(8);
    profiles
}

/// Scan one Firefox profile directory (places.sqlite history + bookmarks).
pub fn scan_firefox_profile(profile: &Path) -> (Vec<BrowserEntry>, Vec<String>) {
    scan_firefox_profile_with_cache(profile, &crate::default_cache_dir())
}

/// Cache-injectable variant of scan_firefox_profile (used by tests).
pub fn scan_firefox_profile_with_cache(
    profile: &Path,
    cache_root: &Path,
) -> (Vec<BrowserEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let places = profile.join("places.sqlite");
    if !places.is_file() {
        return (entries, warnings);
    }
    let snapshot = match copy_database(&places, cache_root) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            warnings.push(format!("Firefox: could not snapshot places.sqlite: {error}"));
            return (entries, warnings);
        }
    };
    let result = (|| -> anyhow::Result<Vec<BrowserEntry>> {
        let conn = rusqlite::Connection::open_with_flags(
            snapshot.path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
        let mut statement = conn.prepare(
            "SELECT url, COALESCE(title, ''), COALESCE(visit_count, 0), \
                    COALESCE(last_visit_date, 0) \
             FROM moz_places WHERE url IS NOT NULL \
             ORDER BY last_visit_date DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([MAX_PER_PROFILE as i64], |row| {
            let url: String = row.get(0)?;
            let title: String = row.get(1)?;
            let visit_count: i64 = row.get(2)?;
            let last_visit: i64 = row.get(3)?;
            Ok(BrowserEntry {
                url,
                title,
                source: "Firefox".to_string(),
                visit_count: visit_count.max(0) as u32,
                last_visit_ms: firefox_time_to_unix_ms(last_visit),
                bookmark: false,
            })
        })?;
        Ok(rows.flatten().collect())
    })();
    match result {
        Ok(mut found) => entries.append(&mut found),
        Err(error) => warnings.push(format!("Firefox: history unavailable: {error}")),
    }

    let bookmarks = (|| -> anyhow::Result<Vec<BrowserEntry>> {
        let conn = rusqlite::Connection::open_with_flags(
            snapshot.path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let mut statement = conn.prepare(
            "SELECT p.url, COALESCE(b.title, p.title, '') \
             FROM moz_bookmarks b \
             JOIN moz_places p ON p.id = b.fk \
             WHERE b.type = 1 AND p.url IS NOT NULL LIMIT ?1",
        )?;
        let rows = statement.query_map([MAX_PER_PROFILE as i64], |row| {
            let url: String = row.get(0)?;
            let title: String = row.get(1)?;
            Ok(BrowserEntry {
                url,
                title,
                source: "Firefox".to_string(),
                visit_count: 0,
                last_visit_ms: 0,
                bookmark: true,
            })
        })?;
        Ok(rows.flatten().collect())
    })();
    match bookmarks {
        Ok(mut found) => entries.append(&mut found),
        Err(error) => warnings.push(format!("Firefox: bookmarks unavailable: {error}")),
    }
    (entries, warnings)
}

/// Firefox timestamps are microseconds since the unix epoch.
pub fn firefox_time_to_unix_ms(firefox_us: i64) -> i64 {
    if firefox_us <= 0 {
        return 0;
    }
    firefox_us / 1000
}

// ---------------------------------------------------------------------------
// Snapshot helper: never open a live browser database
// ---------------------------------------------------------------------------

struct DatabaseSnapshot {
    path: PathBuf,
}

impl DatabaseSnapshot {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DatabaseSnapshot {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_file(format!("{}-wal", self.path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.path.display()));
    }
}

fn copy_database(source: &Path, cache_root: &Path) -> anyhow::Result<DatabaseSnapshot> {
    let cache = cache_root.join("browser-snapshots");
    std::fs::create_dir_all(&cache)?;
    let unique = format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0),
        source
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "db".to_string())
    );
    let target = cache.join(unique);
    std::fs::copy(source, &target)?;
    for suffix in ["-wal", "-shm"] {
        let wal = source.with_file_name(format!(
            "{}{suffix}",
            source
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        if wal.is_file() {
            let target_wal = PathBuf::from(format!("{}{suffix}", target.display()));
            let _ = std::fs::copy(&wal, target_wal);
        }
    }
    Ok(DatabaseSnapshot { path: target })
}
