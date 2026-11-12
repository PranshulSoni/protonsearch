//! OCR retrieval baseline: ps-ocr Windows.Media.Ocr extraction + OcrIndex FTS5.
//!
//! Scope label in every report: "v10 OCR baseline: Windows.Media.Ocr extraction
//! through ps-ocr + OcrIndex FTS5 retrieval over a defined 90-image synthetic
//! corpus". This measures indexed-OCR retrieval (searching already extracted
//! text) separately from extraction throughput/accuracy, and never re-runs OCR
//! per query.
//!
//! OWNER: perf-bench (L1-C). ps-ocr is owned by sources-content.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::args::Args;
use crate::childproc;
use crate::dataset::is_cjk;
use crate::env::{host_info, self_memory, ProcessMemory};
use crate::ocrbench::{cer, load_manifest, normalize, ManifestEntry};
use crate::stats::{summarize, Summary};
use crate::{write_json, RESULT_SCHEMA};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrQuery {
    pub id: String,
    pub category: String,
    pub query: String,
    pub expected_hits: u64,
    pub note: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Accuracy {
    pub images: usize,
    pub mean_cer: f64,
    pub median_cer: f64,
    pub exact_rate: f64,
    pub char_accuracy: f64,
    pub chars: usize,
    pub errors: usize,
}

fn accuracy(rows: &[(f64, usize, usize, bool)]) -> Accuracy {
    if rows.is_empty() {
        return Accuracy::default();
    }
    let mut cers: Vec<f64> = rows.iter().map(|r| r.0).collect();
    cers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let chars: usize = rows.iter().map(|r| r.2).sum();
    let errors: usize = rows.iter().map(|r| r.1).sum();
    let exact = rows.iter().filter(|r| r.3).count();
    Accuracy {
        images: rows.len(),
        mean_cer: rows.iter().map(|r| r.0).sum::<f64>() / rows.len() as f64,
        median_cer: cers[cers.len() / 2],
        exact_rate: exact as f64 / rows.len() as f64,
        char_accuracy: if chars > 0 {
            (chars.saturating_sub(errors)) as f64 / chars as f64
        } else {
            0.0
        },
        chars,
        errors,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractionPass {
    pub pass: usize,
    pub images: usize,
    pub none_results: usize,
    pub errors: usize,
    pub wall_ms: f64,
    pub images_per_sec: f64,
    pub image_ms: Summary,
    pub accuracy_all: Accuracy,
    pub accuracy_english: Accuracy,
    pub accuracy_cjk: Accuracy,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorpusFacts {
    pub manifest_path: String,
    pub images: usize,
    pub scenarios: usize,
    pub ground_truth_chars: usize,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexFacts {
    pub db_path: String,
    pub db_bytes: u64,
    pub wal_bytes: u64,
    pub documents: u64,
    pub text_bytes: u64,
    pub estimated_fts_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryTiming {
    pub id: String,
    pub category: String,
    pub query: String,
    pub expected_hits: u64,
    pub returned: usize,
    pub first_path: Option<String>,
    pub first_snippet: String,
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
    pub queries_with_results: usize,
    pub zero_result_queries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColdReport {
    pub restarts: usize,
    pub open_ms: Vec<f64>,
    pub first_query_ms: Vec<f64>,
    pub wall_ms: Vec<f64>,
    pub wall_raw_ms: Vec<f64>,
    pub rss_probe_ms: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OcrBaselineReport {
    pub schema: String,
    pub kind: String,
    pub status: String,
    pub reason: String,
    pub scope: String,
    pub host: crate::env::HostInfo,
    pub generated_at_utc: String,
    pub command: String,
    pub engine: String,
    pub corpus: CorpusFacts,
    pub extraction: Vec<ExtractionPass>,
    pub index: IndexFacts,
    pub open_ms: f64,
    pub warm: WarmReport,
    pub cold: Option<ColdReport>,
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

fn opt_sum(conn: &rusqlite::Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0))
        .unwrap_or(0)
        .max(0) as u64
}

fn index_facts(db_path: &Path) -> IndexFacts {
    let mut facts = IndexFacts {
        db_path: db_path.display().to_string(),
        db_bytes: std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0),
        wal_bytes: std::fs::metadata(format!("{}-wal", db_path.display()))
            .map(|m| m.len())
            .unwrap_or(0),
        documents: 0,
        text_bytes: 0,
        estimated_fts_bytes: 0,
    };
    if let Ok(conn) = rusqlite::Connection::open(db_path) {
        let _ = conn.execute_batch("PRAGMA query_only=ON;");
        facts.documents = opt_sum(&conn, "SELECT COUNT(*) FROM ocr_documents");
        facts.text_bytes = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(text)),0) FROM ocr_documents");
        let data = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(block)),0) FROM ocr_fts_data");
        let idx = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(term)),0) FROM ocr_fts_idx");
        let content = opt_sum(
            &conn,
            "SELECT COALESCE(SUM(LENGTH(c0)+LENGTH(c1)),0) FROM ocr_fts_content",
        );
        let docsize = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(sz)),0) FROM ocr_fts_docsize");
        facts.estimated_fts_bytes = data + idx + content + docsize;
    }
    facts
}

fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            cur.extend(ch.to_lowercase());
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn keep(t: &str) -> bool {
    let n = t.chars().count();
    if n < 4 || n > 24 {
        return false;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if t.chars().any(is_cjk) {
        return n <= 12;
    }
    t.chars().all(|c| c.is_ascii_alphabetic())
}

/// Build a deterministic query suite from the OCR-recognized texts.
fn build_queries(docs: &[(String, String)], seed: u64) -> Vec<OcrQuery> {
    let mut df: HashMap<String, u32> = HashMap::new();
    for (_, text) in docs {
        let mut seen = std::collections::HashSet::new();
        for tok in tokenize(text) {
            if keep(&tok) && seen.insert(tok.clone()) {
                *df.entry(tok).or_insert(0) += 1;
            }
        }
    }
    let mut ranked: Vec<(String, u32)> = df.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let common: std::collections::HashSet<String> = ranked
        .iter()
        .filter(|(_, n)| *n >= 2)
        .map(|(t, _)| t.clone())
        .collect();
    let plausible = |t: &str| keep(t) && t.chars().all(|c| c.is_ascii_alphabetic()) && common.contains(t);

    let mut rng = crate::dataset::Rng::new(seed);
    let mut pick = |items: &[(String, u32)], n: usize, rng: &mut crate::dataset::Rng| -> Vec<(String, u32)> {
        let mut idx: Vec<usize> = (0..items.len()).collect();
        for i in (1..idx.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            idx.swap(i, j);
        }
        idx.into_iter().take(n).map(|i| items[i].clone()).collect()
    };

    let high: Vec<(String, u32)> = ranked.iter().filter(|(_, n)| *n >= 20).cloned().collect();
    let medium: Vec<(String, u32)> = ranked
        .iter()
        .filter(|(_, n)| (3..20).contains(n))
        .cloned()
        .collect();
    let mut suite: Vec<OcrQuery> = Vec::new();
    let mut used = std::collections::HashSet::new();
    let mut push = |id: String,
                    category: &str,
                    query: String,
                    note: String,
                    suite: &mut Vec<OcrQuery>,
                    used: &mut std::collections::HashSet<String>| {
        if used.insert(query.clone()) {
            suite.push(OcrQuery {
                id,
                category: category.to_string(),
                query,
                expected_hits: 0,
                note,
            });
        }
    };
    for (i, (term, n)) in pick(&high, 8, &mut rng).into_iter().enumerate() {
        push(
            format!("hi_{i}"),
            "single_high_df",
            term.clone(),
            format!("df={n} in recognized OCR text"),
            &mut suite,
            &mut used,
        );
    }
    for (i, (term, n)) in pick(&medium, 8, &mut rng).into_iter().enumerate() {
        push(
            format!("med_{i}"),
            "single_medium_df",
            term.clone(),
            format!("df={n}"),
            &mut suite,
            &mut used,
        );
    }
    // pairs from one recognized document
    let mut pairs = 0;
    for (_, text) in docs {
        if pairs >= 4 {
            break;
        }
        let terms: Vec<String> = tokenize(text).into_iter().filter(|t| plausible(t)).collect();
        if terms.len() >= 4 {
            push(
                format!("pair_{pairs}"),
                "multi_term",
                format!("{} {}", terms[0], terms[1]),
                "two terms from one OCR document".to_string(),
                &mut suite,
                &mut used,
            );
            pairs += 1;
        }
    }
    // phrases (two consecutive words) from recognized text
    let mut phrases = 0;
    for (_, text) in docs {
        if phrases >= 3 {
            break;
        }
        let toks = tokenize(text);
        let mut found = None;
        for w in toks.windows(2) {
            if plausible(&w[0]) && plausible(&w[1]) {
                found = Some(format!("\"{}\"", w.join(" ")));
                break;
            }
        }
        if let Some(q) = found {
            push(
                format!("phrase_{phrases}"),
                "phrase",
                q,
                "adjacent-word phrase".to_string(),
                &mut suite,
                &mut used,
            );
            phrases += 1;
        }
    }
    // CJK tokens
    let cjk_terms: Vec<(String, u32)> = ranked
        .iter()
        .filter(|(t, n)| *n >= 1 && t.chars().any(is_cjk))
        .cloned()
        .collect();
    for (i, (term, _)) in pick(&cjk_terms, 3, &mut rng).into_iter().enumerate() {
        let short: String = term.chars().take(4).collect();
        push(
            format!("cjk_{i}"),
            "cjk",
            short,
            "CJK token substring".to_string(),
            &mut suite,
            &mut used,
        );
    }
    for i in 0..3 {
        push(
            format!("zero_{i}"),
            "zero_match",
            format!("zzqxocr{i}plkjhgf"),
            "must return no OCR documents".to_string(),
            &mut suite,
            &mut used,
        );
    }
    suite
}

fn write_queries(path: &Path, queries: &[OcrQuery]) -> anyhow::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = String::new();
    for q in queries {
        text.push_str(&serde_json::to_string(q)?);
        text.push('\n');
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn load_queries(path: &Path) -> anyhow::Result<Vec<OcrQuery>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(line).map_err(|e| anyhow::anyhow!("{}:{}: {e}", path.display(), i + 1))?,
        );
    }
    Ok(out)
}

pub fn run(args: &Args) -> anyhow::Result<()> {
    let manifest_path = PathBuf::from(
        args.get("--manifest")
            .unwrap_or(".bench/ocr-corpus/manifest.jsonl"),
    );
    let db_path = PathBuf::from(args.get("--db").unwrap_or(".bench/v10-ocr-index.db"));
    let queries_out = PathBuf::from(
        args.get("--queries-out")
            .unwrap_or(".bench/v10-ocr-queries.jsonl"),
    );
    let out = PathBuf::from(args.required("--out")?);
    let iters = args.usize_or("--iters", 30)?.max(1);
    let limit = args.usize_or("--limit", 20)?;
    let cold_restarts = args.usize_or("--cold-restarts", 2)?;
    let manifest = load_manifest(&manifest_path)?;
    let base_dir = manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let languages = ps_ocr::available_languages();
    let corpus = CorpusFacts {
        manifest_path: manifest_path.display().to_string(),
        images: manifest.len(),
        scenarios: {
            let mut s: Vec<&str> = manifest.iter().map(|m| m.scenario.as_str()).collect();
            s.sort();
            s.dedup();
            s.len()
        },
        ground_truth_chars: manifest.iter().map(|m| m.text.chars().count()).sum(),
        languages: languages.clone(),
    };

    if !ps_ocr::available() {
        let empty_samples: Vec<f64> = Vec::new();
        let report = OcrBaselineReport {
            schema: RESULT_SCHEMA.to_string(),
            kind: "ocr-retrieval-baseline".to_string(),
            status: "unverified".to_string(),
            reason: "ps_ocr::available() == false on this host".to_string(),
            scope: "v10 OCR baseline: Windows.Media.Ocr extraction through ps-ocr + OcrIndex FTS5 retrieval over a defined 90-image synthetic corpus".to_string(),
            host: host_info().clone(),
            generated_at_utc: now_utc(),
            command: format!("ps-bench ocr-baseline --manifest {} --out {}", manifest_path.display(), out.display()),
            engine: "ps-ocr / Windows.Media.Ocr".to_string(),
            corpus,
            extraction: Vec::new(),
            index: index_facts(&db_path),
            open_ms: 0.0,
            warm: WarmReport { iterations: iters, overall: summarize(&empty_samples), by_category: Vec::new(), queries: Vec::new(), queries_with_results: 0, zero_result_queries: 0 },
            cold: None,
            memory_after_warm: self_memory(),
            limitations: vec!["OCR unavailable; report is explicitly unverified".to_string()],
        };
        write_json(&out, &report)?;
        println!("ps-bench ocr-baseline: UNVERIFIED - ps_ocr::available() == false");
        println!("  results: {}", out.display());
        return Ok(());
    }

    // fresh index
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
    if let Some(dir) = db_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let index = ps_ocr::OcrIndex::open(&db_path)?;

    let mut passes: Vec<ExtractionPass> = Vec::new();
    let mut docs: Vec<(String, String)> = Vec::new();
    for pass in 1..=2 {
        let wall_start = Instant::now();
        let mut timings: Vec<f64> = Vec::new();
        let mut errors = 0usize;
        let mut none_results = 0usize;
        let mut all_rows: Vec<(f64, usize, usize, bool)> = Vec::new();
        let mut en_rows: Vec<(f64, usize, usize, bool)> = Vec::new();
        let mut cjk_rows: Vec<(f64, usize, usize, bool)> = Vec::new();
        for entry in &manifest {
            let path = base_dir.join(&entry.file);
            match ps_ocr::extract_image_timed(&path, ps_ocr::MAX_IMAGE_PIXELS) {
                Ok((Some(doc), elapsed)) => {
                    let ms = elapsed.as_secs_f64() * 1000.0;
                    timings.push(ms);
                    let (c, errs, n) = cer(&entry.text, &doc.text);
                    let exact = normalize(&entry.text) == normalize(&doc.text);
                    let row = (c, errs, n, exact);
                    all_rows.push(row);
                    if entry.text.chars().any(is_cjk) {
                        cjk_rows.push(row)
                    } else {
                        en_rows.push(row)
                    }
                    if pass == 1 {
                        if let Ok(hash) = ps_ocr::content_hash_file(&path) {
                            let _ = index.store(&path.display().to_string(), &hash, &doc);
                        }
                        docs.push((entry.file.clone(), doc.text.clone()));
                    }
                }
                Ok((None, elapsed)) => {
                    timings.push(elapsed.as_secs_f64() * 1000.0);
                    none_results += 1;
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }
        let wall_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
        passes.push(ExtractionPass {
            pass,
            images: timings.len(),
            none_results,
            errors,
            wall_ms,
            images_per_sec: if wall_ms > 0.0 {
                timings.len() as f64 / (wall_ms / 1000.0)
            } else {
                0.0
            },
            image_ms: summarize(&timings),
            accuracy_all: accuracy(&all_rows),
            accuracy_english: accuracy(&en_rows),
            accuracy_cjk: accuracy(&cjk_rows),
        });
    }

    let mut suite = build_queries(&docs, crate::dataset::DEFAULT_SEED);
    for q in &mut suite {
        q.expected_hits = index.search(&q.query, 200).map(|h| h.len() as u64).unwrap_or(0);
    }
    let mut verified: Vec<OcrQuery> = Vec::new();
    for q in suite {
        if q.category != "zero_match" && q.expected_hits == 0 {
            continue;
        }
        verified.push(q);
    }
    write_queries(&queries_out, &verified)?;

    let open_start = Instant::now();
    let _ = index.stats();
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;

    let mut all_samples: Vec<Vec<f64>> = Vec::new();
    let mut by_cat: HashMap<String, Vec<Vec<f64>>> = HashMap::new();
    let mut timings = Vec::new();
    let mut with_results = 0usize;
    let mut zero_results = 0usize;
    for q in &verified {
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
                    first_snippet = h.snippet.chars().take(160).collect();
                }
            }
        }
        if returned > 0 {
            with_results += 1
        } else {
            zero_results += 1
        }
        let summary = summarize(&samples);
        by_cat
            .entry(q.category.clone())
            .or_default()
            .push(samples.clone());
        all_samples.push(samples.clone());
        timings.push(QueryTiming {
            id: q.id.clone(),
            category: q.category.clone(),
            query: q.query.clone(),
            expected_hits: q.expected_hits,
            returned,
            first_path,
            first_snippet,
            samples_ms: samples,
            summary,
        });
    }
    let memory_after_warm = self_memory();
    let mut categories: Vec<CategoryRow> = by_cat
        .into_iter()
        .map(|(category, slices)| {
            let queries = slices.len();
            CategoryRow {
                category,
                summary: crate::stats::summarize_slices(&slices),
                queries,
            }
        })
        .collect();
    categories.sort_by(|a, b| a.category.cmp(&b.category));
    let warm = WarmReport {
        iterations: iters,
        overall: crate::stats::summarize_slices(&all_samples),
        by_category: categories,
        queries: timings,
        queries_with_results: with_results,
        zero_result_queries: zero_results,
    };

    let mut cold = None;
    if cold_restarts > 0 {
        let mut open_vec = Vec::new();
        let mut first_vec = Vec::new();
        let mut wall_vec = Vec::new();
        let mut raw_vec = Vec::new();
        let mut probe_vec = Vec::new();
        for _ in 0..cold_restarts {
            let child_args: Vec<String> = vec![
                "ocr-baseline-cold-child".to_string(),
                "--db".to_string(),
                db_path.display().to_string(),
                "--queries".to_string(),
                queries_out.display().to_string(),
                "--limit".to_string(),
                limit.to_string(),
            ];
            let outcome = childproc::run_child_json(&child_args)?;
            let probe_ms = outcome
                .value
                .get("rss_probe_ms")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            raw_vec.push(outcome.wall_ms);
            wall_vec.push((outcome.wall_ms - probe_ms).max(0.0));
            probe_vec.push(probe_ms);
            open_vec.push(
                outcome
                    .value
                    .get("open_ms")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
            );
            let qs = outcome
                .value
                .get("queries")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let first = qs
                .first()
                .and_then(|q| q.get("ms"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            first_vec.push(first);
        }
        cold = Some(ColdReport {
            restarts: cold_restarts,
            open_ms: open_vec,
            first_query_ms: first_vec,
            wall_ms: wall_vec,
            wall_raw_ms: raw_vec,
            rss_probe_ms: probe_vec,
        });
    }

    let report = OcrBaselineReport {
        schema: RESULT_SCHEMA.to_string(),
        kind: "ocr-retrieval-baseline".to_string(),
        status: "ok".to_string(),
        reason: "ps_ocr available; extraction + OcrIndex FTS5 retrieval measured".to_string(),
        scope: "v10 OCR baseline: Windows.Media.Ocr extraction through ps-ocr + OcrIndex FTS5 retrieval over a defined 90-image synthetic corpus (synthetic GDI+ renders, not real screenshots/scans)".to_string(),
        host: host_info().clone(),
        generated_at_utc: now_utc(),
        command: format!("ps-bench ocr-baseline --manifest {} --db {} --iters {} --limit {} --cold-restarts {} --out {}", manifest_path.display(), db_path.display(), iters, limit, cold_restarts, out.display()),
        engine: "ps-ocr / Windows.Media.Ocr + SQLite FTS5 OcrIndex".to_string(),
        corpus,
        extraction: passes,
        index: index_facts(&db_path),
        open_ms,
        warm,
        cold,
        memory_after_warm,
        limitations: vec![
            "synthetic GDI+ rendered text corpus, not real screenshots/scans; accuracy on real screenshots will differ".to_string(),
            "CJK recognition is limited by installed recognizer languages on this host (see corpus.languages); CJK accuracy is reported separately, not merged into a headline".to_string(),
            "indexed OCR retrieval measures OcrIndex FTS5 search of already-extracted text; OCR is never run per query".to_string(),
            "93 images -> small index; this is a baseline for the pipeline, not a large-scale OCR corpus claim".to_string(),
        ],
    };
    write_json(&out, &report)?;

    println!("ps-bench ocr-baseline (ps-ocr + OcrIndex FTS5)");
    println!("  languages: {:?}", report.corpus.languages);
    for p in &report.extraction {
        println!(
            "  extraction pass {}: {} imgs {} none {} errs in {:.0} ms = {:.1} img/s; p50 {:.1}/p95 {:.1} ms; char-acc {:.1}% english {:.1}% cjk {:.1}%",
            p.pass, p.images, p.none_results, p.errors, p.wall_ms, p.images_per_sec,
            p.image_ms.p50, p.image_ms.p95,
            p.accuracy_all.char_accuracy * 100.0, p.accuracy_english.char_accuracy * 100.0, p.accuracy_cjk.char_accuracy * 100.0
        );
    }
    println!(
        "  index: docs={} text_bytes={} db_bytes={} wal_bytes={} est_fts_bytes={}",
        report.index.documents,
        report.index.text_bytes,
        report.index.db_bytes,
        report.index.wal_bytes,
        report.index.estimated_fts_bytes
    );
    println!(
        "  warm retrieval ({} iters/query, {} queries): p50={:.3} p95={:.3} p99={:.3} ms",
        iters,
        verified.len(),
        report.warm.overall.p50,
        report.warm.overall.p95,
        report.warm.overall.p99
    );
    if let Some(cold) = &report.cold {
        println!(
            "  cold ({} restarts): open p50={:.2} ms first-query p50={:.2} ms wall p50={:.0} ms",
            cold.restarts,
            summarize(&cold.open_ms).p50,
            summarize(&cold.first_query_ms).p50,
            summarize(&cold.wall_ms).p50
        );
    }
    for note in &report.limitations {
        println!("  limit: {note}");
    }
    println!("  results: {}", out.display());
    Ok(())
}

/// Child process: open the OCR index fresh, run the first queries once.
pub fn cold_child(args: &Args) -> anyhow::Result<()> {
    let db_path = PathBuf::from(args.required("--db")?);
    let queries_path = PathBuf::from(args.required("--queries")?);
    let limit = args.usize_or("--limit", 20)?;
    let suite = load_queries(&queries_path)?;
    let start = Instant::now();
    let index = ps_ocr::OcrIndex::open(&db_path)?;
    let open_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut out = Vec::new();
    for q in suite.iter().take(5) {
        let t = Instant::now();
        let hits = index.search(&q.query, limit)?;
        out.push(json!({ "id": q.id, "query": q.query, "ms": t.elapsed().as_secs_f64() * 1000.0, "returned": hits.len() }));
    }
    let probe_start = Instant::now();
    let _mem = self_memory();
    let rss_probe_ms = probe_start.elapsed().as_secs_f64() * 1000.0;
    let payload = json!({ "rss_probe_ms": rss_probe_ms, "open_ms": open_ms, "queries": out });
    println!("PSBENCH_JSON:{}", serde_json::to_string(&payload)?);
    Ok(())
}
