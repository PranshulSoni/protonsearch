//! Icon/app-index probe: bounded evidence tool for real application icons.
//!
//! Run with PROTONSEARCH_DATA_DIR set (portable/E2E mode) to prove the icon
//! cache is written under the override instead of %APPDATA%:
//!   $env:PROTONSEARCH_DATA_DIR = "target/ps-icon-probe-data"
//!   cargo run -p ps-sources --example icon_probe
//!
//! Reports counts (apps, icons, classic vs packaged), duplicate display names,
//! icon-source mapping violations, and a spot list for QA-named apps. No GUI.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

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
        "apps={} with_icons={} classic={}/{} uwp={}/{} elapsed_ms={:.0} warnings={}",
        apps.len(),
        with_icons,
        classic_with,
        classic_total,
        uwp_with,
        uwp.len(),
        elapsed.as_secs_f64() * 1000.0,
        warnings.len()
    );

    // Duplicate display names (must be zero after identity dedupe).
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for app in &apps {
        *by_name.entry(normalize_name(&app.name)).or_default() += 1;
    }
    let duplicates: Vec<_> = by_name.iter().filter(|(_, count)| **count > 1).collect();
    println!("duplicate display names: {}", duplicates.len());
    for (name, count) in duplicates.iter().take(10) {
        println!("  DUP {name:?} x{count}");
        for app in apps.iter().filter(|app| normalize_name(&app.name) == **name) {
            println!("    target={:?} icon={:?}", app.target, app.icon_path.as_deref());
        }
    }

    // Icon-source mapping: one cache file must map to exactly one source.
    let mut icon_sources: HashMap<&str, HashSet<String>> = HashMap::new();
    for app in &apps {
        if let Some(icon) = app.icon_path.as_deref() {
            icon_sources
                .entry(icon)
                .or_default()
                .insert(app.target.replace('/', "\\").to_lowercase());
        }
    }
    let violations: Vec<_> = icon_sources
        .iter()
        .filter(|(_, sources)| sources.len() > 1)
        .collect();
    println!(
        "unique icon paths: {} ; icon files mapped to >1 distinct source: {}",
        icon_sources.len(),
        violations.len()
    );
    for (icon, sources) in violations.iter().take(5) {
        println!(
            "  MAPPING VIOLATION {}: {sources:?}",
            icon.rsplit('\\').next().unwrap_or(icon)
        );
    }

    let mut verified = 0usize;
    let mut bad = 0usize;
    let mut missing = 0usize;
    let mut total_bytes = 0u64;
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

    // Spot list for the QA-named apps.
    for needle in [
        "chrome",
        "visual studio code",
        "notepad",
        "amd software",
        "about itunes",
    ] {
        let matches: Vec<_> = apps
            .iter()
            .filter(|app| normalize_name(&app.name).contains(needle))
            .collect();
        if matches.is_empty() {
            println!("spot {needle:?}: absent from app index");
        }
        for app in matches {
            println!(
                "spot {needle:?}: name={:?} target={:?} icon={:?}",
                app.name,
                app.target,
                app.icon_path.as_deref().and_then(|path| path.rsplit('\\').next())
            );
        }
    }
    for warning in warnings.iter().take(6) {
        println!("  warning: {warning}");
    }
}
