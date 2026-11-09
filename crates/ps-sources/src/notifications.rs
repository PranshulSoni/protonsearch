//! Local ProtonSearch notification store (v10 Notifications card).
//!
//! This is the source of truth for app notifications (add/list/read/dismiss/
//! clear). It does NOT read other applications' Windows notifications: Windows
//! does not expose those to normal desktop apps. Optional Windows toasts are
//! best-effort via ps-windows and only for ProtonSearch-originated messages.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

const MAX_TITLE_CHARS: usize = 200;
const MAX_BODY_CHARS: usize = 8_000;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'info',
    read INTEGER NOT NULL DEFAULT 0,
    dismissed INTEGER NOT NULL DEFAULT 0,
    action_url TEXT
);
CREATE INDEX IF NOT EXISTS idx_notifications_timestamp
    ON notifications(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_unread
    ON notifications(read, dismissed);
";

/// One stored local notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationEntry {
    pub id: i64,
    pub timestamp_ms: i64,
    pub title: String,
    pub body: String,
    pub source: String,
    pub kind: String,
    pub read: bool,
    pub dismissed: bool,
    #[serde(default)]
    pub action_url: Option<String>,
}

/// Add a notification to the local store; returns its id.
pub fn add(
    db_path: &Path,
    title: &str,
    body: &str,
    source: &str,
    kind: &str,
) -> Result<i64> {
    let title = truncate(text_clean(title), MAX_TITLE_CHARS);
    let body = truncate(text_clean(body), MAX_BODY_CHARS);
    if title.trim().is_empty() {
        bail!("notification title is empty");
    }
    let conn = open_or_create(db_path)?;
    conn.execute(
        "INSERT INTO notifications(timestamp, title, body, source, kind, read, dismissed)
         VALUES(?1, ?2, ?3, ?4, ?5, 0, 0)",
        rusqlite::params![now_ms(), title, body, source, kind],
    )?;
    Ok(conn.last_insert_rowid())
}

/// List notifications newest-first (optionally including dismissed ones).
pub fn list(db_path: &Path, limit: usize, include_dismissed: bool) -> Vec<NotificationEntry> {
    let Ok(conn) = open_existing(db_path) else {
        return Vec::new();
    };
    let sql = if include_dismissed {
        "SELECT id, timestamp, title, body, source, kind, read, dismissed, action_url
         FROM notifications ORDER BY timestamp DESC, id DESC LIMIT ?1"
    } else {
        "SELECT id, timestamp, title, body, source, kind, read, dismissed, action_url
         FROM notifications WHERE dismissed = 0 ORDER BY timestamp DESC, id DESC LIMIT ?1"
    };
    let mut statement = match conn.prepare(sql) {
        Ok(statement) => statement,
        Err(_) => return Vec::new(),
    };
    let rows = statement.query_map([limit.max(1) as i64], map_entry);
    match rows {
        Ok(rows) => rows.flatten().collect(),
        Err(_) => Vec::new(),
    }
}

/// Number of unread, non-dismissed notifications.
pub fn unread_count(db_path: &Path) -> u64 {
    let Ok(conn) = open_existing(db_path) else {
        return 0;
    };
    conn.query_row(
        "SELECT COUNT(*) FROM notifications WHERE read = 0 AND dismissed = 0",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count.max(0) as u64)
    .unwrap_or(0)
}

pub fn mark_read(db_path: &Path, id: i64) -> Result<bool> {
    let conn = open_or_create(db_path)?;
    let changed = conn.execute("UPDATE notifications SET read = 1 WHERE id = ?1", [id])?;
    Ok(changed > 0)
}

pub fn dismiss(db_path: &Path, id: i64) -> Result<bool> {
    let conn = open_or_create(db_path)?;
    let changed = conn.execute(
        "UPDATE notifications SET dismissed = 1, read = 1 WHERE id = ?1",
        [id],
    )?;
    Ok(changed > 0)
}

/// Delete every notification (returns the number removed).
pub fn clear(db_path: &Path) -> Result<usize> {
    let conn = open_or_create(db_path)?;
    Ok(conn.execute("DELETE FROM notifications", [])?)
}

/// Store a notification and, when Windows accepts app toasts on this identity,
/// show it as a local toast. The store write always succeeds even when toasts
/// are unsupported; the toast failure is returned as part of the tuple.
pub fn add_and_maybe_toast(
    db_path: &Path,
    title: &str,
    body: &str,
    source: &str,
    kind: &str,
) -> Result<(i64, Option<String>)> {
    let id = add(db_path, title, body, source, kind)?;
    let toast_error = if ps_windows::toast_supported() {
        ps_windows::show_toast(title, body).err().map(|error| format!("{error:#}"))
    } else {
        Some("Windows toast notifications are not available for this app identity".to_string())
    };
    Ok((id, toast_error))
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<NotificationEntry> {
    Ok(NotificationEntry {
        id: row.get(0)?,
        timestamp_ms: row.get(1)?,
        title: row.get(2)?,
        body: row.get(3)?,
        source: row.get(4)?,
        kind: row.get(5)?,
        read: row.get::<_, i64>(6)? != 0,
        dismissed: row.get::<_, i64>(7)? != 0,
        action_url: row.get(8).unwrap_or_default(),
    })
}

fn text_clean(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .collect()
}

fn truncate(mut value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    let cutoff = value
        .char_indices()
        .nth(max_chars)
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    value.truncate(cutoff);
    value.push('…');
    value
}

fn open_existing(db_path: &Path) -> Result<rusqlite::Connection> {
    if !db_path.is_file() {
        bail!("database does not exist: {}", db_path.display());
    }
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let _ = conn.busy_timeout(std::time::Duration::from_millis(2000));
    Ok(conn)
}

fn open_or_create(db_path: &Path) -> Result<rusqlite::Connection> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    let conn = rusqlite::Connection::open(db_path)?;
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;
    conn.execute_batch(SCHEMA)
        .context("migrating notifications schema")?;
    Ok(conn)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
