//! Search/IPC payload probe (task-64). Not shipped.
//!
//! Measures SearchResponse JSON size, serialization time and per-field payload
//! breakdown for typing sequences and island card scopes, so engine payload cost
//! can be separated from React work.
//!
//! Usage: cargo run -p ps-core --example payload_perf_probe -- <v2-data-dir> [out.json]

use std::path::{Path, PathBuf};
use std::time::Instant;

use ps_ipc::{IconRef, SearchRequest, SearchResponse, SearchScope};

fn summarize(mut samples: Vec<f64>) -> (f64, f64) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p = |p: f64| {
        if samples.is_empty() {
            0.0
        } else {
            let idx = ((samples.len() as f64 - 1.0) * p).round() as usize;
            samples[idx.min(samples.len() - 1)]
        }
    };
    (p(0.5), p(0.95))
}

fn json_len(value: &impl serde::Serialize) -> usize {
    serde_json::to_vec(value).map(|bytes| bytes.len()).unwrap_or(0)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data_dir = PathBuf::from(
        args.first()
            .cloned()
            .unwrap_or_else(|| ".devdata-perf64/v2".into()),
    );
    let out_path = Path::new(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("target/payload-perf.json"),
    );

    match std::fs::read_to_string(data_dir.join("settings.json"))
        .map_err(|err| err.to_string())
        .and_then(|raw| serde_json::from_str::<ps_ipc::Settings>(&raw).map_err(|err| err.to_string()))
    {
        Ok(settings) => println!(
            "settings.json parse: ok clipboard={} content={}",
            settings.privacy.clipboard_history, settings.indexing.content_enabled
        ),
        Err(err) => println!("settings.json parse: ERROR {err}"),
    }
    let direct_clipboard = ps_sources::clipboard::list_recent(&data_dir.join("index.db"), 8);
    println!("direct clipboard list_recent: {} rows", direct_clipboard.len());

    let engine = std::sync::Arc::new(ps_core::Engine::open(ps_core::EngineConfig {
        data_dir: data_dir.clone(),
        legacy_db: None,
        version: "payload-probe".to_string(),
    })?);
    {
        let settings = engine.settings();
        println!(
            "engine settings: clipboard_history={} pause={} content_enabled={}",
            settings.privacy.clipboard_history, settings.clipboard.pause, settings.indexing.content_enabled
        );
    }
    // Load the sources snapshot (apps/browser/clipboard) the same way the app
    // does, without scanning any roots: the probe copy has roots disabled.
    if std::env::var("PS_PROBE_BACKGROUND").is_ok() {
        let background = std::sync::Arc::clone(&engine);
        background.spawn_background();
        let deadline = Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let status = engine.status();
            let state = format!("{:?}", status.state);
            if state != "Starting" && state != "Indexing" {
                println!("background ready: state={state} apps_loaded");
                break;
            }
            if Instant::now() > deadline {
                println!("background load timed out (state={state})");
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    // (label, query, scope, limit)
    let cases: Vec<(&str, &str, SearchScope, usize)> = vec![
        ("type r", "r", SearchScope::All, 30),
        ("type re", "re", SearchScope::All, 30),
        ("type rea", "rea", SearchScope::All, 30),
        ("type read", "read", SearchScope::All, 30),
        ("type readm", "readm", SearchScope::All, 30),
        ("type readme", "readme", SearchScope::All, 30),
        ("type s", "s", SearchScope::All, 30),
        ("type sc", "sc", SearchScope::All, 30),
        ("type scre", "scre", SearchScope::All, 30),
        ("type screenshot", "screenshot", SearchScope::All, 30),
        ("type p", "p", SearchScope::All, 30),
        ("type pro", "pro", SearchScope::All, 30),
        ("type project", "project", SearchScope::All, 30),
        ("app chrome", "chrome", SearchScope::All, 30),
        ("app code", "code", SearchScope::All, 30),
        ("app edge", "edge", SearchScope::All, 30),
        ("app word", "word", SearchScope::All, 30),
        ("apps scope c", "c", SearchScope::Apps, 30),
        ("empty all", "", SearchScope::All, 30),
        ("empty clipboard", "", SearchScope::Clipboard, 30),
        ("empty apps", "", SearchScope::Apps, 30),
        ("limit 10 readme", "readme", SearchScope::All, 10),
        ("limit 50 readme", "readme", SearchScope::All, 50),
    ];

    let mut rows = Vec::new();
    for (label, query, scope, limit) in cases {
        let mut request = SearchRequest::new(1, query);
        request.scope = scope;
        request.limit = limit;
        for _ in 0..2 {
            let _ = engine.search(&request);
        }
        let mut engine_ms = Vec::new();
        let mut serialize_ms = Vec::new();
        let mut last: Option<SearchResponse> = None;
        for _ in 0..10 {
            let started = Instant::now();
            let response = engine.search(&request);
            engine_ms.push(started.elapsed().as_secs_f64() * 1000.0);
            let started = Instant::now();
            let bytes = serde_json::to_vec(&response).unwrap();
            serialize_ms.push(started.elapsed().as_secs_f64() * 1000.0);
            let _ = bytes.len();
            last = Some(response);
        }
        let response = last.expect("response");
        let payload_bytes = json_len(&response);
        let mut icon_data_bytes = 0usize;
        let mut icon_data_count = 0usize;
        let mut icon_other_count = 0usize;
        let mut thumbnail_bytes = 0usize;
        let mut thumbnail_count = 0usize;
        let mut action_count = 0usize;
        let mut action_bytes = 0usize;
        let mut snippet_bytes = 0usize;
        let mut path_bytes = 0usize;
        let mut subtitle_bytes = 0usize;
        let mut result_bytes = 0usize;
        let mut app_rows = 0usize;
        let mut clipboard_rows = 0usize;
        for result in &response.results {
            result_bytes += json_len(result);
            match &result.icon {
                IconRef::AppIconData { data } => {
                    icon_data_count += 1;
                    icon_data_bytes += data.len();
                }
                _ => icon_other_count += 1,
            }
            if let Some(thumb) = result.meta.thumbnail_data_url.as_deref() {
                thumbnail_count += 1;
                thumbnail_bytes += thumb.len();
            }
            action_count += result.actions.len();
            action_bytes += json_len(&result.actions);
            snippet_bytes += result.meta.snippet.as_deref().map(str::len).unwrap_or(0);
            path_bytes += result.path.as_deref().map(str::len).unwrap_or(0);
            subtitle_bytes += result.subtitle.len();
            if result.kind == ps_ipc::SourceKind::Application {
                app_rows += 1;
            }
            if result.kind == ps_ipc::SourceKind::Clipboard {
                clipboard_rows += 1;
            }
        }
        let (engine_p50, engine_p95) = summarize(engine_ms);
        let (ser_p50, _ser_p95) = summarize(serialize_ms);
        let serialized = serde_json::to_vec(&response).unwrap();
        let null_count = String::from_utf8_lossy(&serialized).matches("null").count();
        println!(
            "{label:<20} q={query:<12} scope={scope:?} limit={limit:<3} rows={:<3} engine={engine_p50:6.2}/{engine_p95:6.2}ms ser={ser_p50:5.2}ms json={payload_bytes:7}B icons={icon_data_count}x{icon_data_bytes:6}B thumbs={thumbnail_count}x{thumbnail_bytes:6}B actions={action_count}x{action_bytes:5}B snippets={snippet_bytes:5}B",
            response.results.len()
        );
        rows.push(serde_json::json!({
            "label": label,
            "query": query,
            "scope": format!("{scope:?}"),
            "limit": limit,
            "rows": response.results.len(),
            "engineP50": engine_p50,
            "engineP95": engine_p95,
            "serializeP50": ser_p50,
            "jsonBytes": payload_bytes,
            "resultBytes": result_bytes,
            "iconDataCount": icon_data_count,
            "iconDataBytes": icon_data_bytes,
            "iconOtherCount": icon_other_count,
            "thumbnailCount": thumbnail_count,
            "thumbnailBytes": thumbnail_bytes,
            "actionCount": action_count,
            "actionBytes": action_bytes,
            "snippetBytes": snippet_bytes,
            "pathBytes": path_bytes,
            "subtitleBytes": subtitle_bytes,
            "appRows": app_rows,
            "clipboardRows": clipboard_rows,
            "timingsBytes": json_len(&response.timings),
            "statusBytes": json_len(&response.status),
            "nullCount": null_count,
        }));
    }

    // Icon cache cost model: what does reading + base64-ing one icon cost, and
    // how big is the icon corpus the UI receives?
    let icon_dir = data_dir.join("cache").join("icons");
    let mut icon_files: Vec<PathBuf> = std::fs::read_dir(&icon_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().map(|ext| ext == "png").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    icon_files.sort();
    let mut read_ms = Vec::new();
    let mut icon_total = 0usize;
    for path in icon_files.iter().take(64) {
        let started = Instant::now();
        if let Ok(bytes) = std::fs::read(path) {
            icon_total += bytes.len();
            let _ = base64_encode(&bytes);
        }
        read_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    let read_count = read_ms.len();
    let (read_p50, read_p95) = summarize(read_ms);
    println!(
        "icon cache: {} png files, sampled {read_count} read+base64 p50={read_p50:.2} p95={read_p95:.2} ms",
        icon_files.len()
    );

    let doc = serde_json::json!({
        "schema": "protonsearch-v2/payload-perf/1",
        "dataDir": data_dir.display().to_string(),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "rows": rows,
        "iconCache": { "files": icon_files.len(), "sampledBytes": icon_total, "readPlusBase64P50Ms": read_p50, "readPlusBase64P95Ms": read_p95 },
    });
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(out_path, serde_json::to_string_pretty(&doc)?)?;
    println!("wrote {}", out_path.display());
    Ok(())
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}
