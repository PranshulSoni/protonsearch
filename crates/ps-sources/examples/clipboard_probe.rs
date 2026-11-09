//! Clipboard pipeline probe: bounded evidence tool (no GUI).
//!
//! Exercises the real OS clipboard end-to-end: capture_current (seed path),
//! the live listener, categorization and count(). Uses its own database under
//! target/ so it never touches the user's v2 data.
//!
//!   cargo run -p ps-sources --example clipboard_probe

use std::path::PathBuf;
use std::time::Duration;

fn main() {
    let root = PathBuf::from("target/clipboard-probe");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create probe dir");
    let db = root.join("index.db");
    let images = root.join("images");

    println!("db: {}", db.display());
    println!("count before: {}", ps_sources::clipboard::count(&db));

    // 1) Seed path: capture_current stores whatever is on the clipboard now.
    ps_windows::set_clipboard_text("https://example.com/clipboard-probe").expect("set clipboard");
    match ps_sources::clipboard::capture_current(&db, &images) {
        Ok(Some(entry)) => println!(
            "capture_current -> id={} category={:?} text={:?}",
            entry.id,
            entry.category,
            entry.text
        ),
        other => println!("capture_current unexpected: {other:?}"),
    }

    // 2) Deterministic categories through the record API.
    for (text, label) in [
        ("review.team@example.com", "email"),
        (r"C:\Users\Probe\Documents\notes.txt", "path"),
        ("SELECT * FROM items WHERE id = 7;", "code"),
        ("buy milk on the way home", "text"),
    ] {
        let inserted = ps_sources::clipboard::record(&db, text, "probe").expect("record");
        println!("record {label}: inserted={inserted}");
    }
    println!("count after records: {}", ps_sources::clipboard::count(&db));
    for entry in ps_sources::clipboard::list_recent(&db, 10) {
        println!(
            "  recent id={} category={:?} pinned={} text={:?}",
            entry.id,
            entry.category,
            entry.pinned,
            entry.text
        );
    }

    // 3) Live listener: a new copy must appear in the DB.
    let listener = ps_sources::clipboard::start_listener(db.clone(), images.clone())
        .expect("start clipboard listener");
    let marker = "live listener probe marker 42";
    ps_windows::set_clipboard_text(marker).expect("set clipboard");
    let mut found = false;
    for _ in 0..50 {
        found = ps_sources::clipboard::search(&db, "listener probe", 5)
            .iter()
            .any(|entry| entry.text == marker);
        if found {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    listener.stop();
    println!("live listener captured new copy: {found}");
    println!("count final: {}", ps_sources::clipboard::count(&db));
    if !found {
        std::process::exit(2);
    }
}
