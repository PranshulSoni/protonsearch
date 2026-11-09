//! Integration tests for ps-sources against real Windows data and synthetic
//! browser/git/database fixtures created on this machine.

use std::path::{Path, PathBuf};
use std::process::Command;

use ps_sources::{
    browser, clipboard, git, quicklinks, BrowserEntry, ClipCategory, ClipboardEntry, GitEntry,
    Sources, WorkflowKind,
};

/// The OS clipboard is process-global; serialize tests that read/write it.
static CLIPBOARD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Calculator (module has its own unit tests; these exercise the public API)
// ---------------------------------------------------------------------------

#[test]
fn calculator_public_api_works() {
    assert_eq!(ps_sources::calc::evaluate("2+2").unwrap().value, 4.0);
    assert_eq!(ps_sources::calc::evaluate("15% of 340").unwrap().value, 51.0);
    assert!(ps_sources::calc::convert("5 km to mi").is_some());
    assert!(ps_sources::calc::evaluate("just a search phrase").is_none());
}

// ---------------------------------------------------------------------------
// Quicklinks + snippets
// ---------------------------------------------------------------------------

#[test]
fn quicklinks_roundtrip_and_delete() {
    let dir = workdir("quicklinks");
    let db = dir.join("index.db");
    quicklinks::save_quicklink(&db, "Docs", "https://example.com/docs", "docs").unwrap();
    quicklinks::save_snippet(&db, "Signature", "Best regards, Pranshul", "sig").unwrap();

    let entries = quicklinks::load(&db);
    assert_eq!(entries.len(), 2);
    let quicklink = entries
        .iter()
        .find(|entry| entry.kind == WorkflowKind::Quicklink)
        .unwrap();
    assert_eq!(quicklink.name, "Docs");
    assert_eq!(quicklink.id, "quicklink:Docs");
    assert_eq!(quicklink.payload, "https://example.com/docs");
    let snippet = entries
        .iter()
        .find(|entry| entry.kind == WorkflowKind::Snippet)
        .unwrap();
    assert_eq!(snippet.payload, "Best regards, Pranshul");

    assert!(quicklinks::delete(&db, "quicklink:Docs").unwrap());
    assert_eq!(quicklinks::load(&db).len(), 1);
    assert!(!quicklinks::delete(&db, "quicklink:Docs").unwrap());
}

#[test]
fn quicklinks_load_missing_db_returns_empty() {
    let dir = workdir("quicklinks-missing");
    assert!(quicklinks::load(&dir.join("nope.db")).is_empty());
}

// ---------------------------------------------------------------------------
// Clipboard history
// ---------------------------------------------------------------------------

#[test]
fn clipboard_records_dedupes_and_searches() {
    let dir = workdir("clipboard");
    let db = dir.join("index.db");
    assert!(clipboard::record(&db, "first copied text", "Code").unwrap());
    assert!(!clipboard::record(&db, "first copied text", "Code").unwrap(), "consecutive duplicate must be deduped");
    assert!(clipboard::record(&db, "second copied protonsearch text", "chrome").unwrap());
    assert!(!clipboard::record(&db, "   ", "x").unwrap());
    assert!(!clipboard::record(&db, &"x".repeat(2 * 1024 * 1024), "x").unwrap());

    let hits = clipboard::search(&db, "protonsearch", 10);
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert_eq!(hits[0].source_app, "chrome");

    let recent = clipboard::list_recent(&db, 10);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].text, "second copied protonsearch text");
    assert!(clipboard::set_pinned(&db, recent[0].id, true).unwrap());
    let pinned_first = clipboard::list_recent(&db, 10);
    assert!(pinned_first[0].pinned);
}

/// Build a PNG with known dimensions and a gradient (test fixture).
fn test_png(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&[
                (x % 256) as u8,
                (y % 256) as u8,
                ((x + y) % 256) as u8,
                255,
            ]);
        }
    }
    let buffer = image::RgbaImage::from_raw(width, height, pixels).expect("rgba buffer");
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    image::DynamicImage::ImageRgba8(buffer)
        .write_to(&mut cursor, image::ImageOutputFormat::Png)
        .expect("png encode");
    out
}

#[test]
fn clipboard_image_is_stored_as_png() {
    let dir = workdir("clipboard-image");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let png = test_png(32, 24);
    assert!(clipboard::record_image(&db, &png, "snippingtool", &images).unwrap());
    assert!(!clipboard::record_image(&db, &png, "snippingtool", &images).unwrap());
    let entries = clipboard::list_recent(&db, 5);
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.category, ClipCategory::Image);
    assert_eq!((entry.width, entry.height), (Some(32), Some(24)));
    assert!(entry.image_path.as_deref().map(|p| Path::new(p).is_file()).unwrap_or(false));
    let thumbnail = entry.thumbnail_path.as_deref().expect("thumbnail path");
    assert!(Path::new(thumbnail).is_file(), "thumbnail missing: {thumbnail}");
    let thumb = image::open(thumbnail).expect("decode thumbnail");
    assert!(thumb.width() <= 256 && thumb.height() <= 256);
    assert_eq!(entry.mime.as_deref(), Some("image/png"));
    assert_eq!(entry.format.as_deref(), Some("png"));
    let files: Vec<_> = std::fs::read_dir(&images).unwrap().flatten().collect();
    assert_eq!(files.len(), 2, "expected one full PNG and one thumbnail");
}

#[test]
fn clipboard_image_metadata_survives_restart_and_copies_back() {
    let _clip = CLIPBOARD_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let dir = workdir("clipboard-image-restart");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let png = test_png(40, 30);
    assert!(clipboard::record_image_captured(
        &db, &png, 40, 30, "dib", "image/png", "paint", &images
    )
    .unwrap());

    // Restart persistence: a fresh reconnect to the same DB returns the row and
    // the payload files are still valid.
    let entries = clipboard::list_recent(&db, 5);
    assert_eq!(entries.len(), 1);
    let entry = entries[0].clone();
    assert_eq!(entry.category, ClipCategory::Image);
    assert_eq!((entry.width, entry.height), (Some(40), Some(30)));
    let stored = image::open(entry.image_path.as_deref().expect("image path")).expect("decode");
    assert_eq!((stored.width(), stored.height()), (40, 30));

    // Copy/paste-back: restore to the real OS clipboard and read it again.
    clipboard::copy_to_clipboard(&db, entry.id).expect("copy image back to clipboard");
    let pasted = ps_windows::get_clipboard_image_png().expect("clipboard image after copy_to_clipboard");
    let pasted = image::load_from_memory(&pasted).expect("decode pasted png");
    assert_eq!((pasted.width(), pasted.height()), (40, 30));
}

#[test]
fn mixed_image_and_text_snapshot_is_one_image_item() {
    let dir = workdir("clipboard-mixed");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let png = test_png(16, 12);
    let snapshot = ps_windows::ClipboardSnapshot {
        text: Some("caption text from the same copy event".to_string()),
        png: Some(png),
        width: Some(16),
        height: Some(12),
        image_format: Some("dib".to_string()),
        mime: Some("image/png".to_string()),
        ..Default::default()
    };
    let ids = clipboard::record_snapshot(&db, &images, &snapshot).expect("record snapshot");
    assert_eq!(ids.len(), 1, "one history item per copy event");
    assert_eq!(clipboard::count(&db), 1);
    let entries = clipboard::list_recent(&db, 5);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, ClipCategory::Image);
    assert_eq!(entries[0].width, Some(16));
    assert_eq!(entries[0].height, Some(12));
    assert!(entries[0].image_path.is_some());
}

#[test]
fn files_snapshot_uses_files_category() {
    let dir = workdir("clipboard-files");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let snapshot = ps_windows::ClipboardSnapshot {
        files: vec![r"C:\Windows\win.ini".to_string(), r"C:\Windows\notepad.exe".to_string()],
        text: Some("C:\\Windows\\win.ini\r\nC:\\Windows\\notepad.exe".to_string()),
        ..Default::default()
    };
    let ids = clipboard::record_snapshot(&db, &images, &snapshot).expect("record files");
    assert_eq!(ids.len(), 1);
    let entries = clipboard::list_recent(&db, 5);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, ClipCategory::Files);
    assert_eq!(entries[0].format.as_deref(), Some("hdrop"));
    assert_eq!(entries[0].mime.as_deref(), Some("text/x-file-list"));
    assert_eq!(clipboard::search(&db, "notepad.exe", 5).len(), 1);
}

#[test]
fn clipboard_capture_options_filter_formats_not_master_gate() {
    let dir = workdir("clipboard-capture-options");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let snapshot = ps_windows::ClipboardSnapshot {
        text: Some("caption for the image copy".to_string()),
        png: Some(test_png(12, 10)),
        width: Some(12),
        height: Some(10),
        image_format: Some("dib".to_string()),
        mime: Some("image/png".to_string()),
        ..Default::default()
    };

    let images_only = clipboard::filter_snapshot(
        snapshot.clone(),
        clipboard::ClipboardCaptureOptions {
            capture_text: false,
            capture_images: true,
        },
    );
    assert!(images_only.png.is_some());
    assert!(images_only.text.is_none(), "captureText=false must drop text alternatives");

    let text_only = clipboard::filter_snapshot(
        snapshot.clone(),
        clipboard::ClipboardCaptureOptions {
            capture_text: true,
            capture_images: false,
        },
    );
    assert!(text_only.png.is_none(), "captureImages=false must drop the image");
    assert_eq!(text_only.text.as_deref(), Some("caption for the image copy"));

    let ids = clipboard::record_snapshot(&db, &images, &text_only).expect("persist text-only");
    assert_eq!(ids.len(), 1);
    let entries = clipboard::list_recent(&db, 5);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, ClipCategory::Text);
    assert!(entries[0].image_path.is_none());
}

#[test]
fn clipboard_thumbnail_is_within_ipc_contract() {
    let dir = workdir("clipboard-thumb-contract");
    let db = dir.join("index.db");
    let images = dir.join("images");
    // Large enough that a naive thumbnail would be big.
    let png = test_png(640, 480);
    assert!(clipboard::record_image_captured(
        &db, &png, 640, 480, "dib", "image/png", "paint", &images
    )
    .unwrap());
    let entry = clipboard::list_recent(&db, 1).remove(0);
    assert_eq!((entry.width, entry.height), (Some(640), Some(480)));
    assert_eq!(entry.mime.as_deref(), Some("image/png"));
    let thumbnail_path = entry.thumbnail_path.expect("thumbnail path");
    let thumbnail_bytes = std::fs::read(&thumbnail_path).expect("thumbnail file");
    assert!(
        thumbnail_bytes.len() <= 128 * 1024,
        "thumbnail payload exceeds 128 KiB: {} bytes",
        thumbnail_bytes.len()
    );
    let thumbnail = image::open(&thumbnail_path).expect("decode thumbnail");
    assert!(
        thumbnail.width() <= 128 && thumbnail.height() <= 128,
        "thumbnail exceeds 128px: {}x{}",
        thumbnail.width(),
        thumbnail.height()
    );
}

#[test]
fn clipboard_rejects_oversized_image_dimensions() {
    let dir = workdir("clipboard-oversized");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let png = test_png(4, 4);
    let result = clipboard::record_image_captured(
        &db, &png, 9000, 9000, "dib", "image/png", "test", &images,
    );
    assert!(result.is_err(), "dims above the IPC contract must be rejected");
    assert_eq!(clipboard::count(&db), 0);
}

#[test]
fn clipboard_copy_to_clipboard_text_roundtrip() {
    let _clip = CLIPBOARD_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let dir = workdir("clipboard-copy-text");
    let db = dir.join("index.db");
    assert!(clipboard::record(&db, "copy-to-clipboard test 42", "test").unwrap());
    let entry = clipboard::list_recent(&db, 1).remove(0);
    clipboard::copy_to_clipboard(&db, entry.id).expect("copy text back");
    assert_eq!(
        ps_windows::get_clipboard_text().as_deref(),
        Some("copy-to-clipboard test 42")
    );
}

#[test]
fn clipboard_delete_removes_image_files() {
    let dir = workdir("clipboard-delete-image");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let png = test_png(8, 8);
    assert!(clipboard::record_image_captured(&db, &png, 8, 8, "png", "image/png", "test", &images).unwrap());
    let entry = clipboard::list_recent(&db, 1).remove(0);
    let image_path = entry.image_path.clone().expect("image path");
    let thumb = entry.thumbnail_path.clone().expect("thumbnail path");
    assert!(clipboard::delete(&db, entry.id).unwrap());
    assert!(!Path::new(&image_path).exists(), "full PNG not deleted");
    assert!(!Path::new(&thumb).exists(), "thumbnail not deleted");
    assert_eq!(clipboard::count(&db), 0);
}

#[test]
fn clipboard_retention_keeps_history_bounded() {
    let dir = workdir("clipboard-retention");
    let db = dir.join("index.db");
    let total = clipboard::MAX_HISTORY_ITEMS + 5;
    for index in 0..total {
        assert!(clipboard::record(&db, &format!("retention item {index}"), "test").unwrap());
    }
    let count = clipboard::count(&db);
    assert!(
        count <= clipboard::MAX_HISTORY_ITEMS,
        "retention kept {count} rows, expected <= {}",
        clipboard::MAX_HISTORY_ITEMS
    );
}

#[test]
fn clipboard_categories_are_classified_and_returned() {
    let dir = workdir("clipboard-categories");
    let db = dir.join("index.db");
    assert!(clipboard::record(&db, "https://example.com/page", "browser").unwrap());
    assert!(clipboard::record(&db, "finance.team@example.com", "mail").unwrap());
    assert!(clipboard::record(&db, r"C:\Users\me\Documents\report.pdf", "explorer").unwrap());
    assert!(clipboard::record(&db, "SELECT * FROM users WHERE id = 1;", "editor").unwrap());
    assert!(clipboard::record(&db, "meeting notes for tomorrow", "notes").unwrap());
    assert_eq!(clipboard::count(&db), 5);

    let entries = clipboard::list_recent(&db, 10);
    let by_text: std::collections::HashMap<&str, ClipCategory> =
        entries.iter().map(|entry| (entry.text.as_str(), entry.category)).collect();
    assert_eq!(by_text["https://example.com/page"], ClipCategory::Link);
    assert_eq!(by_text["finance.team@example.com"], ClipCategory::Email);
    assert_eq!(by_text[r"C:\Users\me\Documents\report.pdf"], ClipCategory::Path);
    assert_eq!(by_text["SELECT * FROM users WHERE id = 1;"], ClipCategory::Code);
    assert_eq!(by_text["meeting notes for tomorrow"], ClipCategory::Text);

    // search() returns categories too.
    let hits = clipboard::search(&db, "https://example.com/page", 10);
    assert_eq!(hits.len(), 1, "hits={hits:?}");
    assert_eq!(hits[0].category, ClipCategory::Link);
    let email_hits = clipboard::search(&db, "finance.team", 10);
    assert_eq!(email_hits.len(), 1, "hits={email_hits:?}");
    assert_eq!(email_hits[0].category, ClipCategory::Email);

    // Wire/serde contract for engine-core.
    assert_eq!(serde_json::to_string(&ClipCategory::Path).unwrap(), "\"path\"");
    let decoded: ClipboardEntry = serde_json::from_str(
        r#"{"id":1,"text":"x","timestamp_ms":0,"source_app":"","pinned":false}"#,
    )
    .unwrap();
    assert_eq!(decoded.category, ClipCategory::Text, "missing category must default to text");
}

#[test]
fn clipboard_category_migration_is_idempotent() {
    let dir = workdir("clipboard-migration");
    let db = dir.join("index.db");
    // Old v2 schema: no category column yet.
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(
            r#"
CREATE TABLE clipboard_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT UNIQUE,
    timestamp INTEGER NOT NULL,
    source_app TEXT NOT NULL DEFAULT '',
    is_image INTEGER NOT NULL DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,
    ocr_text TEXT,
    image_path TEXT
);
INSERT INTO clipboard_history(content, timestamp, source_app, is_image, pinned)
    VALUES('https://old.example.com/x', 1000, 'old', 0, 0);
INSERT INTO clipboard_history(content, timestamp, source_app, is_image, pinned)
    VALUES('plain old note', 1001, 'old', 0, 0);
"#,
        )
        .unwrap();
    }

    // First write migrates the schema (ALTER TABLE) and backfills categories.
    assert!(clipboard::record(&db, r"C:\temp\new-file.txt", "test").unwrap());
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        let mut statement = conn.prepare("PRAGMA table_info(clipboard_history)").unwrap();
        let columns: Vec<String> = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .flatten()
            .collect();
        assert!(
            columns.iter().any(|column| column == "category"),
            "category column missing after migration: {columns:?}"
        );
    }

    let entries = clipboard::list_recent(&db, 10);
    assert_eq!(entries.len(), 3, "entries={entries:?}");
    let by_text: std::collections::HashMap<&str, ClipCategory> =
        entries.iter().map(|entry| (entry.text.as_str(), entry.category)).collect();
    assert_eq!(by_text["https://old.example.com/x"], ClipCategory::Link, "backfill link");
    assert_eq!(by_text["plain old note"], ClipCategory::Text, "backfill text");
    assert_eq!(by_text[r"C:\temp\new-file.txt"], ClipCategory::Path);
    assert_eq!(clipboard::count(&db), 3);

    // Idempotent: a second write after migration keeps working.
    assert!(clipboard::record(&db, "second plain note", "test").unwrap());
    assert_eq!(clipboard::count(&db), 4);
}

#[test]
fn clipboard_capture_current_seeds_the_history() {
    let _clip = CLIPBOARD_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let dir = workdir("clipboard-capture");
    let db = dir.join("index.db");
    let images = dir.join("images");

    ps_windows::set_clipboard_text("https://example.com/captured-link").expect("set clipboard");
    let captured = clipboard::capture_current(&db, &images)
        .expect("capture current clipboard")
        .expect("an entry");
    assert_eq!(captured.text, "https://example.com/captured-link");
    assert_eq!(captured.category, ClipCategory::Link);
    assert_eq!(clipboard::count(&db), 1);

    // Same consecutive content refreshes freshness without a second row.
    let again = clipboard::capture_current(&db, &images)
        .expect("capture again")
        .expect("an entry");
    assert_eq!(again.id, captured.id);
    assert_eq!(clipboard::count(&db), 1);

    ps_windows::set_clipboard_text("plain captured note").expect("set clipboard");
    let plain = clipboard::capture_current(&db, &images)
        .expect("capture plain text")
        .expect("an entry");
    assert_eq!(plain.category, ClipCategory::Text);
    assert_eq!(clipboard::count(&db), 2);
}

#[test]
fn clipboard_live_listener_records_new_copies() {
    let _clip = CLIPBOARD_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let dir = workdir("clipboard-listener");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let listener = clipboard::start_listener(db.clone(), images.clone()).expect("start listener");

    let marker = format!("live listener marker {}", std::process::id());
    ps_windows::set_clipboard_text(&marker).expect("set clipboard");
    let mut found = false;
    for _ in 0..60 {
        found = clipboard::search(&db, "live listener marker", 10)
            .iter()
            .any(|entry| entry.text == marker);
        if found {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    listener.stop();
    assert!(found, "clipboard listener did not record the new clipboard text");
    assert!(clipboard::count(&db) >= 1);
}

#[test]
fn clipboard_image_rows_are_categorised_as_image() {
    let dir = workdir("clipboard-image-category");
    let db = dir.join("index.db");
    let images = dir.join("images");
    let png: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
        0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
        0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
        0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    assert!(clipboard::record_image(&db, &png, "snippingtool", &images).unwrap());
    let recent = clipboard::list_recent(&db, 5);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].category, ClipCategory::Image);
    assert_eq!(clipboard::count(&db), 1);
    assert!(
        clipboard::search(&db, "image", 5).is_empty(),
        "text search must exclude image rows"
    );
}

#[test]
fn clipboard_count_and_reads_handle_missing_database() {
    let dir = workdir("clipboard-missing");
    let db = dir.join("absent.db");
    assert_eq!(clipboard::count(&db), 0);
    assert!(clipboard::list_recent(&db, 5).is_empty());
    assert!(clipboard::search(&db, "anything", 5).is_empty());
}

// ---------------------------------------------------------------------------
// Browser history / bookmarks (synthetic profiles)
// ---------------------------------------------------------------------------

#[test]
fn chromium_profile_history_and_bookmarks_are_read() {
    let dir = workdir("browser-chromium");
    let profile = dir.join("Default");
    std::fs::create_dir_all(&profile).unwrap();

    // Chromium epoch: 1601-01-01 in microseconds.
    let chrome_time: i64 = 11_644_473_600_000_000 + 1_760_000_000_000 * 1000;
    let conn = rusqlite::Connection::open(profile.join("History")).unwrap();
    conn.execute_batch(
        "CREATE TABLE urls (
            id INTEGER PRIMARY KEY,
            url TEXT,
            title TEXT,
            visit_count INTEGER,
            last_visit_time INTEGER,
            hidden INTEGER DEFAULT 0
        );",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO urls(url, title, visit_count, last_visit_time, hidden) VALUES
            ('https://example.com/rust', 'Rust docs', 7, ?1, 0),
            ('chrome-extension://abc/page.html', 'extension', 1, ?1, 0)",
        rusqlite::params![chrome_time],
    )
    .unwrap();
    drop(conn);

    let bookmarks = serde_json::json!({
        "roots": {
            "bookmark_bar": {
                "type": "folder",
                "children": [
                    {"type": "url", "name": "ProtonSearch", "url": "https://protonsearch.dev"},
                    {"type": "folder", "children": [
                        {"type": "url", "name": "Nested", "url": "https://example.com/nested"}
                    ]}
                ]
            }
        }
    });
    std::fs::write(
        profile.join("Bookmarks"),
        serde_json::to_vec(&bookmarks).unwrap(),
    )
    .unwrap();

    let cache = dir.join("cache");
    let (entries, warnings) =
        browser::scan_chromium_profile_with_cache(&profile, "Chrome", &cache);
    assert!(warnings.is_empty(), "warnings={warnings:?}");
    let history = entries
        .iter()
        .find(|entry| entry.url == "https://example.com/rust")
        .expect("history entry");
    assert_eq!(history.title, "Rust docs");
    assert_eq!(history.visit_count, 7);
    assert!(!history.bookmark);
    assert!(history.last_visit_ms > 0, "epoch conversion must produce unix ms");
    assert!(
        entries.iter().all(|entry| !entry.url.starts_with("chrome-extension://")),
        "extension urls must be filtered"
    );
    let bookmarks: Vec<&BrowserEntry> = entries.iter().filter(|entry| entry.bookmark).collect();
    assert_eq!(bookmarks.len(), 2, "nested bookmark must be found");
    assert!(bookmarks.iter().any(|entry| entry.url == "https://example.com/nested"));
}

#[test]
fn firefox_profile_history_and_bookmarks_are_read() {
    let dir = workdir("browser-firefox");
    let profile = dir.join("abc.default");
    std::fs::create_dir_all(&profile).unwrap();
    let conn = rusqlite::Connection::open(profile.join("places.sqlite")).unwrap();
    conn.execute_batch(
        "CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER, last_visit_date INTEGER);
         CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER, title TEXT);",
    )
    .unwrap();
    let firefox_us: i64 = 1_760_000_000_000_000;
    conn.execute(
        "INSERT INTO moz_places VALUES (1, 'https://mozilla.org', 'Mozilla', 3, ?1)",
        rusqlite::params![firefox_us],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO moz_bookmarks VALUES (1, 1, 1, 'Mozilla bookmarked')",
        [],
    )
    .unwrap();
    drop(conn);

    let cache = dir.join("cache");
    let (entries, warnings) = browser::scan_firefox_profile_with_cache(&profile, &cache);
    assert!(warnings.is_empty(), "warnings={warnings:?}");
    assert!(entries.iter().any(|entry| {
        entry.url == "https://mozilla.org" && entry.title == "Mozilla" && entry.visit_count == 3
    }));
    assert!(entries.iter().any(|entry| entry.bookmark && entry.title == "Mozilla bookmarked"));
}

#[test]
fn browser_privacy_setting_disables_scan() {
    let mut privacy = ps_ipc::PrivacySettings::default();
    privacy.browser_history = false;
    let (entries, warnings) = browser::scan_configured(&privacy);
    assert!(entries.is_empty());
    assert!(warnings.is_empty());
}

#[test]
fn epoch_conversion_helpers_are_correct() {
    // 1970-01-01 in chromium epoch.
    assert_eq!(browser::chromium_time_to_unix_ms(11_644_473_600_000_000), 0);
    assert_eq!(browser::chromium_time_to_unix_ms(0), 0);
    assert_eq!(browser::firefox_time_to_unix_ms(1_760_000_000_000_000), 1_760_000_000_000);
}

// ---------------------------------------------------------------------------
// Git
// ---------------------------------------------------------------------------

fn git_run(dir: &Path, args: &[&str]) -> bool {
    let mut command = Command::new("git");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .current_dir(dir)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[test]
fn git_repo_commits_and_todos_are_scanned() {
    let dir = workdir("git-repo");
    let repo = dir.join("sample-repo");
    std::fs::create_dir_all(&repo).unwrap();
    if !git_run(&repo, &["init", "-q"]) {
        eprintln!("SKIP: git CLI not available");
        return;
    }
    write_file(
        &repo,
        "src/main.rs",
        b"fn main() {}\n// TODO: improve ranking quality\n// FIXME: handle unicode\n",
    );
    git_run(&repo, &["add", "."]);
    let committed = git_run(
        &repo,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=Proton Test",
            "commit",
            "-qm",
            "initial commit",
        ],
    );
    if !committed {
        eprintln!("SKIP: git commit unavailable");
        return;
    }

    let (entries, warnings) = git::scan_repo(&repo);
    assert!(warnings.is_empty(), "warnings={warnings:?}");
    assert!(
        entries.iter().any(|entry| matches!(entry, GitEntry::Repo { name, .. } if name == "sample-repo")),
        "repo entry missing: {entries:?}"
    );
    assert!(entries.iter().any(|entry| matches!(entry, GitEntry::Commit { message, author, .. } if message == "initial commit" && author == "Proton Test")));
    let todos: Vec<&GitEntry> = entries
        .iter()
        .filter(|entry| matches!(entry, GitEntry::Todo { .. }))
        .collect();
    assert_eq!(todos.len(), 2, "todos={todos:?}");
    assert!(matches!(todos[0], GitEntry::Todo { line: 2, .. }));
}

#[test]
fn git_discovery_finds_repo_roots() {
    let dir = workdir("git-discovery");
    let repo = dir.join("projects").join("repo-one");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    // Junk directories must be skipped even if they contain .git.
    let junk = dir.join("projects").join("node_modules").join("fake-repo");
    std::fs::create_dir_all(junk.join(".git")).unwrap();
    let found = git::discover_repos(&dir.join("projects"), 8, 50);
    assert!(found.iter().any(|path| path.ends_with("repo-one")));
    assert!(!found.iter().any(|path| path.to_string_lossy().contains("node_modules")));
}

// ---------------------------------------------------------------------------
// Installed apps (real machine)
// ---------------------------------------------------------------------------

#[test]
fn scans_real_installed_apps() {
    let dir = workdir("apps-scan");
    let (apps, warnings) = ps_sources::apps::scan_with_icons(&dir.join("icons"));
    eprintln!(
        "apps discovered: {} (warnings: {})",
        apps.len(),
        warnings.len()
    );
    assert!(!apps.is_empty(), "expected at least one installed app");
    assert!(
        apps.iter().all(|app| !app.name.trim().is_empty() && !app.target.trim().is_empty()),
        "every app needs a name and target"
    );
    let mut keys = std::collections::HashSet::new();
    for app in &apps {
        let key = format!("{}|{}", app.name.to_lowercase(), app.target.to_lowercase());
        assert!(keys.insert(key), "duplicate app entry: {} {}", app.name, app.target);
    }
    let with_icons = apps.iter().filter(|app| app.icon_path.is_some()).count();
    eprintln!("apps with extracted icons: {with_icons}/{}", apps.len());
    assert!(with_icons > 0, "at least some real icons must extract");
}

#[test]
fn icon_cache_honours_portable_data_dir_and_populates_icons() {
    let dir = workdir("icons-portable");
    let data_dir = dir.join("portable-data");
    let previous = std::env::var_os("PROTONSEARCH_DATA_DIR");
    std::env::set_var("PROTONSEARCH_DATA_DIR", &data_dir);

    // Always restore the environment, even if an assertion below fails.
    let probe = std::panic::catch_unwind(|| {
        let cache = ps_sources::default_icon_cache_dir();
        assert_eq!(
            cache,
            data_dir.join("v2").join("cache").join("icons"),
            "icon cache must live under <PROTONSEARCH_DATA_DIR>/v2/cache/icons"
        );
        let (apps, warnings) = ps_sources::apps::scan_with_icons(&cache);
        (cache, apps, warnings)
    });
    match previous {
        Some(value) => std::env::set_var("PROTONSEARCH_DATA_DIR", value),
        None => std::env::remove_var("PROTONSEARCH_DATA_DIR"),
    }
    let (cache, apps, warnings) = probe.expect("icon scan must not panic");

    let with_icons = apps.iter().filter(|app| app.icon_path.is_some()).count();
    let uwp: Vec<_> = apps
        .iter()
        .filter(|app| app.keywords.iter().any(|keyword| keyword == "uwp"))
        .collect();
    let uwp_with = uwp.iter().filter(|app| app.icon_path.is_some()).count();
    let classic_total = apps.len().saturating_sub(uwp.len());
    let classic_with = with_icons.saturating_sub(uwp_with);
    eprintln!(
        "portable icon cache: apps={} with_icons={} classic={}/{} uwp={}/{} warnings={}",
        apps.len(),
        with_icons,
        classic_with,
        classic_total,
        uwp_with,
        uwp.len(),
        warnings.len()
    );

    assert!(!apps.is_empty(), "expected installed apps on this host");
    assert!(cache.is_dir(), "icon cache dir was not created: {}", cache.display());
    assert!(with_icons > 0, "at least one app must get a real icon");
    assert!(
        classic_with > 0,
        "Start-Menu .lnk apps must get real icons ({classic_with}/{classic_total})"
    );
    if !uwp.is_empty() {
        assert!(
            uwp_with > 0,
            "packaged/UWP apps must get real icons ({uwp_with}/{}); shell item factory failed",
            uwp.len()
        );
    }
    // Every reported icon_path must be a decodable-looking PNG file.
    for app in &apps {
        let Some(path) = app.icon_path.as_deref() else {
            continue;
        };
        let bytes = std::fs::read(path)
            .unwrap_or_else(|error| panic!("icon_path {} unreadable: {error}", path));
        assert!(
            bytes.len() > 8 && bytes.starts_with(&[0x89, b'P', b'N', b'G']),
            "icon_path {} is not a PNG",
            path
        );
    }
    let png_files = std::fs::read_dir(&cache)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0);
    eprintln!("portable icon cache files on disk: {png_files}");
    assert!(png_files > 0, "no PNGs were written to the icon cache");
}


/// Normalized display name for duplicate detection.
fn normalized_name(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Extensions the application index accepts for classic Start-Menu targets.
fn is_application_extension(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let extension = lower.rsplit('.').next().unwrap_or("");
    ["exe", "com", "bat", "cmd", "ps1", "psm1", "msc", "cpl", "scr", "appref-ms", "jar", "hta", "lnk"]
        .contains(&extension)
}

#[test]
fn app_index_has_no_duplicates_or_document_shortcuts() {
    let dir = workdir("app-index-hygiene");
    let cache = dir.join("icons");
    let (apps, warnings) = ps_sources::apps::scan_with_icons(&cache);
    eprintln!(
        "app index hygiene: apps={} warnings={}",
        apps.len(),
        warnings.len()
    );
    assert!(!apps.is_empty(), "expected installed apps on this host");

    // 1) One row per normalized display name (classic beats packaged).
    let mut names = std::collections::HashSet::new();
    for app in &apps {
        assert!(
            names.insert(normalized_name(&app.name)),
            "duplicate application row for {:?}",
            app.name
        );
    }

    // 2) No documentation/file shortcuts in the application index: classic
    // targets must be launchable, packaged targets must be shell AUMIDs.
    let mut packaged = 0usize;
    let mut classic = 0usize;
    for app in &apps {
        if app.target.starts_with("shell:") {
            packaged += 1;
        } else {
            classic += 1;
            assert!(
                is_application_extension(&app.target),
                "non-application target leaked into app index: {:?} ({})",
                app.name,
                app.target
            );
        }
    }
    eprintln!("app index composition: classic={classic} packaged={packaged}");
    assert!(classic > 0, "expected classic Start-Menu apps on this host");

    // 3) Icon cache is keyed by source identity: one icon file may only be
    // shared by entries that resolve to the same target.
    let mut icon_sources: std::collections::HashMap<&str, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for app in &apps {
        if let Some(icon) = app.icon_path.as_deref() {
            icon_sources
                .entry(icon)
                .or_default()
                .insert(app.target.to_lowercase());
        }
    }
    for (icon, sources) in &icon_sources {
        assert_eq!(
            sources.len(),
            1,
            "icon {:?} maps to multiple distinct sources: {:?}",
            icon,
            sources
        );
    }
    eprintln!(
        "app index icons: entries_with_icon={} unique_icon_files={}",
        apps.iter().filter(|app| app.icon_path.is_some()).count(),
        icon_sources.len()
    );
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[test]
fn actions_dispatch_is_structured_and_safe() {
    use ps_ipc::{ActionSpec, NativeCommand};

    // CopyText is a safe real action: verify the clipboard actually changed.
    let payload = format!("ps-sources action test {}", std::process::id());
    {
        let _clip = CLIPBOARD_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let outcome = ps_sources::actions::execute(&ActionSpec::CopyText {
            text: payload.clone(),
        });
        assert!(outcome.ok, "copy action failed: {}", outcome.message);
        assert_eq!(
            ps_windows::get_clipboard_text().as_deref(),
            Some(payload.as_str())
        );
    }

    // Missing paths return structured failures, never panics.
    let outcome = ps_sources::actions::execute(&ActionSpec::OpenPath {
        path: "C:\\definitely\\not\\here.txt".to_string(),
        line: None,
        column: None,
    });
    assert!(!outcome.ok);
    assert!(!outcome.message.is_empty());

    // Non-http url is rejected by the action layer too.
    let outcome = ps_sources::actions::execute(&ActionSpec::OpenUrl {
        url: "file:///C:/Windows/System32/calc.exe".to_string(),
    });
    assert!(!outcome.ok);

    // Indexing pause/resume are engine-owned acknowledgements.
    let outcome = ps_sources::actions::run_native(NativeCommand::PauseIndexing);
    assert!(outcome.ok, "{}", outcome.message);
    let outcome = ps_sources::actions::run_native(NativeCommand::OpenSettings);
    let _ = outcome;
}

#[test]
#[ignore = "invokes the real default file handler; run explicitly for E2E evidence"]
fn open_path_action_really_invokes_shell() {
    let dir = workdir("action-open-smoke");
    let marker = dir.join("marker.txt");
    let probe = dir.join("probe.cmd");
    // ShellExecuteW opens .cmd with the system handler (console host). The
    // script writes a marker file and exits immediately, proving invocation
    // without leaving any GUI application running.
    let script = format!(
        "@echo off\r\necho shell-executed > \"{}\"\r\n",
        marker.display()
    );
    std::fs::write(&probe, script).unwrap();
    let outcome = ps_sources::actions::execute(&ps_ipc::ActionSpec::OpenPath {
        path: probe.to_string_lossy().to_string(),
        line: None,
        column: None,
    });
    assert!(outcome.ok, "open_path action failed: {}", outcome.message);
    let mut seen = false;
    for _ in 0..50 {
        if marker.is_file() {
            seen = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        seen,
        "marker not written: ShellExecuteW did not run the default handler"
    );
    let _ = std::fs::remove_file(&probe);
    let _ = std::fs::remove_file(&marker);
}

// ---------------------------------------------------------------------------
// Aggregate load
// ---------------------------------------------------------------------------

#[test]
fn sources_load_returns_real_apps_and_respects_privacy_defaults() {
    let dir = workdir("sources-load");
    let settings = ps_ipc::Settings::default();
    let (sources, warnings) = Sources::load_with_paths(
        &settings,
        None,
        &dir.join("index.db"),
        &dir.join("icons"),
    );
    eprintln!(
        "sources: apps={} recent={} clipboard={} browser={} git={} workflows={} warnings={}",
        sources.apps.len(),
        sources.recent.len(),
        sources.clipboard.len(),
        sources.browser.len(),
        sources.git.len(),
        sources.workflows.len(),
        warnings.len()
    );
    assert!(!sources.apps.is_empty(), "apps source must load on a real machine");
    assert!(sources.recent.len() <= 400);
    assert!(sources.browser.is_empty(), "browser disabled by default");
    assert!(sources.git.is_empty(), "git disabled by default");
}