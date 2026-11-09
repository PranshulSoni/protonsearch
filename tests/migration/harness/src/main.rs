//! Migration contract probe (dev-only).
//!
//! Calls the real ps_core::migration::migrate_legacy_db API and emits a JSON
//! report so tests/migration can assert the contract without duplicating the
//! import logic. Never writes to the legacy database; dry-run is the default.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ps_core::migration::{migrate_legacy_db, MigrationOptions};
use sha2::{Digest, Sha256};

struct Args {
    legacy: Option<PathBuf>,
    v2: Option<PathBuf>,
    dry_run: bool,
    json_out: Option<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Self { legacy: None, v2: None, dry_run: true, json_out: None }
    }
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--legacy" => args.legacy = it.next().map(PathBuf::from),
            "--v2" => args.v2 = it.next().map(PathBuf::from),
            "--json-out" => args.json_out = it.next().map(PathBuf::from),
            "--dry-run" => args.dry_run = true,
            "--import" => args.dry_run = false,
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }
    args
}

fn sha256_file(path: &Path) -> Option<String> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::with_capacity(1024 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return None,
        }
    }
    Some(format!("{:x}", hasher.finalize()))
}

fn dir_snapshot(dir: &Path) -> Vec<(String, u64)> {
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
            entries.push((name, len));
        }
    }
    entries.sort();
    entries
}

fn main() -> ExitCode {
    let args = parse_args();
    let Some(legacy) = args.legacy.clone() else {
        eprintln!("missing --legacy <path>");
        return ExitCode::from(2);
    };
    if !legacy.is_file() {
        eprintln!("legacy database not found: {}", legacy.display());
        return ExitCode::from(2);
    }
    let v2 = args.v2.clone().unwrap_or_else(|| {
        legacy
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("v2-migration-probe.db")
    });
    let legacy_dir = legacy.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let wal = PathBuf::from(format!("{}-wal", legacy.display()));
    let shm = PathBuf::from(format!("{}-shm", legacy.display()));

    let db_before = sha256_file(&legacy);
    let wal_before = sha256_file(&wal);
    let shm_before = sha256_file(&shm);
    let dir_before = dir_snapshot(&legacy_dir);
    let v2_existed_before = v2.exists();

    let options = MigrationOptions {
        import_clipboard: false,
        import_ai_keys: false,
        dry_run: args.dry_run,
    };
    let started = std::time::Instant::now();
    let outcome = migrate_legacy_db(&legacy, &v2, &options);
    let elapsed = started.elapsed().as_millis() as u64;

    let db_after = sha256_file(&legacy);
    let wal_after = sha256_file(&wal);
    let shm_after = sha256_file(&shm);
    let dir_after = dir_snapshot(&legacy_dir);
    let new_files: Vec<String> = dir_after
        .iter()
        .filter(|entry| !dir_before.contains(entry))
        .map(|entry| entry.0.clone())
        .collect();
    let v2_exists_after = v2.exists();

    let mut json = serde_json::json!({
        "ok": outcome.is_ok(),
        "dry_run": args.dry_run,
        "legacy": legacy.display().to_string(),
        "v2": v2.display().to_string(),
        "v2_existed_before": v2_existed_before,
        "v2_exists_after": v2_exists_after,
        "elapsed_ms": elapsed,
        "legacy_files": {
            "db_before": db_before,
            "db_after": db_after,
            "db_unchanged": db_before.is_some() && db_before == db_after,
            "wal_before": wal_before,
            "wal_after": wal_after,
            "wal_unchanged": wal_before == wal_after,
            "shm_before": shm_before,
            "shm_after": shm_after,
            "shm_unchanged": shm_before == shm_after,
            "dir_new_files": new_files
        }
    });

    match outcome {
        Ok(report) => {
            json["report"] = serde_json::json!({
                "files": report.files,
                "dirs": report.dirs,
                "content_docs": report.content_docs,
                "browser_items": report.browser_items,
                "quicklinks": report.quicklinks,
                "snippets": report.snippets,
                "memory_events": report.memory_events,
                "skipped_tables": report.skipped_tables,
                "source_unchanged": report.source_unchanged,
                "duration_ms": report.duration_ms,
                "warnings": report.warnings
            });
        }
        Err(error) => {
            json["error"] = serde_json::Value::String(format!("{error:#}"));
        }
    }

    let rendered = serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".to_string());
    println!("{rendered}");
    if let Some(out) = &args.json_out {
        let _ = std::fs::write(out, rendered.as_bytes());
    }

    let db_ok = json["legacy_files"]["db_unchanged"].as_bool().unwrap_or(false);
    let report_ok = json["report"]["source_unchanged"].as_bool().unwrap_or(false);
    let dry_run_ok = !args.dry_run || !v2_exists_after;
    if json["ok"].as_bool().unwrap_or(false) && db_ok && report_ok && dry_run_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
