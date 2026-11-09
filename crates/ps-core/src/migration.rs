//! One-time legacy v1 migration. OWNER: L0 (integration layer).
//!
//! Contract (also used by tests/migration): the legacy database is opened
//! READ-ONLY (WAL-aware URI, no writes) and is guaranteed byte-identical
//! afterwards (SHA-256 before == after). Only a user-facing summary and
//! non-secret data are imported. AI credentials and clipboard contents are
//! never imported without explicit opt-in.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct MigrationOptions {
    /// Clipboard history is sensitive; default is OFF.
    pub import_clipboard: bool,
    /// AI keys are never imported from legacy storage; default OFF.
    pub import_ai_keys: bool,
    /// Compute counts without writing anything to the v2 database.
    pub dry_run: bool,
}

impl Default for MigrationOptions {
    fn default() -> Self {
        Self {
            import_clipboard: false,
            import_ai_keys: false,
            dry_run: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub files: u64,
    pub dirs: u64,
    pub content_docs: u64,
    pub browser_items: u64,
    pub quicklinks: u64,
    pub snippets: u64,
    pub memory_events: u64,
    pub skipped_tables: Vec<&'static str>,
    pub source_unchanged: bool,
    pub duration_ms: u64,
    pub warnings: Vec<String>,
}

fn read_only_uri(path: &Path) -> anyhow::Result<String> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("legacy database not found: {}", path.display()))?;
    let mut uri = String::from("file:");
    uri.push_str(&canonical.to_string_lossy().replace('\\', "/"));
    uri.push_str("?mode=ro");
    Ok(uri)
}

fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading legacy database {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
    .unwrap_or(false)
}

fn count_rows(conn: &Connection, table: &str) -> u64 {
    if !table_exists(conn, table) {
        return 0;
    }
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get::<_, i64>(0)
    })
    .unwrap_or(0) as u64
}

fn ensure_v2_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS browser_items (
             url TEXT PRIMARY KEY,
             title TEXT NOT NULL,
             source TEXT NOT NULL,
             visit_count INTEGER NOT NULL DEFAULT 0,
             last_visit_time INTEGER NOT NULL DEFAULT 0,
             bookmark INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS quicklinks (
             name TEXT PRIMARY KEY,
             url TEXT NOT NULL,
             keyword TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS snippets (
             name TEXT PRIMARY KEY,
             content TEXT NOT NULL,
             keyword TEXT
         );",
    )?;
    Ok(())
}

fn copy(table: &str, columns: &[&str], source: &Connection, target: &Connection) -> anyhow::Result<u64> {
    if !table_exists(source, table) {
        return Ok(0);
    }
    let list = columns.join(", ");
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let select = format!("SELECT {list} FROM {table}");
    let insert = format!("INSERT OR IGNORE INTO {table} ({list}) VALUES ({placeholders})");
    let mut select_stmt = source.prepare(&select)?;
    let mut insert_stmt = target.prepare(&insert)?;
    let mut rows = select_stmt.query([])?;
    let mut copied = 0u64;
    while let Some(row) = rows.next()? {
        let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            values.push(row.get(index)?);
        }
        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|value| value as &dyn rusqlite::ToSql).collect();
        inserted(&mut insert_stmt, &params)?;
        copied += 1;
    }
    Ok(copied)
}

fn inserted(stmt: &mut rusqlite::Statement<'_>, params: &[&dyn rusqlite::ToSql]) -> anyhow::Result<()> {
    stmt.execute(params)?;
    Ok(())
}

/// Import a legacy v1 database into a v2 database without mutating the source.
pub fn migrate_legacy_db(
    legacy_db: &Path,
    v2_db: &Path,
    opts: &MigrationOptions,
) -> anyhow::Result<MigrationReport> {
    let started = std::time::Instant::now();
    let before = sha256_file(legacy_db)?;
    let uri = read_only_uri(legacy_db)?;

    // ps-content owns the documents/files import (idempotent, read-only ATTACH).
    let import_summary = if opts.dry_run {
        None
    } else {
        Some(ps_content::legacy::import_legacy(legacy_db, v2_db)?)
    };

    let source = Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .context("opening legacy database read-only")?;

    let mut report = MigrationReport::default();
    report.files = count_rows(&source, "files");
    if table_exists(&source, "files") {
        report.dirs = source
            .query_row("SELECT COUNT(*) FROM files WHERE is_dir != 0", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as u64;
        report.files = report.files.saturating_sub(report.dirs);
    }
    report.content_docs = count_rows(&source, "files_fts");
    report.browser_items = count_rows(&source, "browser_items");
    report.quicklinks = count_rows(&source, "quicklinks");
    report.snippets = count_rows(&source, "snippets");
    report.memory_events = count_rows(&source, "memory_events");

    if !opts.import_clipboard {
        report.skipped_tables.push("clipboard_history");
        report.skipped_tables.push("timeline_events");
    }
    if !opts.import_ai_keys {
        report.skipped_tables.push("ai_settings");
    }

    if !opts.dry_run {
        let target = Connection::open(v2_db).context("opening v2 database")?;
        target.busy_timeout(std::time::Duration::from_millis(5000))?;
        target.pragma_update(None, "journal_mode", "WAL").ok();
        ensure_v2_tables(&target)?;
        let mut copied: BTreeMap<&str, u64> = BTreeMap::new();
        copied.insert(
            "browser_items",
            copy(
                "browser_items",
                &["url", "title", "source", "visit_count", "last_visit_time"],
                &source,
                &target,
            )?,
        );
        copied.insert(
            "quicklinks",
            copy("quicklinks", &["name", "url", "keyword"], &source, &target)?,
        );
        copied.insert(
            "snippets",
            copy("snippets", &["name", "content", "keyword"], &source, &target)?,
        );
        if opts.import_clipboard && table_exists(&source, "clipboard_history") {
            // Clipboard rows need the v2 schema; ps-sources owns that table.
            report
                .warnings
                .push("clipboard_history exists but v2 clipboard import is owned by ps-sources; not copied".into());
        }
        if let Some(summary) = &import_summary {
            report.warnings.extend(summary.warnings.iter().cloned());
        }
        let _ = copied;
    }

    let after = sha256_file(legacy_db)?;
    report.source_unchanged = before == after;
    if !report.source_unchanged {
        return Err(anyhow::anyhow!(
            "legacy database changed during migration; aborting (investigate immediately)"
        ));
    }
    report.duration_ms = started.elapsed().as_millis() as u64;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_legacy(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE files (path TEXT PRIMARY KEY, name TEXT NOT NULL, extension TEXT NOT NULL,
                modified INTEGER NOT NULL, size INTEGER NOT NULL, is_dir INTEGER NOT NULL,
                last_seen_scan INTEGER NOT NULL DEFAULT 0);
             INSERT INTO files VALUES ('C:/Docs/a.txt','a.txt','txt',1700000000,12,0,1);
             INSERT INTO files VALUES ('C:/Docs','Docs','',1700000000,0,1,1);
             INSERT INTO files VALUES ('C:/Docs/b.pdf','b.pdf','pdf',1700000001,1200,0,1);
             CREATE VIRTUAL TABLE files_fts USING fts5(path UNINDEXED, content);
             INSERT INTO files_fts VALUES ('C:/Docs/a.txt','hello world');
             CREATE TABLE browser_items (url TEXT PRIMARY KEY, title TEXT NOT NULL, source TEXT NOT NULL,
                visit_count INTEGER NOT NULL, last_visit_time INTEGER NOT NULL);
             INSERT INTO browser_items VALUES ('https://example.com','Example','chrome',3,1700000000);
             CREATE TABLE quicklinks (name TEXT PRIMARY KEY, url TEXT NOT NULL, keyword TEXT NOT NULL);
             INSERT INTO quicklinks VALUES ('Docs','https://docs.example.com','docs');
             CREATE TABLE snippets (name TEXT PRIMARY KEY, content TEXT NOT NULL, keyword TEXT);
             CREATE TABLE ai_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO ai_settings VALUES ('endpoint','https://api.example.com');
             CREATE TABLE memory_events (id INTEGER PRIMARY KEY, event_type TEXT);
             INSERT INTO memory_events(event_type) VALUES ('visited');
             CREATE TABLE clipboard_history (id INTEGER PRIMARY KEY, content TEXT);
             CREATE TABLE timeline_events (id INTEGER PRIMARY KEY, app_name TEXT);",
        )
        .unwrap();
    }

    #[test]
    fn dry_run_reports_counts_and_never_mutates_source() {
        let dir = std::env::temp_dir().join(format!("ps-migration-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join("legacy.db");
        let v2 = dir.join("v2.db");
        make_legacy(&legacy);
        let before = sha256_file(&legacy).unwrap();
        let report = migrate_legacy_db(&legacy, &v2, &MigrationOptions::default()).unwrap();
        assert!(report.source_unchanged);
        assert_eq!(report.files, 2);
        assert_eq!(report.dirs, 1);
        assert_eq!(report.content_docs, 1);
        assert_eq!(report.browser_items, 1);
        assert_eq!(report.quicklinks, 1);
        assert_eq!(report.memory_events, 1);
        assert_eq!(sha256_file(&legacy).unwrap(), before);
        assert!(!v2.exists(), "dry run must not create the v2 database");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn real_import_copies_safe_tables_only() {
        let dir = std::env::temp_dir().join(format!("ps-migration-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join("legacy.db");
        let v2 = dir.join("v2.db");
        make_legacy(&legacy);
        let opts = MigrationOptions { dry_run: false, ..Default::default() };
        let report = migrate_legacy_db(&legacy, &v2, &opts).unwrap();
        assert!(report.source_unchanged);
        assert!(report.skipped_tables.contains(&"ai_settings"));
        let target = Connection::open(&v2).unwrap();
        let copied: i64 = target
            .query_row("SELECT COUNT(*) FROM browser_items", [], |row| row.get(0))
            .unwrap();
        assert_eq!(copied, 1);
    }
}
