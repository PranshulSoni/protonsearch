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

/// A clipboard history item captured by v2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub text: String,
    pub timestamp_ms: i64,
    pub source_app: String,
    pub pinned: bool,
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
            let _ = clipboard::import_windows_history(v2_db);
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
pub fn default_cache_dir() -> PathBuf {
    v2_data_dir().join("cache")
}

pub fn default_icon_cache_dir() -> PathBuf {
    default_cache_dir().join("icons")
}

pub fn default_clipboard_images_dir() -> PathBuf {
    default_cache_dir().join("clipboard")
}
