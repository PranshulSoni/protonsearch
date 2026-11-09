//! Indexed document / code / full-text search on SQLite FTS5. OWNER: sources-content.
//!
//! FTS5 table layout (v2 DB):
//!   documents(id INTEGER PK, path TEXT UNIQUE, title TEXT, kind TEXT,
//!             size INTEGER, mtime INTEGER, content_hash TEXT, text TEXT,
//!             truncated INTEGER, indexed_at INTEGER)
//!   documents_fts USING fts5(path UNINDEXED, title, text, content='documents', content_rowid='id')
//!
//! Extraction is bounded (max bytes, timeout) and runs in background jobs.
//! Search never triggers extraction.

use std::path::Path;
use serde::{Deserialize, Serialize};

/// Supported document families, used to bound extraction work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocKind {
    Pdf,
    Docx,
    Pptx,
    Xlsx,
    Text,
    Code,
    Markdown,
    Json,
    Epub,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedDocument {
    pub kind: DocKind,
    pub text: String,
    pub truncated: bool,
}

/// Extract plain text from a supported file. Returns Ok(None) for unsupported
/// formats and must never panic on malformed input.
pub fn extract_text(_path: &Path, _max_bytes: usize) -> anyhow::Result<Option<ExtractedDocument>> {
    Ok(None)
}

/// A search hit inside indexed document content.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub snippet_line: Option<u32>,
    pub bm25: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContentStats {
    pub documents: u64,
    pub errors: u64,
    pub bytes_indexed: u64,
}

/// Open (and migrate) the content index inside the shared v2 SQLite database.
pub struct ContentIndex {
    _private: (),
}

impl ContentIndex {
    pub fn open(_db_path: &Path) -> anyhow::Result<Self> {
        anyhow::bail!("not implemented yet")
    }

    pub fn stats(&self) -> ContentStats {
        ContentStats::default()
    }

    /// Index one file if its content is supported and changed (mtime/hash).
    /// Returns true when a row was written.
    pub fn index_file(&self, _path: &Path, _max_bytes: usize) -> anyhow::Result<bool> {
        anyhow::bail!("not implemented yet")
    }

    pub fn remove_file(&self, _path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    /// BM25 full-text search. Query is user text, sanitised into FTS terms.
    pub fn search(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<ContentHit>> {
        Ok(Vec::new())
    }
}
