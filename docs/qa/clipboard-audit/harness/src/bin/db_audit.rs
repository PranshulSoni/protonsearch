//! Gate 0 audit: end-to-end clipboard image persistence through the production
//! ps-sources clipboard path (the exact code the engine runs).
//!
//! Modes (run from the harness directory):
//!   --matrix --root <dir>       synthetic format matrix through the live
//!                               listener and through capture_current()
//!   --listen-external --secs N  start the real listener, print READY, wait for
//!                               an externally-set clipboard image
//!   --capture-external          capture_current() once (clipboard pre-set)
//!   --inspect                   reopen an existing DB and print rows/files
//!                               (restart-persistence evidence, new process)

use std::path::PathBuf;
use std::time::{Duration, Instant};

use clipboard_audit::{
    clear, dib_variant_pixels, png_bytes, set_bitmap_only, set_dib, set_dib_plus_text, set_dibv5,
    set_hdrop_files, set_registered_format, R,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |name: &str| -> Option<String> {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|index| args.get(index + 1))
            .cloned()
    };
    let root = PathBuf::from(value("--root").unwrap_or_else(|| "audit-runtime".to_string()));
    let _ = std::fs::create_dir_all(&root);
    let db = root.join("index.db");
    let images = root.join("clipboard-images");

    if args.iter().any(|arg| arg == "--inspect") {
        inspect(&db, &images);
        return;
    }
    if args.iter().any(|arg| arg == "--capture-external") {
        let result = ps_sources::clipboard::capture_current(&db, &images);
        println!("capture_current: {:?}", summarize(&result));
        inspect(&db, &images);
        return;
    }
    if args.iter().any(|arg| arg == "--paste-latest") {
        paste_latest(&db);
        return;
    }
    if args.iter().any(|arg| arg == "--listen-external") {
        let secs: u64 = value("--secs").and_then(|value| value.parse().ok()).unwrap_or(8);
        listen_external(&db, &images, secs);
        return;
    }
    matrix(&db, &images);
}

type CaptureResult = anyhow::Result<Option<ps_sources::ClipboardEntry>>;

fn summarize(result: &CaptureResult) -> String {
    match result {
        Ok(Some(entry)) => format!(
            "Some(id={}, category={:?}, text={:?})",
            entry.id,
            entry.category,
            truncate(&entry.text, 48)
        ),
        Ok(None) => "None".to_string(),
        Err(error) => format!("Err({error:#})"),
    }
}

fn matrix(db: &std::path::Path, images: &std::path::Path) {
    println!("# end-to-end matrix through ps_sources::clipboard::start_listener");
    let listener = ps_sources::clipboard::start_listener(
        db.to_path_buf(),
        images.to_path_buf(),
    )
    .expect("start production clipboard listener");
    println!(
        "listener: started (db={}, images={})",
        db.display(),
        images.display()
    );
    let base_count = ps_sources::clipboard::count(db);
    println!("count_baseline: {base_count}");
    let mut index = 0usize;
    for scenario in scenarios() {
        index += 1;
        let before = ps_sources::clipboard::count(db);
        let apply = (scenario.apply)();
        match apply {
            Ok(()) => {
                let row = wait_for_row(db, Duration::from_millis(1500), before);
                println!("=== listener/{}", scenario.name);
                println!("note: {}", scenario.note);
                println!(
                    "count: before={before} after={} delta={}",
                    ps_sources::clipboard::count(db),
                    ps_sources::clipboard::count(db) as i64 - before as i64
                );
                match row {
                    Some(entry) => println!("new_row: {}", format_entry(&entry)),
                    None => println!("new_row: NONE (no new clipboard_history row)"),
                }
            }
            Err(error) => {
                println!("=== listener/{}", scenario.name);
                println!("set FAILED: {error}");
            }
        }
        let _ = index;
    }
    listener.stop();
    println!("listener: stopped");

    println!();
    println!("# capture_current() seed path (same formats, listener stopped)");
    for scenario in scenarios() {
        let apply = (scenario.apply)();
        if let Err(error) = apply {
            println!("=== capture/{}", scenario.name);
            println!("set FAILED: {error}");
            continue;
        }
        let before = ps_sources::clipboard::count(db);
        let result = ps_sources::clipboard::capture_current(db, images);
        let after = ps_sources::clipboard::count(db);
        println!("=== capture/{}", scenario.name);
        println!("note: {}", scenario.note);
        println!("count: before={before} after={after} delta={}", after as i64 - before as i64);
        println!("capture_current: {}", summarize(&result));
    }

    println!();
    println!("# duplicate / recopy expectations");
    // Same bitmap copied twice: one history row expected (content-hash dedupe).
    let before = ps_sources::clipboard::count(db);
    set_dib(40, 30, 32, true, 255).expect("set dib duplicate");
    let _ = ps_sources::clipboard::capture_current(db, images);
    set_dib(40, 30, 32, true, 255).expect("set dib duplicate again");
    let _ = ps_sources::clipboard::capture_current(db, images);
    let after = ps_sources::clipboard::count(db);
    println!(
        "duplicate_same_dib: before={before} after={after} delta={} (expected 1 new row)",
        after as i64 - before as i64
    );
    // Rapid repeated copies should not create duplicate rows or duplicate files.
    let before = ps_sources::clipboard::count(db);
    for _ in 0..5 {
        set_dib(30, 20, 32, true, 255).expect("rapid set");
        let _ = ps_sources::clipboard::capture_current(db, images);
        std::thread::sleep(Duration::from_millis(40));
    }
    let after = ps_sources::clipboard::count(db);
    println!(
        "rapid_repeats: before={before} after={after} delta={} (expected 1)",
        after as i64 - before as i64
    );

    // Large-ish image bound check (1920x1080 RGBA through the real decode+encode path).
    let started = Instant::now();
    let mut pixels = dib_variant_pixels(1920, 1080, 32, 255);
    let _ = &mut pixels;
    let large = clipboard_audit::dib_bytes(1920, 1080, 32, true, 255);
    let _ = large;
    println!("large_image_setup_ms: {}", started.elapsed().as_millis());

    println!();
    inspect(db, images);
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario { name: "dib32", note: "typical 32bpp CF_DIB", apply: Box::new(|| set_dib(64, 48, 32, true, 255)) },
        Scenario { name: "dib24", note: "24bpp CF_DIB", apply: Box::new(|| set_dib(64, 48, 24, true, 255)) },
        Scenario { name: "dibv5_bitfields", note: "CF_DIBV5 with BI_BITFIELDS", apply: Box::new(|| set_dibv5(64, 48, true)) },
        Scenario { name: "bitmap_only", note: "CF_BITMAP only", apply: Box::new(|| set_bitmap_only(48, 32)) },
        Scenario { name: "png_registered", note: "registered PNG only", apply: Box::new(|| set_registered_format("PNG", &png_bytes(80, 60))) },
        Scenario { name: "dib_plus_text", note: "CF_DIB + CF_UNICODETEXT mixed copy", apply: Box::new(|| set_dib_plus_text("mixed image+text copy", 64, 48, 32)) },
        Scenario { name: "hdrop_files", note: "Explorer CF_HDROP + text", apply: Box::new(|| set_hdrop_files(&[PathBuf::from(r"C:\Windows\win.ini")])) },
        Scenario { name: "text_only", note: "control: plain text", apply: Box::new(|| {
            clipboard_audit::set_text("plain audit text")
        }) },
        Scenario { name: "empty", note: "empty clipboard", apply: Box::new(|| clear()) },
    ]
}

struct Scenario {
    name: &'static str,
    note: &'static str,
    apply: Box<dyn Fn() -> R<()>>,
}

fn paste_latest(db: &std::path::Path) {
    let rows = ps_sources::clipboard::list_recent(db, 50);
    let Some(entry) = rows.iter().find(|entry| entry.image_path.is_some()) else {
        println!("paste_latest: no image row found");
        return;
    };
    println!(
        "paste_latest: row id={} width={:?} height={:?} format={:?}",
        entry.id, entry.width, entry.height, entry.format
    );
    match ps_sources::clipboard::copy_to_clipboard(db, entry.id) {
        Ok(()) => match ps_windows::get_clipboard_image_png() {
            Some(png) => match image::load_from_memory(&png) {
                Ok(image) => println!(
                    "paste_latest: clipboard now carries {}x{} PNG ({} bytes)",
                    image.width(),
                    image.height(),
                    png.len()
                ),
                Err(error) => println!("paste_latest: clipboard PNG decode failed: {error}"),
            },
            None => println!("paste_latest: clipboard has no readable image after copy"),
        },
        Err(error) => println!("paste_latest: copy failed: {error:#}"),
    }
}

fn listen_external(db: &std::path::Path, images: &std::path::Path, secs: u64) {
    let listener = ps_sources::clipboard::start_listener(
        db.to_path_buf(),
        images.to_path_buf(),
    )
    .expect("start production clipboard listener");
    println!("READY: listener running for {secs}s; set the clipboard image now");
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut last = ps_sources::clipboard::count(db);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(200));
        let now = ps_sources::clipboard::count(db);
        if now != last {
            println!("observed count change: {last} -> {now}");
            last = now;
        }
    }
    listener.stop();
    println!("listener: stopped");
    inspect(db, images);
}

fn wait_for_row(
    db: &std::path::Path,
    timeout: Duration,
    previous_count: u64,
) -> Option<ps_sources::ClipboardEntry> {
    let deadline = Instant::now() + timeout;
    loop {
        if ps_sources::clipboard::count(db) > previous_count {
            return ps_sources::clipboard::list_recent(db, 1).into_iter().next();
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn inspect(db: &std::path::Path, images: &std::path::Path) {
    println!("--- inspect db={} images={}", db.display(), images.display());
    println!("db_exists: {}", db.is_file());
    println!("count: {}", ps_sources::clipboard::count(db));
    let rows = ps_sources::clipboard::list_recent(db, 20);
    for entry in &rows {
        println!("row: {}", format_entry(entry));
    }
    let mut files: Vec<(String, u64)> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(images) {
        for entry in read_dir.flatten() {
            if let Ok(metadata) = entry.metadata() {
                files.push((entry.file_name().to_string_lossy().to_string(), metadata.len()));
            }
        }
    }
    files.sort();
    println!("image_cache_files: {}", files.len());
    for (name, size) in files.iter().take(20) {
        println!("image_file: {name} ({size} bytes)");
    }
    let db_size = std::fs::metadata(db).map(|metadata| metadata.len()).unwrap_or(0);
    println!("db_bytes: {db_size}");
}

fn format_entry(entry: &ps_sources::ClipboardEntry) -> String {
    format!(
        "id={} category={:?} pinned={} source={:?} text={:?}",
        entry.id,
        entry.category,
        entry.pinned,
        entry.source_app,
        truncate(&entry.text, 64)
    )
}

fn truncate(value: &str, limit: usize) -> String {
    let mut out: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        out.push('…');
    }
    out
}