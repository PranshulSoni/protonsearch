//! `walk` subcommand: wall time of the real filesystem scanner (ps_index::scan)
//! over a directory tree. `--materialize` first creates a dataset's entries as
//! empty files/folders so the scanner can be timed on a realistic 100k tree.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;

use crate::args::Args;
use crate::dataset::DatasetEntry;

pub fn run(args: &Args) -> anyhow::Result<()> {
    let root = PathBuf::from(args.required("--root")?);
    let runs = args.usize_or("--runs", 3)?.max(1);
    if let Some(dataset) = args.get("--materialize") {
        materialize(Path::new(dataset), &root)?;
    }
    anyhow::ensure!(root.is_dir(), "root is not a directory: {}", root.display());
    let roots = [root.clone()];
    for run in 1..=runs {
        let label = if run == 1 { "cold" } else { "warm" };
        let started = Instant::now();
        let (index, stats) = ps_index::scan(ps_index::ScanOptions {
            roots: &roots,
            excluded: &[],
            junk_dirs: &[],
            max_entries: None,
            cancelled: &|| false,
            progress: &mut |_| {},
        });
        let secs = started.elapsed().as_secs_f64();
        let files = (0..index.len())
            .filter(|&i| index.entry_kind(i) == ps_index::EntryKind::File)
            .count();
        println!(
            "walk run {run} ({label}): entries={} files={files} folders={} errors={} wall_ms={:.0} files_per_s={:.0}",
            index.len(),
            index.len() - files,
            stats.errors,
            secs * 1000.0,
            files as f64 / secs
        );
    }
    Ok(())
}

/// Creates every dataset entry under `root`. Existing entries are skipped, so
/// repeated runs are cheap and idempotent.
fn materialize(dataset: &Path, root: &Path) -> anyhow::Result<()> {
    let started = Instant::now();
    let file = std::fs::File::open(dataset)
        .with_context(|| format!("open {}", dataset.display()))?;
    let (mut created, mut present, mut failed) = (0usize, 0usize, 0usize);
    let mut first_error: Option<String> = None;
    for (n, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let entry: DatasetEntry =
            serde_json::from_str(&line).with_context(|| format!("dataset line {}", n + 1))?;
        let target = root.join(relative(&entry.path));
        match create(&target, entry.is_folder()) {
            Ok(true) => created += 1,
            Ok(false) => present += 1,
            Err(err) => {
                failed += 1;
                first_error.get_or_insert_with(|| format!("{}: {err}", target.display()));
            }
        }
    }
    println!(
        "materialize: created={created} present={present} failed={failed} in {:.0} ms -> {}",
        started.elapsed().as_secs_f64() * 1000.0,
        root.display()
    );
    if let Some(err) = first_error {
        println!("materialize: first error: {err}");
    }
    Ok(())
}

/// Ok(true) when created, Ok(false) when it already existed.
fn create(target: &Path, folder: bool) -> std::io::Result<bool> {
    if folder {
        let existed = target.is_dir();
        std::fs::create_dir_all(target)?;
        return Ok(!existed);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match OpenOptions::new().write(true).create_new(true).open(target) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(err) => Err(err),
    }
}

/// Drops a leading drive ("C:\") so every dataset path lands under the root.
/// UNC paths keep their server/share components as directories.
fn relative(path: &str) -> &str {
    let b = path.as_bytes();
    if b.len() >= 3 && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') {
        &path[3..]
    } else {
        path.trim_start_matches(['\\', '/'])
    }
}
