//! Real-corpus indexing smoke tool (used for crash/perf evidence).
//!
//! Usage: cargo run -p ps-content --example index_corpus -- <root> <db-path> [max-files]
//!
//! It runs the production extraction pipeline over real files, reports indexed /
//! skipped / error counts (including parser panics that were contained), and
//! never opens a GUI. The process completing normally is the crash evidence.

use std::path::PathBuf;
use std::time::Instant;

use ps_content::{ContentIndex, DEFAULT_MAX_BYTES};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(args.get(1).cloned().unwrap_or_else(|| ".".to_string()));
    let db = PathBuf::from(
        args.get(2)
            .cloned()
            .unwrap_or_else(|| "target/ps-content-bench/smoke-index.db".to_string()),
    );
    let max_files: usize = args
        .get(3)
        .and_then(|value| value.parse().ok())
        .unwrap_or(5000);
    if !root.is_dir() {
        anyhow::bail!("root is not a directory: {}", root.display());
    }

    let mut paths = Vec::new();
    for entry in walkdir::WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        if paths.len() >= max_files {
            break;
        }
        if entry.file_type().is_file() && ps_content::is_supported(entry.path()) {
            paths.push(entry.path().to_path_buf());
        }
    }
    println!(
        "smoke: root={} db={} supported_files={}",
        root.display(),
        db.display(),
        paths.len()
    );

    let index = ContentIndex::open(&db)?;
    let started = Instant::now();
    let summary = index.index_paths(&paths, DEFAULT_MAX_BYTES);
    let elapsed = started.elapsed();
    let stats = index.stats();
    let errors = index.recent_errors(12);
    let panic_errors = errors
        .iter()
        .filter(|(_, error)| error.contains("panicked"))
        .count();

    println!(
        "smoke: indexed={} skipped={} errors={} documents={} panic_contained_in_last_12_errors={} elapsed_ms={:.0}",
        summary.indexed,
        summary.skipped,
        summary.errors,
        stats.documents,
        panic_errors,
        elapsed.as_secs_f64() * 1000.0
    );
    for (path, error) in &errors {
        println!("  error {}: {}", path, error.chars().take(160).collect::<String>());
    }
    println!("smoke: process completed normally (no abort)");
    Ok(())
}
