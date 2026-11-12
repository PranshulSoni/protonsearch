//! Browser history + bookmarks across all installed Chromium-family
//! (Chrome/Beta/Dev/Canary, Chromium, Edge, Brave, Vivaldi, Opera/Opera GX,
//! Arc, Dia, Comet, Thorium, Yandex, CentBrowser, Sidekick, Wavebox, Helium)
//! and Gecko-family (Firefox, Zen, Floorp, LibreWolf, Waterfox, Pale Moon)
//! browsers. OWNER: sources-content.
//!
//! Safety rules:
//! - Live browser databases are never opened: the file (plus -wal/-shm) is
//!   copied to our cache first and only the copy is read.
//! - Only History/Bookmarks/places.sqlite and the favicon databases
//!   (Chromium `Favicons`, Gecko `favicons.sqlite`) are read. Cookie, Login
//!   Data, Web Data and password tables are never touched.
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
// Favicons: the browsers' own icon databases, never the network
// ---------------------------------------------------------------------------

/// Hosts resolved per scan, most recently browsed first. Far above any real
/// history's host count; FAVICON_TOTAL_BYTES is what bounds memory.
const FAVICON_HOSTS: usize = 5_000;
/// Icon files larger than this are skipped (touch icons, oversized bitmaps).
const FAVICON_MAX_BYTES: i64 = 8 * 1024;
/// Icon bytes kept across all hosts. shortcut: counts raw bytes, not the base64
/// data URL that crosses IPC (about 4/3 larger).
const FAVICON_TOTAL_BYTES: usize = 2 * 1024 * 1024;
/// Favicons draw at about 16 px; a 32 px source stays sharp on HiDPI screens.
const FAVICON_TARGET_PX: i64 = 32;
const PNG_MAGIC: [u8; 4] = [0x89, b'P', b'N', b'G'];

/// How one browser family stores icons. `mapping` lists (page url, icon id),
/// newest first; `bitmaps` lists (width, bytes) of one icon within the size cap
/// (?1 = icon id, ?2 = size cap).
struct FaviconSchema {
    mapping: &'static str,
    bitmaps: &'static str,
}

const CHROMIUM_FAVICONS: FaviconSchema = FaviconSchema {
    mapping: "SELECT page_url, icon_id FROM icon_mapping ORDER BY id DESC",
    bitmaps: "SELECT width, image_data FROM favicon_bitmaps \
              WHERE icon_id = ?1 AND image_data IS NOT NULL AND length(image_data) <= ?2",
};

const GECKO_FAVICONS: FaviconSchema = FaviconSchema {
    mapping: "SELECT p.page_url, ip.icon_id FROM moz_pages_w_icons p \
              JOIN moz_icons_to_pages ip ON ip.page_id = p.id ORDER BY p.id DESC",
    bitmaps: "SELECT width, data FROM moz_icons \
              WHERE id = ?1 AND data IS NOT NULL AND length(data) <= ?2",
};

/// Lower-case host of an http(s) URL, without userinfo, port or a leading "www.".
pub fn host_of(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    let host = host.strip_prefix("www.").unwrap_or(host).to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

fn favicon_databases() -> Vec<(PathBuf, &'static FaviconSchema)> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut found = Vec::new();
    let mut add = |db: PathBuf, schema: &'static FaviconSchema| {
        if db.is_file() && seen.insert(db.to_string_lossy().to_ascii_lowercase()) {
            found.push((db, schema));
        }
    };
    for (_, root) in chromium_roots().into_iter().filter(|(_, root)| root.is_dir()) {
        for profile in chromium_profiles(&root) {
            add(profile.join("Favicons"), &CHROMIUM_FAVICONS);
        }
    }
    for (_, root) in gecko_roots().into_iter().filter(|(_, root)| root.is_dir()) {
        for profile in firefox_profiles(&root) {
            add(profile.join("favicons.sqlite"), &GECKO_FAVICONS);
        }
    }
    found
}

/// An icon's MIME type and bytes.
pub type Favicon = (&'static str, Vec<u8>);

/// Icon per host for the hosts `entries` lists first (scans return them newest
/// first), read from the installed browsers' local icon databases through
/// snapshots. Nothing touches the network.
pub fn favicons(entries: &[BrowserEntry]) -> HashMap<String, Favicon> {
    favicons_from(entries, &favicon_databases(), &crate::default_cache_dir())
}

fn favicons_from(
    entries: &[BrowserEntry],
    databases: &[(PathBuf, &FaviconSchema)],
    cache_root: &Path,
) -> HashMap<String, Favicon> {
    let mut wanted: Vec<String> = Vec::new();
    let mut listed: HashSet<String> = HashSet::new();
    for host in entries.iter().filter_map(|entry| host_of(&entry.url)) {
        if wanted.len() == FAVICON_HOSTS {
            break;
        }
        if listed.insert(host.clone()) {
            wanted.push(host);
        }
    }
    let sites: HashSet<&str> = wanted.iter().map(|host| registrable_domain(host)).collect();
    let mut icons: HashMap<String, Favicon> = HashMap::new();
    for (path, schema) in databases {
        let Ok(snapshot) = copy_database(path, cache_root) else {
            continue;
        };
        if read_favicons(snapshot.path(), schema, &sites, &mut icons).is_err() {
            // A snapshot caught mid-write can be unreadable with its -wal: retry on the main file.
            snapshot.strip_wal();
            let _ = read_favicons(snapshot.path(), schema, &sites, &mut icons);
        }
    }
    let by_site = icon_sites(&icons);
    let mut found = HashMap::new();
    let mut total = 0;
    for host in &wanted {
        let Some((mime, bytes)) = icon_for(host, &icons, &by_site) else {
            continue;
        };
        if total + bytes.len() > FAVICON_TOTAL_BYTES {
            continue;
        }
        total += bytes.len();
        found.insert(host.clone(), (*mime, bytes.clone()));
    }
    found
}

fn read_favicons(
    db: &Path,
    schema: &FaviconSchema,
    sites: &HashSet<&str>,
    icons: &mut HashMap<String, Favicon>,
) -> rusqlite::Result<()> {
    let conn = rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    // Every distinct icon of a host, not just the newest few: a site points its
    // pages at several icons (sizes, touch icons) and the small one may be any of them.
    let mut ids: HashMap<String, HashSet<i64>> = HashMap::new();
    let mut mapping = conn.prepare(schema.mapping)?;
    let rows = mapping.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
    for (page_url, icon_id) in rows.flatten() {
        let Some(host) = host_of(&page_url) else {
            continue;
        };
        if sites.contains(registrable_domain(&host)) && !icons.contains_key(&host) {
            ids.entry(host).or_default().insert(icon_id);
        }
    }
    let mut bitmaps = conn.prepare(schema.bitmaps)?;
    for (host, host_ids) in ids {
        let mut best: Option<(i64, Favicon)> = None;
        for id in host_ids {
            let rows = bitmaps.query_map(rusqlite::params![id, FAVICON_MAX_BYTES], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;
            for (width, bytes) in rows.flatten() {
                let Some(mime) = image_mime(&bytes) else {
                    continue;
                };
                let distance = (width - FAVICON_TARGET_PX).abs();
                if best.as_ref().is_none_or(|(nearest, _)| distance < *nearest) {
                    best = Some((distance, (mime, bytes)));
                }
            }
        }
        if let Some((_, favicon)) = best {
            icons.insert(host, favicon);
        }
    }
    Ok(())
}

/// Registrable domain by the two-label rule: "music.youtube.com" -> "youtube.com".
/// Three labels under a common second-level label: "shop.example.co.uk" ->
/// "example.co.uk". IP literals stay whole. shortcut: no public-suffix list (that
/// would be a new dependency), so rarer multi-label suffixes keep two labels.
fn registrable_domain(host: &str) -> &str {
    let labels: Vec<&str> = host.split('.').collect();
    let count = labels.len();
    if labels[count - 1].bytes().all(|byte| byte.is_ascii_digit()) {
        return host;
    }
    let under_second_level = count >= 3
        && labels[count - 1].len() == 2
        && matches!(labels[count - 2], "co" | "com" | "net" | "org" | "gov" | "ac" | "edu");
    let keep = if under_second_level { 3 } else { 2 };
    if count <= keep {
        return host;
    }
    let start: usize = labels[..count - keep].iter().map(|label| label.len() + 1).sum();
    &host[start..]
}

/// One icon host per registrable domain, for sibling fallback: the domain's own
/// icon host when it has one, else the first sibling in sorted order (stable).
fn icon_sites(icons: &HashMap<String, Favicon>) -> HashMap<&str, &str> {
    let mut hosts: Vec<&str> = icons.keys().map(String::as_str).collect();
    hosts.sort_unstable();
    let mut by_site: HashMap<&str, &str> = HashMap::new();
    for host in hosts {
        let site = registrable_domain(host);
        if host == site || !by_site.contains_key(site) {
            by_site.insert(site, host);
        }
    }
    by_site
}

/// The icon for `host`, best first: its own, the same host without a mobile "m."
/// label, its registrable domain's, then a sibling host's (see `icon_sites`).
fn icon_for<'a>(
    host: &str,
    icons: &'a HashMap<String, Favicon>,
    by_site: &HashMap<&str, &str>,
) -> Option<&'a Favicon> {
    let site = registrable_domain(host);
    let bare = host.strip_prefix("m.").unwrap_or(host);
    let icon_host = [host, bare, site]
        .into_iter()
        .find(|candidate| icons.contains_key(*candidate))
        .or_else(|| by_site.get(site).copied())?;
    icons.get(icon_host)
}

/// MIME type by the leading bytes; None for anything an <img> cannot show. Chromium
/// stores PNG only; Gecko keeps the site's own file, often SVG or ICO.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    let head = &bytes[..bytes.len().min(512)];
    if head.starts_with(&PNG_MAGIC) {
        Some("image/png")
    } else if head.starts_with(&[0, 0, 1, 0]) {
        Some("image/x-icon")
    } else if head.starts_with(b"GIF8") {
        Some("image/gif")
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if head.len() >= 12 && head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else if String::from_utf8_lossy(head).contains("<svg") {
        Some("image/svg+xml")
    } else {
        None
    }
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

#[cfg(test)]
mod favicon_tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ps-favicons-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn visit(url: &str) -> BrowserEntry {
        BrowserEntry {
            url: url.to_string(),
            title: String::new(),
            source: "Test".to_string(),
            visit_count: 1,
            last_visit_ms: 1,
            bookmark: false,
        }
    }

    fn png(tag: u8) -> Vec<u8> {
        let mut bytes = PNG_MAGIC.to_vec();
        bytes.push(tag);
        bytes
    }

    #[test]
    fn host_of_keeps_only_the_site() {
        assert_eq!(host_of("https://www.Example.com:8443/a?b#c").as_deref(), Some("example.com"));
        assert_eq!(host_of("http://user:pw@docs.rs/x").as_deref(), Some("docs.rs"));
        assert_eq!(host_of("chrome://settings"), None);
        assert_eq!(host_of("https:///nohost"), None);
    }

    #[test]
    fn chromium_favicons_pick_the_size_nearest_32_and_skip_unwanted_or_unusable_icons() {
        let dir = scratch("chromium");
        let db = dir.join("Favicons");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE icon_mapping (id INTEGER PRIMARY KEY, page_url TEXT NOT NULL, icon_id INTEGER);
             CREATE TABLE favicon_bitmaps (id INTEGER PRIMARY KEY, icon_id INTEGER, width INTEGER, image_data BLOB);",
        )
        .unwrap();
        let insert = |sql: &str, params: &[&dyn rusqlite::ToSql]| conn.execute(sql, params).unwrap();
        insert("INSERT INTO icon_mapping (page_url, icon_id) VALUES (?1, 1)", &[&"https://www.example.com/page"]);
        insert("INSERT INTO icon_mapping (page_url, icon_id) VALUES (?1, 2)", &[&"https://big.org/"]);
        insert("INSERT INTO icon_mapping (page_url, icon_id) VALUES (?1, 3)", &[&"https://unseen.net/"]);
        insert("INSERT INTO icon_mapping (page_url, icon_id) VALUES (?1, 4)", &[&"https://notpng.io/"]);
        let bitmap = "INSERT INTO favicon_bitmaps (icon_id, width, image_data) VALUES (?1, ?2, ?3)";
        insert(bitmap, &[&1, &16, &png(16)]);
        insert(bitmap, &[&1, &32, &png(32)]);
        insert(bitmap, &[&2, &32, &vec![PNG_MAGIC[0]; FAVICON_MAX_BYTES as usize + 1]]);
        insert(bitmap, &[&3, &32, &png(3)]);
        insert(bitmap, &[&4, &32, &b"<html>not an icon".to_vec()]);
        drop(conn);

        let entries = [
            visit("https://example.com/other"),
            visit("https://big.org/"),
            visit("https://notpng.io/"),
            visit("chrome://newtab"),
        ];
        let found = favicons_from(&entries, &[(db, &CHROMIUM_FAVICONS)], &dir);
        assert_eq!(found.get("example.com"), Some(&("image/png", png(32))));
        assert!(!found.contains_key("big.org"), "oversized icons are skipped");
        assert!(!found.contains_key("notpng.io"), "only image files are used");
        assert!(!found.contains_key("unseen.net"), "hosts that were not browsed are not read");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gecko_favicons_follow_page_to_icon_links() {
        let dir = scratch("gecko");
        let db = dir.join("favicons.sqlite");
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE moz_icons (id INTEGER PRIMARY KEY, width INTEGER, data BLOB);
             CREATE TABLE moz_pages_w_icons (id INTEGER PRIMARY KEY, page_url TEXT);
             CREATE TABLE moz_icons_to_pages (page_id INTEGER, icon_id INTEGER);
             INSERT INTO moz_pages_w_icons (id, page_url) VALUES (1, 'https://mozilla.org/en-US/');
             INSERT INTO moz_icons_to_pages (page_id, icon_id) VALUES (1, 7);",
        )
        .unwrap();
        conn.execute("INSERT INTO moz_icons (id, width, data) VALUES (7, 32, ?1)", [png(7)]).unwrap();
        drop(conn);

        let found = favicons_from(&[visit("https://www.mozilla.org/")], &[(db, &GECKO_FAVICONS)], &dir);
        assert_eq!(found.get("mozilla.org"), Some(&("image/png", png(7))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn icon_lookup_falls_back_from_subdomains_to_the_site() {
        let icon = |tag: u8| ("image/png", png(tag));
        let icons: HashMap<String, Favicon> = HashMap::from([
            ("youtube.com".to_string(), icon(1)),
            ("shop.example.co.uk".to_string(), icon(2)),
        ]);
        let by_site = icon_sites(&icons);
        assert_eq!(icon_for("youtube.com", &icons, &by_site), Some(&icon(1)));
        assert_eq!(icon_for("music.youtube.com", &icons, &by_site), Some(&icon(1)));
        assert_eq!(icon_for("m.youtube.com", &icons, &by_site), Some(&icon(1)));
        assert_eq!(icon_for("cdn.shop.example.co.uk", &icons, &by_site), Some(&icon(2)));
        assert_eq!(icon_for("example.org", &icons, &by_site), None);
    }
}
