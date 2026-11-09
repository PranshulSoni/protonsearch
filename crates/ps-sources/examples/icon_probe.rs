//! Icon cache probe: bounded evidence tool for real application icons.
//!
//! Run with PROTONSEARCH_DATA_DIR set (portable/E2E mode) to prove the icon
//! cache is written under the override instead of %APPDATA%:
//!   $env:PROTONSEARCH_DATA_DIR = "target/ps-icon-probe-data"
//!   cargo run -p ps-sources --example icon_probe
//!
//! It prints total apps, icon coverage split between Start-Menu apps and
//! packaged/UWP apps, verifies every icon_path is a real PNG file, and lists
//! bounded warnings. No GUI is launched.

use std::path::Path;
use std::time::Instant;

fn main() {
    println!(
        "PROTONSEARCH_DATA_DIR={:?}",
        std::env::var_os("PROTONSEARCH_DATA_DIR")
    );
    let cache = ps_sources::default_icon_cache_dir();
    println!("resolved icon cache dir: {}", cache.display());

    let started = Instant::now();
    let (apps, warnings) = ps_sources::apps::scan_with_icons(&cache);
    let elapsed = started.elapsed();

    let with_icons = apps.iter().filter(|app| app.icon_path.is_some()).count();
    let uwp: Vec<_> = apps
        .iter()
        .filter(|app| app.keywords.iter().any(|keyword| keyword == "uwp"))
        .collect();
    let uwp_with = uwp.iter().filter(|app| app.icon_path.is_some()).count();
    let classic_total = apps.len().saturating_sub(uwp.len());
    let classic_with = with_icons.saturating_sub(uwp_with);

    println!(
        "apps={} with_icons={} classic={}/{} uwp={}/{} elapsed_ms={:.0}",
        apps.len(),
        with_icons,
        classic_with,
        classic_total,
        uwp_with,
        uwp.len(),
        elapsed.as_secs_f64() * 1000.0
    );

    let mut verified = 0usize;
    let mut bad = 0usize;
    let mut total_bytes = 0u64;
    let mut missing = 0usize;
    for app in &apps {
        let Some(path) = app.icon_path.as_deref() else {
            continue;
        };
        match std::fs::read(path) {
            Ok(bytes) if bytes.len() > 8 && bytes.starts_with(&[0x89, b'P', b'N', b'G']) => {
                verified += 1;
                total_bytes += bytes.len() as u64;
            }
            Ok(_) => bad += 1,
            Err(_) => missing += 1,
        }
    }
    println!(
        "icons verified_png={} bad_png={} missing_file={} total_bytes={}",
        verified, bad, missing, total_bytes
    );

    let on_disk = std::fs::read_dir(&cache)
        .map(|entries| entries.flatten().count())
        .unwrap_or(0);
    println!("icons on disk in cache dir: {on_disk}");
    for warning in warnings.iter().take(8) {
        println!("  warning: {warning}");
    }
    println!("total warnings: {}", warnings.len());

    if !Path::new(&cache).exists() {
        eprintln!("PROBE FAILURE: icon cache dir was not created: {}", cache.display());
        std::process::exit(2);
    }
}
