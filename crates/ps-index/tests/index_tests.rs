//! Integration tests for the ps-index public contract: 10k-entry correctness,
//! determinism, Unicode/CJK/long paths, filters, persistence and ranking order.

use std::path::PathBuf;

use ps_index::{
    scan, EntryFilter, EntryKind, FilenameIndex, IndexEntry, ScanOptions,
};

struct TempTree {
    root: PathBuf,
}

fn base_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PROTONSEARCH_TEST_DIR") {
        return PathBuf::from(dir);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("target").join("psidx-tests"))
        .unwrap_or_else(std::env::temp_dir)
}

impl TempTree {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = base_dir().join(format!("it-{}-{}-{}", std::process::id(), n, tag));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn file(&self, rel: &str, contents: &str) -> PathBuf {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn entry(path: &str, kind: EntryKind, size: u64, modified: i64) -> IndexEntry {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
    let ext = name
        .rfind('.')
        .filter(|&i| i > 0)
        .map(|i| name[i + 1..].to_string())
        .unwrap_or_default();
    IndexEntry {
        path: path.to_string(),
        name,
        ext,
        kind,
        size,
        modified,
    }
}

fn synthetic_10k() -> Vec<IndexEntry> {
    let words = [
        "report", "budget", "invoice", "design", "notes", "meeting", "project", "summary",
        "analysis", "proposal", "draft", "final", "photo", "screenshot", "backup", "archive",
    ];
    let exts = ["pdf", "docx", "txt", "md", "png", "rs"];
    let mut out = Vec::with_capacity(10_000);
    for i in 0..10_000usize {
        let dir = format!(
            "C:\\Users\\X\\{}\\{}",
            words[i % words.len()],
            words[(i / 7) % words.len()]
        );
        let name = format!("{}_{:05}.{}", words[(i / 3) % words.len()], i, exts[i % exts.len()]);
        out.push(entry(
            &format!("{dir}\\{name}"),
            EntryKind::File,
            (i as u64) * 17,
            1_700_000_000_000 + (i as i64) * 1000,
        ));
    }
    out
}

#[test]
fn ten_thousand_entries_are_deterministic_and_correct() {
    let index = FilenameIndex::from_entries(synthetic_10k());
    assert_eq!(index.len(), 10_000);

    // Exact name search ranks the exact file first.
    let hits = index.search("report_00000.pdf", 10);
    assert!(!hits.is_empty(), "exact name should match");
    assert_eq!(index.entry_name(hits[0].entry_index), "report_00000.pdf");
    assert!((hits[0].score - 1.0).abs() < 1e-6);
    // entry_path maps back without materializing entries().
    assert!(index.entry_path(hits[0].entry_index).ends_with("report_00000.pdf"));

    // Determinism: identical results on repeated calls.
    for q in [
        "report",
        "invoice_0004",
        "design",
        "rprt",
        "00007",
        "analysis_001",
        "zzzqqq",
    ] {
        let a = index.search(q, 25);
        let b = index.search(q, 25);
        assert_eq!(a, b, "search must be deterministic for {q}");
        // Scores are non-increasing.
        for w in a.windows(2) {
            assert!(w[0].score >= w[1].score, "scores must be sorted for {q}");
        }
    }

    // Prefix tier and scan tier split.
    let prefix = index.search_prefix("report", 20);
    assert!(!prefix.is_empty());
    assert!(prefix.iter().all(|h| h.score >= 0.94 - 1e-6));
    let scan = index.search_scan("report", 20);
    assert!(scan.iter().all(|h| h.score < 1.0));
}

#[test]
fn unicode_cjk_hidden_and_long_paths() {
    let long_dir: String = (0..10)
        .map(|i| format!("nested_directory_{:02}\\", i))
        .collect();
    let entries = vec![
        entry("C:\\Users\\X\\Documents\\报告-最终版.pdf", EntryKind::File, 10, 1000),
        entry("C:\\Users\\X\\Documents\\会议纪要.docx", EntryKind::File, 20, 2000),
        entry("C:\\Users\\X\\Documents\\日本語テスト.txt", EntryKind::File, 30, 3000),
        entry(
            &format!("C:\\Users\\X\\{long_dir}deep_leaf.txt"),
            EntryKind::File,
            40,
            4000,
        ),
        entry("C:\\Users\\X\\Documents\\.gitignore", EntryKind::File, 5, 5000),
        entry("C:\\Users\\X\\Documents\\hidden_report.pdf", EntryKind::File, 6, 6000),
    ];
    let index = FilenameIndex::from_entries(entries);

    let cjk = index.search("最终版", 5);
    assert!(!cjk.is_empty());
    assert_eq!(index.entry_name(cjk[0].entry_index), "报告-最终版.pdf");
    // UTF-16 ranges: the query starts after 3 CJK chars (3 UTF-16 units).
    assert_eq!(cjk[0].ranges, vec![(3, 6)]);

    let cjk2 = index.search("会议", 5);
    assert!(!cjk2.is_empty());
    let jp = index.search("日本語", 5);
    assert!(!jp.is_empty());

    let deep = index.search("deep_leaf", 5);
    assert!(!deep.is_empty());
    assert!(index.entry_path(deep[0].entry_index).ends_with("deep_leaf.txt"));

    let hidden = index.search("gitignore", 5);
    assert!(!hidden.is_empty(), "hidden files are indexed");

    let long_hit = index.search("nested_directory_09", 5);
    assert!(!long_hit.is_empty(), "path segment of a long path matches");
}

#[test]
fn ranking_order_is_exact_stem_prefix_word_substring_path_fuzzy() {
    let entries = vec![
        entry("C:\\a\\report", EntryKind::File, 1, 1),
        entry("C:\\a\\report.pdf", EntryKind::File, 1, 2),
        entry("C:\\a\\reports.txt", EntryKind::File, 1, 3),
        entry("C:\\a\\annual-report.txt", EntryKind::File, 1, 4),
        entry("C:\\a\\myreport.txt", EntryKind::File, 1, 5),
        entry("C:\\a\\r_e_p_o_r_t.txt", EntryKind::File, 1, 6),
        entry("C:\\a\\other.txt", EntryKind::File, 1, 7),
        entry("C:\\projects\\report\\other.txt", EntryKind::File, 1, 8),
    ];
    let index = FilenameIndex::from_entries(entries);
    let hits = index.search("report", 20);
    let names: Vec<&str> = hits
        .iter()
        .map(|h| index.entry_name(h.entry_index))
        .collect();
    let pos = |n: &str| names.iter().position(|x| *x == n).unwrap_or(usize::MAX);
    assert!(pos("report") < pos("report.pdf"), "exact name first: {names:?}");
    assert!(pos("report.pdf") < pos("reports.txt"), "stem before prefix: {names:?}");
    assert!(pos("reports.txt") < pos("annual-report.txt"), "prefix before word-prefix: {names:?}");
    assert!(pos("annual-report.txt") < pos("myreport.txt"), "word-prefix before substring: {names:?}");
    assert!(pos("myreport.txt") < pos("r_e_p_o_r_t.txt"), "substring before fuzzy: {names:?}");
}

#[test]
fn filters_restrict_results() {
    let entries = vec![
        entry("C:\\docs\\a.pdf", EntryKind::File, 100, 1000),
        entry("C:\\docs\\b.pdf", EntryKind::File, 200, 5000),
        entry("C:\\docs\\c.txt", EntryKind::File, 300, 9000),
        entry("C:\\docs\\sub\\d.pdf", EntryKind::File, 400, 1000),
        entry("C:\\other\\e.pdf", EntryKind::File, 500, 1000),
        entry("C:\\docs\\folder", EntryKind::Folder, 0, 1000),
    ];
    let index = FilenameIndex::from_entries(entries);
    let exts = vec!["pdf".to_string()];
    let filter = EntryFilter {
        extensions: &exts,
        ..Default::default()
    };
    let hits = index.search_filtered(".", 50, &filter);
    assert!(hits.is_empty(), "dot is a poor query; filters should still be safe");

    let hits = index.search_filtered("d", 50, &filter);
    assert!(!hits.is_empty());
    assert!(hits
        .iter()
        .all(|h| index.entry_ext(h.entry_index).eq_ignore_ascii_case("pdf")));

    let filter = EntryFilter {
        kind: Some(EntryKind::Folder),
        ..Default::default()
    };
    let hits = index.search_filtered("folder", 10, &filter);
    assert_eq!(hits.len(), 1);
    assert_eq!(index.entry_kind(hits[0].entry_index), EntryKind::Folder);

    let filter = EntryFilter {
        min_size: Some(350),
        max_size: Some(450),
        ..Default::default()
    };
    let hits = index.search_filtered("d", 50, &filter);
    assert_eq!(hits.len(), 1);
    assert_eq!(index.entry_size(hits[0].entry_index), 400);

    let filter = EntryFilter {
        modified_after: Some(4000),
        ..Default::default()
    };
    let hits = index.search_filtered("b", 50, &filter);
    assert_eq!(hits.len(), 1);
    assert_eq!(index.entry_name(hits[0].entry_index), "b.pdf");

    let filter = EntryFilter {
        under_path: Some("C:\\docs"),
        ..Default::default()
    };
    let hits = index.search_filtered("pdf", 50, &filter);
    assert!(hits
        .iter()
        .all(|h| index.entry_path(h.entry_index).starts_with("C:\\docs\\")));
    assert!(!hits
        .iter()
        .any(|h| index.entry_path(h.entry_index).contains("C:\\other")));
}

#[test]
fn persistence_roundtrip_of_10k_entries() {
    let tree = TempTree::new("persist10k");
    let path = tree.root.join("index.psidx");
    let mut index = FilenameIndex::from_entries(synthetic_10k());
    index.set_roots(vec!["C:\\Users\\X".to_string()]);
    assert_eq!(index.root_counts().len(), 1);
    index.save(&path).unwrap();
    let loaded = FilenameIndex::load(&path).unwrap();
    assert_eq!(loaded.len(), 10_000);
    assert_eq!(loaded.roots(), index.roots());
    for q in ["report", "invoice_0004", "rprt", "00007", "zzzqqq"] {
        assert_eq!(index.search(q, 20), loaded.search(q, 20), "query {q}");
    }
    // Corrupt file: truncate and expect a graceful error (not a panic).
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() / 3]).unwrap();
    assert!(FilenameIndex::load(&path).is_err());
}

#[test]
fn scan_then_search_end_to_end() {
    let tree = TempTree::new("e2e");
    tree.file("alpha_report.pdf", "x");
    tree.file("sub/beta_report.docx", "x");
    tree.file("sub/deeper/gamma.txt", "x");
    tree.file("node_modules/pkg/index.js", "junk");
    let roots = vec![tree.root.clone()];
    let (index, stats) = scan(ScanOptions {
        roots: &roots,
        excluded: &[],
        junk_dirs: &[],
        max_entries: None,
        cancelled: &|| false,
        progress: &mut |_| {},
    });
    assert!(stats.errors == 0);
    assert_eq!(index.len(), 5, "3 files + 2 folder entries");
    let hits = index.search("report", 10);
    assert_eq!(hits.len(), 2);
    assert!(hits
        .iter()
        .all(|h| index.entry_name(h.entry_index).contains("report")));
    let hits = index.search("deeper", 5);
    assert!(!hits.is_empty(), "folder entry matches its own name");
}

#[test]
fn performance_smoke_10k_prints_timings() {
    let index = FilenameIndex::from_entries(synthetic_10k());
    let queries = [
        "report_00000.pdf",
        "report_0012",
        "invoice",
        "design_000",
        "rprt_00",
        "zzzqqq",
    ];
    for q in queries {
        let iters = 200;
        let start = std::time::Instant::now();
        let mut total_hits = 0usize;
        for _ in 0..iters {
            total_hits += index.search(q, 20).len();
        }
        let per = start.elapsed().as_secs_f64() * 1e6 / iters as f64;
        println!("10k search {q:>20} : {per:8.2} us/call ({total_hits} hits total)");
        // Loose upper bound only: catches pathological regressions, not perf claims.
        assert!(per < 5_000.0, "search {q} took {per} us");
    }
}
