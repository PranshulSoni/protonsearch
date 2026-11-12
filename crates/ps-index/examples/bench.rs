//! Honest, in-repo filename search benchmark. OWNER: engine-core.
//!
//! Generates deterministic synthetic corpora (10k / 100k / 1M entries), builds
//! the compact index, and measures warm search latency per query class with
//! p50/p95/p99. Numbers are printed, not asserted. Run in release mode:
//!
//!   cargo run --release -p ps-index --example bench
//!   cargo run --release -p ps-index --example bench -- --counts 10000,100000 --iters 30
//!
//! The formal harness (datasets, golden relevance, cold start) lives in
//! ps-bench; this exists so engine changes can be measured immediately.

use std::time::Instant;

use ps_index::{EntryKind, FilenameIndex, IndexEntry};

const WORDS: &[&str] = &[
    "report",
    "budget",
    "invoice",
    "design",
    "notes",
    "meeting",
    "project",
    "summary",
    "analysis",
    "proposal",
    "draft",
    "final",
    "photo",
    "screenshot",
    "backup",
    "archive",
    "config",
    "readme",
    "test",
    "data",
];
const EXTENSIONS: &[&str] = &["pdf", "docx", "txt", "md", "png", "rs", "json", "csv"];

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn pick(&mut self, n: usize) -> usize {
        (self.next() >> 33) as usize % n.max(1)
    }
}

fn generate(count: usize) -> Vec<IndexEntry> {
    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    let mut out = Vec::with_capacity(count);
    let dirs = 2_000usize.max(count / 64);
    for i in 0..count {
        // Realistic-ish depth: 2-4 directories, shared prefixes so the dir table
        // de-duplicates like a real profile tree.
        let depth = 2 + (i % 3);
        let mut path = String::with_capacity(96);
        path.push_str("C:\\Users\\Pranshul");
        for d in 0..depth {
            path.push('\\');
            path.push_str(WORDS[rng.pick(WORDS.len())]);
            path.push('_');
            path.push_str(&format!("{:03}", rng.pick(dirs)));
            let _ = d;
        }
        let name = if i % 97 == 0 {
            // CJK names for the Unicode query class.
            format!("报告-会议纪要_{:06}.pdf", i)
        } else {
            format!(
                "{}_{:06}.{}",
                WORDS[rng.pick(WORDS.len())],
                i,
                EXTENSIONS[rng.pick(EXTENSIONS.len())]
            )
        };
        path.push('\\');
        path.push_str(&name);
        let size = (rng.next() >> 20) % 50_000_000;
        let modified = 1_600_000_000_000 + (i as i64) * 60_000;
        out.push(IndexEntry {
            path,
            name,
            ext: String::new(),
            kind: EntryKind::File,
            size,
            modified,
        });
    }
    out
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

struct QueryCase {
    label: &'static str,
    text: String,
}

fn query_suite(count: usize) -> Vec<QueryCase> {
    let mid = count / 2;
    let word = WORDS[mid % WORDS.len()];
    let ext = EXTENSIONS[mid % EXTENSIONS.len()];
    let exact = format!("{}_{:06}.{}", word, mid, ext);
    let prefix = format!("{}_{:04}", WORDS[0], (mid / 10) % 1000);
    let typo = {
        // Swap two adjacent chars of a known stem to exercise typo tolerance.
        let stem = format!("{}_{:06}", word, mid);
        let mut chars: Vec<char> = stem.chars().collect();
        if chars.len() > 3 {
            chars.swap(1, 2);
        }
        chars.into_iter().collect::<String>()
    };
    vec![
        QueryCase {
            label: "exact",
            text: exact,
        },
        QueryCase {
            label: "prefix",
            text: prefix,
        },
        QueryCase {
            label: "substring",
            text: "port_00".to_string(),
        },
        QueryCase {
            label: "fuzzy",
            text: "rprt_00".to_string(),
        },
        QueryCase {
            label: "typo",
            text: typo,
        },
        QueryCase {
            label: "cjk",
            text: "会议纪要".to_string(),
        },
        QueryCase {
            label: "path-segment",
            text: "budget_0".to_string(),
        },
        QueryCase {
            label: "multi-term",
            text: "final report".to_string(),
        },
        QueryCase {
            label: "zero-match",
            text: "zzzqqqxxx".to_string(),
        },
    ]
}

fn run_case(index: &FilenameIndex, case: &QueryCase, iters: usize) -> (usize, f64, f64, f64) {
    // Warmup.
    let mut hits = 0usize;
    for _ in 0..3 {
        hits = index.search(&case.text, 30).len();
    }
    let mut samples: Vec<f64> = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        let found = index.search(&case.text, 30);
        let elapsed = start.elapsed().as_secs_f64() * 1e6;
        std::hint::black_box(found);
        samples.push(elapsed);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    (
        hits,
        percentile(&samples, 0.50),
        percentile(&samples, 0.95),
        percentile(&samples, 0.99),
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut counts: Vec<usize> = vec![10_000, 100_000];
    let mut iters = 30usize;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--counts" => {
                if let Some(v) = args.get(i + 1) {
                    counts = v
                        .split(',')
                        .filter_map(|s| s.trim().parse::<usize>().ok())
                        .collect();
                }
                i += 2;
            }
            "--iters" => {
                if let Some(v) = args.get(i + 1) {
                    iters = v.parse().unwrap_or(30);
                }
                i += 2;
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }
    println!("ps-index filename benchmark (release build, synthetic corpus)");
    println!(
        "{:>9} | {:>10} | {:>12} | {:>12} | {:>13}",
        "entries", "build ms", "build/s", "index MB", "bytes/entry"
    );
    for &count in &counts {
        let entries = generate(count);
        let start = Instant::now();
        let index = FilenameIndex::from_entries(entries);
        let build_ms = start.elapsed().as_secs_f64() * 1000.0;
        let memory = index.memory_bytes();
        println!(
            "{:>9} | {:>10.1} | {:>12.0} | {:>12.1} | {:>13.1}",
            index.len(),
            build_ms,
            count as f64 / (build_ms / 1000.0).max(1e-9),
            memory as f64 / (1024.0 * 1024.0),
            memory as f64 / count as f64,
        );
        println!(
            "{:>14} {:>8} {:>10} {:>10} {:>10}",
            "query", "hits", "p50 us", "p95 us", "p99 us"
        );
        for case in query_suite(count) {
            let (hits, p50, p95, p99) = run_case(&index, &case, iters);
            println!(
                "{:>14} {:>8} {:>10.1} {:>10.1} {:>10.1}   [{}]",
                case.label, hits, p50, p95, p99, case.text
            );
        }
        println!();
    }
}
