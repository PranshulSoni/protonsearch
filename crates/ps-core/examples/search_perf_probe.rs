//! Engine search wall-time probe (task-51). Not shipped.
//!
//! Measures Engine::search p50/p95 on a real data directory and decomposes it
//! into tier timings plus direct ps-content / ps-index calls, so engine time can
//! be separated from UI render time.
//!
//! Usage:
//!   cargo run -p ps-core --example search_perf_probe -- <v2-data-dir> [iters] [out.json]

use std::path::{Path, PathBuf};
use std::time::Instant;

use ps_ipc::{SearchRequest, SearchResponse};

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn summarize(mut samples: Vec<f64>) -> (f64, f64, f64) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        percentile(&samples, 0.99),
    )
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data_dir = PathBuf::from(
        args.first()
            .cloned()
            .unwrap_or_else(|| ".devdata-perf51/v2".into()),
    );
    let iters: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(30);
    let out_path = Path::new(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("target/search-perf-before.json"),
    );

    let queries: &[&str] = &[
        "readme",
        "index",
        "screenshot",
        "report",
        "project phoenix",
        "final",
        "invoice",
        "notes",
        "config",
        "pdf",
        "x",
        "zzzzqqqq",
        "screenshot 2026-11-09",
    ];

    // Optional WAL maintenance before opening (diagnostic): a huge uncheckpointed
    // WAL makes every commit attempt a checkpoint, which shows up as ~10 ms
    // search-path stalls.
    if std::env::var("PS_PROBE_CHECKPOINT").is_ok() {
        let conn = rusqlite::Connection::open(data_dir.join("index.db"))?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        let _ = conn.pragma_update(None, "wal_autocheckpoint", 0);
        let _: (i64, i64, i64) = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        drop(conn);
        println!("wal checkpointed before open");
    }

    let open_start = Instant::now();
    let engine = ps_core::Engine::open(ps_core::EngineConfig {
        data_dir: data_dir.clone(),
        legacy_db: None,
        version: "perf-probe".to_string(),
    })?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let status = engine.status();

    // Direct components for decomposition.
    let content = ps_content::ContentIndex::open(&data_dir.join("index.db"))?;
    let filename = ps_index::FilenameIndex::load(&data_dir.join("filename-index.bin"))?;

    let mut rows = Vec::new();
    for (index, query) in queries.iter().enumerate() {
        // Warmup.
        for _ in 0..3 {
            let _ = engine.search(&SearchRequest::new(1, *query));
            let _ = content.search(query, 12);
            let _ = filename.search(query, 60);
        }
        let mut wall = Vec::with_capacity(iters);
        let mut content_ms = Vec::with_capacity(iters);
        let mut filename_ms = Vec::with_capacity(iters);
        let mut last_response: Option<SearchResponse> = None;
        for iteration in 0..iters {
            let started = Instant::now();
            let response = engine.search(&SearchRequest::new(iteration as u64 + 1, *query));
            wall.push(started.elapsed().as_secs_f64() * 1000.0);
            last_response = Some(response);

            let started = Instant::now();
            let _ = content.search(query, 12);
            content_ms.push(started.elapsed().as_secs_f64() * 1000.0);

            let started = Instant::now();
            let _ = filename.search(query, 60);
            filename_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        let response = last_response.expect("response");
        let tier_avg: Vec<serde_json::Value> = response
            .timings
            .iter()
            .map(|timing| {
                serde_json::json!({
                    "tier": timing.tier,
                    "label": timing.label,
                    "hits": timing.hits,
                    "tookMs": timing.took_ms,
                })
            })
            .collect();
        let (w50, w95, w99) = summarize(wall);
        let (c50, c95, _c99) = summarize(content_ms);
        let (f50, f95, _f99) = summarize(filename_ms);
        println!(
            "{query:<26} engine p50={w50:6.2} p95={w95:6.2} p99={w99:6.2} | content p50={c50:6.2} p95={c95:6.2} | filename p50={f50:5.2} p95={f95:5.2} | total={} timings={}",
            response.total,
            response.timings.iter().map(|t| format!("{}:{:.2}", t.label, t.took_ms)).collect::<Vec<_>>().join(", "),
        );
        let _ = index;
        rows.push(serde_json::json!({
            "query": query,
            "results": response.total,
            "engine": { "p50": w50, "p95": w95, "p99": w99 },
            "contentDirect": { "p50": c50, "p95": c95 },
            "filenameDirect": { "p50": f50, "p95": f95 },
            "tiers": tier_avg,
        }));
    }
    // Second pass with activity logging disabled, to separate SQLite activity
    // writes from the search path itself.
    {
        let mut privacy = engine.settings().privacy;
        privacy.activity_history = false;
        engine
            .update_settings(ps_ipc::SettingsPatch {
                privacy: Some(privacy),
                ..Default::default()
            })
            .unwrap();
        println!("--- engine with activity logging disabled ---");
        for query in queries {
            for _ in 0..3 {
                let _ = engine.search(&SearchRequest::new(1, *query));
            }
            let mut samples = Vec::with_capacity(iters);
            for iteration in 0..iters {
                let started = Instant::now();
                let _ = engine.search(&SearchRequest::new(iteration as u64 + 1, *query));
                samples.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            let (p50, p95, p99) = summarize(samples);
            println!("{query:<26} engine(no-activity) p50={p50:6.2} p95={p95:6.2} p99={p99:6.2}");
        }
    }

    let doc = serde_json::json!({
        "dataDir": data_dir.display().to_string(),
        "iterations": iters,
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "openMs": open_ms,
        "filenameEntries": status.filename_entries,
        "contentDocuments": status.content_documents,
        "ocrDocuments": status.ocr_documents,
        "rows": rows,
    });
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(out_path, serde_json::to_string_pretty(&doc)?)?;
    println!("wrote {}", out_path.display());
    Ok(())
}
