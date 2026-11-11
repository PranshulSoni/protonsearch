//! Quicklinks and snippets (workflows). OWNER: sources-content.
//!
//! v2 schema (inside the shared v2 database):
//!   quicklinks(name TEXT PRIMARY KEY, url TEXT NOT NULL, keyword TEXT NOT NULL)
//!   snippets(name TEXT PRIMARY KEY, content TEXT NOT NULL, keyword TEXT)

use std::path::Path;

use anyhow::{Context, Result};

use super::{WorkflowEntry, WorkflowKind};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS quicklinks (
    name TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    keyword TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS snippets (
    name TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    keyword TEXT
);
";

pub fn load(db_path: &Path) -> Vec<WorkflowEntry> {
    let Ok(conn) = open_existing(db_path) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    if let Ok(mut statement) = conn.prepare("SELECT name, url, keyword FROM quicklinks ORDER BY name") {
        if let Ok(rows) = statement.query_map([], |row| {
            let name: String = row.get(0)?;
            let url: String = row.get(1)?;
            let keyword: String = row.get(2)?;
            Ok(WorkflowEntry {
                id: format!("quicklink:{name}"),
                name,
                keyword,
                kind: WorkflowKind::Quicklink,
                payload: url,
            })
        }) {
            entries.extend(rows.flatten());
        }
    }
    if let Ok(mut statement) =
        conn.prepare("SELECT name, content, COALESCE(keyword, '') FROM snippets ORDER BY name")
    {
        if let Ok(rows) = statement.query_map([], |row| {
            let name: String = row.get(0)?;
            let content: String = row.get(1)?;
            let keyword: String = row.get(2)?;
            Ok(WorkflowEntry {
                id: format!("snippet:{name}"),
                name,
                keyword,
                kind: WorkflowKind::Snippet,
                payload: content,
            })
        }) {
            entries.extend(rows.flatten());
        }
    }
    entries
}

/// Insert or update a quicklink.
pub fn save_quicklink(db_path: &Path, name: &str, url: &str, keyword: &str) -> Result<()> {
    let conn = open_or_create(db_path)?;
    conn.execute(
        "INSERT INTO quicklinks(name, url, keyword) VALUES(?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET url = excluded.url, keyword = excluded.keyword",
        rusqlite::params![name, url, keyword],
    )
    .context("writing quicklink")?;
    Ok(())
}

/// Insert or update a snippet.
pub fn save_snippet(db_path: &Path, name: &str, content: &str, keyword: &str) -> Result<()> {
    let conn = open_or_create(db_path)?;
    conn.execute(
        "INSERT INTO snippets(name, content, keyword) VALUES(?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET content = excluded.content, keyword = excluded.keyword",
        rusqlite::params![name, content, keyword],
    )
    .context("writing snippet")?;
    Ok(())
}

/// Delete a workflow by its WorkflowEntry id ("quicklink:name"/"snippet:name").
pub fn delete(db_path: &Path, id: &str) -> Result<bool> {
    let conn = open_or_create(db_path)?;
    let (kind, name) = id
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("invalid workflow id: {id}"))?;
    let changed = match kind {
        "quicklink" => conn.execute("DELETE FROM quicklinks WHERE name = ?1", [name])?,
        "snippet" => conn.execute("DELETE FROM snippets WHERE name = ?1", [name])?,
        _ => return Err(anyhow::anyhow!("invalid workflow kind: {kind}")),
    };
    Ok(changed > 0)
}

fn open_existing(db_path: &Path) -> Result<rusqlite::Connection> {
    if !db_path.is_file() {
        anyhow::bail!("database does not exist: {}", db_path.display());
    }
    let conn = rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    Ok(conn)
}

fn open_or_create(db_path: &Path) -> Result<rusqlite::Connection> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let conn = rusqlite::Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}
