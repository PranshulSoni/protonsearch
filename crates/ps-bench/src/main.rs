//! ps-bench: reproducible benchmark + relevance harness (dev-only, not shipped).
//! OWNER: perf-bench. Every number printed here must be traceable to a command.

use std::path::PathBuf;

use ps_bench::args::Args;
use ps_bench::contentbench;
use ps_bench::dataset::{self, DEFAULT_SEED};
use ps_bench::filenamebench;
use ps_bench::ocrbench;
use ps_bench::queries;
use ps_bench::realfiles;
use ps_bench::relevance;

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() {
        usage();
        std::process::exit(2);
    }
    let cmd = raw[0].clone();
    let args = Args::parse(&raw[1..]);
    if let Err(err) = run(&cmd, &args) {
        eprintln!("ps-bench: error: {err:#}");
        std::process::exit(1);
    }
}

fn run(cmd: &str, args: &Args) -> anyhow::Result<()> {
    match cmd {
        "gen" => cmd_gen(args),
        "queries" => cmd_queries(args),
        "real-export" => cmd_real_export(args),
        "content-queries" => cmd_content_queries(args),
        "content" => contentbench::run(args),
        "content-cold-child" => contentbench::cold_child(args),
        "filename" => filenamebench::run(args),
        "filename-cold-child" => filenamebench::cold_child(args),
        "relevance" => relevance::run(args),
        "golden-synth" => cmd_golden_synth(args),
        "golden-real" => cmd_golden_real(args),
        "env" => {
            println!("{}", serde_json::to_string_pretty(ps_bench::env::host_info())?);
            Ok(())
        }
        "ocr-corpus" => ocrbench::run_generate(args),
        "ocr" => ocrbench::run(args),
        other => {
            usage();
            anyhow::bail!("unknown subcommand {other}")
        }
    }
}

fn cmd_gen(args: &Args) -> anyhow::Result<()> {
    let count = args.usize_or("--count", 100_000)?;
    let seed = args.u64_or("--seed", DEFAULT_SEED)?;
    let out = PathBuf::from(args.required("--out")?);
    if count < dataset::ANCHORS.len() {
        anyhow::bail!("--count must be at least {} to fit planted anchors", dataset::ANCHORS.len());
    }
    let stats = dataset::generate_to_file(seed, count, &out)?;
    println!("ps-bench gen: seed={seed} requested={count}");
    println!(
        "wrote {} entries ({} files, {} folders) in {:.0} ms -> {}",
        stats.entries, stats.files, stats.folders, stats.generated_ms, out.display()
    );
    println!(
        "corpus: bytes={} fnv1a64={} sha256={}",
        stats.bytes,
        stats.fnv1a64,
        stats.sha256.as_deref().unwrap_or("unavailable")
    );
    println!(
        "characteristics: hidden={} cjk={} non_ascii={} long_path_gt200={} anchors={}",
        stats.hidden, stats.cjk, stats.non_ascii, stats.long_paths, stats.anchors
    );
    println!("top extensions: {:?}", &stats.top_exts[..stats.top_exts.len().min(10)]);
    println!("meta: {}", out.with_extension("meta.json").display());
    Ok(())
}

fn cmd_queries(args: &Args) -> anyhow::Result<()> {
    let out = PathBuf::from(args.get("--out").unwrap_or(".bench/queries.jsonl"));
    let n = queries::write_suite(&out)?;
    println!("ps-bench queries: wrote {n} fixed queries -> {}", out.display());
    for q in queries::filename_suite() {
        println!("  [{:16}] {:?}", q.category, q.query);
    }
    Ok(())
}

fn cmd_real_export(args: &Args) -> anyhow::Result<()> {
    let legacy_db = match args.get("--legacy-db") {
        Some(p) => PathBuf::from(p),
        None => realfiles::default_legacy_db(),
    };
    let out = PathBuf::from(args.get("--out").unwrap_or(".bench/real-paths.jsonl"));
    let meta_out = PathBuf::from(args.get("--meta-out").unwrap_or(".bench/real-corpus.meta.json"));
    let limit = args.usize_or("--limit", 0)?;
    if !legacy_db.exists() {
        anyhow::bail!(
            "legacy database not found: {} (pass --legacy-db; read-only immutable access only)",
            legacy_db.display()
        );
    }
    let report = realfiles::export_legacy(&legacy_db, &out, &meta_out, limit)?;
    println!("ps-bench real-export: {}", legacy_db.display());
    println!(
        "legacy db: {} bytes (wal {}), files={} dirs={} content_docs={} browser={} memory_events={}",
        report.legacy_db.db_bytes,
        report.legacy_db.wal_bytes,
        report.legacy_db.files,
        report.legacy_db.files_dirs,
        report.legacy_db.content_docs,
        report.legacy_db.browser_items,
        report.legacy_db.memory_events
    );
    println!(
        "exported {} entries ({} files, {} folders) to {} in {:.0} ms",
        report.corpus.entries,
        report.corpus.files,
        report.corpus.folders,
        out.display(),
        report.export_ms
    );
    println!(
        "corpus: bytes={} fnv1a64={} sha256={} max_path={} cjk_paths={} non_ascii={} hidden={}",
        report.bytes,
        report.fnv1a64,
        report.sha256.as_deref().unwrap_or("unavailable"),
        report.corpus.max_path_chars,
        report.corpus.cjk_paths,
        report.corpus.non_ascii_paths,
        report.corpus.hidden_names
    );
    for note in &report.notes {
        println!("note: {note}");
    }
    println!("meta: {}", meta_out.display());
    Ok(())
}

fn cmd_content_queries(args: &Args) -> anyhow::Result<()> {
    let legacy_db = match args.get("--legacy-db") {
        Some(p) => PathBuf::from(p),
        None => realfiles::default_legacy_db(),
    };
    let out = PathBuf::from(args.get("--out").unwrap_or(".bench/queries-content.jsonl"));
    let sample_docs = args.usize_or("--sample-docs", 4000)?;
    let seed = args.u64_or("--seed", DEFAULT_SEED)?;
    let suite = contentbench::generate_queries(&legacy_db, &out, sample_docs, seed)?;
    println!(
        "ps-bench content-queries: {} queries from {} sampled docs -> {}",
        suite.len(),
        sample_docs,
        out.display()
    );
    for q in &suite {
        println!("  [{:18}] hits={:<6} {:?}", q.category, q.hits, q.query);
    }
    Ok(())
}

fn cmd_golden_synth(args: &Args) -> anyhow::Result<()> {
    let out = PathBuf::from(args.get("--out").unwrap_or("tests/golden/synthetic-golden.json"));
    let seed = args.u64_or("--seed", DEFAULT_SEED)?;
    let entries = args.usize_or("--entries", 10_000)?;
    let golden = relevance::write_synthetic_fixture(&out, seed, entries)?;
    println!(
        "ps-bench golden-synth: {} queries for synthetic corpus (seed={} entries={}) -> {}",
        golden.queries.len(),
        seed,
        entries,
        out.display()
    );
    Ok(())
}

fn cmd_golden_real(args: &Args) -> anyhow::Result<()> {
    let paths = PathBuf::from(args.get("--paths").unwrap_or(".bench/real-paths.jsonl"));
    let out = PathBuf::from(args.get("--out").unwrap_or("tests/golden/real-corpus-golden.json"));
    let max_queries = args.usize_or("--max-queries", 80)?;
    let seed = args.u64_or("--seed", DEFAULT_SEED)?;
    anyhow::ensure!(paths.exists(), "real corpus export not found: {} (run ps-bench real-export first)", paths.display());
    let golden = relevance::build_real_golden(&paths, &out, max_queries, seed)?;
    println!(
        "ps-bench golden-real: {} queries from {} real entries -> {}",
        golden.queries.len(),
        golden.entries,
        out.display()
    );
    let mut by_cat: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for q in &golden.queries {
        *by_cat.entry(q.category.as_str()).or_insert(0) += 1;
    }
    println!("categories: {:?}", by_cat);
    Ok(())
}

fn usage() {
    eprintln!(
        "ps-bench: reproducible benchmark harness (dev-only)
usage:
  ps-bench gen --count <n> [--seed <n>] --out <path.jsonl>       deterministic realistic dataset
  ps-bench queries [--out <path.jsonl>]                          fixed filename query suite
  ps-bench real-export [--legacy-db <db>] [--out <jsonl>] [--meta-out <json>]
                                                                 read-only export of the legacy corpus
  ps-bench content-queries [--legacy-db <db>] [--sample-docs <n>] [--out <jsonl>]
                                                                 real FTS5 query suite
  ps-bench content [--legacy-db <db>] --queries <jsonl> --out <json> [--iters 30] [--cold-restarts 3]
                                                                 legacy FTS5 latency baseline
  ps-bench filename --dataset <jsonl> --queries <jsonl> --out <json> [--iters 30] [--cold-restarts 0] [--save-index <bin>]
                                                                 filename index build/load/search
  ps-bench relevance --dataset <jsonl> --golden <json> --out <json>
                                                                 golden relevance evaluation
  ps-bench golden-synth [--out tests/golden/synthetic-golden.json]
  ps-bench golden-real [--paths .bench/real-paths.jsonl] [--out tests/golden/real-corpus-golden.json]
  ps-bench ocr-corpus [--out-dir .bench/ocr-corpus] [--count 0]  ground-truth OCR corpus
  ps-bench ocr [--manifest .bench/ocr-corpus/manifest.jsonl] [--passes 2] --out <json>
                                                                 OCR speed + accuracy (Windows.Media.Ocr)
  ps-bench env                                                   host facts (JSON)
See docs/performance/METHODOLOGY.md. No network calls; no telemetry."
    );
}
