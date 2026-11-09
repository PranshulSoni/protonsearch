//! v2 content benchmark: ps-content FTS5 (current schema/query path).
//!
//! Corpus: the same 14,609 documents imported from the user's legacy v1
//! files_fts via ps_content::import_legacy (read-only ATTACH on the legacy DB).
//! Scope label in every report: "v2 ps-content FTS5 (current schema/query path)".
//!
//! Measures: import wall/ms, index size facts, open time, warm search p50/p95/p99
//! per category (snippet included by the API), cold process restart first query,
//! and process RSS. Owned by perf-bench; ps-content owned by sources-content.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;
use serde_json::json;

use crate::args::Args;
use crate::childproc;
use crate::contentbench::{self, ContentQuery};
use crate::env::{host_info, self_memory, ProcessMemory};
use crate::stats::{summarize, Summary};
use crate::{write_json, RESULT_SCHEMA};

#[derive(Debug, Clone, Serialize)]
pub struct ImportFacts {
    pub files_copied: u64,
    pub content_documents: u64,
    pub skipped: u64,
    pub errors: u64,
    pub warnings: Vec<String>,
    pub import_reported_ms: u64,
    pub import_wall_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct V2IndexFacts {
    pub db_path: String,
    pub db_bytes: u64,
    pub page_size: u64,
    pub page_count: u64,
    pub documents: u64,
    pub errors: u64,
    pub indexed_text_bytes: u64,
    pub fts_data_bytes: u64,
    pub fts_idx_term_bytes: u64,
    pub fts_content_bytes: u64,
    pub fts_docsize_bytes: u64,
    pub estimated_fts_bytes: u64,
}

fn opt_sum(conn: &rusqlite::Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0).max(0) as u64
}

fn v2_facts(db_path: &Path) -> anyhow::Result<V2IndexFacts> {
    let conn = rusqlite::Connection::open(db_path)?;
    conn.execute_batch("PRAGMA query_only=ON;")?;
    let page_size = conn.query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64;
    let page_count = conn.query_row("PRAGMA page_count", [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64;
    let fts_data = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(block)),0) FROM documents_fts_data");
    let fts_idx = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(term)),0) FROM documents_fts_idx");
    let fts_content = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(c0)+LENGTH(c1)+LENGTH(c2)),0) FROM documents_fts_content");
    let fts_docsize = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(sz)),0) FROM documents_fts_docsize");
    Ok(V2IndexFacts {
        db_path: db_path.display().to_string(),
        db_bytes: std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0),
        page_size,
        page_count,
        documents: opt_sum(&conn, "SELECT COUNT(*) FROM documents"),
        errors: opt_sum(&conn, "SELECT COUNT(*) FROM content_errors"),
        indexed_text_bytes: opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(text)),0) FROM documents"),
        fts_data_bytes: fts_data,
        fts_idx_term_bytes: fts_idx,
        fts_content_bytes: fts_content,
        fts_docsize_bytes: fts_docsize,
        estimated_fts_bytes: fts_data + fts_idx + fts_content + fts_docsize,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct V2QueryTiming {
    pub id: String,
    pub category: String,
    pub query: String,
    pub returned: usize,
    pub first_hit_path: Option<String>,
    pub first_snippet: String,
    pub samples_ms: Vec<f64>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
pub struct V2CategoryRow {
    pub category: String,
    pub summary: Summary,
    pub queries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct V2WarmReport {
    pub iterations: usize,
    pub overall: Summary,
    pub by_category: Vec<V2CategoryRow>,
    pub queries: Vec<V2QueryTiming>,
    pub queries_with_results: usize,
    pub zero_result_queries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct V2ColdReport {
    pub restarts: usize,
    pub open_ms: Vec<f64>,
    pub first_query_ms: Vec<f64>,
    pub wall_ms: Vec<f64>,
    pub wall_raw_ms: Vec<f64>,
    pub rss_probe_ms: Vec<f64>,
    pub per_query: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentV2Report {
    pub schema: String,
    pub kind: String,
    pub status: String,
    pub host: crate::env::HostInfo,
    pub generated_at_utc: String,
    pub command: String,
    pub dataset_scope: String,
    pub import: Option<ImportFacts>,
    pub index: V2IndexFacts,
    pub open_ms: f64,
    pub warm: V2WarmReport,
    pub cold: Option<V2ColdReport>,
    pub memory_after_warm: Option<ProcessMemory>,
    pub limitations: Vec<String>,
}

fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

fn remove_db_files(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

pub fn run(args: &Args) -> anyhow::Result<()> {
    let legacy_db = args
        .get("--legacy-db")
        .map(PathBuf::from)
        .unwrap_or_else(crate::realfiles::default_legacy_db);
    let v2_db = PathBuf::from(args.get("--v2-db").unwrap_or(".bench/v2-content.db"));
    let queries_path = PathBuf::from(args.get("--queries").unwrap_or(".bench/queries-content.jsonl"));
    let out = PathBuf::from(args.required("--out")?);
    let iters = args.usize_or("--iters", 30)?.max(1);
    let limit = args.usize_or("--limit", 20)?;
    let cold_restarts = args.usize_or("--cold-restarts", 2)?;
    let recreate = !args.has("--keep-db");
    let do_import = !args.has("--no-import");

    let suite = contentbench::load_queries(&queries_path)?;
    anyhow::ensure!(!suite.is_empty(), "empty query suite: {}", queries_path.display());
    if let Some(dir) = v2_db.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let mut import = None;
    if do_import {
        if recreate {
            remove_db_files(&v2_db);
        }
        let start = Instant::now();
        let summary = ps_content::import_legacy(&legacy_db, &v2_db)?;
        let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
        import = Some(ImportFacts {
            files_copied: summary.files_copied,
            content_documents: summary.content_documents,
            skipped: summary.skipped,
            errors: summary.errors,
            warnings: summary.warnings.clone(),
            import_reported_ms: summary.duration_ms,
            import_wall_ms: wall_ms,
        });
        println!(
            "ps-bench content-v2: imported {} files / {} documents in {:.0} ms (reported {} ms, {} errors)",
            summary.files_copied, summary.content_documents, wall_ms, summary.duration_ms, summary.errors
        );
    }

    let index_facts = v2_facts(&v2_db)?;
    let open_start = Instant::now();
    let index = ps_content::ContentIndex::open(&v2_db)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let stats = index.stats();

    let mut timing_rows = Vec::new();
    let mut all_samples: Vec<Vec<f64>> = Vec::new();
    let mut by_cat: std::collections::HashMap<String, Vec<Vec<f64>>> = std::collections::HashMap::new();
    let mut with_results = 0usize;
    let mut zero_result = 0usize;
    for q in &suite {
        for _ in 0..2 {
            let _ = index.search(&q.query, limit)?;
        }
        let mut samples = Vec::with_capacity(iters);
        let mut returned = 0usize;
        let mut first_path = None;
        let mut first_snippet = String::new();
        for iter in 0..iters {
            let start = Instant::now();
            let hits = index.search(&q.query, limit)?;
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            if iter == 0 {
                returned = hits.len();
                if let Some(h) = hits.first() {
                    first_path = Some(h.path.clone());
                    first_snippet = format!(
                        "{} || {}",
                        h.title.chars().take(60).collect::<String>(),
                        h.snippet.chars().take(160).collect::<String>()
                    );
                }
            }
        }
        if returned > 0 {
            with_results += 1;
        } else {
            zero_result += 1;
        }
        let summary = summarize(&samples);
        by_cat.entry(q.category.clone()).or_default().push(samples.clone());
        all_samples.push(samples.clone());
        timing_rows.push(V2QueryTiming {
            id: q.id.clone(),
            category: q.category.clone(),
            query: q.query.clone(),
            returned,
            first_hit_path: first_path,
            first_snippet,
            samples_ms: samples,
            summary,
        });
    }
    let memory_after_warm = self_memory();
    drop(index);

    let mut categories: Vec<V2CategoryRow> = by_cat
        .into_iter()
        .map(|(category, slices)| {
            let queries = slices.len();
            V2CategoryRow { category, summary: crate::stats::summarize_slices(&slices), queries }
        })
        .collect();
    categories.sort_by(|a, b| a.category.cmp(&b.category));
    let warm = V2WarmReport {
        iterations: iters,
        overall: crate::stats::summarize_slices(&all_samples),
        by_category: categories,
        queries: timing_rows,
        queries_with_results: with_results,
        zero_result_queries: zero_result,
    };

    let mut cold = None;
    if cold_restarts > 0 {
        let mut open_vec = Vec::new();
        let mut first_vec = Vec::new();
        let mut wall_vec = Vec::new();
        let mut raw_vec = Vec::new();
        let mut probe_vec = Vec::new();
        let mut per_query_sums: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for _ in 0..cold_restarts {
            let child_args: Vec<String> = vec![
                "content-v2-cold-child".to_string(),
                "--v2-db".to_string(),
                v2_db.display().to_string(),
                "--queries".to_string(),
                queries_path.display().to_string(),
                "--limit".to_string(),
                limit.to_string(),
            ];
            let outcome = childproc::run_child_json(&child_args)?;
            let probe_ms = outcome.value.get("rss_probe_ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
            raw_vec.push(outcome.wall_ms);
            wall_vec.push((outcome.wall_ms - probe_ms).max(0.0));
            probe_vec.push(probe_ms);
            open_vec.push(outcome.value.get("open_ms").and_then(|v| v.as_f64()).unwrap_or(0.0));
            let qs = outcome.value.get("queries").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let mut first = None;
            for q in &qs {
                let id = q.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let ms = q.get("ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if first.is_none() {
                    first = Some(ms);
                }
                if !order.contains(&id) {
                    order.push(id.clone());
                }
                per_query_sums.entry(id).or_default().push(ms);
            }
            first_vec.push(first.unwrap_or(0.0));
        }
        let per_query = order
            .iter()
            .map(|id| (id.clone(), per_query_sums.get(id).map(|v| summarize(v).p50).unwrap_or(0.0)))
            .collect();
        cold = Some(V2ColdReport {
            restarts: cold_restarts,
            open_ms: open_vec,
            first_query_ms: first_vec,
            wall_ms: wall_vec,
            wall_raw_ms: raw_vec,
            rss_probe_ms: probe_vec,
            per_query,
        });
    }

    let command = format!(
        "ps-bench content-v2 --legacy-db {} --v2-db {} --queries {} --iters {} --limit {} --cold-restarts {} --out {}",
        legacy_db.display(),
        v2_db.display(),
        queries_path.display(),
        iters,
        limit,
        cold_restarts,
        out.display()
    );

    let report = ContentV2Report {
        schema: RESULT_SCHEMA.to_string(),
        kind: "content-v2-benchmark".to_string(),
        status: "ok".to_string(),
        host: host_info().clone(),
        generated_at_utc: now_utc(),
        command: command.clone(),
        dataset_scope: "v2 ps-content FTS5 (current schema/query path); corpus = 14,609 legacy v1 files_fts documents imported read-only via ps_content::import_legacy".to_string(),
        import,
        index: index_facts,
        open_ms,
        warm,
        cold,
        memory_after_warm,
        limitations: vec![
            "corpus is the imported v1 text corpus, not freshly extracted v2 documents; extractor quality is out of scope for this run".to_string(),
            "queries are the fixed real-content suite (plain user text); v2 sanitizes FTS operators, so prefix/phrase categories are not equivalent to the legacy FTS syntax".to_string(),
            "snippet and snippet_line are produced by ps-content on every hit; snippet cost is included in the measured latency".to_string(),
            "single-process warm measurements; OS page cache state recorded but not controlled".to_string(),
            "import_legacy also copies the legacy files rows; db size includes the filename table, so per-document index size is derived from FTS shadow tables".to_string(),
        ],
    };
    write_json(&out, &report)?;

    println!("ps-bench content-v2 (v2 ps-content FTS5, current schema/query path)");
    println!(
        "  db: {} ({} bytes, {} pages x {}); documents={} errors={} text_bytes={}",
        v2_db.display(),
        report.index.db_bytes,
        report.index.page_count,
        report.index.page_size,
        report.index.documents,
        report.index.errors,
        report.index.indexed_text_bytes
    );
    if let Some(import) = &report.import {
        println!("  import: {} docs / {} files in {:.0} ms", import.content_documents, import.files_copied, import.import_wall_ms);
    }
    println!("  open_ms: {:.2} (stats: docs={} bytes={})", report.open_ms, stats.documents, stats.bytes_indexed);
    println!("  warm ({} iters/query, {} queries):", iters, suite.len());
    println!(
        "   overall: p50={:.3} p95={:.3} p99={:.3} min={:.3} max={:.3} mean={:.3} n={}",
        report.warm.overall.p50, report.warm.overall.p95, report.warm.overall.p99,
        report.warm.overall.min, report.warm.overall.max, report.warm.overall.mean, report.warm.overall.count
    );
    for row in &report.warm.by_category {
        println!("   [{:18}] p50={:7.3} p95={:7.3} p99={:7.3} n={}", row.category, row.summary.p50, row.summary.p95, row.summary.p99, row.summary.count);
    }
    if let Some(cold) = &report.cold {
        println!(
            "  cold ({} restarts): open p50={:.2} ms first-query p50={:.2} ms wall p50={:.0} ms",
            cold.restarts,
            summarize(&cold.open_ms).p50,
            summarize(&cold.first_query_ms).p50,
            summarize(&cold.wall_ms).p50
        );
    }
    println!(
        "  queries_with_results={} zero_result={} est_fts_bytes={}",
        report.warm.queries_with_results, report.warm.zero_result_queries, report.index.estimated_fts_bytes
    );
    for note in &report.limitations {
        println!("  limit: {note}");
    }
    println!("  results: {}", out.display());
    Ok(())
}

/// Child: fresh process, open the v2 index, run the first N queries once.
pub fn cold_child(args: &Args) -> anyhow::Result<()> {
    let v2_db = PathBuf::from(args.required("--v2-db")?);
    let queries_path = PathBuf::from(args.required("--queries")?);
    let limit = args.usize_or("--limit", 20)?;
    let suite: Vec<ContentQuery> = contentbench::load_queries(&queries_path)?;
    let start = Instant::now();
    let index = ps_content::ContentIndex::open(&v2_db)?;
    let open_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut out = Vec::new();
    for q in suite.iter().take(5) {
        let t = Instant::now();
        let hits = index.search(&q.query, limit)?;
        out.push(json!({
            "id": q.id,
            "query": q.query,
            "ms": t.elapsed().as_secs_f64() * 1000.0,
            "returned": hits.len(),
        }));
    }
    let probe_start = Instant::now();
    let mem = self_memory();
    let rss_probe_ms = probe_start.elapsed().as_secs_f64() * 1000.0;
    let payload = json!({
        "rss_probe_ms": rss_probe_ms,
        "open_ms": open_ms,
        "queries": out,
    });
    println!("PSBENCH_JSON:{}", serde_json::to_string(&payload)?);
    Ok(())
}
