//! Temporary phase-timing probe (engine-core).
use std::time::Instant;
use ps_index::{EntryKind, FilenameIndex, IndexEntry};

const WORDS: &[&str] = &["report","budget","invoice","design","notes","meeting","project","summary","analysis","proposal","draft","final","photo","screenshot","backup","archive","config","readme","test","data"];

fn gen(n: usize) -> Vec<IndexEntry> {
    let mut out = Vec::with_capacity(n);
    let mut state = 0x1234_5678_9abc_def0u64;
    let mut next = || { state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (state >> 33) as usize };
    for i in 0..n {
        let depth = 2 + (i % 3);
        let mut path = String::from("C:\\Users\\Pranshul");
        for _ in 0..depth {
            path.push('\\');
            path.push_str(WORDS[next() % WORDS.len()]);
            path.push('_');
            path.push_str(&format!("{:03}", next() % 2000));
        }
        let name = format!("{}_{:06}.pdf", WORDS[next() % WORDS.len()], i);
        path.push('\\');
        path.push_str(&name);
        out.push(IndexEntry { path, name, ext: String::new(), kind: EntryKind::File, size: 1, modified: 1 });
    }
    out
}

fn bench(label: &str, iters: usize, mut f: impl FnMut() -> usize) {
    for _ in 0..5 { std::hint::black_box(f()); }
    let start = Instant::now();
    let mut hits = 0;
    for _ in 0..iters { hits += f(); }
    let us = start.elapsed().as_secs_f64() * 1e6 / iters as f64;
    println!("{label:>28}: {us:9.1} us  (hits {hits})");
}

fn main() {
    let count: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let iters: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let build = Instant::now();
    let index = FilenameIndex::from_entries(gen(count));
    println!("entries={} build={:.1}ms memory={}KB", index.len(), build.elapsed().as_secs_f64()*1000.0, index.memory_bytes()/1024);
    for q in ["zzzqqqxxx", "report_005000.pdf", "port_00", "report_0500"] {
        println!("--- query {q}");
        bench("search", iters, || index.search(q, 30).len());
        bench("search_scan", iters, || index.search_scan(q, 30).len());
    }
}
