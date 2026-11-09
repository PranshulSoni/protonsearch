//! Tests for the local notification store (task-35 milestone 1).

use std::path::{Path, PathBuf};

use ps_sources::notifications;

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn notifications_store_roundtrip_and_counts() {
    let dir = workdir("notifications-store");
    let db = dir.join("index.db");

    assert_eq!(notifications::unread_count(&db), 0);
    assert!(notifications::list(&db, 10, true).is_empty());

    let first = notifications::add(&db, "Index ready", "Your documents are indexed.", "indexer", "info").unwrap();
    let second = notifications::add(&db, "Agent finished", "The workflow completed.", "agent", "success").unwrap();
    assert_eq!(notifications::unread_count(&db), 2);

    let entries = notifications::list(&db, 10, false);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, second, "newest first");
    assert_eq!(entries[1].id, first);
    assert!(!entries[0].read);
    assert!(!entries[0].dismissed);

    assert!(notifications::mark_read(&db, first).unwrap());
    assert_eq!(notifications::unread_count(&db), 1);

    assert!(notifications::dismiss(&db, second).unwrap());
    assert_eq!(notifications::list(&db, 10, false).len(), 1, "dismissed hidden by default");
    assert_eq!(notifications::list(&db, 10, true).len(), 2);
    assert_eq!(notifications::unread_count(&db), 0, "dismissed is not unread");

    assert_eq!(notifications::clear(&db).unwrap(), 2);
    assert!(notifications::list(&db, 10, true).is_empty());
}

#[test]
fn notifications_validate_and_truncate_input() {
    let dir = workdir("notifications-validate");
    let db = dir.join("index.db");
    assert!(notifications::add(&db, "   ", "body", "test", "info").is_err(), "empty title rejected");
    let long = "x".repeat(20_000);
    let id = notifications::add(&db, "Long", &long, "test", "info").unwrap();
    let entry = notifications::list(&db, 1, true).remove(0);
    assert_eq!(entry.id, id);
    assert!(entry.body.chars().count() <= 8_001, "body must be truncated");
}

#[test]
fn notifications_toast_fallback_is_honest() {
    let dir = workdir("notifications-toast");
    let db = dir.join("index.db");
    let (id, toast_error) = notifications::add_and_maybe_toast(
        &db,
        "ProtonSearch",
        "Local notification store test",
        "test",
        "info",
    )
    .unwrap();
    assert!(id > 0, "store write must succeed even when toasts are unavailable");
    // Windows toasts may be unavailable for unpackaged apps; when they are we
    // must report why instead of pretending success.
    if ps_windows::toast_supported() {
        eprintln!("toast supported on this host (error={toast_error:?})");
    } else {
        assert!(toast_error.is_some(), "unsupported toast must carry a reason");
    }
    assert_eq!(notifications::unread_count(&db), 1);
}
