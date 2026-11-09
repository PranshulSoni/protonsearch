//! Engine-owned SQLite tables: activity history and the OCR text index.
//! Owner: L0 (integration layer); content documents live in ps-content.
//!
//! Both tables live in the shared v2 database (WAL). Writes are short and
//! transactional so search/status reads are never blocked for long.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Counts activity writes so retention pruning + WAL checkpoints are amortized
/// instead of running on every search.
static ACTIVITY_WRITES: AtomicU64 = AtomicU64::new(0);
use std::time::Duration;

use anyhow::Context;
use ps_ipc::{ActivityItem, ActivityKind};
use rusqlite::{params, Connection, OptionalExtension};

pub struct EngineDb {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcrHit {
    pub path: String,
    pub text: String,
    pub line: Option<u32>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    detail TEXT,
    path TEXT
);
CREATE INDEX IF NOT EXISTS idx_activity_timestamp ON activity(timestamp DESC);

CREATE TABLE IF NOT EXISTS ocr_documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    hash TEXT NOT NULL,
    text TEXT NOT NULL,
    lang TEXT NOT NULL DEFAULT '',
    width INTEGER NOT NULL DEFAULT 0,
    height INTEGER NOT NULL DEFAULT 0,
    indexed_at INTEGER NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS ocr_fts USING fts5(
    path UNINDEXED,
    text,
    content='ocr_documents',
    content_rowid='id'
);
CREATE TRIGGER IF NOT EXISTS ocr_ai AFTER INSERT ON ocr_documents BEGIN
    INSERT INTO ocr_fts(rowid, path, text) VALUES (new.id, new.path, new.text);
END;
CREATE TRIGGER IF NOT EXISTS ocr_ad AFTER DELETE ON ocr_documents BEGIN
    INSERT INTO ocr_fts(ocr_fts, rowid, path, text) VALUES ('delete', old.id, old.path, old.text);
END;
CREATE TRIGGER IF NOT EXISTS ocr_au AFTER UPDATE ON ocr_documents BEGIN
    INSERT INTO ocr_fts(ocr_fts, rowid, path, text) VALUES ('delete', old.id, old.path, old.text);
    INSERT INTO ocr_fts(rowid, path, text) VALUES (new.id, new.path, new.text);
END;

CREATE TABLE IF NOT EXISTS migration_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

impl EngineDb {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening engine database {}", path.display()))?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "synchronous", "NORMAL").ok();
        // WAL maintenance is explicit (see checkpoint()): a large uncheckpointed
        // WAL otherwise makes EVERY commit attempt a checkpoint, which showed up
        // as ~10 ms stalls on the search path (activity logging commits).
        conn.pragma_update(None, "wal_autocheckpoint", 0).ok();
        conn.execute_batch(SCHEMA)
            .context("creating engine tables")?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn lock(&self) -> anyhow::Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| anyhow::anyhow!("engine database lock poisoned"))
    }

    pub fn log_activity(
        &self,
        kind: ActivityKind,
        title: &str,
        detail: Option<&str>,
        path: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO activity(timestamp, kind, title, detail, path) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![
                crate::engine::now_ms(),
                activity_kind_str(kind),
                title,
                detail,
                path
            ],
        )?;
        // Keep local history bounded (privacy default: 20k rows), but do not
        // scan/delete on every keystroke-search: prune every 64 writes and
        // checkpoint explicitly so the search path never pays WAL maintenance.
        let writes = ACTIVITY_WRITES.fetch_add(1, Ordering::Relaxed) + 1;
        if writes % 64 == 0 {
            conn.execute(
                "DELETE FROM activity WHERE id NOT IN (SELECT id FROM activity ORDER BY id DESC LIMIT 20000)",
                [],
            )
            .ok();
            let _ = conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |_| Ok(()));
        }
        Ok(())
    }

    /// Fold the WAL back into the database without blocking readers. Called from
    /// the background indexer after batches (auto-checkpoint is disabled so
    /// commits on the search path stay cheap).
    pub fn checkpoint(&self) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |_| Ok(()))
            .ok();
        Ok(())
    }

    pub fn activity(&self, query: &ps_ipc::ActivityQuery) -> anyhow::Result<Vec<ActivityItem>> {
        let conn = self.lock()?;
        let limit = query.limit.clamp(1, 500) as i64;
        let since = query.since.unwrap_or(0);
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, kind, title, detail, path FROM activity \
             WHERE timestamp >= ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since, limit], |row| {
            let kind: String = row.get(2)?;
            Ok(ActivityItem {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                kind: activity_kind_from_str(&kind),
                title: row.get(3)?,
                detail: row.get(4)?,
                path: row.get(5)?,
            })
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        if !query.kinds.is_empty() {
            items.retain(|item| query.kinds.contains(&item.kind));
        }
        Ok(items)
    }

    pub fn clear_activity(&self) -> anyhow::Result<u64> {
        let conn = self.lock()?;
        let removed = conn.execute("DELETE FROM activity", [])?;
        Ok(removed as u64)
    }

    // -- OCR -----------------------------------------------------------------

    pub fn ocr_count(&self) -> u64 {
        let Ok(conn) = self.lock() else {
            return 0;
        };
        conn.query_row("SELECT COUNT(*) FROM ocr_documents", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as u64
    }

    pub fn ocr_has(&self, path: &str, hash: &str) -> bool {
        let Ok(conn) = self.lock() else {
            return false;
        };
        conn.query_row(
            "SELECT 1 FROM ocr_documents WHERE path = ?1 AND hash = ?2",
            params![path, hash],
            |_| Ok(()),
        )
        .optional()
        .ok()
        .flatten()
        .is_some()
    }

    pub fn ocr_upsert(
        &self,
        path: &str,
        hash: &str,
        text: &str,
        lang: &str,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO ocr_documents(path, hash, text, lang, width, height, indexed_at) \
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(path) DO UPDATE SET hash=excluded.hash, text=excluded.text, \
             lang=excluded.lang, width=excluded.width, height=excluded.height, indexed_at=excluded.indexed_at",
            params![path, hash, text, lang, width, height, crate::engine::now_ms()],
        )?;
        Ok(())
    }

    pub fn ocr_clear(&self) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute_batch(
            "DELETE FROM ocr_documents; INSERT INTO ocr_fts(ocr_fts) VALUES('rebuild');",
        )?;
        Ok(())
    }

    /// Indexed OCR text for one path (preview path; no extraction, no query).
    pub fn ocr_text_for_path(&self, path: &str) -> Option<String> {
        let conn = self.lock().ok()?;
        conn.query_row(
            "SELECT text FROM ocr_documents WHERE path = ?1 LIMIT 1",
            params![path],
            |row| row.get::<_, String>(0),
        )
        .ok()
    }

    pub fn ocr_search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<OcrHit>> {
        let terms = ps_content::query_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = ps_content::sanitize_fts_query(query);
        if fts_query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT d.path, d.text FROM ocr_fts f JOIN ocr_documents d ON d.id = f.rowid \
             WHERE ocr_fts MATCH ?1 ORDER BY bm25(ocr_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let (path, text) = row?;
            let (line, snippet) = snippet_around(&text, &terms);
            let _ = snippet;
            hits.push(OcrHit { path, text, line });
        }
        Ok(hits)
    }
}

/// 1-based line number of the first term occurrence, for open-at-match.
fn snippet_around(text: &str, terms: &[String]) -> (Option<u32>, String) {
    let lower = text.to_lowercase();
    let mut line = None;
    if let Some(pos) = terms
        .iter()
        .filter_map(|term| lower.find(term.as_str()))
        .min()
    {
        line = Some((text[..pos].matches('\n').count() + 1) as u32);
    }
    (line, String::new())
}

pub fn activity_kind_str(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Search => "search",
        ActivityKind::Open => "open",
        ActivityKind::Launch => "launch",
        ActivityKind::Index => "index",
        ActivityKind::Workflow => "workflow",
        ActivityKind::Error => "error",
        ActivityKind::System => "system",
    }
}

pub fn activity_kind_from_str(value: &str) -> ActivityKind {
    match value {
        "search" => ActivityKind::Search,
        "open" => ActivityKind::Open,
        "launch" => ActivityKind::Launch,
        "index" => ActivityKind::Index,
        "workflow" => ActivityKind::Workflow,
        "error" => ActivityKind::Error,
        _ => ActivityKind::System,
    }
}
