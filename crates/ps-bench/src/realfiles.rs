//! Read-only export of the legacy v1 SQLite index
//! (%APPDATA%\protonsearch\file_index.db) for realistic benchmark corpora.
//!
//! HARD RULE: the legacy database is opened through a read-only immutable URI and
//! is never written. immutable=1 means SQLite takes no locks and creates no
//! -shm/-wal files; the tradeoff (a pending -wal is not replayed) is recorded in
//! the meta file by comparing row counts against the documented baseline.
//!
//! OWNER: perf-bench.

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::dataset::DatasetEntry;
use crate::env::{host_info, self_memory};
use crate::{fnv1a64_file, sha256_via_certutil, write_json, RESULT_SCHEMA};

/// Default location of the legacy v1 index on this machine.
pub fn default_legacy_db() -> PathBuf {
    let appdata =
        std::env::var("APPDATA").unwrap_or_else(|_| "C:\\Users\\Default\\AppData\\Roaming".to_string());
    PathBuf::from(appdata).join("protonsearch").join("file_index.db")
}

/// Build a SQLite URI that opens path read-only and immutable.
pub fn sqlite_ro_uri(path: &Path) -> String {
    let mut enc = String::with_capacity(path.as_os_str().len() + 24);
    for ch in path.to_string_lossy().replace('\\', "/").chars() {
        match ch {
            '%' => enc.push_str("%25"),
            '?' => enc.push_str("%3F"),
            '#' => enc.push_str("%23"),
            ' ' => enc.push_str("%20"),
            c => enc.push(c),
        }
    }
    format!("file:{enc}?immutable=1")
}

pub fn open_legacy_ro(path: &Path) -> anyhow::Result<Connection> {
    let uri = sqlite_ro_uri(path);
    let conn = Connection::open_with_flags(
        uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("open legacy db read-only (immutable): {}", path.display()))?;
    conn.busy_timeout(Duration::from_millis(2_000))?;
    conn.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")?;
    Ok(conn)
}

#[derive(Debug, Clone, Serialize)]
pub struct DbFacts {
    pub db_path: String,
    pub db_bytes: u64,
    pub wal_bytes: u64,
    pub shm_bytes: u64,
    pub page_size: u64,
    pub page_count: u64,
    pub files: u64,
    pub files_dirs: u64,
    pub content_docs: u64,
    pub browser_items: u64,
    pub memory_events: u64,
    pub git_repos: u64,
    pub git_commits: u64,
    pub git_todos: u64,
    pub quicklinks: u64,
    pub snippets: u64,
}

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn count(conn: &Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64
}

pub fn db_facts(legacy_db: &Path) -> anyhow::Result<DbFacts> {
    let conn = open_legacy_ro(legacy_db)?;
    let page_size: u64 = conn
        .query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as u64;
    let page_count: u64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as u64;
    Ok(DbFacts {
        db_path: legacy_db.display().to_string(),
        db_bytes: file_len(legacy_db),
        wal_bytes: file_len(&wal_path(legacy_db)),
        shm_bytes: file_len(&shm_path(legacy_db)),
        page_size,
        page_count,
        files: count(&conn, "SELECT COUNT(*) FROM files"),
        files_dirs: count(&conn, "SELECT COUNT(*) FROM files WHERE is_dir=1"),
        content_docs: count(&conn, "SELECT COUNT(*) FROM files_fts"),
        browser_items: count(&conn, "SELECT COUNT(*) FROM browser_items"),
        memory_events: count(&conn, "SELECT COUNT(*) FROM memory_events"),
        git_repos: count(&conn, "SELECT COUNT(*) FROM git_repos"),
        git_commits: count(&conn, "SELECT COUNT(*) FROM git_commits"),
        git_todos: count(&conn, "SELECT COUNT(*) FROM git_todos"),
        quicklinks: count(&conn, "SELECT COUNT(*) FROM quicklinks"),
        snippets: count(&conn, "SELECT COUNT(*) FROM snippets"),
    })
}

pub fn wal_path(db: &Path) -> PathBuf {
    PathBuf::from(format!("{}-wal", db.display()))
}

pub fn shm_path(db: &Path) -> PathBuf {
    PathBuf::from(format!("{}-shm", db.display()))
}

#[derive(Debug, Clone, Serialize)]
pub struct CorpusFacts {
    pub entries: u64,
    pub files: u64,
    pub folders: u64,
    pub distinct_ext: u64,
    pub max_path_chars: u64,
    pub cjk_paths: u64,
    pub non_ascii_paths: u64,
    pub hidden_names: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportReport {
    pub schema: String,
    pub kind: String,
    pub host: crate::env::HostInfo,
    pub generated_at_utc: String,
    pub legacy_db: DbFacts,
    pub output_path: String,
    pub bytes: u64,
    pub fnv1a64: String,
    pub sha256: Option<String>,
    pub corpus: CorpusFacts,
    pub export_ms: f64,
    pub notes: Vec<String>,
}

fn ext_of(name: &str) -> String {
    if name.starts_with('.') && name[1..].find('.').is_none() {
        return String::new();
    }
    match name.rfind('.') {
        Some(i) if i + 1 < name.len() => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// Export the real legacy corpus to JSONL in deterministic path order.
pub fn export_legacy(
    legacy_db: &Path,
    out: &Path,
    meta_out: &Path,
    limit: usize,
) -> anyhow::Result<ExportReport> {
    let start = Instant::now();
    let facts = db_facts(legacy_db)?;
    if facts.files == 0 {
        anyhow::bail!(
            "legacy db has 0 rows in files (immutable URI may not see the live WAL): {}",
            legacy_db.display()
        );
    }
    let conn = open_legacy_ro(legacy_db)?;

    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = std::fs::File::create(out).with_context(|| format!("create {}", out.display()))?;
    let mut writer = BufWriter::with_capacity(1 << 20, file);

    let sql = if limit > 0 {
        "SELECT path, name, extension, modified, size, is_dir FROM files ORDER BY path LIMIT ?1"
    } else {
        "SELECT path, name, extension, modified, size, is_dir FROM files ORDER BY path"
    };
    let mut stmt = conn.prepare(sql)?;
    let mut rows = if limit > 0 {
        stmt.query([limit as i64])?
    } else {
        stmt.query([])?
    };

    let mut entries: u64 = 0;
    let mut files = 0u64;
    let mut folders = 0u64;
    let mut cjk = 0u64;
    let mut non_ascii = 0u64;
    let mut hidden = 0u64;
    let mut max_path = 0u64;
    let mut exts = std::collections::HashSet::<String>::new();

    while let Some(row) = rows.next()? {
        let path: String = row.get(0)?;
        let name: String = row.get(1)?;
        let extension: String = row.get(2)?;
        let modified: i64 = row.get(3)?;
        let size: i64 = row.get(4)?;
        let is_dir: i64 = row.get(5)?;
        let ext = if extension.is_empty() {
            ext_of(&name)
        } else {
            extension.to_ascii_lowercase()
        };
        let entry = DatasetEntry {
            path: path.clone(),
            name: name.clone(),
            ext: ext.clone(),
            kind: if is_dir != 0 {
                "folder".to_string()
            } else {
                "file".to_string()
            },
            size: size.max(0) as u64,
            modified,
        };
        serde_json::to_writer(&mut writer, &entry)?;
        writer.write_all(b"\n")?;
        entries += 1;
        if is_dir != 0 {
            folders += 1
        } else {
            files += 1
        }
        if !ext.is_empty() {
            exts.insert(ext);
        }
        if path.chars().any(crate::dataset::is_cjk) {
            cjk += 1
        }
        if !path.is_ascii() {
            non_ascii += 1
        }
        if name.starts_with('.') {
            hidden += 1
        }
        max_path = max_path.max(path.chars().count() as u64);
    }
    writer.flush()?;
    drop(writer);
    drop(rows);
    drop(stmt);
    let export_ms = start.elapsed().as_secs_f64() * 1000.0;

    let (bytes, fnv) = fnv1a64_file(out)?;
    let sha = sha256_via_certutil(out);
    let mem = self_memory();

    let mut notes = vec![
        "legacy database opened with file:...?immutable=1 (read-only); WAL contents are not replayed; row counts compared in legacy_db".to_string(),
        format!("export order: files.path ascending (deterministic); export_ms={:.1}", export_ms),
    ];
    if facts.files != entries {
        notes.push(format!(
            "WARNING: files row count {} != exported {} (immutable ignores a pending -wal; documented baseline 89,813)",
            facts.files, entries
        ));
    }
    if facts.wal_bytes > 0 {
        notes.push(format!(
            "legacy -wal present ({} bytes) and intentionally not replayed",
            facts.wal_bytes
        ));
    }
    if let Some(m) = mem {
        notes.push(format!(
            "exporting process working_set={} bytes peak={} bytes",
            m.working_set_bytes, m.peak_working_set_bytes
        ));
    }

    let report = ExportReport {
        schema: RESULT_SCHEMA.to_string(),
        kind: "legacy-corpus-export".to_string(),
        host: host_info().clone(),
        generated_at_utc: host_info().captured_at_utc.clone(),
        legacy_db: facts,
        output_path: out.display().to_string(),
        bytes,
        fnv1a64: format!("{fnv:016x}"),
        sha256: sha,
        corpus: CorpusFacts {
            entries,
            files,
            folders,
            distinct_ext: exts.len() as u64,
            max_path_chars: max_path,
            cjk_paths: cjk,
            non_ascii_paths: non_ascii,
            hidden_names: hidden,
        },
        export_ms,
        notes,
    };
    write_json(meta_out, &report)?;
    Ok(report)
}
