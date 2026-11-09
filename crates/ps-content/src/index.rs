//! SQLite FTS5 content index.
//!
//! Schema (v2 database, owned by ps-content):
//!   documents(id INTEGER PK, path TEXT UNIQUE, title TEXT, kind TEXT,
//!             size INTEGER, mtime INTEGER, content_hash TEXT, text TEXT,
//!             truncated INTEGER, indexed_at INTEGER)
//!   documents_fts USING fts5(path UNINDEXED, title, text,
//!                             content='documents', content_rowid='id')
//!   content_errors(path TEXT PK, error TEXT, at INTEGER)
//!
//! Writes are transactional; reads never trigger extraction.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::extract::{self, ExtractOptions};
use crate::{ContentHit, ContentStats, DocKind};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'other',
    size INTEGER NOT NULL DEFAULT 0,
    mtime INTEGER NOT NULL DEFAULT 0,
    content_hash TEXT NOT NULL DEFAULT '',
    text TEXT NOT NULL DEFAULT '',
    truncated INTEGER NOT NULL DEFAULT 0,
    indexed_at INTEGER NOT NULL DEFAULT 0
);
CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
    path UNINDEXED,
    title,
    text,
    content='documents',
    content_rowid='id'
);
CREATE TABLE IF NOT EXISTS content_errors (
    path TEXT PRIMARY KEY,
    error TEXT NOT NULL,
    at INTEGER NOT NULL
);
"#;

/// Result of indexing a batch of files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexBatchSummary {
    pub indexed: u64,
    pub skipped: u64,
    pub errors: u64,
    pub warnings: Vec<String>,
}

/// Handle to the content FTS index inside the shared v2 database.
pub struct ContentIndex {
    conn: Mutex<Connection>,
    indexed_bytes: AtomicU64,
    write_errors: AtomicU64,
}

impl ContentIndex {
    /// Open (and migrate) the content index inside a v2 SQLite database.
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating database directory {}", parent.display()))?;
            }
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("opening v2 database {}", db_path.display()))?;
        Self::from_connection(conn)
    }

    /// In-memory index for tests and previews.
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().context("opening in-memory database")?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> anyhow::Result<Self> {
        conn.busy_timeout(Duration::from_millis(5000))
            .context("setting sqlite busy_timeout")?;
        // WAL is requested for file databases; in-memory databases stay in
        // memory mode and simply ignore the request.
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON;");
        conn.execute_batch(SCHEMA).context("migrating content schema")?;
        Ok(Self {
            conn: Mutex::new(conn),
            indexed_bytes: AtomicU64::new(0),
            write_errors: AtomicU64::new(0),
        })
    }

    pub fn stats(&self) -> ContentStats {
        let fallback = ContentStats {
            documents: 0,
            errors: self.write_errors.load(Ordering::Relaxed),
            bytes_indexed: self.indexed_bytes.load(Ordering::Relaxed),
        };
        let Ok(conn) = self.conn.lock() else {
            return fallback;
        };
        let documents: u64 = conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .unwrap_or(0);
        let errors: u64 = conn
            .query_row("SELECT COUNT(*) FROM content_errors", [], |row| row.get(0))
            .unwrap_or(0);
        let bytes_indexed: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(text)), 0) FROM documents",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|v| v.max(0) as u64)
            .unwrap_or(0);
        ContentStats {
            documents,
            errors: errors.max(fallback.errors),
            bytes_indexed,
        }
    }

    /// Index one file if its content is supported and changed (mtime/size then
    /// hash). Returns true when document content was (re)written.
    pub fn index_file(&self, path: &Path, max_bytes: usize) -> anyhow::Result<bool> {
        self.index_file_with(path, ExtractOptions::new(max_bytes))
    }

    pub fn index_file_with(
        &self,
        path: &Path,
        options: ExtractOptions,
    ) -> anyhow::Result<bool> {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => return Ok(false),
        };
        if !extract::is_supported(path) {
            return Ok(false);
        }
        let path_string = path.to_string_lossy().to_string();
        let mtime = mtime_ms(&metadata);
        let size = metadata.len();

        // Fast path: unchanged (mtime, size) -> no read, no write.
        {
            let conn = self.lock()?;
            let existing: Option<(i64, i64)> = conn
                .query_row(
                    "SELECT mtime, size FROM documents WHERE path = ?1",
                    params![path_string],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((old_mtime, old_size)) = existing {
                if old_mtime == mtime && old_size as u64 == size {
                    return Ok(false);
                }
            }
        }

        let extracted = match extract::extract_text_with(path, options) {
            Ok(Some(document)) => document,
            Ok(None) => return Ok(false),
            Err(error) => {
                self.record_error(&path_string, &error);
                return Err(error);
            }
        };
        let content_hash = hash_text(&extracted.text);
        let kind = kind_str(extracted.kind);
        let title = title_for(path);
        let indexed_at = now_ms();
        let truncated = if extracted.truncated { 1i64 } else { 0 };

        let conn = self.lock()?;
        let transaction = conn
            .unchecked_transaction()
            .context("starting content write transaction")?;
        let existing: Option<(i64, String)> = transaction
            .query_row(
                "SELECT id, content_hash FROM documents WHERE path = ?1",
                params![path_string],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let wrote = match existing {
            Some((id, previous_hash)) => {
                if previous_hash == content_hash {
                    // Same extracted text with a new mtime: refresh metadata only.
                    transaction.execute(
                        "UPDATE documents SET mtime = ?1, size = ?2, indexed_at = ?3 WHERE id = ?4",
                        params![mtime, size as i64, indexed_at, id],
                    )?;
                    transaction
                        .execute("DELETE FROM content_errors WHERE path = ?1", params![path_string])?;
                    transaction.commit()?;
                    return Ok(false);
                }
                let old: (String, String, String) = transaction.query_row(
                    "SELECT path, title, text FROM documents WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
                transaction.execute(
                    "INSERT INTO documents_fts(documents_fts, rowid, path, title, text) VALUES('delete', ?1, ?2, ?3, ?4)",
                    params![id, old.0, old.1, old.2],
                )?;
                transaction.execute(
                    "UPDATE documents SET path = ?1, title = ?2, kind = ?3, size = ?4, mtime = ?5, \
                     content_hash = ?6, text = ?7, truncated = ?8, indexed_at = ?9 WHERE id = ?10",
                    params![
                        path_string, title, kind, size as i64, mtime, content_hash,
                        extracted.text, truncated, indexed_at, id
                    ],
                )?;
                transaction.execute(
                    "INSERT INTO documents_fts(rowid, path, title, text) VALUES(?1, ?2, ?3, ?4)",
                    params![id, path_string, title, extracted.text],
                )?;
                true
            }
            None => {
                transaction.execute(
                    "INSERT INTO documents(path, title, kind, size, mtime, content_hash, \
                     text, truncated, indexed_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        path_string, title, kind, size as i64, mtime, content_hash,
                        extracted.text, truncated, indexed_at
                    ],
                )?;
                let id = transaction.last_insert_rowid();
                transaction.execute(
                    "INSERT INTO documents_fts(rowid, path, title, text) VALUES(?1, ?2, ?3, ?4)",
                    params![id, path_string, title, extracted.text],
                )?;
                true
            }
        };
        transaction.execute(
            "DELETE FROM content_errors WHERE path = ?1",
            params![path_string],
        )?;
        transaction.commit().context("committing content write")?;
        if wrote {
            self.indexed_bytes
                .fetch_add(extracted.text.len() as u64, Ordering::Relaxed);
        }
        Ok(wrote)
    }

    /// Index a batch of paths, collecting per-file errors as warnings.
    pub fn index_paths(&self, paths: &[PathBuf], max_bytes: usize) -> IndexBatchSummary {
        let mut summary = IndexBatchSummary::default();
        for path in paths {
            match self.index_file(path, max_bytes) {
                Ok(true) => summary.indexed += 1,
                Ok(false) => summary.skipped += 1,
                Err(error) => {
                    summary.errors += 1;
                    if summary.warnings.len() < 20 {
                        summary.warnings.push(format!("{}: {error}", path.display()));
                    }
                }
            }
        }
        summary
    }

    /// Remove one document (and its FTS row) from the index.
    pub fn remove_file(&self, path: &Path) -> anyhow::Result<()> {
        let path_string = path.to_string_lossy().to_string();
        let conn = self.lock()?;
        let transaction = conn.unchecked_transaction()?;
        let existing: Option<(i64, String, String, String)> = transaction
            .query_row(
                "SELECT id, path, title, text FROM documents WHERE path = ?1",
                params![path_string],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        if let Some((id, old_path, old_title, old_text)) = existing {
            transaction.execute(
                "INSERT INTO documents_fts(documents_fts, rowid, path, title, text) VALUES('delete', ?1, ?2, ?3, ?4)",
                params![id, old_path, old_title, old_text],
            )?;
            transaction.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
        }
        transaction.execute(
            "DELETE FROM content_errors WHERE path = ?1",
            params![path_string],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Remove every document under a path prefix (used when a root is removed).
    pub fn remove_under(&self, prefix: &Path) -> anyhow::Result<u64> {
        let prefix = prefix.to_string_lossy().to_string();
        let like = format!("{}%", prefix.replace('%', "\\%").replace('_', "\\_"));
        let paths: Vec<String> = {
            let conn = self.lock()?;
            let mut statement =
                conn.prepare("SELECT path FROM documents WHERE path LIKE ?1 ESCAPE '\\'")?;
            let rows = statement.query_map(params![like], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        for path in &paths {
            self.remove_file(Path::new(path))?;
        }
        Ok(paths.len() as u64)
    }

    /// BM25 full-text search. The query is user text; it is turned into quoted
    /// FTS terms so FTS syntax can never be injected.
    ///
    /// Performance (task-42): the FTS5 snippet() function re-scans the stored
    /// document text for every hit and dominated query latency (p95 > 60 ms on
    /// high-df terms). Instead we fetch a bounded text prefix (32 KB covers the
    /// overwhelming majority of real documents; the corpus max is ~51 KB) and
    /// compute the snippet + first-match line in one allocation-free scan per
    /// term. Ranking (bm25) is unchanged.
    pub fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<ContentHit>> {
        let terms = query_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let match_expression = terms
            .iter()
            .map(|term| format!("\"{}\"", term))
            .collect::<Vec<_>>()
            .join(" ");
        let limit = limit.clamp(1, 200);
        let conn = self.lock()?;
        let mut statement = conn.prepare(
            "SELECT d.path, d.title, bm25(documents_fts) AS rank, \
                    CASE WHEN LENGTH(d.text) <= ?3 THEN d.text ELSE SUBSTR(d.text, 1, ?3) END \
             FROM documents_fts \
             JOIN documents d ON d.id = documents_fts.rowid \
             WHERE documents_fts MATCH ?1 \
             ORDER BY rank \
             LIMIT ?2",
        )?;
        let prefix_chars = SNIPPET_TEXT_PREFIX_CHARS as i64;
        let rows = statement.query_map(params![match_expression, limit as i64, prefix_chars], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let (path, title, rank, text) = row?;
            let position = first_term_position(&text, &terms);
            let snippet = match position {
                Some(position) => snippet_around(&text, position),
                None => text_head(&text),
            };
            hits.push(ContentHit {
                path,
                title,
                snippet,
                snippet_line: position.map(|position| line_of_position(&text, position)),
                bm25: rank,
            });
        }
        Ok(hits)
    }

    /// Remove index rows whose file no longer exists (bounded to `limit` rows).
    pub fn prune_missing(&self, limit: usize) -> anyhow::Result<u64> {
        let paths: Vec<String> = {
            let conn = self.lock()?;
            let mut statement = conn.prepare("SELECT path FROM documents LIMIT ?1")?;
            let rows = statement.query_map(params![limit.max(1) as i64], |row| {
                row.get::<_, String>(0)
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut removed = 0u64;
        for path in paths {
            if !Path::new(&path).exists() {
                self.remove_file(Path::new(&path))?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// Delete all content rows (rebuild support).
    pub fn clear(&self) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute_batch(
            "DELETE FROM documents_fts; DELETE FROM documents; DELETE FROM content_errors;",
        )
        .context("clearing content index")
    }

    fn record_error(&self, path: &str, error: &anyhow::Error) {
        self.write_errors.fetch_add(1, Ordering::Relaxed);
        if let Ok(conn) = self.conn.lock() {
            let message = format!("{error:#}");
            let _ = conn.execute(
                "INSERT OR REPLACE INTO content_errors(path, error, at) VALUES(?1, ?2, ?3)",
                params![path, message, now_ms()],
            );
        }
    }

    /// Most recent extraction errors (path, message), newest first.
    pub fn recent_errors(&self, limit: usize) -> Vec<(String, String)> {
        let Ok(conn) = self.lock() else {
            return Vec::new();
        };
        let mut statement = match conn
            .prepare("SELECT path, error FROM content_errors ORDER BY at DESC LIMIT ?1")
        {
            Ok(statement) => statement,
            Err(_) => return Vec::new(),
        };
        let rows = statement.query_map([limit.max(1) as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        match rows {
            Ok(rows) => rows.flatten().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub(crate) fn lock(&self) -> anyhow::Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow!("content index lock poisoned"))
    }
}

// ---------------------------------------------------------------------------
// Query sanitisation
// ---------------------------------------------------------------------------

/// Split user query text into safe FTS5 terms.
///
/// FTS operators/syntax characters are treated as separators; every remaining
/// run of non-space characters becomes a quoted term (FTS5 implicit AND).
pub fn query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let push_current = |terms: &mut Vec<String>, current: &mut String| {
        let term = current.trim();
        if !term.is_empty() && terms.len() < 16 {
            terms.push(term.chars().take(128).collect::<String>());
        }
        current.clear();
    };
    for ch in query.chars() {
        if ch.is_control() {
            continue;
        }
        // Characters that carry FTS5 syntax meaning; treat as separators.
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '"' | '(' | ')' | '*' | '^' | ':' | '+' | ',' | '{' | '}') {
            push_current(&mut terms, &mut current);
            continue;
        }
        current.push(ch);
    }
    push_current(&mut terms, &mut current);
    terms
}

/// Public helper: the exact FTS expression used for a user query (empty when
/// the query has no searchable terms).
pub fn sanitize_fts_query(query: &str) -> String {
    query_terms(query)
        .iter()
        .map(|term| format!("\"{}\"", term))
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn title_for(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn kind_str(kind: DocKind) -> &'static str {
    match kind {
        DocKind::Pdf => "pdf",
        DocKind::Docx => "docx",
        DocKind::Pptx => "pptx",
        DocKind::Xlsx => "xlsx",
        DocKind::Text => "text",
        DocKind::Code => "code",
        DocKind::Markdown => "markdown",
        DocKind::Json => "json",
        DocKind::Epub => "epub",
        DocKind::Other => "other",
    }
}

/// Bounded text prefix used for snippets/line numbers (characters, not bytes:
/// SQLite LENGTH/SUBSTR count characters).
const SNIPPET_TEXT_PREFIX_CHARS: usize = 32 * 1024;

/// First byte offset at which any query term occurs, case-insensitively
/// (ASCII folded; non-ASCII bytes compared exactly so CJK works).
fn first_term_position(text: &str, terms: &[String]) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut best: Option<usize> = None;
    for term in terms {
        if term.is_empty() {
            continue;
        }
        if let Some(position) = find_ci_bytes(bytes, term.as_bytes()) {
            best = Some(best.map_or(position, |current| current.min(position)));
        }
    }
    best
}

fn find_ci_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    let first = needle[0];
    let first_upper = first.to_ascii_uppercase();
    let mut start = 0usize;
    while start + needle.len() <= hay.len() {
        let found = if first_upper != first {
            memchr::memchr2(first, first_upper, &hay[start..])
        } else {
            memchr::memchr(first, &hay[start..])
        };
        let Some(relative) = found else {
            return None;
        };
        let position = start + relative;
        if position + needle.len() > hay.len() {
            return None;
        }
        let candidate = &hay[position..position + needle.len()];
        if candidate
            .iter()
            .zip(needle.iter())
            .all(|(left, right)| left.to_ascii_lowercase() == *right)
        {
            return Some(position);
        }
        start = position + 1;
    }
    None
}

fn line_of_position(text: &str, position: usize) -> u32 {
    let prefix = &text.as_bytes()[..position.min(text.len())];
    (prefix.iter().filter(|byte| **byte == b'\n').count() + 1) as u32
}

/// Excerpt around a match, single-line, bounded to ~240 chars.
fn snippet_around(text: &str, position: usize) -> String {
    const BEFORE: usize = 80;
    const AFTER: usize = 160;
    let mut start = position.saturating_sub(BEFORE);
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (position + AFTER).min(text.len());
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    let mut out = String::with_capacity(end - start + 2);
    if start > 0 {
        out.push('…');
    }
    let mut previous_space = false;
    for ch in text[start..end].chars() {
        let ch = if ch.is_whitespace() { ' ' } else { ch };
        if ch == ' ' {
            if previous_space {
                continue;
            }
            previous_space = true;
        } else {
            previous_space = false;
        }
        out.push(ch);
    }
    if end < text.len() {
        out.push('…');
    }
    out.trim().to_string()
}

fn text_head(text: &str) -> String {
    let head: String = text.chars().take(240).collect();
    let mut out = String::with_capacity(head.len() + 1);
    let mut previous_space = false;
    for ch in head.chars() {
        let ch = if ch.is_whitespace() { ' ' } else { ch };
        if ch == ' ' {
            if previous_space {
                continue;
            }
            previous_space = true;
        } else {
            previous_space = false;
        }
        out.push(ch);
    }
    let trimmed = out.trim();
    if text.chars().count() > 240 {
        format!("{trimmed}…")
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn mtime_ms(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 1-based line number of the first query-term match, or None.
///
/// The caller passes at most a bounded prefix of the document text, so this is
/// a linear scan with a first-byte fast path (no allocation, ASCII case fold;
/// non-ASCII terms match exactly).
pub fn find_line_of_first_term(text: &str, terms: &[String]) -> Option<u32> {
    let haystack = text.as_bytes();
    let mut best: Option<usize> = None;
    for term in terms {
        let needle = term.as_bytes();
        if needle.is_empty() || needle.len() > haystack.len() {
            continue;
        }
        let first = needle[0];
        let last_start = haystack.len() - needle.len();
        let mut index = 0usize;
        while index <= last_start {
            if haystack[index].eq_ignore_ascii_case(&first)
                && haystack[index..index + needle.len()].eq_ignore_ascii_case(needle)
            {
                best = Some(best.map_or(index, |current: usize| current.min(index)));
                break;
            }
            index += 1;
        }
        if best == Some(0) {
            break;
        }
    }
    best.map(|index| {
        (haystack[..index].iter().filter(|byte| **byte == b'\n').count() as u32) + 1
    })
}