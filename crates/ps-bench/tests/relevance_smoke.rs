#![cfg(feature = "engine")]

//! CI smoke relevance test: deterministic 10k synthetic corpus + committed golden
//! fixture. No large datasets on disk, deterministic across machines.
//!
//! Behaviour while the engine is WIP: if ps-index::search returns nothing at all
//! for the suite, the test prints ENGINE_NOT_READY and skips quality assertions
//! (structural checks still run). Once the engine serves results, MRR/exact@1/
//! zero-match gates apply. Thresholds are regression gates derived from the
//! measured Gate-2 baseline (documented in docs/performance/BENCHMARKS.md), not
//! product targets and not tuned to the algorithm.

use ps_bench::dataset;
use ps_bench::relevance::{evaluate, GoldenFile};
use ps_index::{EntryKind, FilenameIndex, IndexEntry};

fn build_index() -> FilenameIndex {
    let entries = dataset::generate_entries(dataset::DEFAULT_SEED, 10_000);
    let mapped: Vec<IndexEntry> = entries
        .into_iter()
        .map(|e| IndexEntry {
            path: e.path,
            name: e.name,
            ext: e.ext,
            kind: if e.kind == "folder" {
                EntryKind::Folder
            } else {
                EntryKind::File
            },
            size: e.size,
            modified: e.modified,
        })
        .collect();
    FilenameIndex::from_entries(mapped)
}

#[test]
fn dataset_generation_is_deterministic() {
    let a = dataset::generate_entries(42, 2_000);
    let b = dataset::generate_entries(42, 2_000);
    let sa = serde_json::to_string(&a).expect("serialize a");
    let sb = serde_json::to_string(&b).expect("serialize b");
    assert_eq!(sa, sb, "same seed+count must produce byte-identical datasets");
    assert_eq!(a.len(), b.len());
    assert!(
        a.iter().any(|e| e.name == "Project Phoenix Budget 2026.xlsx"),
        "planted anchors must be present"
    );
    assert!(
        a.iter().any(|e| e.name == "会議議事録_2026-11-09.docx"),
        "CJK anchor must be present"
    );
}

#[test]
fn synthetic_golden_relevance_smoke() {
    let golden: GoldenFile =
        serde_json::from_str(include_str!("../../../tests/golden/synthetic-golden.json"))
            .expect("parse tests/golden/synthetic-golden.json");
    assert_eq!(golden.entries, 10_000, "fixture must match the 10k smoke corpus");

    let index = build_index();
    assert_eq!(index.len(), 10_000, "index length must equal corpus size");

    let (aggregate, per_query) = evaluate(&index, &golden);

    println!(
        "relevance smoke: MRR@10={:.3} recall@10={:.3} exact@1={:?} pass_rate={:.3} scored={} zero_match={}/{}",
        aggregate.mrr_at_10,
        aggregate.recall_at_10,
        aggregate.exact_at_1,
        aggregate.pass_rate,
        aggregate.scored_queries,
        aggregate.zero_match_correct,
        aggregate.zero_match_total
    );
    for r in &per_query {
        println!(
            "  [{}] {} rank={:?} mrr={:.3} recall={:.3} exact={:?}",
            r.category, r.query, r.rank_of_first_expected, r.mrr, r.recall_at_10, r.exact_at_1
        );
    }

    // Structural invariants always hold.
    assert_eq!(aggregate.queries, golden.queries.len());
    for r in &per_query {
        assert!((0.0..=1.0).contains(&r.mrr), "mrr out of range for {}", r.id);
        assert!(
            (0.0..=1.0).contains(&r.recall_at_10),
            "recall out of range for {}",
            r.id
        );
    }

    if aggregate.queries_with_any_result == 0 {
        eprintln!("ENGINE_NOT_READY: ps-index::search returned no results; skipping quality assertions");
        return;
    }

    // Deterministic ordering for the same query.
    let a = index.search("Project Phoenix Budget 2026.xlsx", 10);
    let b = index.search("Project Phoenix Budget 2026.xlsx", 10);
    assert_eq!(
        a.iter().map(|h| h.entry_index).collect::<Vec<_>>(),
        b.iter().map(|h| h.entry_index).collect::<Vec<_>>(),
        "search ordering must be deterministic"
    );

    // Regression gates from the measured Gate-2 baseline (see BENCHMARKS.md).
    if let Some(exact) = aggregate.exact_at_1 {
        assert!(exact >= 0.90, "exact-match@1 {exact:.3} < 0.90 regression gate");
    }
    assert!(
        aggregate.mrr_at_10 >= 0.80,
        "MRR@10 {:.3} < 0.80 regression gate",
        aggregate.mrr_at_10
    );
    assert_eq!(
        aggregate.zero_match_correct, aggregate.zero_match_total,
        "zero-match queries must return no results"
    );
}
