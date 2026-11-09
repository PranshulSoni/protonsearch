//! OCR result storage + FTS5 search (v2 database).

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::OcrDocument;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ocr_documents (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    content_hash TEXT NOT NULL DEFAULT '',
    text TEXT NOT NULL DEFAULT '',
    lang TEXT NOT NULL DEFAULT '',
    width INTEGER NOT NULL DEFAULT 0,
    height INTEGER NOT NULL DEFAULT 0,
    indexed_at INTEGER NOT NULL DEFAULT 0
);
CREATE VIRTUAL TABLE IF NOT EXISTS ocr_fts USING fts5(
    path UNINDEXED,
    text,
    content='ocr_documents',
    content_rowid='id'
);
";

#[derive(Debug, Clone, PartialEq)]
pub struct OcrHit {
    pub path: String,
    pub lang: String,
    pub snippet: String,
    pub bm25: f64,
}

/// Handle to the OCR FTS index inside the shared v2 database.
pub struct OcrIndex {
    conn: Mutex<Connection>,
}

impl OcrIndex {
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("opening v2 database {}", db_path.display()))?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        let _ = conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;",
        );
        conn.execute_batch(SCHEMA)
            .context("migrating OCR schema")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow!("OCR index lock poisoned"))
    }

    /// Store OCR text for a path. Deduplicated by content hash: an unchanged
    /// hash is a no-op and returns false.
    pub fn store(&self, path: &str, content_hash: &str, document: &OcrDocument) -> Result<bool> {
        let conn = self.lock()?;
        let transaction = conn.unchecked_transaction()?;
        let existing: Option<(i64, String)> = transaction
            .query_row(
                "SELECT id, content_hash FROM ocr_documents WHERE path = ?1",
                params![path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let indexed_at = now_ms();
        let wrote = match existing {
            Some((id, previous_hash)) => {
                if previous_hash == content_hash {
                    transaction.commit()?;
                    return Ok(false);
                }
                let old_text: String = transaction.query_row(
                    "SELECT text FROM ocr_documents WHERE id = ?1",
                    params![id],
                    |row| row.get(0),
                )?;
                transaction.execute(
                    "INSERT INTO ocr_fts(ocr_fts, rowid, path, text) VALUES('delete', ?1, ?2, ?3)",
                    params![id, path, old_text],
                )?;
                transaction.execute(
                    "UPDATE ocr_documents SET content_hash = ?1, text = ?2, lang = ?3,
                     width = ?4, height = ?5, indexed_at = ?6 WHERE id = ?7",
                    params![
                        content_hash,
                        document.text,
                        document.language,
                        document.width as i64,
                        document.height as i64,
                        indexed_at,
                        id
                    ],
                )?;
                transaction.execute(
                    "INSERT INTO ocr_fts(rowid, path, text) VALUES(?1, ?2, ?3)",
                    params![id, path, document.text],
                )?;
                true
            }
            None => {
                transaction.execute(
                    "INSERT INTO ocr_documents(path, content_hash, text, lang, width, height, indexed_at)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        path,
                        content_hash,
                        document.text,
                        document.language,
                        document.width as i64,
                        document.height as i64,
                        indexed_at
                    ],
                )?;
                let id = transaction.last_insert_rowid();
                transaction.execute(
                    "INSERT INTO ocr_fts(rowid, path, text) VALUES(?1, ?2, ?3)",
                    params![id, path, document.text],
                )?;
                true
            }
        };
        transaction.commit()?;
        Ok(wrote)
    }

    pub fn remove(&self, path: &str) -> Result<()> {
        let conn = self.lock()?;
        let transaction = conn.unchecked_transaction()?;
        let existing: Option<(i64, String)> = transaction
            .query_row(
                "SELECT id, text FROM ocr_documents WHERE path = ?1",
                params![path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((id, text)) = existing {
            transaction.execute(
                "INSERT INTO ocr_fts(ocr_fts, rowid, path, text) VALUES('delete', ?1, ?2, ?3)",
                params![id, path, text],
            )?;
            transaction.execute("DELETE FROM ocr_documents WHERE id = ?1", params![id])?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// (documents, extracted-text bytes)
    pub fn stats(&self) -> (u64, u64) {
        let Ok(conn) = self.lock() else {
            return (0, 0);
        };
        let documents: u64 = conn
            .query_row("SELECT COUNT(*) FROM ocr_documents", [], |row| row.get(0))
            .unwrap_or(0);
        let bytes: u64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(text)), 0) FROM ocr_documents",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value.max(0) as u64)
            .unwrap_or(0);
        (documents, bytes)
    }

    /// FTS search over stored OCR text (never runs OCR).
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<OcrHit>> {
        let terms = query_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let match_expression = terms
            .iter()
            .map(|term| format!("\"{}\"", term))
            .collect::<Vec<_>>()
            .join(" ");
        let limit = limit.clamp(1, 200) as i64;
        let conn = self.lock()?;
        let mut statement = conn.prepare(
            "SELECT d.path, d.lang,
                    snippet(ocr_fts, 1, '', '', '…', 24) AS snip,
                    bm25(ocr_fts) AS rank
             FROM ocr_fts
             JOIN ocr_documents d ON d.id = ocr_fts.rowid
             WHERE ocr_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![match_expression, limit], |row| {
            Ok(OcrHit {
                path: row.get(0)?,
                lang: row.get(1)?,
                snippet: row.get(2)?,
                bm25: row.get(3)?,
            })
        })?;
        Ok(rows.flatten().collect())
    }
}

/// Split user text into safe quoted FTS terms (no operator injection).
pub fn query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut current = String::new();
    let flush = |terms: &mut Vec<String>, current: &mut String| {
        let term = current.trim();
        if !term.is_empty() && terms.len() < 16 {
            terms.push(term.chars().take(128).collect());
        }
        current.clear();
    };
    for ch in query.chars() {
        if ch.is_control()
            || matches!(
                ch,
                ' ' | '\t' | '\n' | '\r' | '"' | '(' | ')' | '*' | '^' | ':' | '+' | ',' | '{' | '}'
            )
        {
            flush(&mut terms, &mut current);
        } else {
            current.push(ch);
        }
    }
    flush(&mut terms, &mut current);
    terms
}

pub fn sanitize_fts_query(query: &str) -> String {
    query_terms(query)
        .iter()
        .map(|term| format!("\"{}\"", term))
        .collect::<Vec<_>>()
        .join(" ")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
