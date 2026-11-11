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
use ps_ipc::{ActivityItem, ActivityKind, SearchFilters};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

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
file_size INTEGER NOT NULL DEFAULT 0,
file_modified INTEGER NOT NULL DEFAULT 0,
path_lower TEXT NOT NULL DEFAULT '',
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
DROP TRIGGER IF EXISTS ocr_au;
CREATE TRIGGER ocr_au AFTER UPDATE OF path, text ON ocr_documents BEGIN
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
        let conn =
            Connection::open(path).with_context(|| format!("opening engine database {}", path.display()))?;
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "synchronous", "NORMAL").ok();
        // WAL maintenance is explicit (see checkpoint()): a large uncheckpointed
        // WAL otherwise makes EVERY commit attempt a checkpoint, which showed up
        // as ~10 ms stalls on the search path (activity logging commits).
        conn.pragma_update(None, "wal_autocheckpoint", 0).ok();
        conn.execute_batch(SCHEMA).context("creating engine tables")?;
        let mut columns = conn.prepare("PRAGMA table_info(ocr_documents)")?;
        let names = columns
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(columns);
        for (name, definition) in [
            ("file_size", "INTEGER NOT NULL DEFAULT 0"),
            ("file_modified", "INTEGER NOT NULL DEFAULT 0"),
            ("path_lower", "TEXT NOT NULL DEFAULT ''"),
        ] {
            if !names.iter().any(|column| column == name) {
                conn.execute_batch(&format!(
                    "ALTER TABLE ocr_documents ADD COLUMN {name} {definition}"
                ))?;
            }
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
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

    pub fn ocr_paths(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT path FROM ocr_documents")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }

    pub fn ocr_metadata_pending(&self) -> anyhow::Result<u64> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT COUNT(*) FROM ocr_documents WHERE path_lower = ''",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count.max(0) as u64)
        .map_err(Into::into)
    }

    pub fn ocr_paths_needing_metadata(
        &self,
        after_id: i64,
        limit: usize,
    ) -> anyhow::Result<Vec<(i64, String)>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, path FROM ocr_documents WHERE path_lower = '' AND id > ?1 ORDER BY id LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![after_id, limit.clamp(1, 128) as i64], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
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
        file_size: u64,
        file_modified: i64,
    ) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute(
"INSERT INTO ocr_documents(path, hash, text, lang, width, height, file_size, file_modified, path_lower, indexed_at) \
VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT(path) DO UPDATE SET hash=excluded.hash, text=excluded.text, \
lang=excluded.lang, width=excluded.width, height=excluded.height, file_size=excluded.file_size, \
file_modified=excluded.file_modified, path_lower=excluded.path_lower, indexed_at=excluded.indexed_at",
params![path, hash, text, lang, width, height, i64::try_from(file_size).unwrap_or(i64::MAX), file_modified, path.replace('/', "\\").to_lowercase(), crate::engine::now_ms()],
        )?;
        Ok(())
    }

    pub fn ocr_update_metadata(&self, path: &str, file_size: u64, file_modified: i64) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute("UPDATE ocr_documents SET file_size = ?2, file_modified = ?3, path_lower = ?4 WHERE path = ?1 AND (file_size != ?2 OR file_modified != ?3 OR path_lower != ?4)", params![path, i64::try_from(file_size).unwrap_or(i64::MAX), file_modified, path.replace('/', "\\").to_lowercase()])?;
        Ok(())
    }

    pub fn ocr_backfill_metadata(&self, entries: &[(i64, String, u64, i64)]) -> anyhow::Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare("UPDATE ocr_documents SET file_size = ?2, file_modified = ?3, path_lower = ?4 WHERE id = ?1 AND path_lower = ''")?;
            for (id, path, size, modified) in entries {
                let normalized = path.replace('/', "\\").to_lowercase();
                stmt.execute(params![
                    id,
                    i64::try_from(*size).unwrap_or(i64::MAX),
                    modified,
                    normalized
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn ocr_delete(&self, path: &str) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM ocr_documents WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn ocr_clear(&self) -> anyhow::Result<()> {
        let conn = self.lock()?;
        conn.execute_batch("DELETE FROM ocr_documents; INSERT INTO ocr_fts(ocr_fts) VALUES('rebuild');")?;
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
        self.ocr_search_inner(query, limit, 0, false)
    }

    pub fn ocr_recent(&self, limit: usize) -> anyhow::Result<Vec<OcrHit>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT path, text FROM ocr_documents ORDER BY indexed_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let (path, text) = row?;
            hits.push(OcrHit {
                path,
                text,
                line: Some(1),
            });
        }
        Ok(hits)
    }

    pub fn ocr_search_photos(
        &self,
        query: &str,
        limit: usize,
        filters: &SearchFilters,
    ) -> anyhow::Result<Vec<OcrHit>> {
        if filters.min_size.is_some_and(|size| size > i64::MAX as u64) {
            return Ok(Vec::new());
        }
        let terms = ps_content::query_terms(query);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = ps_content::sanitize_fts_query(query);
        if fts_query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut sql = "SELECT d.path, d.text FROM ocr_fts f JOIN ocr_documents d ON d.id = f.rowid \
WHERE ocr_fts MATCH ?1 AND (lower(d.path) GLOB '*.png' OR lower(d.path) GLOB '*.jpg' \
OR lower(d.path) GLOB '*.jpeg' OR lower(d.path) GLOB '*.bmp' OR lower(d.path) GLOB '*.gif' \
OR lower(d.path) GLOB '*.webp' OR lower(d.path) GLOB '*.tif' OR lower(d.path) GLOB '*.tiff') \
AND (?3 IS NULL OR d.path_lower = ?3 \
OR substr(d.path_lower, 1, length(?3) + 1) = ?3 || char(92)) \
AND (?4 IS NULL OR d.file_modified >= ?4) AND (?5 IS NULL OR d.file_modified <= ?5) \
AND (?6 IS NULL OR d.file_size >= ?6) AND (?7 IS NULL OR d.file_size <= ?7)"
            .to_string();
        let under_path = filters
            .under_path
            .as_ref()
            .map(|path| path.replace('/', "\\").trim_end_matches('\\').to_lowercase());
        let mut values = vec![
            Value::Text(fts_query),
            Value::Integer(limit as i64),
            under_path.map(Value::Text).unwrap_or(Value::Null),
            filters.modified_after.map(Value::Integer).unwrap_or(Value::Null),
            filters.modified_before.map(Value::Integer).unwrap_or(Value::Null),
            filters
                .min_size
                .map(|value| Value::Integer(value as i64))
                .unwrap_or(Value::Null),
            filters
                .max_size
                .map(|value| Value::Integer(value.min(i64::MAX as u64) as i64))
                .unwrap_or(Value::Null),
        ];
        if !filters.extensions.is_empty() {
            let mut clauses = Vec::new();
            for extension in &filters.extensions {
                let placeholder = values.len() + 1;
                clauses.push(format!("lower(d.path) GLOB ?{placeholder}"));
                let extension = extension.trim_start_matches('.').to_lowercase();
                let extension = if extension.chars().all(|ch| ch.is_ascii_alphanumeric()) {
                    extension
                } else {
                    "__no_matching_extension__".to_string()
                };
                values.push(Value::Text(format!("*.{extension}")));
            }
            sql.push_str(" AND (");
            sql.push_str(&clauses.join(" OR "));
            sql.push(')');
        }
        sql.push_str(" ORDER BY bm25(ocr_fts) LIMIT ?2");
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut hits = Vec::new();
        for row in rows {
            let (path, text) = row?;
            let (line, _) = snippet_around(&text, &terms);
            hits.push(OcrHit { path, text, line });
        }
        Ok(hits)
    }

    fn ocr_search_inner(
        &self,
        query: &str,
        limit: usize,
        offset: usize,
        photos_only: bool,
    ) -> anyhow::Result<Vec<OcrHit>> {
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
             WHERE ocr_fts MATCH ?1 AND (?4 = 0 OR lower(d.path) GLOB '*.png' \
             OR lower(d.path) GLOB '*.jpg' OR lower(d.path) GLOB '*.jpeg' \
             OR lower(d.path) GLOB '*.bmp' OR lower(d.path) GLOB '*.gif' \
             OR lower(d.path) GLOB '*.webp' OR lower(d.path) GLOB '*.tif' \
             OR lower(d.path) GLOB '*.tiff') ORDER BY bm25(ocr_fts) LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(
            params![fts_query, limit as i64, offset as i64, photos_only as i32],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
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
    if let Some(pos) = terms.iter().filter_map(|term| lower.find(term.as_str())).min() {
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

#[cfg(test)]
mod photos_tests {
    use super::*;

    #[test]
    fn old_ocr_rows_gain_photo_filter_metadata() {
        let dir = std::env::temp_dir().join(format!("ps-ocr-metadata-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("index.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
"CREATE TABLE ocr_documents (id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE, hash TEXT NOT NULL, text TEXT NOT NULL, lang TEXT NOT NULL DEFAULT '', width INTEGER NOT NULL DEFAULT 0, height INTEGER NOT NULL DEFAULT 0, indexed_at INTEGER NOT NULL);
CREATE VIRTUAL TABLE ocr_fts USING fts5(path UNINDEXED, text, content='ocr_documents', content_rowid='id');
INSERT INTO ocr_documents(id, path, hash, text, lang, indexed_at) VALUES(1, 'C:\\Photos\\Résumé.png', 'old', 'legacy caption', 'en', 1);
INSERT INTO ocr_fts(rowid, path, text) VALUES(1, 'C:\\Photos\\Résumé.png', 'legacy caption');",
).unwrap();
        drop(conn);
        let index = ps_index::FilenameIndex::from_entries(vec![ps_index::IndexEntry {
            path: "C:\\Photos\\Résumé.png".into(),
            name: "Résumé.png".into(),
            ext: "png".into(),
            kind: ps_index::EntryKind::File,
            size: 42,
            modified: 1234,
        }]);
        index.save(&dir.join(crate::engine::INDEX_FILE)).unwrap();
        let engine = std::sync::Arc::new(
            crate::engine::Engine::open(crate::engine::EngineConfig {
                data_dir: dir.clone(),
                legacy_db: None,
                version: "test".into(),
            })
            .unwrap(),
        );
        engine.clone().spawn_background();
        for _ in 0..200 {
            if EngineDb::open(&path).unwrap().ocr_metadata_pending().unwrap() == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(EngineDb::open(&path).unwrap().ocr_metadata_pending().unwrap(), 0);
        let db = EngineDb::open(&path).unwrap();
        let filters = SearchFilters {
            under_path: Some("c:\\photos\\résumé.png".into()),
            modified_after: Some(1234),
            min_size: Some(42),
            ..SearchFilters::default()
        };
        let hits = db.ocr_search_photos("legacy", 5, &filters).unwrap();
        assert_eq!(hits.len(), 1);
        drop(db);
        drop(engine);
        let _ = std::fs::remove_dir_all(dir);
    }
}
