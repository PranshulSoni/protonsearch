//! Content retrieval performance probe (task-42 evidence). Not shipped.
//!
//! Measures the current ContentIndex::search path and candidate SQL/Rust
//! variants against a real v2 content database, printing p50/p95/p99.
//!
//! Usage: cargo run --release -p ps-content --example content_perf_probe -- <db> <queries.jsonl> [iters] [limit]

use std::path::Path;
use std::time::Instant;

use ps_content::ContentIndex;

#[derive(serde::Deserialize)]
struct QueryDef {
    query: String,
    #[serde(default)]
    category: String,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn summarize(samples: &mut Vec<f64>) -> (f64, f64, f64) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (
        percentile(samples, 0.50),
        percentile(samples, 0.95),
        percentile(samples, 0.99),
    )
}

fn bench(label: &str, queries: &[QueryDef], iters: usize, mut run: impl FnMut(&str) -> usize) {
    let mut all = Vec::new();
    let mut per_category: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
    for q in queries {
        let mut samples = Vec::with_capacity(iters);
        let mut hits = 0usize;
        for _ in 0..iters {
            let start = Instant::now();
            hits += run(&q.query);
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        per_category
            .entry(q.category.clone())
            .or_default()
            .extend_from_slice(&samples);
        all.extend(samples);
        let _ = hits;
    }
    let (p50, p95, p99) = summarize(&mut all);
    let n = all.len();
    println!("{label:<28} n={n:<5} p50={p50:7.2} p95={p95:7.2} p99={p99:7.2} ms");
    for (category, mut samples) in per_category {
        let (p50, p95, p99) = summarize(&mut samples);
        println!("  {category:<26} p50={p50:7.2} p95={p95:7.2} p99={p99:7.2} ms");
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = Path::new(&args[0]);
    let queries_path = Path::new(&args[1]);
    let iters: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(20);
    let limit: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(12);

    let queries: Vec<QueryDef> = std::fs::read_to_string(queries_path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    println!(
        "db={} queries={} iters={} limit={}",
        db.display(),
        queries.len(),
        iters,
        limit
    );

    let index = ContentIndex::open(db)?;
    bench("current ContentIndex::search", &queries, iters, |query| {
        index.search(query, limit).map(|hits| hits.len()).unwrap_or(0)
    });

    // Interleaved A/B: the pre-task-42 implementation (FTS5 snippet() + 128 KB
    // text fetch + lowercase line scan) versus the current ContentIndex::search.
    // Alternating per iteration cancels machine-load drift.
    let conn = rusqlite::Connection::open_with_flags(db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    conn.busy_timeout(std::time::Duration::from_millis(2000))?;
    let mut old_samples = Vec::new();
    let mut new_samples = Vec::new();
    for q in &queries {
        for _ in 0..2 {
            let _ = index.search(&q.query, limit);
            let _ = old_search(&conn, &q.query, limit);
        }
        for _ in 0..iters {
            let start = Instant::now();
            let _ = old_search(&conn, &q.query, limit);
            old_samples.push(start.elapsed().as_secs_f64() * 1000.0);
            let start = Instant::now();
            let _ = index.search(&q.query, limit).map(|hits| hits.len()).unwrap_or(0);
            new_samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let (op50, op95, op99) = summarize(&mut old_samples);
    let (np50, np95, np99) = summarize(&mut new_samples);
    println!(
        "A/B old (snippet()+128KB text)  n={} p50={op50:7.2} p95={op95:7.2} p99={op99:7.2} ms",
        iters * queries.len()
    );
    println!(
        "A/B new (Rust snippet, 32KB)    n={} p50={np50:7.2} p95={np95:7.2} p99={np99:7.2} ms",
        iters * queries.len()
    );
    Ok(())
}

/// Faithful copy of the pre-task-42 search hot path for A/B comparison.
fn old_search(conn: &rusqlite::Connection, query: &str, limit: usize) -> anyhow::Result<usize> {
    let terms = ps_content::query_terms(query);
    if terms.is_empty() {
        return Ok(0);
    }
    let match_expression = terms
        .iter()
        .map(|term| format!("\"{term}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let mut statement = conn.prepare(
        "SELECT d.path, d.title, bm25(documents_fts), \
                snippet(documents_fts, 2, '', '', '…', 24), \
                CASE WHEN LENGTH(d.text) <= 131072 THEN d.text ELSE SUBSTR(d.text, 1, 131072) END \
         FROM documents_fts JOIN documents d ON d.id = documents_fts.rowid \
         WHERE documents_fts MATCH ?1 ORDER BY bm25(documents_fts) LIMIT ?2",
    )?;
    let rows = statement.query_map(rusqlite::params![match_expression, limit as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    let mut count = 0usize;
    for row in rows.flatten() {
        let (_path, _title, _rank, _snippet, text) = row;
        let _line = find_line(&text, &terms);
        count += 1;
    }
    Ok(count)
}

fn find_line(text: &str, terms: &[String]) -> Option<u32> {
    if terms.is_empty() {
        return None;
    }
    for (index, line) in text.lines().enumerate() {
        let lower = line.to_lowercase();
        if terms.iter().any(|term| lower.contains(term.as_str())) {
            return Some((index + 1) as u32);
        }
    }
    None
}

fn make_snippet(text: &str, terms: &[String]) -> String {
    let lower = text.to_lowercase();
    let mut best: Option<usize> = None;
    for term in terms {
        if let Some(position) = lower.find(term.as_str()) {
            best = Some(best.map_or(position, |current| current.min(position)));
        }
    }
    let Some(position) = best else {
        return String::new();
    };
    let start = position.saturating_sub(80);
    let end = (position + 160).min(text.len());
    let mut start = start;
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = end;
    while end < text.len() && !text.is_char_boundary(end) {
        end += 1;
    }
    text[start..end].replace('\n', " ").trim().to_string()
}
