//! Content / FTS5 benchmark.
//!
//! CURRENT MEASUREMENT TARGET: the real legacy v1 FTS5 index
//! (files_fts, 14,609 documents) opened read-only immutable. This is a
//! BASELINE-FOR-MIGRATION, not a v2 number, and every report says so. v2
//! ps-content measurements reuse the same harness once its API is real.
//!
//! OWNER: perf-bench.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::args::Args;
use crate::childproc;
use crate::env::{host_info, self_memory, ProcessMemory};
use crate::realfiles::{db_facts, open_legacy_ro, DbFacts};
use crate::stats::{summarize, Summary};
use crate::{write_json, RESULT_SCHEMA};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentQuery {
    pub id: String,
    pub category: String,
    pub query: String,
    pub hits: u64,
    pub note: String,
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "any", "can", "had", "her", "was",
    "one", "our", "out", "day", "get", "has", "him", "his", "how", "its", "may", "new", "now",
    "old", "see", "two", "who", "boy", "did", "she", "use", "way", "will", "with", "this",
    "that", "from", "they", "have", "were", "said", "each", "which", "their", "there", "about",
    "would", "been", "more", "when", "what", "some", "than", "into", "only", "also", "such",
    "then", "them", "over", "very", "most", "much", "many", "just", "like", "make", "made",
    "does", "here", "where", "while", "your", "file", "files", "data", "text", "page", "pages",
    "https", "http", "www", "com", "org", "net", "content", "type", "true", "false", "null",
    "line", "lines", "yes", "etc", "per", "via", "using", "used", "user", "users", "test",
    "tests", "value", "values", "name", "names", "item", "items", "set", "sets", "add", "added",
    "error", "info", "debug", "warn", "warning", "version", "current", "next", "first", "last",
    "should", "could", "must", "need", "needs", "want", "wants", "thank", "thanks", "please",
    "hello", "world", "example", "docs", "document", "documents", "note", "notes", "section",
    "chapter", "table", "list", "generic", "default", "support", "system", "systems", "work",
];

fn is_cjk_char(c: char) -> bool {
    crate::dataset::is_cjk(c)
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            cur.extend(ch.to_lowercase());
        } else if !cur.is_empty() {
            tokens.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

fn keep_token(t: &str) -> bool {
    let chars = t.chars().count();
    if chars < 3 || chars > 30 {
        return false;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if STOPWORDS.contains(&t) {
        return false;
    }
    if t.chars().any(is_cjk_char) {
        return chars <= 12;
    }
    // ASCII-ish and mostly letters: keeps code identifiers such as read_model.
    let ascii_alpha = t.chars().filter(|c| c.is_ascii_alphabetic() || *c == '_').count();
    ascii_alpha * 2 >= chars
}

/// Build a deterministic FTS5 query suite from the real corpus text.
pub fn generate_queries(
    legacy_db: &Path,
    out: &Path,
    sample_docs: usize,
    seed: u64,
) -> anyhow::Result<Vec<ContentQuery>> {
    let conn = open_legacy_ro(legacy_db)?;
    let mut stmt = conn.prepare("SELECT content FROM files_fts ORDER BY rowid LIMIT ?1")?;
    let mut rows = stmt.query([sample_docs as i64])?;

    let mut df: HashMap<String, u32> = HashMap::new();
    let mut samples: Vec<String> = Vec::new();
    while let Some(row) = rows.next()? {
        let content: String = row.get(0)?;
        if samples.len() < 40 {
            samples.push(content.clone());
        }
        let mut seen: HashSet<String> = HashSet::new();
        for tok in tokenize(&content) {
            if !keep_token(&tok) {
                continue;
            }
            if seen.insert(tok.clone()) {
                *df.entry(tok).or_insert(0) += 1;
            }
        }
    }

    let mut ranked: Vec<(String, u32)> = df.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut suite: Vec<ContentQuery> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    let high: Vec<&(String, u32)> = ranked.iter().filter(|(_, n)| *n >= 200).take(60).collect();
    let medium: Vec<&(String, u32)> = ranked.iter().filter(|(_, n)| (10..=200).contains(n)).take(200).collect();

    let mut push = |id: String, category: &str, query: String, note: String| {
        if used.insert(query.clone()) {
            suite.push(ContentQuery { id, category: category.to_string(), query, hits: 0, note });
        }
    };

    // 8 high-df single terms (worst case for FTS posting traversal).
    for (i, (term, n)) in high.choose_n(seed, 8).into_iter().enumerate() {
        push(format!("hi_{i}"), "single_high_df", term.to_string(), format!("document frequency >= 200 (sampled df={n})"));
    }
    // 8 medium-df single terms (typical query).
    for (i, (term, n)) in medium.choose_n(seed.wrapping_add(1), 8).into_iter().enumerate() {
        push(format!("med_{i}"), "single_medium_df", term.to_string(), format!("sampled df={n}"));
    }
    // 4 FTS prefix queries.
    for (i, (term, n)) in medium.choose_n(seed.wrapping_add(2), 4).into_iter().enumerate() {
        if term.is_ascii() && term.len() >= 5 {
            push(format!("prefix_{i}"), "prefix", format!("{}*", &term[..5]), format!("prefix of sampled df={n} term"));
        }
    }
    // 4 two-term implicit AND queries taken from the same document.
    let mut pair_count = 0;
    for content in &samples {
        if pair_count >= 4 {
            break;
        }
        let terms: Vec<String> = tokenize(content)
            .into_iter()
            .filter(|t| keep_token(t))
            .collect();
        if terms.len() >= 6 {
            let a = terms[0].clone();
            let b = terms[terms.len() / 2].clone();
            push(
                format!("pair_{pair_count}"),
                "multi_term_and",
                format!("{a} {b}"),
                "two terms from one sampled document (FTS implicit AND)".to_string(),
            );
            pair_count += 1;
        }
    }
    // 3 phrase queries from sampled raw text.
    let mut phrase_count = 0;
    for content in &samples {
        if phrase_count >= 3 {
            break;
        }
        let toks = tokenize(content);
        let mut found = None;
        for window in toks.windows(3) {
            let (a, b, c) = (&window[0], &window[1], &window[2]);
            if keep_token(a) && keep_token(b) && keep_token(c) && a.len() > 3 && b.len() > 3 && c.len() > 3 {
                found = Some(format!("\"{a} {b} {c}\""));
                break;
            }
        }
        if let Some(q) = found {
            push(format!("phrase_{phrase_count}"), "phrase", q, "three-token phrase from sampled document".to_string());
            phrase_count += 1;
        }
    }
    // CJK terms when the real corpus contains them.
    let cjk_terms: Vec<&(String, u32)> = ranked.iter().filter(|(t, n)| *n >= 2 && t.chars().any(is_cjk_char)).take(30).collect();
    for (i, (term, n)) in cjk_terms.choose_n(seed.wrapping_add(3), 3).into_iter().enumerate() {
        let short: String = term.chars().take(6).collect();
        push(format!("cjk_{i}"), "cjk", format!("{short}*"), format!("CJK prefix query, sampled df={n}"));
    }
    // 4 zero-match tokens.
    for i in 0..4 {
        push(
            format!("zero_{i}"),
            "zero_match",
            format!("zzqxvbnm{i}plkjhgfdsa"),
            "must return no documents".to_string(),
        );
    }

    // Verify hit counts and drop non-zero categories with no hits.
    let mut verified = Vec::new();
    for mut q in suite {
        let hits: i64 = conn
            .query_row("SELECT COUNT(*) FROM files_fts WHERE files_fts MATCH ?1", [q.query.as_str()], |r| r.get(0))
            .unwrap_or(-1);
        q.hits = hits.max(0) as u64;
        if q.category == "zero_match" && q.hits != 0 {
            q.note = format!("INVALID zero-match query (hits={})", q.hits);
        } else if q.category != "zero_match" && q.hits == 0 {
            continue;
        }
        verified.push(q);
    }
    write_queries(out, &verified)?;
    Ok(verified)
}

trait ChooseN<'a> {
    fn choose_n(&'a self, seed: u64, n: usize) -> Vec<&'a (String, u32)>;
}

impl<'a> ChooseN<'a> for Vec<&'a (String, u32)> {
    fn choose_n(&'a self, seed: u64, n: usize) -> Vec<&'a (String, u32)> {
        if self.is_empty() {
            return Vec::new();
        }
        let mut rng = crate::dataset::Rng::new(seed);
        let mut idx: Vec<usize> = (0..self.len()).collect();
        // Deterministic shuffle (Fisher-Yates with splitmix64).
        for i in (1..idx.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            idx.swap(i, j);
        }
        idx.into_iter().take(n).take(self.len()).map(|i| self[i]).collect()
    }
}

pub fn write_queries(path: &Path, queries: &[ContentQuery]) -> anyhow::Result<()> {
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

pub fn load_queries(path: &Path) -> anyhow::Result<Vec<ContentQuery>> {
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(line).map_err(|e| anyhow::anyhow!("{}:{}: {e}", path.display(), i + 1))?);
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexFacts {
    pub documents: u64,
    pub content_bytes: u64,
    pub avg_doc_bytes: u64,
    pub max_doc_bytes: u64,
    pub fts_data_bytes: u64,
    pub fts_idx_term_bytes: u64,
    pub fts_content_bytes: u64,
    pub fts_docsize_bytes: u64,
    pub estimated_fts_bytes: u64,
    pub db_file_bytes: u64,
}

fn opt_sum(conn: &rusqlite::Connection, sql: &str) -> u64 {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0).max(0) as u64
}

pub fn index_facts(legacy_db: &Path) -> anyhow::Result<IndexFacts> {
    let conn = open_legacy_ro(legacy_db)?;
    let documents = opt_sum(&conn, "SELECT COUNT(*) FROM files_fts");
    let content_bytes = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(c1)),0) FROM files_fts_content");
    let max_doc_bytes = opt_sum(&conn, "SELECT COALESCE(MAX(LENGTH(c1)),0) FROM files_fts_content");
    let fts_data_bytes = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(block)),0) FROM files_fts_data");
    let fts_idx_term_bytes = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(term)),0) FROM files_fts_idx");
    let fts_content_bytes = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(c0)+LENGTH(c1)),0) FROM files_fts_content");
    let fts_docsize_bytes = opt_sum(&conn, "SELECT COALESCE(SUM(LENGTH(sz)),0) FROM files_fts_docsize");
    Ok(IndexFacts {
        documents,
        content_bytes,
        avg_doc_bytes: if documents > 0 { content_bytes / documents } else { 0 },
        max_doc_bytes,
        fts_data_bytes,
        fts_idx_term_bytes,
        fts_content_bytes,
        fts_docsize_bytes,
        estimated_fts_bytes: fts_data_bytes + fts_idx_term_bytes + fts_content_bytes + fts_docsize_bytes,
        db_file_bytes: std::fs::metadata(legacy_db).map(|m| m.len()).unwrap_or(0),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryTiming {
    pub id: String,
    pub category: String,
    pub query: String,
    pub expected_hits: u64,
    pub rows_returned: usize,
    pub first_snippet: String,
    pub samples_ms: Vec<f64>,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryRow {
    pub category: String,
    pub summary: Summary,
}

#[derive(Debug, Clone, Serialize)]
pub struct WarmReport {
    pub iterations: usize,
    pub with_snippet: Summary,
    pub rank_only: Summary,
    pub by_category: Vec<CategoryRow>,
    pub queries: Vec<QueryTiming>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColdQuery {
    pub id: String,
    pub query: String,
    pub ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColdReport {
    pub restarts: usize,
    pub open_ms: Vec<f64>,
    pub first_query_ms: Vec<f64>,
    pub per_query: Vec<ColdQuery>,
    pub wall_ms: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentBenchReport {
    pub schema: String,
    pub kind: String,
    pub host: crate::env::HostInfo,
    pub generated_at_utc: String,
    pub command: String,
    pub dataset_scope: String,
    pub legacy_db: DbFacts,
    pub index: IndexFacts,
    pub open_ms: f64,
    pub warm: WarmReport,
    pub cold: Option<ColdReport>,
    pub memory_before: Option<ProcessMemory>,
    pub memory_after_open: Option<ProcessMemory>,
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

pub fn run(args: &Args) -> anyhow::Result<()> {
    let legacy_db = crate::realfiles::default_legacy_db();
    let legacy_db = args.get("--legacy-db").map(std::path::PathBuf::from).unwrap_or(legacy_db);
    let queries_path = std::path::PathBuf::from(args.get("--queries").unwrap_or(".bench/queries-content.jsonl"));
    let out = std::path::PathBuf::from(args.required("--out")?);
    let iters = args.usize_or("--iters", 30)?.max(1);
    let limit = args.usize_or("--limit", 20)?;
    let cold_restarts = args.usize_or("--cold-restarts", 3)?;

    let queries = load_queries(&queries_path)?;
    anyhow::ensure!(!queries.is_empty(), "query suite is empty: {}", queries_path.display());

    let memory_before = self_memory();
    let db = db_facts(&legacy_db)?;
    let index = index_facts(&legacy_db)?;
    let open_start = Instant::now();
    let conn = open_legacy_ro(&legacy_db)?;
    let open_ms = open_start.elapsed().as_secs_f64() * 1000.0;
    let memory_after_open = self_memory();

    let sql_snippet = "SELECT path, snippet(files_fts, 1, '[', ']', '...', 24) FROM files_fts WHERE files_fts MATCH ?1 ORDER BY rank LIMIT ?2";
    let sql_rank = "SELECT path FROM files_fts WHERE files_fts MATCH ?1 ORDER BY rank LIMIT ?2";

    let mut snippet_samples: Vec<Vec<f64>> = Vec::new();
    let mut rank_samples: Vec<Vec<f64>> = Vec::new();
    let mut timings: Vec<QueryTiming> = Vec::new();
    let mut by_cat: HashMap<String, Vec<Vec<f64>>> = HashMap::new();

    for q in &queries {
        let mut stmt = conn.prepare(sql_snippet)?;
        let mut rank_stmt = conn.prepare(sql_rank)?;
        let mut rows_returned = 0usize;
        let mut first_snippet = String::new();
        // Warmup
        for _ in 0..2 {
            let mut rows = stmt.query(rusqlite::params![q.query.as_str(), limit as i64])?;
            while rows.next()?.is_some() {}
            let mut rows = rank_stmt.query(rusqlite::params![q.query.as_str(), limit as i64])?;
            while rows.next()?.is_some() {}
        }
        let mut samples = Vec::with_capacity(iters);
        let mut rank_only = Vec::with_capacity(iters);
        for _ in 0..iters {
            let start = Instant::now();
            let mut rows = stmt.query(rusqlite::params![q.query.as_str(), limit as i64])?;
            let mut count = 0usize;
            while let Some(row) = rows.next()? {
                if count == 0 {
                    let path: String = row.get(0).unwrap_or_default();
                    let snip: String = row.get(1).unwrap_or_default();
                    first_snippet = format!(
                        "{} || {}",
                        path.chars().take(80).collect::<String>(),
                        snip.chars().take(120).collect::<String>()
                    );
                }
                count += 1;
            }
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            rows_returned = count;

            let start = Instant::now();
            let mut rows = rank_stmt.query(rusqlite::params![q.query.as_str(), limit as i64])?;
            while rows.next()?.is_some() {}
            rank_only.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let summary = summarize(&samples);
        by_cat.entry(q.category.clone()).or_default().push(samples.clone());
        snippet_samples.push(samples.clone());
        rank_samples.push(rank_only.clone());
        timings.push(QueryTiming {
            id: q.id.clone(),
            category: q.category.clone(),
            query: q.query.clone(),
            expected_hits: q.hits,
            rows_returned,
            first_snippet,
            samples_ms: samples,
            summary,
        });
    }
    drop(conn);
    let memory_after_warm = self_memory();

    let warm = WarmReport {
        iterations: iters,
        with_snippet: crate::stats::summarize_slices(&snippet_samples),
        rank_only: crate::stats::summarize_slices(&rank_samples),
        by_category: {
            let mut rows: Vec<CategoryRow> = by_cat
                .into_iter()
                .map(|(category, slices)| CategoryRow { category, summary: crate::stats::summarize_slices(&slices) })
                .collect();
            rows.sort_by(|a, b| a.category.cmp(&b.category));
            rows
        },
        queries: timings,
    };

    let mut cold = None;
    if cold_restarts > 0 {
        let mut open_ms_vec = Vec::new();
        let mut first_vec = Vec::new();
        let mut wall_vec = Vec::new();
        let mut per_query: HashMap<String, Vec<f64>> = HashMap::new();
        let mut order: Vec<(String, String)> = Vec::new();
        for _ in 0..cold_restarts {
            let child_args: Vec<String> = vec![
                "content-cold-child".to_string(),
                "--legacy-db".to_string(),
                legacy_db.display().to_string(),
                "--queries".to_string(),
                queries_path.display().to_string(),
                "--limit".to_string(),
                limit.to_string(),
            ];
            let outcome = childproc::run_child_json(&child_args)?;
            wall_vec.push(outcome.wall_ms);
            open_ms_vec.push(outcome.value.get("open_ms").and_then(|v| v.as_f64()).unwrap_or(0.0));
            let qs = outcome.value.get("queries").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            let mut first = None;
            for q in &qs {
                let id = q.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let query = q.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let ms = q.get("ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
                if first.is_none() {
                    first = Some(ms);
                }
                if !order.iter().any(|(oid, _)| oid == &id) {
                    order.push((id.clone(), query.clone()));
                }
                per_query.entry(id).or_default().push(ms);
            }
            first_vec.push(first.unwrap_or(0.0));
        }
        let per_query_rows = order
            .iter()
            .map(|(id, query)| ColdQuery {
                id: id.clone(),
                query: query.clone(),
                ms: per_query.get(id).map(|v| summarize(v).p50).unwrap_or(0.0),
            })
            .collect();
        cold = Some(ColdReport {
            restarts: cold_restarts,
            open_ms: open_ms_vec,
            first_query_ms: first_vec,
            per_query: per_query_rows,
            wall_ms: wall_vec,
        });
    }

    let command = format!("ps-bench content --legacy-db {} --queries {} --iters {} --limit {} --cold-restarts {} --out {}", legacy_db.display(), queries_path.display(), iters, limit, cold_restarts, out.display());

    let report = ContentBenchReport {
        schema: RESULT_SCHEMA.to_string(),
        kind: "content-fts5-benchmark".to_string(),
        host: host_info().clone(),
        generated_at_utc: now_utc(),
        command: command.clone(),
        dataset_scope: "LEGACY v1 FTS5 index on the real historical corpus (files_fts, path UNINDEXED + content); baseline for migration, NOT a v2 ps-content number".to_string(),
        legacy_db: db,
        index,
        open_ms,
        warm,
        cold,
        memory_before,
        memory_after_open,
        memory_after_warm,
        limitations: vec![
            "legacy v1 schema measured read-only immutable; v2 ps-content schema/query path may differ".to_string(),
            "immutable URI does not replay a pending -wal; document count compared against 14,609 baseline".to_string(),
            "single-process warm measurements; OS page cache state is whatever the host had at measurement time (recorded, not controlled)".to_string(),
            "snippet() cost is reported separately from rank-only to avoid hiding highlight cost".to_string(),
        ],
    };
    write_json(&out, &report)?;

    println!("ps-bench content (LEGACY v1 FTS5 baseline, read-only)");
    println!("  db: {} ({} bytes, {} docs, content {} bytes, est. FTS shadow bytes {})", legacy_db.display(), report.legacy_db.db_bytes, report.index.documents, report.index.content_bytes, report.index.estimated_fts_bytes);
    println!("  open_ms: {:.2}", report.open_ms);
    println!("  warm ({} iters/query, {} queries):", iters, report.warm.queries.len());
    print_summary("  with snippet", &report.warm.with_snippet);
    print_summary("  rank only", &report.warm.rank_only);
    for row in &report.warm.by_category {
        println!("  [{:18}] p50={:7.3} p95={:7.3} p99={:7.3} n={}", row.category, row.summary.p50, row.summary.p95, row.summary.p99, row.summary.count);
    }
    if let Some(cold) = &report.cold {
        println!("  cold ({} process restarts): open p50={:.2} ms, first-query p50={:.2} ms, wall p50={:.0} ms",
            cold.restarts,
            summarize(&cold.open_ms).p50,
            summarize(&cold.first_query_ms).p50,
            summarize(&cold.wall_ms).p50);
    }
    if let (Some(a), Some(b)) = (&report.memory_before, &report.memory_after_warm) {
        println!("  process RSS: before={} MB after_warm={} MB peak={} MB", a.working_set_bytes / 1_000_000, b.working_set_bytes / 1_000_000, b.peak_working_set_bytes / 1_000_000);
    }
    for note in &report.limitations {
        println!("  limit: {note}");
    }
    println!("  results: {}", out.display());
    Ok(())
}

fn print_summary(label: &str, s: &Summary) {
    println!(
        "{}: p50={:.3} p95={:.3} p99={:.3} min={:.3} max={:.3} mean={:.3} n={}",
        label, s.p50, s.p95, s.p99, s.min, s.max, s.mean, s.count
    );
}

/// Child process: open fresh, run every query once, emit JSON.
pub fn cold_child(args: &Args) -> anyhow::Result<()> {
    let legacy_db = crate::realfiles::default_legacy_db();
    let legacy_db = args.get("--legacy-db").map(std::path::PathBuf::from).unwrap_or(legacy_db);
    let queries_path = std::path::PathBuf::from(args.required("--queries")?);
    let limit = args.usize_or("--limit", 20)?;
    let queries = load_queries(&queries_path)?;
    let start = Instant::now();
    let conn = open_legacy_ro(&legacy_db)?;
    let open_ms = start.elapsed().as_secs_f64() * 1000.0;
    let sql = "SELECT path, snippet(files_fts, 1, '[', ']', '...', 24) FROM files_fts WHERE files_fts MATCH ?1 ORDER BY rank LIMIT ?2";
    let mut stmt = conn.prepare(sql)?;
    let mut out = Vec::new();
    for q in &queries {
        let t = Instant::now();
        let mut rows = stmt.query(rusqlite::params![q.query.as_str(), limit as i64])?;
        while rows.next()?.is_some() {}
        out.push(json!({"id": q.id, "query": q.query, "ms": t.elapsed().as_secs_f64() * 1000.0}));
    }
    let mem = self_memory();
    let payload = json!({
        "open_ms": open_ms,
        "queries": out,
        "memory": mem,
        "rss_bytes": mem.as_ref().map(|m| m.working_set_bytes).unwrap_or(0),
    });
    println!("PSBENCH_JSON:{}", serde_json::to_string(&payload)?);
    Ok(())
}
