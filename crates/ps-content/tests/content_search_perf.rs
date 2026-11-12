//! Content search snippet/line regression tests (task-42).
//! The search path computes snippets and first-match line numbers in Rust from a
//! bounded text prefix instead of SQLite's snippet() (which was the p95 cost).

use std::path::{Path, PathBuf};

use ps_content::ContentIndex;

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("ps-content-tests")
        .join(format!("{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn index_file(index: &ContentIndex, path: &Path) {
    index
        .index_file(path, ps_content::DEFAULT_MAX_BYTES)
        .expect("index file");
}

#[test]
fn snippet_and_line_are_computed_from_the_bounded_text() {
    let dir = scratch_dir("snippet-lines");
    let index = ContentIndex::open_in_memory().unwrap();
    let path = dir.join("lines.txt");
    let mut body = String::new();
    for line in 1..=10 {
        if line == 5 {
            body.push_str("the needle UNICORN-SNIPPET-2026 appears here\n");
        } else {
            body.push_str(&format!("filler line number {line}\n"));
        }
    }
    std::fs::write(&path, &body).unwrap();
    index_file(&index, &path);
    let hits = index.search("UNICORN-SNIPPET-2026", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].snippet_line, Some(5));
    assert!(hits[0].snippet.contains("UNICORN-SNIPPET-2026"));
    assert!(!hits[0].snippet.contains('\n'), "snippet must be single-line");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn snippet_is_honest_when_the_term_is_beyond_the_text_prefix() {
    let dir = scratch_dir("snippet-tail");
    let index = ContentIndex::open_in_memory().unwrap();
    let path = dir.join("big.txt");
    let mut body = String::new();
    for _ in 0..5000 {
        body.push_str("padding padding padding padding padding\n");
    }
    body.push_str("TAIL-TOKEN-2026 beyond the prefix\n");
    std::fs::write(&path, &body).unwrap();
    index_file(&index, &path);
    let hits = index.search("TAIL-TOKEN-2026", 5).unwrap();
    assert_eq!(hits.len(), 1);
    // The term is past the 32 KB preview prefix: no fabricated line/snippet.
    assert_eq!(hits[0].snippet_line, None);
    assert!(!hits[0].snippet.contains("TAIL-TOKEN-2026"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cjk_terms_match_without_ascii_case_folding() {
    let dir = scratch_dir("snippet-cjk");
    let index = ContentIndex::open_in_memory().unwrap();
    let path = dir.join("cjk.txt");
    std::fs::write(&path, "报告内容：会议纪要 2026\n").unwrap();
    index_file(&index, &path);
    let hits = index.search("会议纪要", 5).unwrap();
    assert!(!hits.is_empty());
    assert!(hits[0].snippet.contains("会议纪要"));
    assert_eq!(hits[0].snippet_line, Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn high_df_query_is_fast_and_still_ranked() {
    // Perf smoke: prints timings; the assertion is deliberately loose (catches
    // pathological regressions, not a performance claim).
    let dir = scratch_dir("snippet-perf");
    let index = ContentIndex::open_in_memory().unwrap();
    for doc in 0..64 {
        let path = dir.join(format!("doc{doc}.txt"));
        let body = format!(
            "message number {doc}\n{}\n",
            "message common token line\n".repeat(20)
        );
        std::fs::write(&path, body).unwrap();
        index_file(&index, &path);
    }
    let started = std::time::Instant::now();
    let mut hits = 0usize;
    for _ in 0..50 {
        hits += index.search("message", 12).unwrap().len();
    }
    let per_query = started.elapsed().as_secs_f64() * 1000.0 / 50.0;
    println!("content search (64 docs, high-df term): {per_query:.2} ms/query, {hits} hits");
    assert!(per_query < 100.0, "content search too slow: {per_query} ms");
    let _ = std::fs::remove_dir_all(&dir);
}
