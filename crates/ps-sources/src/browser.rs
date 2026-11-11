//! Browser history + bookmarks across all installed Chromium-family
//! (Chrome/Beta/Dev/Canary, Chromium, Edge, Brave, Vivaldi, Opera/Opera GX,
//! Arc, Dia, Comet, Thorium, Yandex, CentBrowser, Sidekick, Wavebox, Helium)
//! and Gecko-family (Firefox, Zen, Floorp, LibreWolf, Waterfox, Pale Moon)
//! browsers. OWNER: sources-content.
//!
//! Safety rules:
//! - Live browser databases are never opened: the file (plus -wal/-shm) is
//!   copied to our cache first and only the copy is read.
//! - Only History/Bookmarks/places.sqlite are read. Cookie, Login Data,
//!   Web Data and password tables are never touched.
//! - Fully local; no network access.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::BrowserEntry;

pub fn scan() -> (Vec<BrowserEntry>, Vec<String>) {
    scan_with_cache(&crate::default_cache_dir())
}

/// Scan all browsers and all profiles, storing temporary database snapshots
/// under cache_root. An injected cache root keeps tests (and sandboxed callers)
/// off %APPDATA%.
pub fn scan_with_cache(cache_root: &Path) -> (Vec<BrowserEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_profiles: HashSet<String> = HashSet::new();

    for (source, root) in chromium_roots() {
        if !root.is_dir() {
            continue;
        }
        for profile in chromium_profiles(&root) {
            let key = profile.to_string_lossy().to_ascii_lowercase();
            if !seen_profiles.insert(key) {
                continue;
            }
            let (mut found, mut profile_warnings) =
                scan_chromium_profile_with_cache(&profile, source, cache_root);
            entries.append(&mut found);
            warnings.append(&mut profile_warnings);
        }
    }

    for (source, profiles_root) in gecko_roots() {
        if !profiles_root.is_dir() {
            continue;
        }
        for profile in firefox_profiles(&profiles_root) {
            let key = profile.to_string_lossy().to_ascii_lowercase();
            if !seen_profiles.insert(key) {
                continue;
            }
            let (mut found, mut profile_warnings) =
                scan_gecko_profile_with_cache(&profile, source, cache_root);
            entries.append(&mut found);
            warnings.append(&mut profile_warnings);
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

fn merge_entries(entries: Vec<BrowserEntry>, warnings: Vec<String>) -> (Vec<BrowserEntry>, Vec<String>) {
    let mut merged: HashMap<String, BrowserEntry> = HashMap::new();
    for entry in entries {
        match merged.get_mut(&entry.url) {
            Some(existing) => {
                if entry.last_visit_ms > existing.last_visit_ms {
                    existing.source = entry.source;
                }
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
        b.last_visit_ms
            .cmp(&a.last_visit_ms)
            .then_with(|| b.bookmark.cmp(&a.bookmark))
            .then_with(|| b.visit_count.cmp(&a.visit_count))
            .then_with(|| a.url.cmp(&b.url))
    });
    (out, warnings)
}

// ---------------------------------------------------------------------------
// Chromium
// ---------------------------------------------------------------------------

fn find_msix_roots(local: &Path, package_prefix: &str, subpath: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let packages = local.join("Packages");
    let Ok(entries) = std::fs::read_dir(&packages) else {
        return out;
    };
    let prefix_lower = package_prefix.to_ascii_lowercase();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.starts_with(&prefix_lower) {
            continue;
        }
        let mut candidate = entry.path();
        for part in subpath {
            candidate = candidate.join(part);
        }
        if candidate.is_dir() {
            out.push(candidate);
        }
    }
    out
}

fn chromium_roots() -> Vec<(&'static str, PathBuf)> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    let mut roots = Vec::new();
    if let Some(local) = &local {
        roots.push(("Chrome", local.join("Google").join("Chrome").join("User Data")));
        roots.push(("Chrome Beta", local.join("Google").join("Chrome Beta").join("User Data")));
        roots.push(("Chrome Dev", local.join("Google").join("Chrome Dev").join("User Data")));
        roots.push(("Chrome Canary", local.join("Google").join("Chrome SxS").join("User Data")));
        roots.push(("Chromium", local.join("Chromium").join("User Data")));

        roots.push(("Edge", local.join("Microsoft").join("Edge").join("User Data")));
        roots.push(("Edge Beta", local.join("Microsoft").join("Edge Beta").join("User Data")));
        roots.push(("Edge Dev", local.join("Microsoft").join("Edge Dev").join("User Data")));
        roots.push(("Edge Canary", local.join("Microsoft").join("Edge SxS").join("User Data")));

        roots.push(("Brave", local.join("BraveSoftware").join("Brave-Browser").join("User Data")));
        roots.push(("Brave Beta", local.join("BraveSoftware").join("Brave-Browser-Beta").join("User Data")));
        roots.push(("Brave Nightly", local.join("BraveSoftware").join("Brave-Browser-Nightly").join("User Data")));

        roots.push(("Vivaldi", local.join("Vivaldi").join("User Data")));
        roots.push(("Vivaldi Snapshot", local.join("Vivaldi Snapshot").join("User Data")));

        roots.push(("Opera", local.join("Opera Software").join("Opera Stable")));
        roots.push(("Opera GX", local.join("Opera Software").join("Opera GX Stable")));

        roots.push(("Arc", local.join("Arc").join("User Data")));
        for msix in find_msix_roots(local, "TheBrowserCompany.Arc_", &["LocalCache", "Local", "Arc", "User Data"]) {
            roots.push(("Arc", msix));
        }
        roots.push(("Dia", local.join("The Browser Company").join("Dia").join("User Data")));
        for msix in find_msix_roots(local, "TheBrowserCompany.Dia_", &["LocalCache", "Local", "Dia", "User Data"]) {
            roots.push(("Dia", msix));
        }

        roots.push(("Comet", local.join("Perplexity").join("Comet").join("User Data")));
        roots.push(("Thorium", local.join("Thorium").join("User Data")));
        roots.push(("Yandex", local.join("Yandex").join("YandexBrowser").join("User Data")));
        roots.push(("CentBrowser", local.join("CentBrowser").join("User Data")));
        roots.push(("Sidekick", local.join("Sidekick").join("User Data")));
        roots.push(("Wavebox", local.join("WaveboxApp").join("User Data")));
        roots.push(("Helium", local.join("imput").join("Helium").join("User Data")));
        roots.push(("Ungoogled Chromium", local.join("ungoogled-chromium").join("User Data")));
    }
    if let Some(roaming) = &roaming {
        roots.push(("Opera", roaming.join("Opera Software").join("Opera Stable")));
        roots.push(("Opera GX", roaming.join("Opera Software").join("Opera GX Stable")));
        roots.push(("Opera Developer", roaming.join("Opera Software").join("Opera Developer")));
    }
    roots
}

fn has_chromium_data(dir: &Path) -> bool {
    dir.join("History").is_file() || dir.join("Bookmarks").is_file()
}

fn chromium_profiles(root: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    if has_chromium_data(root) {
        profiles.push(root.to_path_buf());
    }
    if let Ok(read_dir) = std::fs::read_dir(root) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if has_chromium_data(&path) {
                profiles.push(path);
            } else if entry.file_name().to_string_lossy().eq_ignore_ascii_case("_side_profiles") {
                if let Ok(side_entries) = std::fs::read_dir(&path) {
                    for side in side_entries.flatten() {
                        let side_path = side.path();
                        if side_path.is_dir() && has_chromium_data(&side_path) {
                            profiles.push(side_path);
                        }
                    }
                }
            }
        }
    }
    profiles.sort();
    profiles
}

/// Scan one Chromium-family profile directory (History + Bookmarks).
pub fn scan_chromium_profile(profile: &Path, source: &str) -> (Vec<BrowserEntry>, Vec<String>) {
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

fn query_chromium_history_db(db_path: &Path, source: &str) -> anyhow::Result<Vec<BrowserEntry>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    let mut statement = conn.prepare(
        "SELECT url, COALESCE(title, ''), COALESCE(visit_count, 0), \
                COALESCE(last_visit_time, 0) \
         FROM urls \
         WHERE hidden = 0 \
           AND url IS NOT NULL \
           AND url NOT LIKE 'chrome-extension://%' \
           AND url NOT LIKE 'extension://%' \
           AND url NOT LIKE 'data:%' \
           AND url NOT LIKE 'javascript:%' \
           AND url NOT LIKE 'blob:%' \
         ORDER BY last_visit_time DESC",
    )?;
    let rows = statement.query_map([], |row| {
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
}

fn read_chromium_history(
    history: &Path,
    source: &str,
    cache_root: &Path,
) -> anyhow::Result<Vec<BrowserEntry>> {
    let snapshot = copy_database(history, cache_root)?;
    match query_chromium_history_db(snapshot.path(), source) {
        Ok(rows) => Ok(rows),
        Err(first_err) => {
            // If the live -wal/-shm snapshot was mid-write and unreadable, strip the
            // sidecars from the temp copy and read the main SQLite file alone.
            snapshot.strip_wal();
            query_chromium_history_db(snapshot.path(), source).map_err(|_| first_err)
        }
    }
}

fn parse_chromium_bookmarks(bookmarks: &Path, source: &str) -> anyhow::Result<Vec<BrowserEntry>> {
    let raw = std::fs::read_to_string(bookmarks)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let mut out = Vec::new();
    if let Some(roots) = value.get("roots").and_then(|v| v.as_object()) {
        for (key, node) in roots {
            if key == "trash" || !node.is_object() {
                continue;
            }
            collect_chromium_bookmarks(node, source, &mut out);
        }
    }
    Ok(out)
}

fn collect_chromium_bookmarks(node: &serde_json::Value, source: &str, out: &mut Vec<BrowserEntry>) {
    if let Some(children) = node.get("children").and_then(|children| children.as_array()) {
        for child in children {
            let node_type = child.get("type").and_then(|value| value.as_str());
            if node_type == Some("url") {
                if let (Some(url), name) = (
                    child.get("url").and_then(|value| value.as_str()),
                    child.get("name").and_then(|value| value.as_str()),
                ) {
                    if !url.is_empty() && !url.starts_with("javascript:") && !url.starts_with("data:") {
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
// Gecko (Firefox, Zen, Floorp, LibreWolf, Waterfox, Pale Moon)
// ---------------------------------------------------------------------------

fn gecko_roots() -> Vec<(&'static str, PathBuf)> {
    let mut roots = Vec::new();
    if let Some(roaming) = std::env::var_os("APPDATA").map(PathBuf::from) {
        roots.push(("Firefox", roaming.join("Mozilla").join("Firefox").join("Profiles")));
        roots.push(("Zen", roaming.join("zen").join("Profiles")));
        roots.push(("Floorp", roaming.join("Floorp").join("Profiles")));
        roots.push(("LibreWolf", roaming.join("librewolf").join("Profiles")));
        roots.push(("LibreWolf", roaming.join("LibreWolf").join("Profiles")));
        roots.push(("Waterfox", roaming.join("Waterfox").join("Profiles")));
        roots.push(("Pale Moon", roaming.join("Moonchild Productions").join("Pale Moon").join("Profiles")));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        for msix in find_msix_roots(&local, "Mozilla.Firefox_", &["LocalCache", "Roaming", "Mozilla", "Firefox", "Profiles"]) {
            roots.push(("Firefox", msix));
        }
    }
    roots
}

fn firefox_profiles(root: &Path) -> Vec<PathBuf> {
    let mut profiles = Vec::new();
    if root.join("places.sqlite").is_file() {
        profiles.push(root.to_path_buf());
    }
    if let Ok(read_dir) = std::fs::read_dir(root) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("places.sqlite").is_file() {
                profiles.push(path);
            }
        }
    }
    profiles.sort();
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
    scan_gecko_profile_with_cache(profile, "Firefox", cache_root)
}

fn query_gecko_history_db(db_path: &Path, source: &str) -> anyhow::Result<Vec<BrowserEntry>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    let mut statement = conn.prepare(
        "SELECT url, COALESCE(title, ''), COALESCE(visit_count, 0), \
                COALESCE(last_visit_date, 0) \
         FROM moz_places \
         WHERE url IS NOT NULL \
           AND url NOT LIKE 'place:%' \
           AND url NOT LIKE 'data:%' \
           AND url NOT LIKE 'javascript:%' \
           AND url NOT LIKE 'blob:%' \
           AND (COALESCE(visit_count, 0) > 0 OR COALESCE(last_visit_date, 0) > 0) \
         ORDER BY last_visit_date DESC",
    )?;
    let rows = statement.query_map([], |row| {
        let url: String = row.get(0)?;
        let title: String = row.get(1)?;
        let visit_count: i64 = row.get(2)?;
        let last_visit: i64 = row.get(3)?;
        Ok(BrowserEntry {
            url,
            title,
            source: source.to_string(),
            visit_count: visit_count.max(0) as u32,
            last_visit_ms: firefox_time_to_unix_ms(last_visit),
            bookmark: false,
        })
    })?;
    Ok(rows.flatten().collect())
}

fn query_gecko_bookmarks_db(db_path: &Path, source: &str) -> anyhow::Result<Vec<BrowserEntry>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    let mut statement = conn.prepare(
        "SELECT p.url, COALESCE(b.title, p.title, '') \
         FROM moz_bookmarks b \
         JOIN moz_places p ON p.id = b.fk \
         WHERE b.type = 1 \
           AND p.url IS NOT NULL \
           AND p.url NOT LIKE 'place:%' \
           AND p.url NOT LIKE 'data:%' \
           AND p.url NOT LIKE 'javascript:%'",
    )?;
    let rows = statement.query_map([], |row| {
        let url: String = row.get(0)?;
        let title: String = row.get(1)?;
        Ok(BrowserEntry {
            url,
            title,
            source: source.to_string(),
            visit_count: 0,
            last_visit_ms: 0,
            bookmark: true,
        })
    })?;
    Ok(rows.flatten().collect())
}

/// Scan one Gecko-family profile directory (places.sqlite history + bookmarks).
pub fn scan_gecko_profile_with_cache(
    profile: &Path,
    source: &str,
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
            warnings.push(format!("{source}: could not snapshot places.sqlite: {error}"));
            return (entries, warnings);
        }
    };
    let history_res = match query_gecko_history_db(snapshot.path(), source) {
        Ok(rows) => Ok(rows),
        Err(first_err) => {
            snapshot.strip_wal();
            query_gecko_history_db(snapshot.path(), source).map_err(|_| first_err)
        }
    };
    match history_res {
        Ok(mut found) => entries.append(&mut found),
        Err(error) => warnings.push(format!("{source}: history unavailable: {error}")),
    }

    match query_gecko_bookmarks_db(snapshot.path(), source) {
        Ok(mut found) => entries.append(&mut found),
        Err(error) => warnings.push(format!("{source}: bookmarks unavailable: {error}")),
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

    fn strip_wal(&self) {
        let _ = std::fs::remove_file(format!("{}-wal", self.path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", self.path.display()));
    }
}

impl Drop for DatabaseSnapshot {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        self.strip_wal();
    }
}

fn copy_file_resilient(src: &Path, dst: &Path) -> std::io::Result<()> {
    match std::fs::copy(src, dst) {
        Ok(_) => Ok(()),
        Err(primary_err) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE (0x7)
                if let Ok(mut reader) = std::fs::OpenOptions::new()
                    .read(true)
                    .share_mode(0x7)
                    .open(src)
                {
                    if let Ok(mut writer) = std::fs::File::create(dst) {
                        if std::io::copy(&mut reader, &mut writer).is_ok() {
                            return Ok(());
                        }
                    }
                }
            }
            Err(primary_err)
        }
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
    copy_file_resilient(source, &target)?;
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
            let _ = copy_file_resilient(&wal, &target_wal);
        }
    }
    Ok(DatabaseSnapshot { path: target })
}
