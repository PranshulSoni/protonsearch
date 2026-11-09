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
    pub fn load(_settings: &ps_ipc::Settings, _legacy_db: Option<&std::path::Path>) -> (Self, Vec<String>) {
        (Self::default(), Vec::new())
    }
}
