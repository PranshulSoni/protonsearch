//! Indexed document / code / full-text search on SQLite FTS5. OWNER: sources-content.
//!
//! FTS5 table layout (v2 DB):
//!   documents(id INTEGER PK, path TEXT UNIQUE, title TEXT, kind TEXT,
//!             size INTEGER, mtime INTEGER, content_hash TEXT, text TEXT,
//!             truncated INTEGER, indexed_at INTEGER)
//!   documents_fts USING fts5(path UNINDEXED, title, text, content='documents', content_rowid='id')
//!   content_errors(path TEXT PK, error TEXT, at INTEGER)
//!
//! Extraction is bounded (max bytes, timeout) and runs in background jobs.
//! Search never triggers extraction.

use serde::{Deserialize, Serialize};

pub mod extract;
pub mod index;
pub mod legacy;

pub use extract::{
    detect_kind, extract_text, extract_text_with, is_supported, ExtractOptions, ExtractedDocument,
    DEFAULT_MAX_BYTES, DEFAULT_TIMEOUT,
};
pub use index::{query_terms, sanitize_fts_query, ContentIndex, IndexBatchSummary};
pub use legacy::{import_legacy, LegacyImportSummary};

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

/// A search hit inside indexed document content.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
    /// 1-based line of the first query-term match, for "open at match".
    pub snippet_line: Option<u32>,
    /// FTS5 bm25 score (more negative is better).
    pub bm25: f64,
}

/// Aggregate content-index counters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContentStats {
    pub documents: u64,
    pub errors: u64,
    /// Sum of indexed extracted-text bytes across documents.
    pub bytes_indexed: u64,
}
