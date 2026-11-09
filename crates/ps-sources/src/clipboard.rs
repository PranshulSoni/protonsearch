//! Clipboard history capture and search. OWNER: sources-content.

use super::ClipboardEntry;

/// Import Windows clipboard history items (read-only) if available.
pub fn import_windows_history(_db_path: &std::path::Path) -> anyhow::Result<usize> {
    Ok(0)
}

/// Insert a captured clipboard item; deduplicates identical consecutive text.
pub fn record(_db_path: &std::path::Path, _text: &str, _source_app: &str) -> anyhow::Result<bool> {
    Ok(false)
}

pub fn search(_db_path: &std::path::Path, _query: &str, _limit: usize) -> Vec<ClipboardEntry> {
    Vec::new()
}
