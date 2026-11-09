//! Filename index benchmark: dataset load, index build, memory, warm/cold search.
//!
//! Engine-only time is separated from dataset I/O and process startup:
//!   parse_ms  JSONL -> Vec<IndexEntry>
//!   build_ms  FilenameIndex construction (replace_all today; from_entries when
//!             engine-core lands it)
//!   warm search p50/p95/p99 over the fixed query suite (in-process)
//!   cold = fresh process + index load + first query latencies that follow
//!
//! If ps-index::search is still a stub the report says engine_not_ready and no
//! latency claim is made.

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;
use serde_json::json;

use ps_index::{EntryKind, FilenameIndex, IndexEntry};

use crate::args::Args;
use crate::childproc;
use crate::dataset;
use crate::env::{host_info, self_memory, ProcessMemory};
use crate::queries::{self, QueryDef};
use crate::stats::{summarize, Summary};
use crate::{write_json, RESULT_SCHEMA};

/// Stream a dataset JSONL into ps-index entries (no full DatasetEntry Vec).
pub fn load_index_entries(path: &Path) -> anyhow::Result<Vec<IndexEntry>> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)?;
    let reader = BufReader::with_capacity(1 << 20, file);
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let e: dataset::DatasetEntry = serde_json::from_str(&line)
            .map_err(|err| anyhow::anyhow!("{}:{}: {err}", path.display(), i + 1))?;
        out.push(IndexEntry {
            path: e.path,
            name: e.name,
            ext: e.ext,
            kind: if e.kind == "folder" { EntryKind::Folder } else { EntryKind::File },
            size: e.size,
            modified: e.modified,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildReport {
    pub dataset_path: String,
    pub dataset_bytes: u64,
    pub dataset_fnv1a64: String,
    pub entries: usize,
    pub parse_ms: f64,
    pub build_ms: f64,
    pub total_ms: f64,
    pub memory_before: Option<ProcessMemory>,
    pub memory_after_build: Option<ProcessMemory>,
    pub engine_memory_bytes: u64,
    pub rss_delta_bytes: i64,
    pub rss_bytes_per_entry: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryTiming {
    pub id: String,
    pub category: String,
    pub query: String,
    pub result_count: usize,
    pub top1_path: Option<String>,
    pub samples_ms: Vec<f64>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryRow {
    pub category: String,
    pub summary: Summary,
    pub queries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WarmReport {
    pub iterations: usize,
    pub overall: Summary,
    pub by_category: Vec<CategoryRow>,
    pub queries: Vec<QueryTiming>,
    pub queries_returning_zero: usize,
    pub queries_with_results: usize,
    pub deterministic_order: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColdRestart {
    pub wall_ms: f64,
    pub load_ms: f64,
    pub build_ms: f64,
    pub parse_ms: f64,
    pub load_mode: String,
    pub rss_bytes: u64,
    pub rss_probe_ms: f64,
    pub wall_raw_ms: f64,
    pub first_query_ms: Vec<f64>,
    pub per_query: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilenameReport {
    pub schema: String,
    pub kind: String,
    pub status: String,
    pub host: crate::env::HostInfo,
    pub generated_at_utc: String,
    pub command: String,
    pub engine: String,
    pub build: BuildReport,
    pub warm: WarmReport,
    pub cold: Vec<ColdRestart>,
    pub save: Option<String>,
    pub notes: Vec<String>,
}

fn now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

/// Map a hit to a path through the arena accessor; never materializes entries().
fn hit_path(index: &FilenameIndex, hit_index: usize) -> Option<String> {
    if hit_index >= index.len() {
        return None;
    }
    Some(index.entry_path(hit_index).into_owned())
}

fn run_queries(index: &FilenameIndex, suite: &[QueryDef], iters: usize, limit: usize) -> WarmReport {
    let mut overall_slices: Vec<Vec<f64>> = Vec::new();
    let mut by_cat: std::collections::HashMap<String, Vec<Vec<f64>>> = std::collections::HashMap::new();
    let mut timings = Vec::new();
    let mut deterministic = true;
    let mut with_results = 0usize;
    let mut zero_results = 0usize;

    for q in suite {
        // Warmup
        for _ in 0..2 {
            let _ = index.search(&q.query, limit);
        }
        let mut samples = Vec::with_capacity(iters);
        let mut result_count = 0usize;
        let mut top1 = None;
        let mut first_order: Vec<usize> = Vec::new();
        for iter in 0..iters {
            let start = Instant::now();
            let hits = index.search(&q.query, limit);
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            if iter == 0 {
                result_count = hits.len();
                top1 = hits.first().and_then(|h| hit_path(index, h.entry_index));
                first_order = hits.iter().map(|h| h.entry_index).collect();
            } else {
                let order: Vec<usize> = hits.iter().map(|h| h.entry_index).collect();
                if order != first_order {
                    deterministic = false;
                }
            }
        }
        if result_count > 0 {
            with_results += 1;
        } else {
            zero_results += 1;
        }
        let summary = summarize(&samples);
        by_cat.entry(q.category.clone()).or_default().push(samples.clone());
        overall_slices.push(samples.clone());
        timings.push(QueryTiming {
            id: q.id.clone(),
            category: q.category.clone(),
            query: q.query.clone(),
            result_count,
            top1_path: top1,
            samples_ms: samples,
            summary,
        });
    }

    let mut categories: Vec<CategoryRow> = by_cat
        .into_iter()
        .map(|(category, slices)| {
            let queries = slices.len();
            CategoryRow { category, summary: crate::stats::summarize_slices(&slices), queries }
        })
        .collect();
    categories.sort_by(|a, b| a.category.cmp(&b.category));

    WarmReport {
        iterations: iters,
        overall: crate::stats::summarize_slices(&overall_slices),
        by_category: categories,
        queries: timings,
        queries_returning_zero: zero_results,
        queries_with_results: with_results,
        deterministic_order: deterministic,
    }
}

pub fn run(args: &Args) -> anyhow::Result<()> {
    let dataset_path = PathBuf::from(args.required("--dataset")?);
    let queries_path = PathBuf::from(args.get("--queries").unwrap_or(".bench/queries.jsonl"));
    let out = PathBuf::from(args.required("--out")?);
    let iters = args.usize_or("--iters", 30)?.max(1);
    let limit = args.usize_or("--limit", 50)?;
    let cold_restarts = args.usize_or("--cold-restarts", 0)?;
    let cold_queries = args.usize_or("--cold-queries", 5)?;
    let save_index = args.get("--save-index").map(PathBuf::from);

    let suite = queries::load_suite(&queries_path)?;
    anyhow::ensure!(!suite.is_empty(), "empty query suite: {}", queries_path.display());
    let dataset_bytes = std::fs::metadata(&dataset_path).map(|m| m.len()).unwrap_or(0);
    let (_, fnv) = crate::fnv1a64_file(&dataset_path)?;
    let memory_before = self_memory();

    let parse_start = Instant::now();
    let entries = load_index_entries(&dataset_path)?;
    let parse_ms = parse_start.elapsed().as_secs_f64() * 1000.0;
    let entry_count = entries.len();

    let build_start = Instant::now();
    // from_entries is the engine's production construction path (arena build).
    let index = FilenameIndex::from_entries(entries);
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;
    let index_memory_bytes = index.memory_bytes();
    let memory_after_build = self_memory();
    let rss_delta = match (&memory_before, &memory_after_build) {
        (Some(a), Some(b)) => b.working_set_bytes as i64 - a.working_set_bytes as i64,
        _ => 0,
    };

    anyhow::ensure!(
        index.len() == entry_count,
        "index length {} != dataset entries {} (load dropped entries?)",
        index.len(),
        entry_count
    );

    let warm = run_queries(&index, &suite, iters, limit);
    let non_zero_queries = suite.iter().filter(|q| q.category != "zero_match").count();
    let engine_ready = warm.queries_with_results >= non_zero_queries / 2 && warm.queries_with_results > 0;
    let status = if engine_ready { "ok" } else { "engine_not_ready_search_empty" };

    let mut save_status = None;
    if let Some(path) = &save_index {
        let start = Instant::now();
        match index.save(path) {
            Ok(()) => save_status = Some(format!("ok ({:.1} ms, {} bytes)", start.elapsed().as_secs_f64() * 1000.0, std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))),
            Err(err) => save_status = Some(format!("save failed: {err}")),
        }
    }

    let mut cold = Vec::new();
    for _ in 0..cold_restarts {
        let mut child_args: Vec<String> = vec![
            "filename-cold-child".to_string(),
            "--dataset".to_string(),
            dataset_path.display().to_string(),
            "--queries".to_string(),
            queries_path.display().to_string(),
            "--limit".to_string(),
            limit.to_string(),
            "--cold-queries".to_string(),
            cold_queries.to_string(),
        ];
        if let Some(path) = &save_index {
            if save_status.as_deref().map(|s| s.starts_with("ok")).unwrap_or(false) {
                child_args.push("--index-bin".to_string());
                child_args.push(path.display().to_string());
            }
        }
        let outcome = childproc::run_child_json(&child_args)?;
        let v = &outcome.value;
        let per_query: Vec<(String, f64)> = v
            .get("queries")
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|q| {
                        (
                            q.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                            q.get("ms").and_then(|x| x.as_f64()).unwrap_or(0.0),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let first_query_ms: Vec<f64> = per_query.iter().take(1).map(|(_, ms)| *ms).collect();
        let probe_ms = v.get("rss_probe_ms").and_then(|x| x.as_f64()).unwrap_or(0.0);
        cold.push(ColdRestart {
            wall_ms: (outcome.wall_ms - probe_ms).max(0.0),
            wall_raw_ms: outcome.wall_ms,
            rss_probe_ms: probe_ms,
            load_ms: v.get("load_ms").and_then(|x| x.as_f64()).unwrap_or(0.0),
            build_ms: v.get("build_ms").and_then(|x| x.as_f64()).unwrap_or(0.0),
            parse_ms: v.get("parse_ms").and_then(|x| x.as_f64()).unwrap_or(0.0),
            load_mode: v.get("load_mode").and_then(|x| x.as_str()).unwrap_or("unknown").to_string(),
            rss_bytes: v.get("rss_bytes").and_then(|x| x.as_u64()).unwrap_or(0),
            first_query_ms,
            per_query,
        });
    }

    let command = format!(
        "ps-bench filename --dataset {} --queries {} --iters {} --limit {} --cold-restarts {} --out {}",
        dataset_path.display(),
        queries_path.display(),
        iters,
        limit,
        cold_restarts,
        out.display()
    );
    let report = FilenameReport {
        schema: RESULT_SCHEMA.to_string(),
        kind: "filename-benchmark".to_string(),
        status: status.to_string(),
        host: host_info().clone(),
        generated_at_utc: now_utc(),
        command: command.clone(),
        engine: format!("ps-index {} (from_entries + search + save/load + memory_bytes)", env!("CARGO_PKG_VERSION")),
        build: BuildReport {
            dataset_path: dataset_path.display().to_string(),
            dataset_bytes,
            dataset_fnv1a64: format!("{fnv:016x}"),
            entries: entry_count,
            parse_ms,
            build_ms,
            total_ms: parse_ms + build_ms,
            memory_before: memory_before.clone(),
            memory_after_build,
            engine_memory_bytes: index_memory_bytes as u64,
            rss_delta_bytes: rss_delta,
            rss_bytes_per_entry: if entry_count > 0 { rss_delta as f64 / entry_count as f64 } else { 0.0 },
        },
        warm,
        cold,
        save: save_status,
        notes: vec![
            "engine-only time: parse/build measured in-process; process startup excluded from warm search".to_string(),
            "status=engine_not_ready means the engine returned no results for the suite; search numbers must not be cited".to_string(),
            "RSS delta is working-set based and includes allocator slack; peak RSS is reported for context".to_string(),
            "cold restarts measure process spawn + dataset/index load + first queries; wall_ms includes OS process creation cost".to_string(),
        ],
    };

    write_json(&out, &report)?;

    println!("ps-bench filename: dataset={} entries={} status={}", dataset_path.display(), entry_count, status);
    println!("  dataset: {} bytes fnv1a64={:016x} parse_ms={:.1} build_ms={:.1} total_ms={:.1}", dataset_bytes, fnv, report.build.parse_ms, report.build.build_ms, report.build.total_ms);
    if let (Some(a), Some(b)) = (&memory_before, &report.build.memory_after_build) {
        println!("  rss: before={} MB after_build={} MB peak={} MB delta={:.1} MB ({:.1} B/entry); engine.memory_bytes={:.1} MB", a.working_set_bytes / 1_000_000, b.working_set_bytes / 1_000_000, b.peak_working_set_bytes / 1_000_000, report.build.rss_delta_bytes as f64 / 1e6, report.build.rss_bytes_per_entry, report.build.engine_memory_bytes as f64 / 1e6);
    }
    println!("  warm search ({} iters/query, {} queries, limit {}):", iters, suite.len(), limit);
    print_summary("  overall", &report.warm.overall);
    for row in &report.warm.by_category {
        println!("  [{:18}] p50={:8.3} p95={:8.3} p99={:8.3} n={}", row.category, row.summary.p50, row.summary.p95, row.summary.p99, row.summary.count);
    }
    println!(
        "  queries_with_results={} zero={} deterministic_order={}",
        report.warm.queries_with_results, report.warm.queries_returning_zero, report.warm.deterministic_order
    );
    if let Some(save) = &report.save {
        println!("  save: {save}");
    }
    if !report.cold.is_empty() {
        let load: Vec<f64> = report.cold.iter().map(|c| c.load_ms).collect();
        let wall: Vec<f64> = report.cold.iter().map(|c| c.wall_ms).collect();
        let first: Vec<f64> = report.cold.iter().flat_map(|c| c.first_query_ms.iter().copied()).collect();
        let rss: Vec<f64> = report.cold.iter().map(|c| c.rss_bytes as f64).collect();
        println!(
            "  cold restart ({}): load p50={:.1} ms wall p50={:.0} ms rss p50={:.0} MB first-query p50={:.3} ms",
            report.cold.len(),
            summarize(&load).p50,
            summarize(&wall).p50,
            summarize(&rss).p50 / 1e6,
            summarize(&first).p50
        );
        let modes: Vec<&str> = report.cold.iter().map(|c| c.load_mode.as_str()).collect();
        println!("  cold load modes: {:?}", modes);
    }
    if status != "ok" {
        println!("  WARNING: ps-index::search returned no results for the suite; latency numbers are NOT valid for claims.");
    }
    for note in &report.notes {
        println!("  note: {note}");
    }
    println!("  results: {}", out.display());
    Ok(())
}

/// Child: fresh process, load (or build) index, run first N queries once.
pub fn cold_child(args: &Args) -> anyhow::Result<()> {
    let dataset_path = PathBuf::from(args.required("--dataset")?);
    let queries_path = PathBuf::from(args.required("--queries")?);
    let limit = args.usize_or("--limit", 50)?;
    let cold_queries = args.usize_or("--cold-queries", 5)?;
    let suite = queries::load_suite(&queries_path)?;

    let mut load_ms = 0.0f64;
    let mut parse_ms = 0.0f64;
    let mut build_ms = 0.0f64;
    let mut load_mode = "dataset_parse+build".to_string();
    let mut index;
    if let Some(bin) = args.get("--index-bin") {
        let start = Instant::now();
        match FilenameIndex::load(Path::new(bin)) {
            Ok(loaded) => {
                load_ms = start.elapsed().as_secs_f64() * 1000.0;
                index = loaded;
                load_mode = "index_binary_load".to_string();
            }
            Err(err) => {
                load_mode = format!("index_binary_load_failed: {err}; fell back to dataset parse");
                let start = Instant::now();
                let entries = load_index_entries(&dataset_path)?;
                parse_ms = start.elapsed().as_secs_f64() * 1000.0;
                let start = Instant::now();
                index = FilenameIndex::from_entries(entries);
                build_ms = start.elapsed().as_secs_f64() * 1000.0;
            }
        }
    } else {
        let start = Instant::now();
        let entries = load_index_entries(&dataset_path)?;
        parse_ms = start.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        index = FilenameIndex::from_entries(entries);
        build_ms = start.elapsed().as_secs_f64() * 1000.0;
    }

    let mut out = Vec::new();
    for q in suite.iter().take(cold_queries) {
        let start = Instant::now();
        let hits = index.search(&q.query, limit);
        out.push(json!({
            "id": q.id,
            "query": q.query,
            "ms": start.elapsed().as_secs_f64() * 1000.0,
            "results": hits.len(),
        }));
    }
    let probe_start = Instant::now();
    let mem = self_memory();
    let rss_probe_ms = probe_start.elapsed().as_secs_f64() * 1000.0;
    let payload = json!({
        "rss_probe_ms": rss_probe_ms,
        "load_ms": load_ms,
        "parse_ms": parse_ms,
        "build_ms": build_ms,
        "load_mode": load_mode,
        "rss_bytes": mem.as_ref().map(|m| m.working_set_bytes).unwrap_or(0),
        "queries": out,
    });
    println!("PSBENCH_JSON:{}", serde_json::to_string(&payload)?);
    Ok(())
}

fn print_summary(label: &str, s: &Summary) {
    println!(
        "{}: p50={:.3} p95={:.3} p99={:.3} min={:.3} max={:.3} mean={:.3} n={}",
        label, s.p50, s.p95, s.p99, s.min, s.max, s.mean, s.count
    );
}
