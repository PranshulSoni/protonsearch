//! Golden relevance fixtures + evaluation (MRR@10, recall@10, exact-match@1).
//!
//! Metric definitions (fixed, not tunable per run):
//! - expected set = expectedTop1 + expectedTop5 paths, deduplicated.
//! - rank_of_first_expected = 1-based rank of the first result inside the
//!   expected set; MRR@10 = 1/rank when rank <= 10 else 0.
//! - recall@10 = |top10 AND expected set| / |expected set|.
//! - exact@1 = top-1 path equals expectedTop1 (scored only when expectedTop1 is
//!   present; partial/prefix queries intentionally leave it null).
//! - expectEmpty queries are scored separately: correct when zero results.
//!
//! Fixtures are deterministic JSON under tests/golden/. The synthetic fixture is
//! generated from the planted anchors of the corpus generator; the real-corpus
//! fixture is generated from the read-only legacy export and is machine-specific
//! evidence, not a CI test.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use ps_index::FilenameIndex;

use crate::args::Args;
use crate::dataset;
use crate::env::host_info;
use crate::{fnv1a64_file, write_json, RESULT_SCHEMA};

pub const GOLDEN_SCHEMA: &str = "ps-golden/1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenQuery {
    pub id: String,
    pub category: String,
    pub query: String,
    #[serde(default)]
    pub expected_top1: Option<String>,
    #[serde(default)]
    pub expected_top5: Vec<String>,
    #[serde(default)]
    pub expect_empty: bool,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenFile {
    pub schema: String,
    pub corpus: String,
    pub seed: u64,
    pub entries: usize,
    pub notes: Vec<String>,
    pub queries: Vec<GoldenQuery>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PerQueryResult {
    pub id: String,
    pub category: String,
    pub query: String,
    pub expected_set_size: usize,
    pub result_count: usize,
    pub top1: Option<String>,
    pub rank_of_first_expected: Option<usize>,
    pub mrr: f64,
    pub recall_at_10: f64,
    pub exact_at_1: Option<bool>,
    pub passed: bool,
    pub results_preview: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Aggregate {
    pub queries: usize,
    pub scored_queries: usize,
    pub queries_with_any_result: usize,
    pub mrr_at_10: f64,
    pub recall_at_10: f64,
    pub exact_at_1: Option<f64>,
    pub pass_rate: f64,
    pub zero_match_correct: usize,
    pub zero_match_total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelevanceReport {
    pub schema: String,
    pub kind: String,
    pub host: crate::env::HostInfo,
    pub generated_at_utc: String,
    pub command: String,
    pub dataset: DatasetRef,
    pub golden: GoldenRef,
    pub aggregate: Aggregate,
    pub per_query: Vec<PerQueryResult>,
    pub engine_ready: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DatasetRef {
    pub path: String,
    pub entries: usize,
    pub bytes: u64,
    pub fnv1a64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GoldenRef {
    pub corpus: String,
    pub queries: usize,
    pub seed: u64,
}

fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

fn anchor_path(name: &str) -> String {
    dataset::ANCHORS
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.path.to_string())
        .unwrap_or_default()
}

fn duplicate_anchor_paths(name: &str) -> Vec<String> {
    dataset::ANCHORS
        .iter()
        .filter(|a| a.name == name)
        .map(|a| a.path.to_string())
        .collect()
}

/// Golden fixture for the deterministic synthetic corpus (all sizes, anchors planted).
pub fn synthetic_golden(seed: u64, entries: usize) -> GoldenFile {
    let mut queries = Vec::new();
    let mut add = |id: &str, cat: &str, query: &str, top1: Option<String>, top5: Vec<String>, rationale: &str| {
        queries.push(GoldenQuery {
            id: id.to_string(),
            category: cat.to_string(),
            query: query.to_string(),
            expected_top1: top1,
            expected_top5: top5,
            expect_empty: false,
            rationale: rationale.to_string(),
        });
    };

    add(
        "exact_unique",
        "exact",
        "Project Phoenix Budget 2026.xlsx",
        Some(anchor_path("Project Phoenix Budget 2026.xlsx")),
        vec![],
        "planted unique anchor: full filename must rank first",
    );
    add(
        "exact_quarterly",
        "exact",
        "quarterly_report_final_v2.pdf",
        Some(anchor_path("quarterly_report_final_v2.pdf")),
        vec![],
        "planted unique anchor with version suffix",
    );
    add(
        "exact_screenshot",
        "exact",
        "Screenshot 2026-11-09 142233.png",
        Some(anchor_path("Screenshot 2026-11-09 142233.png")),
        vec![],
        "planted screenshot-style anchor",
    );
    add(
        "exact_cjk_jp",
        "cjk",
        "会議議事録_2026-11-09.docx",
        Some(anchor_path("会議議事録_2026-11-09.docx")),
        vec![],
        "planted Japanese anchor: exact unicode filename",
    );
    add(
        "exact_unicode",
        "unicode",
        "café_résumé_naïve.md",
        Some(anchor_path("café_résumé_naïve.md")),
        vec![],
        "planted accented unicode anchor",
    );
    add(
        "exact_hidden",
        "hidden",
        ".env",
        Some(anchor_path(".env")),
        vec![],
        "planted hidden file with leading dot",
    );
    add(
        "exact_long_path",
        "long_path",
        dataset::LONG_ANCHOR_PATH,
        Some(dataset::LONG_ANCHOR_PATH.to_string()),
        vec![],
        "full 200+ char path as query; must resolve to the exact file",
    );
    add(
        "exact_readme",
        "exact_duplicate",
        "README.md",
        None,
        vec![anchor_path("README.md")],
        "common duplicate name; planted anchor must be in top-10 (tie ranking is engine-defined)",
    );
    add(
        "exact_index_dup",
        "exact_duplicate",
        "index.js",
        None,
        duplicate_anchor_paths("index.js"),
        "three planted duplicates must all be found in top-10",
    );
    add(
        "exact_package",
        "exact_duplicate",
        "package.json",
        None,
        vec![anchor_path("package.json")],
        "very common name; planted anchor must appear in top-10",
    );
    add(
        "prefix_screenshot",
        "prefix",
        "Screenshot 2026-11-09",
        None,
        vec![anchor_path("Screenshot 2026-11-09 142233.png")],
        "date prefix shared by screenshot-style names; anchor must be in top-10",
    );
    add(
        "prefix_quarterly",
        "prefix",
        "quarterly_rep",
        None,
        vec![anchor_path("quarterly_report_final_v2.pdf")],
        "snake-case prefix",
    );
    add(
        "multi_term_name",
        "multi_term",
        "project phoenix budget",
        Some(anchor_path("Project Phoenix Budget 2026.xlsx")),
        vec![],
        "all three terms are in one filename; expected first",
    );
    add(
        "multi_term_partial",
        "multi_term",
        "quarterly report final",
        None,
        vec![anchor_path("quarterly_report_final_v2.pdf")],
        "terms spread inside one filename",
    );
    add(
        "typo_quarterly",
        "typo",
        "quaterly report final",
        None,
        vec![anchor_path("quarterly_report_final_v2.pdf")],
        "one deleted character; fuzzy search should still surface the anchor in top-10",
    );
    add(
        "typo_screenshot",
        "typo",
        "screnshot 2026-11-09",
        None,
        vec![anchor_path("Screenshot 2026-11-09 142233.png")],
        "one deleted character in first term",
    );
    add(
        "cjk_prefix",
        "cjk",
        "会議議事録",
        None,
        vec![anchor_path("会議議事録_2026-11-09.docx")],
        "CJK prefix query",
    );
    add(
        "cjk_japanese_memo",
        "cjk",
        "日本語_メモ",
        None,
        vec![anchor_path("日本語_メモ_2026.txt")],
        "mixed CJK + ASCII token query",
    );
    add(
        "folder_anchor",
        "folder",
        "ProtonSearch Phoenix",
        None,
        vec![anchor_path("ProtonSearch Phoenix")],
        "planted folder anchor; folder should be found",
    );

    queries.push(GoldenQuery {
        id: "zero_match".to_string(),
        category: "zero_match".to_string(),
        query: "zzqxvbnmplkjhgfdsa9917".to_string(),
        expected_top1: None,
        expected_top5: vec![],
        expect_empty: true,
        rationale: "nonsense token must return no results".to_string(),
    });
    queries.push(GoldenQuery {
        id: "zero_match_cjk".to_string(),
        category: "zero_match".to_string(),
        query: "存在しないファイル名です".to_string(),
        expected_top1: None,
        expected_top5: vec![],
        expect_empty: true,
        rationale: "CJK nonsense must return no results".to_string(),
    });

    GoldenFile {
        schema: GOLDEN_SCHEMA.to_string(),
        corpus: "synthetic-deterministic-planted-anchors".to_string(),
        seed,
        entries,
        notes: vec![
            "expectedTop1 = must be rank 1; expectedTop5 = expected set scored by MRR@10/recall@10 (not exhaustive, usually <=5 paths)".to_string(),
            "corpus is generated deterministically by ps-bench gen with the same seed; anchors are planted at every size".to_string(),
            "duplicate-name queries intentionally do not assert top-1 identity: tie ordering is engine-defined".to_string(),
        ],
        queries,
    }
}

/// Evaluate a golden fixture against an index. Metrics fixed at top-10.
pub fn evaluate(index: &FilenameIndex, golden: &GoldenFile) -> (Aggregate, Vec<PerQueryResult>) {
    let mut results = Vec::new();
    let mut mrr_sum = 0.0;
    let mut recall_sum = 0.0;
    let mut exact_sum = 0.0;
    let mut exact_n = 0usize;
    let mut scored = 0usize;
    let mut any_result = 0usize;
    let mut passed = 0usize;
    let mut zm_correct = 0usize;
    let mut zm_total = 0usize;

    let entries = index.entries();
    for q in &golden.queries {
        let hits = index.search(&q.query, 10);
        let paths: Vec<String> = hits
            .iter()
            .filter_map(|h| entries.get(h.entry_index).map(|e| e.path.clone()))
            .collect();
        if !paths.is_empty() {
            any_result += 1;
        }
        if q.expect_empty {
            zm_total += 1;
            let ok = paths.is_empty();
            if ok {
                zm_correct += 1;
                passed += 1;
            }
            results.push(PerQueryResult {
                id: q.id.clone(),
                category: q.category.clone(),
                query: q.query.clone(),
                expected_set_size: 0,
                result_count: paths.len(),
                top1: paths.first().cloned(),
                rank_of_first_expected: None,
                mrr: 0.0,
                recall_at_10: 0.0,
                exact_at_1: Some(ok),
                passed: ok,
                results_preview: paths.iter().take(3).cloned().collect(),
            });
            continue;
        }

        let mut expected: Vec<String> = Vec::new();
        if let Some(top1) = &q.expected_top1 {
            expected.push(top1.clone());
        }
        for p in &q.expected_top5 {
            if !expected.contains(p) {
                expected.push(p.clone());
            }
        }
        let rank = paths.iter().position(|p| expected.iter().any(|e| e == p)).map(|i| i + 1);
        let mrr = rank.map(|r| if r <= 10 { 1.0 / r as f64 } else { 0.0 }).unwrap_or(0.0);
        let overlap = paths.iter().filter(|p| expected.iter().any(|e| e == *p)).count();
        let recall = if expected.is_empty() { 0.0 } else { (overlap as f64 / expected.len() as f64).min(1.0) };
        let exact = q.expected_top1.as_ref().map(|top1| paths.first().map(|p| p == top1).unwrap_or(false));
        let ok = match exact {
            Some(e) => e,
            None => rank.is_some(),
        };
        mrr_sum += mrr;
        recall_sum += recall;
        if let Some(e) = exact {
            exact_sum += if e { 1.0 } else { 0.0 };
            exact_n += 1;
        }
        scored += 1;
        if ok {
            passed += 1;
        }
        results.push(PerQueryResult {
            id: q.id.clone(),
            category: q.category.clone(),
            query: q.query.clone(),
            expected_set_size: expected.len(),
            result_count: paths.len(),
            top1: paths.first().cloned(),
            rank_of_first_expected: rank,
            mrr,
            recall_at_10: recall,
            exact_at_1: exact,
            passed: ok,
            results_preview: paths.iter().take(3).cloned().collect(),
        });
    }

    let n = golden.queries.len();
    let aggregate = Aggregate {
        queries: n,
        scored_queries: scored,
        queries_with_any_result: any_result,
        mrr_at_10: if scored > 0 { mrr_sum / scored as f64 } else { 0.0 },
        recall_at_10: if scored > 0 { recall_sum / scored as f64 } else { 0.0 },
        exact_at_1: if exact_n > 0 { Some(exact_sum / exact_n as f64) } else { None },
        pass_rate: if n > 0 { passed as f64 / n as f64 } else { 0.0 },
        zero_match_correct: zm_correct,
        zero_match_total: zm_total,
    };
    (aggregate, results)
}

pub fn load_golden(path: &Path) -> anyhow::Result<GoldenFile> {
    let text = std::fs::read_to_string(path)?;
    let golden: GoldenFile = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    anyhow::ensure!(golden.schema == GOLDEN_SCHEMA, "unexpected golden schema: {}", golden.schema);
    Ok(golden)
}

pub fn write_golden(path: &Path, golden: &GoldenFile) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_json(path, golden)
}

fn print_results(report: &RelevanceReport) {
    println!(
        "  aggregate: MRR@10={:.3} recall@10={:.3} exact@1={} pass_rate={:.3} scored={} zero_match={}/{}",
        report.aggregate.mrr_at_10,
        report.aggregate.recall_at_10,
        report
            .aggregate
            .exact_at_1
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "n/a".to_string()),
        report.aggregate.pass_rate,
        report.aggregate.scored_queries,
        report.aggregate.zero_match_correct,
        report.aggregate.zero_match_total
    );
    println!("  {:<18} {:<16} {:>4} {:>6} {:>7} {:>7}  query", "id", "category", "rank", "recall", "exact", "pass");
    for r in &report.per_query {
        println!(
            "  {:<18} {:<16} {:>4} {:>6.2} {:>7} {:>7}  {}",
            r.id,
            r.category,
            r.rank_of_first_expected.map(|x| x.to_string()).unwrap_or_else(|| "-".to_string()),
            r.recall_at_10,
            r.exact_at_1.map(|x| if x { "yes" } else { "NO" }).unwrap_or("-"),
            if r.passed { "ok" } else { "FAIL" },
            r.query.chars().take(52).collect::<String>()
        );
    }
    let failures: Vec<&PerQueryResult> = report.per_query.iter().filter(|r| !r.passed).collect();
    if !failures.is_empty() {
        println!("  failures:");
        for f in failures {
            println!("    - [{}] {:?} top1={:?} rank={:?}", f.id, f.query, f.top1, f.rank_of_first_expected);
        }
    }
}

fn build_index(entries: Vec<ps_index::IndexEntry>) -> FilenameIndex {
    let mut index = FilenameIndex::new();
    let count = entries.len();
    index.replace_all(entries);
    debug_assert_eq!(index.len(), count);
    index
}

pub fn run(args: &Args) -> anyhow::Result<()> {
    let dataset_path = PathBuf::from(args.required("--dataset")?);
    let golden_path = PathBuf::from(args.required("--golden")?);
    let out = PathBuf::from(args.required("--out")?);
    let golden = load_golden(&golden_path)?;
    let entries = crate::filenamebench::load_index_entries(&dataset_path)?;
    let count = entries.len();
    let dataset_bytes = std::fs::metadata(&dataset_path).map(|m| m.len()).unwrap_or(0);
    let (_, fnv) = fnv1a64_file(&dataset_path)?;
    let index = build_index(entries);
    let (aggregate, per_query) = evaluate(&index, &golden);

    let mut notes = vec![
        "metrics fixed at top-10: MRR@10, recall@10, exact-match@1; expectEmpty scored separately".to_string(),
        "expectedTop5 is the expected set for MRR/recall, not an exhaustive reference list".to_string(),
        "engine_not_ready means ps-index::search returned no results; the table is structural only".to_string(),
    ];
    if golden.corpus.starts_with("real") {
        notes.push("real-corpus fixture is generated from this machine's read-only legacy export; it is evidence, not a portable CI test".to_string());
    }

    let engine_ready = aggregate.queries_with_any_result > 0;
    let report = RelevanceReport {
        schema: RESULT_SCHEMA.to_string(),
        kind: "relevance-evaluation".to_string(),
        host: host_info().clone(),
        generated_at_utc: now_utc(),
        command: format!(
            "ps-bench relevance --dataset {} --golden {} --out {}",
            dataset_path.display(),
            golden_path.display(),
            out.display()
        ),
        dataset: DatasetRef {
            path: dataset_path.display().to_string(),
            entries: count,
            bytes: dataset_bytes,
            fnv1a64: format!("{fnv:016x}"),
        },
        golden: GoldenRef { corpus: golden.corpus.clone(), queries: golden.queries.len(), seed: golden.seed },
        aggregate,
        per_query,
        engine_ready,
        notes,
    };
    write_json(&out, &report)?;
    println!("ps-bench relevance: corpus={} queries={} entries={} engine_ready={}", golden.corpus, golden.queries.len(), count, report.engine_ready);
    print_results(&report);
    if !report.engine_ready {
        println!("  WARNING: search returned no results (ps-index stub); relevance scores are NOT valid.");
    }
    for note in &report.notes {
        println!("  note: {note}");
    }
    println!("  results: {}", out.display());
    Ok(())
}

/// Generate the synthetic fixture file (committed under tests/golden/).
pub fn write_synthetic_fixture(out: &Path, seed: u64, entries: usize) -> anyhow::Result<GoldenFile> {
    let golden = synthetic_golden(seed, entries);
    write_golden(out, &golden)?;
    Ok(golden)
}

/// Build a golden fixture from the real legacy corpus export.
pub fn build_real_golden(paths_path: &Path, out: &Path, max_queries: usize, seed: u64) -> anyhow::Result<GoldenFile> {
    let entries = dataset::load_entries(paths_path)?;
    let total = entries.len();
    let mut name_counts: HashMap<String, u32> = HashMap::new();
    let mut name_paths: HashMap<String, Vec<String>> = HashMap::new();
    for e in &entries {
        *name_counts.entry(e.name.clone()).or_insert(0) += 1;
        if name_paths.len() < 500_000 {
            name_paths.entry(e.name.clone()).or_default().push(e.path.clone());
        }
    }
    let mut rng = dataset::Rng::new(seed);
    let mut queries: Vec<GoldenQuery> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    let push = |q: GoldenQuery, queries: &mut Vec<GoldenQuery>, used: &mut HashSet<String>| {
        if used.insert(q.query.clone()) {
            queries.push(q);
        }
    };

    let simple_name = |n: &str| -> bool {
        n.len() >= 10
            && n.len() <= 120
            && !n.starts_with('.')
            && n.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.' | '(' | ')' | '&' | '+'))
    };

    // exact unique names
    let mut sampled = 0usize;
    let mut idx = rng.below(entries.len() as u64) as usize;
    let mut scanned = 0usize;
    while sampled < max_queries.min(25) && scanned < entries.len() * 4 {
        scanned += 1;
        idx = (idx + 1 + rng.below(97) as usize) % entries.len();
        let e = &entries[idx];
        if e.is_folder() || !simple_name(&e.name) {
            continue;
        }
        if name_counts.get(&e.name).copied().unwrap_or(0) != 1 {
            continue;
        }
        push(
            GoldenQuery {
                id: format!("real_exact_{sampled}"),
                category: "exact".to_string(),
                query: e.name.clone(),
                expected_top1: Some(e.path.clone()),
                expected_top5: vec![],
                expect_empty: false,
                rationale: "unique filename in the real corpus; exact query must rank it first".to_string(),
            },
            &mut queries,
            &mut used,
        );
        sampled += 1;
    }

    // duplicate names (2..=10 occurrences)
    let mut dup_names: Vec<(&String, &u32)> = name_counts.iter().filter(|(n, c)| **c >= 2 && **c <= 10 && n.len() >= 6).collect();
    dup_names.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for i in 0..dup_names.len().min(10) {
        let (name, _) = dup_names[i];
        let paths = name_paths.get(name).cloned().unwrap_or_default();
        if paths.len() < 2 {
            continue;
        }
        push(
            GoldenQuery {
                id: format!("real_dup_{i}"),
                category: "exact_duplicate".to_string(),
                query: name.clone(),
                expected_top1: None,
                expected_top5: paths.into_iter().take(5).collect(),
                expect_empty: false,
                rationale: "duplicated filename in the real corpus; MRR@10 over all listed copies".to_string(),
            },
            &mut queries,
            &mut used,
        );
    }

    // prefix queries from unique names
    let mut pref = 0usize;
    scanned = 0;
    while pref < 10 && scanned < entries.len() {
        scanned += 1;
        idx = (idx + 1 + rng.below(211) as usize) % entries.len();
        let e = &entries[idx];
        if e.is_folder() || !simple_name(&e.name) || name_counts.get(&e.name).copied().unwrap_or(0) != 1 {
            continue;
        }
        let prefix: String = e.name.chars().take(12).collect();
        if prefix.chars().count() < 10 {
            continue;
        }
        // Collect all paths whose name starts with this prefix (expected set).
        let mut matches: Vec<String> = name_paths
            .iter()
            .filter(|(n, _)| n.starts_with(&prefix))
            .flat_map(|(_, ps)| ps.iter().cloned())
            .take(5)
            .collect();
        matches.sort();
        if matches.is_empty() {
            continue;
        }
        push(
            GoldenQuery {
                id: format!("real_prefix_{pref}"),
                category: "prefix".to_string(),
                query: prefix.clone(),
                expected_top1: None,
                expected_top5: matches,
                expect_empty: false,
                rationale: format!("first 12 chars of a unique real filename; expected set = names sharing the prefix"),
            },
            &mut queries,
            &mut used,
        );
        pref += 1;
    }

    // typo queries
    let mut typos = 0usize;
    scanned = 0;
    while typos < 8 && scanned < entries.len() {
        scanned += 1;
        idx = (idx + 1 + rng.below(173) as usize) % entries.len();
        let e = &entries[idx];
        if e.is_folder() || !simple_name(&e.name) || name_counts.get(&e.name).copied().unwrap_or(0) != 1 {
            continue;
        }
        let chars: Vec<char> = e.name.chars().collect();
        if chars.len() < 12 {
            continue;
        }
        // Delete the character at ~40% into the name (still in the stem).
        let del = (chars.len() * 2 / 5).max(4).min(chars.len() - 2);
        let typo: String = chars.iter().enumerate().filter(|(i, _)| *i != del).map(|(_, c)| *c).collect();
        if typo == e.name {
            continue;
        }
        push(
            GoldenQuery {
                id: format!("real_typo_{typos}"),
                category: "typo".to_string(),
                query: typo,
                expected_top1: None,
                expected_top5: vec![e.path.clone()],
                expect_empty: false,
                rationale: "one-deleted-character typo of a unique real filename; fuzzy search should surface it in top-10".to_string(),
            },
            &mut queries,
            &mut used,
        );
        typos += 1;
    }

    // hidden files with unique names
    let mut hid = 0usize;
    for e in entries.iter().filter(|e| e.name.starts_with('.') && !e.is_folder()) {
        if hid >= 5 {
            break;
        }
        if e.name.len() >= 5 && name_counts.get(&e.name).copied().unwrap_or(0) == 1 {
            push(
                GoldenQuery {
                    id: format!("real_hidden_{hid}"),
                    category: "hidden".to_string(),
                    query: e.name.clone(),
                    expected_top1: Some(e.path.clone()),
                    expected_top5: vec![],
                    expect_empty: false,
                    rationale: "unique hidden filename in the real corpus".to_string(),
                },
                &mut queries,
                &mut used,
            );
            hid += 1;
        }
    }

    // long paths
    let mut longs = 0usize;
    for e in entries.iter().filter(|e| !e.is_folder() && e.path.chars().count() >= 180) {
        if longs >= 5 {
            break;
        }
        push(
            GoldenQuery {
                id: format!("real_long_{longs}"),
                category: "long_path".to_string(),
                query: e.path.clone(),
                expected_top1: Some(e.path.clone()),
                expected_top5: vec![],
                expect_empty: false,
                rationale: "full 180+ char path from the real corpus".to_string(),
            },
            &mut queries,
            &mut used,
        );
        longs += 1;
    }

    // CJK names if present
    let mut cjk = 0usize;
    for e in entries.iter().filter(|e| e.path.chars().any(dataset::is_cjk)) {
        if cjk >= 5 {
            break;
        }
        let run: String = e.name.chars().filter(|c| dataset::is_cjk(*c)).collect();
        if run.chars().count() < 2 {
            continue;
        }
        let short: String = run.chars().take(4).collect();
        let matches: Vec<String> = name_paths
            .iter()
            .filter(|(n, _)| n.contains(&short))
            .flat_map(|(_, ps)| ps.iter().cloned())
            .take(5)
            .collect();
        if matches.is_empty() {
            continue;
        }
        push(
            GoldenQuery {
                id: format!("real_cjk_{cjk}"),
                category: "cjk".to_string(),
                query: short,
                expected_top1: None,
                expected_top5: matches,
                expect_empty: false,
                rationale: "CJK substring of a real filename; expected set = names containing it".to_string(),
            },
            &mut queries,
            &mut used,
        );
        cjk += 1;
    }

    // zero-match queries
    for i in 0..5 {
        push(
            GoldenQuery {
                id: format!("real_zero_{i}"),
                category: "zero_match".to_string(),
                query: format!("zzqx{i}vbnmplkjhgfdsa"),
                expected_top1: None,
                expected_top5: vec![],
                expect_empty: true,
                rationale: "nonsense token must return no results".to_string(),
            },
            &mut queries,
            &mut used,
        );
    }

    let golden = GoldenFile {
        schema: GOLDEN_SCHEMA.to_string(),
        corpus: "real-legacy-export(read-only, machine-specific)".to_string(),
        seed,
        entries: total,
        notes: vec![
            "generated from %APPDATA%\\protonsearch\\file_index.db exported read-only immutable by ps-bench real-export".to_string(),
            "machine-specific evidence for docs/performance; not portable to CI".to_string(),
            "expectedTop5 = expected set for MRR@10/recall@10; not an exhaustive match list".to_string(),
            "typo queries are one-deleted-character mutations of unique real filenames; they exercise fuzzy ranking".to_string(),
        ],
        queries,
    };
    write_golden(out, &golden)?;
    Ok(golden)
}
