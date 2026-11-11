//! Parallel filesystem scanning into the compact index. OWNER: engine-core.
//!
//! walkdir is used for the single-root convenience API, while the real scan is
//! a breadth-first directory frontier processed with rayon: each level is
//! listed in parallel, then merged deterministically (sorted names) so the
//! resulting index is byte-for-byte reproducible. Reparse points/symlinks,
//! unreadable directories, junk directories and user exclusions are skipped;
//! cancellation is checked between bounded batches.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::matching::starts_with_ci;
use crate::{normalize_root, EntryKind, FilenameIndex, IndexBuilder, IndexEntry, ScanStats};

/// Directory names skipped by default because they are caches, VCS internals or
/// machine-generated trees that would swamp the index with useless names.
/// Override with ScanOptions::junk_dirs (empty slice = these defaults).
pub const DEFAULT_JUNK_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".svn",
    ".hg",
    "$RECYCLE.BIN",
    "System Volume Information",
    "__pycache__",
    ".venv",
    "venv",
    ".next",
    ".nuxt",
    ".cache",
    ".gradle",
    ".m2",
    ".terraform",
    ".cargo",
    ".rustup",
    ".nuget",
    ".npm",
    ".pnpm-store",
    ".yarn",
    ".idea",
    ".vs",
    "obj",
];

/// Well-known machine-generated paths excluded by default when they exist.
/// ps-core appends its own data directory to this list.
pub fn default_excluded_paths() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for var in ["TEMP", "TMP", "LOCALAPPDATA", "APPDATA"] {
        if let Some(base) = std::env::var_os(var) {
            let base = PathBuf::from(base);
            if var == "TEMP" || var == "TMP" {
                out.push(base.to_string_lossy().into_owned());
            } else if var == "LOCALAPPDATA" {
                out.push(base.join("Temp").to_string_lossy().into_owned());
                out.push(base.join("Packages").to_string_lossy().into_owned());
            } else if var == "APPDATA" {
                out.push(
                    base.join("protonsearch")
                        .join("v2")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    if let Some(windir) = std::env::var_os("WINDIR") {
        out.push(PathBuf::from(windir).join("Temp").to_string_lossy().into_owned());
    }
    out
}

/// Options for a direct root scan.
pub struct ScanOptions<'a> {
    pub roots: &'a [PathBuf],
    /// Absolute path prefixes or bare directory names to skip.
    pub excluded: &'a [String],
    /// Directory names to skip anywhere (empty = DEFAULT_JUNK_DIRS).
    pub junk_dirs: &'a [String],
    /// Hard cap on indexed entries (None = unbounded).
    pub max_entries: Option<usize>,
    /// Cooperative cancellation polled between directory batches.
    pub cancelled: &'a dyn Fn() -> bool,
    /// Bounded progress callback, called once per directory level with the
    /// number of entries collected so far.
    pub progress: &'a mut dyn FnMut(usize),
}

#[derive(Default)]
struct Exclusions {
    components: Vec<String>,
    prefixes: Vec<String>,
}

impl Exclusions {
    fn new(excluded: &[String], junk: &[String]) -> Self {
        let mut this = Self::default();
        for item in junk {
            let trimmed = item.trim();
            if !trimmed.is_empty() {
                this.components.push(trimmed.to_lowercase());
            }
        }
        for item in excluded {
            let trimmed = item.trim().trim_end_matches(['\\', '/']);
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.contains('\\') || trimmed.contains('/') {
                this.prefixes.push(trimmed.to_lowercase());
            } else {
                this.components.push(trimmed.to_lowercase());
            }
        }
        this
    }

    #[inline]
    fn skip_dir(&self, name: &str, path: &str) -> bool {
        if !self.components.is_empty() {
            let lower = name.to_lowercase();
            if self.components.iter().any(|c| *c == lower) {
                return true;
            }
        }
        if !self.prefixes.is_empty() {
            let lower = path.to_lowercase();
            if self.prefixes.iter().any(|p| is_prefix_or_equal(&lower, p)) {
                return true;
            }
        }
        false
    }
}

fn is_prefix_or_equal(path: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    if path.len() < prefix.len() {
        return false;
    }
    if !starts_with_ci(path, prefix) {
        return false;
    }
    path.len() == prefix.len() || {
        let b = path.as_bytes()[prefix.len()];
        b == b'\\' || b == b'/'
    }
}

struct RawFile {
    name: String,
    size: u64,
    modified: i64,
}

struct RawDir {
    path: String,
    name: String,
    modified: i64,
}

struct DirRead {
    path: String,
    files: Vec<RawFile>,
    dirs: Vec<RawDir>,
    errors: usize,
}

fn system_time_ms(t: std::io::Result<std::time::SystemTime>) -> i64 {
    match t {
        Ok(t) => t
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
        Err(_) => 0,
    }
}

#[cfg(windows)]
fn is_reparse(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse(_meta: &std::fs::Metadata) -> bool {
    false
}

fn read_one_dir(path: &str, excl: &Exclusions) -> Result<DirRead, ()> {
    let rd = std::fs::read_dir(path).map_err(|_| ())?;
    let mut out = DirRead {
        path: path.to_string(),
        files: Vec::new(),
        dirs: Vec::new(),
        errors: 0,
    };
    for entry in rd {
        let Ok(entry) = entry else {
            out.errors += 1;
            continue;
        };
        let name_os = entry.file_name();
        let name = name_os.to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            out.errors += 1;
            continue;
        };
        // Never follow symlinks/junctions: they can form cycles.
        if file_type.is_symlink() {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            out.errors += 1;
            continue;
        };
        if is_reparse(&meta) {
            continue;
        }
        if meta.is_dir() {
            let child = join_path(path, &name);
            if excl.skip_dir(&name, &child) {
                continue;
            }
            out.dirs.push(RawDir {
                path: child,
                name,
                modified: system_time_ms(meta.modified()),
            });
        } else if meta.is_file() {
            out.files.push(RawFile {
                name,
                size: meta.len(),
                modified: system_time_ms(meta.modified()),
            });
        }
    }
    out.files.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    out.dirs.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

pub(crate) fn join_path(parent: &str, name: &str) -> String {
    let mut out = String::with_capacity(parent.len() + 1 + name.len());
    out.push_str(parent);
    if !out.ends_with('\\') && !out.ends_with('/') {
        out.push(std::path::MAIN_SEPARATOR);
    }
    out.push_str(name);
    out
}

/// Scan configured roots in parallel and build a compact index.
pub fn scan(options: ScanOptions<'_>) -> (FilenameIndex, ScanStats) {
    let started = Instant::now();
    let junk_owned: Vec<String>;
    let junk: &[String] = if options.junk_dirs.is_empty() {
        junk_owned = DEFAULT_JUNK_DIRS.iter().map(|s| s.to_string()).collect();
        &junk_owned
    } else {
        options.junk_dirs
    };
    let excl = Exclusions::new(options.excluded, junk);
    let mut builder = IndexBuilder::new();
    let mut errors = 0usize;
    let mut cancelled = false;

    for root in options.roots {
        let root_str = normalize_root(root);
        if root_str.is_empty() {
            continue;
        }
        match std::fs::metadata(&root_str) {
            Ok(meta) if meta.is_dir() => {}
            _ => {
                errors += 1;
                continue;
            }
        }
        builder.intern_dir(&root_str);
        builder.roots.push(root_str.clone());
        let mut root_count = 0u64;
        let mut frontier: Vec<String> = vec![root_str];
        while !frontier.is_empty() {
            if (options.cancelled)() {
                cancelled = true;
                break;
            }
            let results: Vec<Result<DirRead, ()>> =
                frontier.par_iter().map(|dir| read_one_dir(dir, &excl)).collect();
            let mut next: Vec<String> = Vec::new();
            for result in results {
                let dr = match result {
                    Ok(dr) => dr,
                    Err(()) => {
                        errors += 1;
                        continue;
                    }
                };
                let parent_id = builder.intern_dir(&dr.path);
                errors += dr.errors;
                for f in dr.files {
                    builder.push_entry(parent_id, &f.name, EntryKind::File, f.size, f.modified);
                    root_count += 1;
                }
                for d in dr.dirs {
                    let _child_id = builder.intern_dir(&d.path);
                    builder.push_entry(parent_id, &d.name, EntryKind::Folder, 0, d.modified);
                    root_count += 1;
                    next.push(d.path);
                }
            }
            next.sort_unstable();
            next.dedup();
            frontier = next;
            (options.progress)(builder.len());
            if let Some(max) = options.max_entries {
                if builder.len() >= max {
                    break;
                }
            }
        }
        builder.root_counts.push(root_count);
        if cancelled {
            break;
        }
    }

    builder.errors += errors;
    let (index, mut stats) = builder.finish_with_stats();
    stats.elapsed_ms = started.elapsed().as_secs_f32() * 1000.0;
    (index, stats)
}

/// Walk roots and produce owned entries (compatibility API; prefer scan()).
///
/// Implementations must skip reparse-point loops, unreadable directories, and
/// user exclusions; they report progress through the callback and honour
/// cooperative cancellation.
pub fn scan_roots(
    roots: &[PathBuf],
    excluded: &[String],
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(usize),
) -> (Vec<IndexEntry>, ScanStats) {
    let (index, stats) = scan(ScanOptions {
        roots,
        excluded,
        junk_dirs: &[],
        max_entries: None,
        cancelled,
        progress,
    });
    let entries = index.entries().to_vec();
    (entries, stats)
}

/// Convenience: a single-root walkdir-based scan used by tests and tools that
/// only need paths (not used by the engine).
pub fn walk_paths(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempTree;

    #[test]
    fn scan_finds_files_and_folders_and_skips_junk() {
        let tree = TempTree::new("basic");
        tree.file("alpha.txt", "a");
        tree.file("sub/beta.pdf", "b");
        tree.file("node_modules/pkg/index.js", "junk");
        tree.file("sub/target/debug/out.bin", "junk");
        tree.file(".hidden/secret.md", "hidden");
        let roots = vec![tree.root.clone()];
        let mut progress_calls = 0;
        let (index, stats) = scan(ScanOptions {
            roots: &roots,
            excluded: &[],
            junk_dirs: &[],
            max_entries: None,
            cancelled: &|| false,
            progress: &mut |_| progress_calls += 1,
        });
        assert!(stats.errors == 0, "unexpected errors: {}", stats.errors);
        let mut paths: Vec<String> = (0..index.len())
            .map(|i| index.entry_path(i).into_owned())
            .collect();
        paths.sort();
        assert!(paths.iter().any(|p| p.ends_with("alpha.txt")));
        assert!(paths.iter().any(|p| p.ends_with("beta.pdf")));
        assert!(
            paths.iter().any(|p| p.ends_with("secret.md")),
            "hidden dirs are indexed"
        );
        assert!(!paths.iter().any(|p| p.contains("node_modules")));
        assert!(!paths.iter().any(|p| p.contains("sub\\target")));
        assert!(progress_calls > 0);
        // Folder entries are indexed too.
        let folders = (0..index.len())
            .filter(|&i| index.entry_kind(i) == EntryKind::Folder)
            .count();
        assert!(folders >= 2, "expected folder entries, got {folders}");
    }

    #[test]
    fn scan_honours_exclusions_and_cancellation() {
        let tree = TempTree::new("excl");
        for i in 0..20 {
            tree.file(&format!("dir{}/file{i}.txt", i % 4), "x");
        }
        let excluded = vec!["dir1".to_string()];
        let roots = vec![tree.root.clone()];
        let (index, _) = scan(ScanOptions {
            roots: &roots,
            excluded: &excluded,
            junk_dirs: &[],
            max_entries: None,
            cancelled: &|| false,
            progress: &mut |_| {},
        });
        let paths: Vec<String> = (0..index.len())
            .map(|i| index.entry_path(i).into_owned())
            .collect();
        assert!(!paths.iter().any(|p| p.contains("dir1")));

        // Cancellation stops the walk after the first level.
        let (index, _) = scan(ScanOptions {
            roots: &roots,
            excluded: &[],
            junk_dirs: &[],
            max_entries: None,
            cancelled: &|| true,
            progress: &mut |_| {},
        });
        assert_eq!(index.len(), 0);
    }

    #[test]
    fn scan_is_deterministic() {
        let tree = TempTree::new("determinism");
        for i in 0..50 {
            tree.file(&format!("d{}/f{i}.txt", i % 5), "x");
        }
        let roots = vec![tree.root.clone()];
        let run = || {
            let (index, _) = scan(ScanOptions {
                roots: &roots,
                excluded: &[],
                junk_dirs: &[],
                max_entries: None,
                cancelled: &|| false,
                progress: &mut |_| {},
            });
            let mut paths: Vec<String> = (0..index.len())
                .map(|i| index.entry_path(i).into_owned())
                .collect();
            paths.sort();
            paths
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn scan_roots_compat_returns_entries() {
        let tree = TempTree::new("compat");
        tree.file("a.txt", "a");
        tree.file("b.txt", "b");
        let roots = vec![tree.root.clone()];
        let (entries, stats) = scan_roots(&roots, &[], &|| false, &mut |_| {});
        assert_eq!(entries.len(), stats.entries);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.kind == EntryKind::File));
        assert!(entries.iter().any(|e| e.name == "a.txt"));
    }

    #[test]
    fn long_paths_are_indexed() {
        let tree = TempTree::new("longpath");
        let mut rel = String::new();
        for i in 0..8 {
            rel.push_str(&format!("segment_{:02}_", i));
            rel.push('/');
        }
        tree.file(&format!("{rel}leaf.txt"), "x");
        let roots = vec![tree.root.clone()];
        let (index, stats) = scan(ScanOptions {
            roots: &roots,
            excluded: &[],
            junk_dirs: &[],
            max_entries: None,
            cancelled: &|| false,
            progress: &mut |_| {},
        });
        assert_eq!(stats.errors, 0);
        let paths: Vec<String> = (0..index.len())
            .map(|i| index.entry_path(i).into_owned())
            .collect();
        assert!(paths.iter().any(|p| p.ends_with("leaf.txt")), "{paths:?}");
        let hits = index.search("leaf", 5);
        assert!(!hits.is_empty());
        assert_eq!(index.entry_name(hits[0].entry_index), "leaf.txt");
    }
}
