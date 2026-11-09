//! Non-filesystem search sources plus safe native actions. OWNER: sources-content.
//!
//! All sources load from local, user-owned data only (Start Menu, Windows
//! Recent, browser profile DBs opened read-only copies, Git CLI, clipboard
//! history captured by us). Nothing here performs network I/O.

pub mod actions;
pub mod apps;
pub mod browser;
pub mod calc;
pub mod clipboard;
pub mod git;
pub mod notifications;
pub mod quicklinks;
pub mod recent;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// An installed application discoverable from Start Menu / AppsFolder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    /// Executable path or shell:AppsFolder target.
    pub target: String,
    #[serde(default)]
    pub icon_path: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

/// A recently used file from the Windows Recent folder or our own history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    pub name: String,
    pub last_used_ms: i64,
}

/// Coarse content category of a clipboard item. The launcher/UI uses it to
/// pick an icon and to offer sensible actions (open link, reveal path, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClipCategory {
    Link,
    Email,
    Path,
    Code,
    Image,
    /// One or more files/folders copied from Explorer (CF_HDROP).
    Files,
    #[default]
    Text,
}

impl ClipCategory {
    /// Stable database/wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            ClipCategory::Link => "link",
            ClipCategory::Email => "email",
            ClipCategory::Path => "path",
            ClipCategory::Code => "code",
            ClipCategory::Image => "image",
            ClipCategory::Files => "files",
            ClipCategory::Text => "text",
        }
    }

    /// Parse a stored category; unknown/empty values fall back to None.
    pub fn from_db(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "link" => Some(ClipCategory::Link),
            "email" => Some(ClipCategory::Email),
            "path" => Some(ClipCategory::Path),
            "code" => Some(ClipCategory::Code),
            "image" => Some(ClipCategory::Image),
            "files" => Some(ClipCategory::Files),
            "text" => Some(ClipCategory::Text),
            _ => None,
        }
    }
}

/// A clipboard history item captured by v2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub text: String,
    pub timestamp_ms: i64,
    pub source_app: String,
    pub pinned: bool,
    /// Content category (link/email/path/code/image/files/text). Serialized
    /// with a default so older consumers keep parsing.
    #[serde(default)]
    pub category: ClipCategory,
    /// Absolute path of the stored full-resolution PNG for image entries.
    #[serde(default)]
    pub image_path: Option<String>,
    /// Original image width in pixels (image entries).
    #[serde(default)]
    pub width: Option<u32>,
    /// Original image height in pixels (image entries).
    #[serde(default)]
    pub height: Option<u32>,
    /// Absolute path of a bounded PNG thumbnail (<=256 px) for image entries.
    #[serde(default)]
    pub thumbnail_path: Option<String>,
    /// MIME type of the stored payload (image/png, text/plain, text/html, ...).
    #[serde(default)]
    pub mime: Option<String>,
    /// Source clipboard format: png | dibv5 | dib | bitmap | hdrop | html | rtf | text.
    #[serde(default)]
    pub format: Option<String>,
    /// OCR text for image entries once the OCR pipeline has indexed them.
    #[serde(default)]
    pub ocr_text: Option<String>,
}

/// Chromium/Firefox history or bookmark item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserEntry {
    pub url: String,
    pub title: String,
    pub source: String,
    pub visit_count: u32,
    pub last_visit_ms: i64,
    pub bookmark: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GitEntry {
    Repo { path: String, name: String, branch: Option<String> },
    Commit { repo: String, hash: String, message: String, author: String, date_ms: i64 },
    Todo { repo: String, path: String, line: u32, text: String },
}

/// A saved quicklink (keyword -> URL template) or snippet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowEntry {
    pub id: String,
    pub name: String,
    pub keyword: String,
    pub kind: WorkflowKind,
    pub payload: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowKind {
    Quicklink,
    Snippet,
}

/// Aggregate of all loaded sources. Rebuilt cheaply when underlying data changes.
#[derive(Debug, Default)]
pub struct Sources {
    pub apps: Vec<AppEntry>,
    pub recent: Vec<RecentEntry>,
    pub clipboard: Vec<ClipboardEntry>,
    pub browser: Vec<BrowserEntry>,
    pub git: Vec<GitEntry>,
    pub workflows: Vec<WorkflowEntry>,
}

impl Sources {
    /// Load every enabled source. Never fails fatally: partial failures are
    /// collected in warnings and the remaining sources still work.
    pub fn load(settings: &ps_ipc::Settings, legacy_db: Option<&std::path::Path>) -> (Self, Vec<String>) {
        Self::load_with_paths(
            settings,
            legacy_db,
            &v2_db_path(),
            &default_icon_cache_dir(),
        )
    }

    /// Testable variant with explicit v2 database and icon-cache paths.
    pub fn load_with_paths(
        settings: &ps_ipc::Settings,
        legacy_db: Option<&Path>,
        v2_db: &Path,
        icon_cache: &Path,
    ) -> (Self, Vec<String>) {
        let mut sources = Self::default();
        let mut warnings = Vec::new();

        // Applications are always available: launching them must never depend
        // on content/OCR indexing being enabled.
        let (apps, mut app_warnings) = apps::scan_with_icons(icon_cache);
        sources.apps = apps;
        warnings.append(&mut app_warnings);

        let (recent, mut recent_warnings) = recent::scan(Some(v2_db));
        sources.recent = recent;
        warnings.append(&mut recent_warnings);

        if settings.privacy.clipboard_history {
            if let Some(legacy) = legacy_db {
                let _ = clipboard::import_legacy_clipboard(v2_db, legacy);
            }
            // Windows exposes no readable clipboard-history store, so seed the
            // current clipboard once; the live listener (started by the engine)
            // appends every subsequent change.
            let _ = clipboard::capture_current(v2_db, &default_clipboard_images_dir());
            sources.clipboard = clipboard::list_recent(v2_db, 200);
        }

        if settings.privacy.browser_history {
            let (browser, mut browser_warnings) = browser::scan_configured(&settings.privacy);
            sources.browser = browser;
            warnings.append(&mut browser_warnings);
        }

        if settings.privacy.git_indexing {
            let roots: Vec<String> = settings
                .indexing
                .roots
                .iter()
                .filter(|root| root.enabled)
                .map(|root| root.path.clone())
                .collect();
            if !roots.is_empty() {
                let (git, mut git_warnings) = git::scan_roots(&roots, &|| false);
                sources.git = git;
                warnings.append(&mut git_warnings);
            }
        }

        sources.workflows = quicklinks::load(v2_db);
        (sources, warnings)
    }
}

/// v2 data directory (%APPDATA%/protonsearch/v2).
pub fn v2_data_dir() -> PathBuf {
    match std::env::var_os("APPDATA") {
        Some(base) => PathBuf::from(base).join("protonsearch").join("v2"),
        None => PathBuf::from(".protonsearch").join("v2"),
    }
}

/// Shared v2 SQLite database path.
pub fn v2_db_path() -> PathBuf {
    v2_data_dir().join("index.db")
}

/// Cache root for derived artefacts (icons, browser snapshots, clipboard images).
///
/// Honours PROTONSEARCH_DATA_DIR exactly like ps_core::paths: the override is the
/// data base, so the cache is <override>/v2/cache. This keeps portable installs
/// and the E2E/test harness (which cannot write %APPDATA%) working, and it is
/// what makes the launcher's real app icons cache correctly under test.
pub fn default_cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PROTONSEARCH_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("v2").join("cache");
        }
    }
    v2_data_dir().join("cache")
}

pub fn default_icon_cache_dir() -> PathBuf {
    default_cache_dir().join("icons")
}

pub fn default_clipboard_images_dir() -> PathBuf {
    default_cache_dir().join("clipboard")
}
#[cfg(test)]
mod path_tests {
    use super::*;

    /// Env vars are process-global; serialize the tests that mutate them.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &std::ffi::OsStr) -> Self {
            let previous = std::env::var_os(key);
            std::env::set_var(key, value);
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn cache_dir_honours_portable_data_dir_override() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        // Path comparison only (no IO): unit tests cannot rely on CARGO_TARGET_TMPDIR.
        let base = std::env::temp_dir().join("protonsearch-test-portable-data");
        let _guard = EnvGuard::set("PROTONSEARCH_DATA_DIR", base.as_os_str());
        assert_eq!(default_cache_dir(), base.join("v2").join("cache"));
        assert_eq!(default_icon_cache_dir(), base.join("v2").join("cache").join("icons"));
        assert_eq!(
            default_clipboard_images_dir(),
            base.join("v2").join("cache").join("clipboard")
        );
    }

    #[test]
    fn cache_dir_falls_back_to_appdata_when_override_absent() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _guard = EnvGuard::remove("PROTONSEARCH_DATA_DIR");
        let cache = default_cache_dir();
        assert!(
            cache.ends_with(Path::new("v2").join("cache")),
            "unexpected cache dir: {}",
            cache.display()
        );
        assert!(default_icon_cache_dir().ends_with(Path::new("v2").join("cache").join("icons")));
    }

    #[test]
    fn empty_override_is_ignored() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let _guard = EnvGuard::set("PROTONSEARCH_DATA_DIR", std::ffi::OsStr::new(""));
        assert!(default_cache_dir().ends_with(Path::new("v2").join("cache")));
    }
}