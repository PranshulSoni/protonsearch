//! One-time, read-only import of the legacy v1 content index.
//!
//! The legacy database (`%APPDATA%\\protonsearch\\file_index.db`) is opened with
//! an ATTACH URI in read-only mode; this module never writes to it. v2 gets a
//! copy of the `files` table (paths) and the `files_fts` text rows, which are
//! identical in shape to what the v1 indexer produced.

use std::path::Path;
use std::time::Instant;

use anyhow::{anyhow, bail, Context};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::extract::{detect_kind, sanitize_text, truncate_utf8, DEFAULT_MAX_BYTES};
use crate::index::now_ms;
use crate::{ContentIndex, DocKind};

/// Upper bounds so a corrupt/huge legacy database cannot stall startup.
const MAX_LEGACY_FILES: i64 = 500_000;
const MAX_LEGACY_DOCS: usize = 200_000;
const MAX_LEGACY_CONTENT_BYTES: usize = 1024 * 1024 * 1024;
const COMMIT_EVERY: usize = 500;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyImportSummary {
    pub files_copied: u64,
    pub content_documents: u64,
    pub skipped: u64,
    pub errors: u64,
    pub warnings: Vec<String>,
    pub duration_ms: u64,
}

/// Import legacy v1 data into a v2 database: copies the filename rows and the
/// extracted-content rows (read-only access to the legacy file).
pub fn import_legacy(legacy_db: &Path, v2_db: &Path) -> anyhow::Result<LegacyImportSummary> {
    if !legacy_db.is_file() {
        bail!("legacy database not found: {}", legacy_db.display());
    }
    if same_file(legacy_db, v2_db) {
        bail!("refusing to import a database into itself");
    }
    let index = ContentIndex::open(v2_db)?;
    index.import_legacy(legacy_db)
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

impl ContentIndex {
    /// Import from a legacy v1 database attached read-only. Idempotent: existing
    /// v2 rows are left untouched (INSERT OR IGNORE).
    pub fn import_legacy(&self, legacy_db: &Path) -> anyhow::Result<LegacyImportSummary> {
        let started = Instant::now();
        let mut summary = LegacyImportSummary::default();
        let conn = self.lock()?;
        let uri = read_only_uri(legacy_db)?;
        conn.execute("ATTACH DATABASE ?1 AS legacy", params![uri])
            .with_context(|| format!("attaching legacy database {}", legacy_db.display()))?;

        let result = (|| -> anyhow::Result<()> {
            let has_files = table_exists(&conn, "files")?;
            let has_content = table_exists(&conn, "files_fts")?;
            if !has_files && !has_content {
                summary
                    .warnings
                    .push("legacy database has no files/files_fts tables".to_string());
                return Ok(());
            }
            if has_files {
                copy_files(&conn, &mut summary)?;
            }
            if has_content {
                copy_content(&conn, &mut summary)?;
            }
            Ok(())
        })();

        let _ = conn.execute_batch("DETACH DATABASE legacy");
        result?;
        summary.duration_ms = started.elapsed().as_millis() as u64;
        Ok(summary)
    }
}

fn table_exists(conn: &Connection, name: &str) -> anyhow::Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM legacy.sqlite_master WHERE type = 'table' AND name = ?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn copy_files(conn: &Connection, summary: &mut LegacyImportSummary) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (
             path TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             extension TEXT NOT NULL DEFAULT '',
             modified INTEGER NOT NULL DEFAULT 0,
             size INTEGER NOT NULL DEFAULT 0,
             is_dir INTEGER NOT NULL DEFAULT 0
         );",
    )?;
    // Legacy `modified` is unix seconds; v2 convention is unix milliseconds.
    let copied = conn.execute(
        "INSERT OR REPLACE INTO files(path, name, extension, modified, size, is_dir) \
         SELECT path, name, COALESCE(extension, ''), \
                CASE WHEN modified > 100000000000 THEN modified ELSE modified * 1000 END, \
                COALESCE(size, 0), COALESCE(is_dir, 0) \
         FROM (SELECT * FROM legacy.files LIMIT ?1)",
        params![MAX_LEGACY_FILES],
    )?;
    summary.files_copied = copied as u64;
    Ok(())
}

fn copy_content(conn: &Connection, summary: &mut LegacyImportSummary) -> anyhow::Result<()> {
    let mut total_bytes = 0usize;
    let mut processed = 0usize;
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let mut statement = conn.prepare("SELECT path, content FROM legacy.files_fts")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if processed >= MAX_LEGACY_DOCS || total_bytes >= MAX_LEGACY_CONTENT_BYTES {
            summary
                .warnings
                .push("legacy import stopped at the bounded row/byte limit".to_string());
            break;
        }
        processed += 1;
        let path: String = match row.get(0) {
            Ok(path) => path,
            Err(_) => {
                summary.skipped += 1;
                continue;
            }
        };
        let content: String = match row.get(1) {
            Ok(content) => content,
            Err(_) => {
                summary.skipped += 1;
                continue;
            }
        };
        if content.trim().is_empty() {
            summary.skipped += 1;
            continue;
        }
        total_bytes = total_bytes.saturating_add(content.len());
        match import_one_content(conn, &path, &content, summary) {
            Ok(true) => summary.content_documents += 1,
            Ok(false) => summary.skipped += 1,
            Err(error) => {
                summary.errors += 1;
                if summary.warnings.len() < 20 {
                    summary.warnings.push(format!("{path}: {error}"));
                }
            }
        }
        if processed % COMMIT_EVERY == 0 {
            conn.execute_batch("COMMIT; BEGIN IMMEDIATE;")?;
        }
    }
    drop(rows);
    drop(statement);
    conn.execute_batch("COMMIT")?;
    Ok(())
}

fn import_one_content(
    conn: &Connection,
    path: &str,
    content: &str,
    _summary: &mut LegacyImportSummary,
) -> anyhow::Result<bool> {
    if Path::new(path).is_dir() {
        return Ok(false);
    }
    let metadata: Option<(String, i64, i64)> = conn
        .query_row(
            "SELECT name, modified, size FROM legacy.files WHERE path = ?1",
            params![path],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (name, modified_seconds, size) = metadata.unwrap_or_else(|| {
        let fallback = Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        (fallback, 0, content.len() as i64)
    });
    let file_path = Path::new(path);
    let kind = match detect_kind(file_path) {
        DocKind::Other => DocKind::Text,
        kind => kind,
    };
    let title = file_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(name);
    let mtime_ms = if modified_seconds > 100_000_000_000 {
        modified_seconds
    } else {
        modified_seconds.saturating_mul(1000)
    };
    let text = sanitize_text(content);
    let (text, truncated) = truncate_utf8(text, DEFAULT_MAX_BYTES);
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO documents(path, title, kind, size, mtime, content_hash, \
         text, truncated, indexed_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            path,
            title,
            kind_str(kind),
            size,
            mtime_ms,
            hash,
            text,
            if truncated { 1i64 } else { 0 },
            now_ms(),
        ],
    )?;
    if inserted == 0 {
        return Ok(false);
    }
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO documents_fts(rowid, path, title, text) VALUES(?1, ?2, ?3, ?4)",
        params![id, path, title, text],
    )?;
    Ok(true)
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

/// Build a SQLite read-only URI for a Windows path (SQLITE_USE_URI is compiled
/// into the bundled sqlite). Percent-encodes characters that break URI parsing.
fn read_only_uri(path: &Path) -> anyhow::Result<String> {
    let absolute = path
        .canonicalize()
        .map_err(|error| anyhow!("cannot resolve legacy database {}: {error}", path.display()))?;
    let raw = absolute.to_string_lossy();
    // Strip the Windows verbatim prefix (\\?\) before building a URI.
    let raw = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    let text = raw.replace('\\', "/");
    if text.starts_with("file:") {
        return Ok(format!("{text}?mode=ro"));
    }
    let mut encoded = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        match ch {
            ' ' => encoded.push_str("%20"),
            '#' => encoded.push_str("%23"),
            '?' => encoded.push_str("%3F"),
            '%' => encoded.push_str("%25"),
            other => encoded.push(other),
        }
    }
    if encoded.starts_with("//") {
        // UNC path: file://server/share
        Ok(format!("file:{encoded}?mode=ro"))
    } else {
        Ok(format!("file:///{encoded}?mode=ro"))
    }
}
