# ProtonSearch v2 — Performance & Relevance Benchmark Methodology

- **Owner:** L1-C Performance & Search Quality (perf-bench)
- **Status:** living document; measured numbers live in [BENCHMARKS.md](./BENCHMARKS.md), raw JSON in [results/](./results/). Filename numbers in that report were measured on committed engine hash a28042d1282e4c9630a952bd5b02894f66850037 (ps-index unchanged through HEAD 8692997; superseding the earlier 3906836 table; see BENCHMARKS.md section 3.4 for the 1M long-path regression fix). v2 content numbers were re-measured at HEAD 8692997.
- **Rule of record:** no number appears in project documentation unless a command in this repository printed it on this machine. Targets are not claims.

## 1. Purpose and rules

1. Publish methodology before numbers so a target can be checked, not admired.
2. Separate **engine time** from **process/UI time**; separate **warm** from **cold**; separate **index building** from **query serving**; separate **OCR inference** from **OCR-text retrieval**.
3. Every result JSON records host facts, OS build, CPU, RAM, command line, corpus path and hash, iteration counts, process/cache state, and known limitations.
4. When an engine API is a stub, the harness writes `status: engine_not_ready` or `status: unverified` and prints a warning. It never substitutes invented numbers.
5. The harness is dev-only (crate `ps-bench`, not shipped) and makes no network calls and no telemetry.

## 2. What is measured (taxonomy)

| Layer | Measured by | Notes |
|---|---|---|
| Dataset parse (JSONL -> entries) | `ps-bench filename` | corpus I/O + allocation; not engine |
| Index build (`FilenameIndex::from_entries`) | `ps-bench filename` | engine arena construction |
| Index persist / load | `ps-bench filename --save-index` + cold child | binary save + fresh-process load |
| Filename warm search | `ps-bench filename` | in-process, 2 warmup + N timed iterations per query |
| Filename cold | child process restart | spawn + load + first queries; startup reported separately |
| FTS5 content warm retrieval (legacy v1) | `ps-bench content` | prepared statement, snippet path and rank-only path measured separately |
| v2 content warm retrieval | `ps-bench content-v2` | `ps_content::ContentIndex::search` (snippet + snippet_line included by the API), after a read-only legacy import |
| FTS5 cold retrieval | child process restart | DB open + first query |
| OCR speed | `ps-bench ocr` | session open, per-image ms, images/sec, cold pass 1 vs warm pass 2 |
| OCR accuracy | `ps-bench ocr` | character error rate + normalized exact match against known text |
| Relevance quality | `ps-bench relevance` | MRR@10, recall@10, exact-match@1 from golden fixtures |
| UI/IPC latency | **not yet measured** (method in section 10) | requires a Tauri build + instrumentation; explicitly UNVERIFIED today |

## 3. Harness commands

The CLI is `ps-bench` (crate `crates/ps-bench`). All paths are relative to the repository root. Outputs marked (R) are result JSON files with host and command recorded.

| Command | Purpose |
|---|---|
| `ps-bench gen --count 100000 --seed 20261109 --out .bench/dataset-100k.jsonl` | deterministic realistic corpus |
| `ps-bench queries --out .bench/queries.jsonl` | fixed filename query suite |
| `ps-bench real-export --out .bench/real-paths.jsonl` | read-only export of the legacy corpus |
| `ps-bench content-queries --out .bench/queries-content.jsonl` | fixed FTS5 suite derived from real content |
| `ps-bench content --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 3 --out (R)` | legacy FTS5 latency baseline |
| `ps-bench content-v2 --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 2 --out (R)` | v2 ps-content FTS5 benchmark (imports the same 14,609-doc corpus read-only) |
| `ps-bench filename --dataset .bench/dataset-100k.jsonl --queries .bench/queries.jsonl --iters 30 --cold-restarts 3 --out (R)` | filename engine benchmark |
| `ps-bench relevance --dataset .bench/real-paths.jsonl --golden tests/golden/real-corpus-golden.json --out (R)` | golden relevance evaluation |
| `ps-bench golden-synth` / `ps-bench golden-real` | regenerate golden fixtures |
| `ps-bench ocr-corpus` | generate ground-truth OCR images + manifest |
| `ps-bench ocr --passes 2 --out (R)` | OCR speed + accuracy |
| `ps-bench env` | host facts used in every result file |

Build and run from source:

    cargo build -p ps-bench --release
    cargo run -p ps-bench --release -- env

Published engine numbers use the **release** profile (workspace `[profile.release] opt-level 3`). Debug runs are exploratory only and must say so.

Cargo features: `engine` (default, needs `ps-index`) gates the filename/relevance commands; `content` (default, needs `ps-content`) gates `content-v2`. Content/OCR-only work can be built with `--no-default-features` while another crate is mid-edit.

## 4. Deterministic filename datasets

Generator: `crates/ps-bench/src/dataset.rs`.

- Deterministic splitmix64 PRNG, seed `20261109`; same `(seed, count)` produces byte-identical JSONL on any machine.
- "Now" for mtimes is frozen at `2026-11-09T00:00:00Z` (unix ms 1794182400000) so wall-clock time cannot leak into the corpus.
- Composition: weighted realistic roots (Documents/Downloads/Desktop/Pictures/Projects/OneDrive/Program Files/network share), 1-8+ directory depth, weighted common extensions, CJK and accented unicode names, hidden files, duplicate filenames, 200+ character long paths, ~8% folder entries.
- **Planted anchors** exist at every size so the fixed suite and the synthetic golden fixture apply to 10k/100k/1M equally: unique exact names (`Project Phoenix Budget 2026.xlsx`, `quarterly_report_final_v2.pdf`, CJK `会議議事録_2026-11-09.docx`, `日本語_メモ_2026.txt`, accented `café_résumé_naïve.md`, `.env`, a 200+ char path), a planted folder, and three `index.js` duplicates.
- Corpus meta (byte count, FNV-1a 64, SHA-256 via certutil, category counts, top extensions) is written next to each dataset as `<dataset>.meta.json`.

### Measured dataset hashes

| Dataset | Entries | Bytes | FNV-1a 64 | SHA-256 | Command |
|---|---:|---:|---|---|---|
| dataset-10k | 10,000 | 1,858,494 | b4025e64e2a1fb0a | 5c53593380543f456907b03147b540597574dfeacd5a8a93b191004c3d1d7d8e | `ps-bench gen --count 10000 --out .bench/dataset-10k.jsonl` |
| dataset-100k | 100,000 | 18,603,792 | e648c61e4cf57ac1 | ca5e8d781384a23058ef073b625305ca28924e53a3ee6b9df5570421026f1c62 | `ps-bench gen --count 100000 --out .bench/dataset-100k.jsonl` |
| dataset-1m | 1,000,000 | 186,145,379 | d3092bcbbb84f6a3 | f497eb88fd03b2e716cae90fd1fe36757ddc732e532fabae848c5505d4d43d96 | `ps-bench gen --count 1000000 --out .bench/dataset-1m.jsonl` |
| real-paths (legacy export) | 89,813 | 20,078,459 | 6457750a37badf7d | a86f0ae2a7497a5fb1294f2f66db1ab1bd5a7c18744fd37b032cfc61b1a732f8 | `ps-bench real-export --out .bench/real-paths.jsonl` |

Corpus characteristics at 1M: 920,260 files / 79,726 folders, 41,152 hidden, 53,993 CJK paths, 125,885 non-ASCII paths, 1,737 paths over 200 chars, 14 planted anchors. Real export: 59,526 files / 30,287 folders, max path 256 chars, 0 CJK paths, 6 non-ASCII paths, 1,463 hidden names, exactly 89,813 rows (matches the Gate-0 baseline, so the immutable read did not lose committed data).

### Fixed filename query suite (`.bench/queries.jsonl`)

22 queries across exact, duplicate, prefix, multi-term, typo, fuzzy, CJK, unicode, hidden, long-path, extension-shaped, folder and 0-match categories. The same suite runs at every size; per-category p50/p95/p99 are reported as well as overall.

## 5. Real corpus export (read-only, non-negotiable)

The real legacy corpus is `%APPDATA%\protonsearch\file_index.db` (149,372,928 bytes; 89,813 `files` rows; 14,609 `files_fts` content docs; 23.8 MB un-checkpointed WAL at Gate 0).

- Opened via `file:<path>?immutable=1` with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI`; `PRAGMA query_only=ON`.
- `immutable=1` takes no locks and creates no `-shm`/`-wal` files but **does not replay a pending WAL**. The harness compares exported row counts with the documented baseline and records the difference in the meta JSON; it never copies or mutates the DB.
- Export order is `ORDER BY path` (deterministic). Each line is `{path,name,ext,kind,size,modified}` matching the synthetic dataset schema.
- Used for: realistic 90k filename corpus, content query suite, real-corpus golden fixtures, and baseline-for-migration content numbers. v2 content numbers will come from `ps-content`'s index, not this DB.

## 6. Percentiles, warm-up, iterations, cold runs

- Percentiles are **nearest-rank** over raw samples: `idx = ceil(p/100 * n) - 1` for p in {50,90,95,99}. `stats.rs` is the only implementation.
- Warm filename/content runs: 2 untimed warm-up executions per query, then `--iters` (default 30) timed executions. Overall percentiles pool all per-query samples; per-category percentiles pool within category.
- Cold runs: `--cold-restarts` fresh child processes (`filename-cold-child` / `content-cold-child`). The child reports its load/open time, first-query latency and its own RSS; wall time (process spawn included) is measured by the parent. Default is 3 restarts.
- The RSS snapshot in a child costs an external PowerShell call; children report `rss_probe_ms` and the parent subtracts it from `wall_ms` (raw wall is kept as `wall_raw_ms`) so process-startup claims are not inflated.
- Reports always include `n` (sample count). A p95 with n=30 is the 29th value ordered sample; treat it as an estimate, not an asymptotic quantile.

## 7. Memory

- RSS: `Get-Process <pid> | WorkingSet64, PeakWorkingSet64` via Windows PowerShell from the same process tree; sampled before load, after build/warm, and in cold children.
- Engine memory: `FilenameIndex::memory_bytes()` is reported next to RSS, because RSS includes allocator slack and the harness's own `Vec<IndexEntry>` during construction.
- For 1M entries the harness never calls the lazily-materializing `entries()` view: hits are mapped through `entry_path(i)` and hits carry stable `entry_index` values.
- CPU and battery are not measured in this pass; no non-invasive method is wired yet. Any CPU claim must name the observer tool.

## 8. Content / FTS5 baseline (v1 -> v2 migration baseline)

- **Scope label:** the first content table measures the **legacy v1 FTS5 index** (`files_fts` in the user's historical DB). It is the baseline for the v2 decision (ADR-0003), **not** a v2 result.
- Query suite is derived from the corpus: frequent single terms (high/medium document frequency), prefix queries, two-term implicit AND, 3-token phrases, CJK terms when present, and 0-match tokens. Generation is deterministic (seed) and stored as `.bench/queries-content.jsonl` with hit counts.
- Two statement shapes are timed separately: retrieval **with** `snippet()` (highlight cost included) and **rank-only** (no snippet), so highlighting overhead is not hidden.
- Index facts: FTS shadow-table byte estimates (`files_fts_data` blocks, `_idx` term bytes, `_content`, `_docsize`) plus whole-DB file size and page size/count. This is an estimate of index bytes, not full page accounting (no `dbstat`).
- The v2 benchmark is `ps-bench content-v2`: it imports the legacy 14,609-doc content corpus read-only via `ps_content::import_legacy` into a v2 SQLite DB under `.bench/` (never `%APPDATA%`, because the agent harness cannot write there), then times `ps_content::ContentIndex::search`, which includes snippet and snippet_line generation. Its scope label is "v2 ps-content FTS5 (current schema/query path)"; import wall time, DB size, FTS shadow-byte estimate, open time and cold-process first-query latency are recorded alongside warm percentiles.

## 9. OCR: speed and accuracy are separate results

- Corpus: `tools/ocr/gen_ocr_corpus.ps1` renders 54 PNGs with GDI+ (`System.Drawing`, built into Windows) across 9 scenarios (light/dark UI text, 11px small text, 30px heading, 1.5 degree rotated document scan, monospace code, numbers, low contrast, CJK), each with known ground-truth text in a UTF-8 (no BOM) manifest. Generated under `.bench/ocr-corpus/` (gitignored scratch), reproducible.
- Engine under test: `Windows.Media.Ocr` through a small WinRT adapter inside `ps-bench` (the shipped path will be `ps-ocr`; the adapter exists so the benchmark is not blocked on crate work). If OCR is unavailable the result is `status: unverified` and no numbers are published.
- **Speed:** session open ms; per-image ms p50/p95/p99; pass 1 (cold session) vs pass 2 (warm) images/sec; wall time per pass.
- **Accuracy:** character error rate (Levenshtein over normalized text: lowercase, whitespace collapsed) and normalized exact-match rate, per scenario. Accuracy and throughput are never mixed into one claim.
- Recorded limitations: synthetic rendered text (not photographs/scans), fonts available on this host, language-pack dependence, recognized text truncated to 400 chars in evidence JSON.
- Measured on this host: Windows.Media.Ocr exposes en-GB/en-US only. English text is measured; CJK text is reported as a language-pack limitation, not as an engine capability claim. Images are read with `std::fs` and decoded through an in-memory WinRT stream because `StorageFile::GetFileFromPathAsync` is denied to processes launched by the agent harness.

## 10. UI / end-to-end latency methodology (not yet measured)

This defines exactly how click/hotkey-to-visible latency will be measured when a real Tauri build is wired to the engine. **No UI numbers are published today**, because (a) the app is not yet wired to the real engine end-to-end and (b) browser-mode mock data would measure the mock, not the product.

### 10.1 Event sequence and marks

| Mark | Where | Meaning |
|---|---|---|
| `ps:hotkey` | Rust hotkey thread, `Instant::now()` before show | summon initiated |
| `ps:shell-visible` | Rust after `window.show()` returns | window presented |
| `ps:keydown` | WebView `keydown` capture listener | user input timestamp (`performance.now`) |
| `ps:ipc-sent` | WebView immediately before `invoke("search")` | request leaves UI |
| `ps:ipc-resolved` | WebView when the invoke promise resolves | response back in UI |
| `ps:results-committed` | WebView after React commits results | DOM has new rows |
| `ps:first-paint` | two `requestAnimationFrame` ticks after commit | pixels for results painted |
| `tookMs` | Rust `SearchResponse.took_ms` | engine-only time from the engine clock |

Derived metrics:
- **hotkey -> visible + input-ready**: Rust `ps:hotkey` -> `ps:shell-visible` plus the WebView's first `ps:first-paint` after summon. Cross-process clocks are aligned by emitting Rust `Instant` deltas in the same event payload where possible; fallback is same-machine `SystemTime` (sub-ms error).
- **input -> first visible result**: `ps:first-paint - ps:keydown`.
- **decomposition**: engine = `tookMs`; IPC + Tauri dispatch = `ps:ipc-resolved - ps:ipc-sent - tookMs`; React commit = `ps:results-committed - ps:ipc-resolved`; paint = `ps:first-paint - ps:results-committed`. Each part is reported separately; a single end-to-end number must never stand in for engine performance.
- p50/p95/p99 over at least 30 keystroke events across the fixed query suite plus 10 initial summons; the display refresh rate (60/120 Hz) is recorded because it sets the paint quantum.

### 10.2 Harnesses and ownership

- **Harness of record for real-app E2E latency: `tools/visual-qa/e2e-drive.mjs`** (Owner: Lead/UX-QA). It launches the Tauri debug binary with WebView2 CDP, serves the production frontend, types real queries, captures screenshots, and records `performance.now()` before the input event plus double-rAF after the first result row. All published real-app UI numbers must come from it.
- **Auxiliary page-mark collector: `tools/ui-latency/cdp-latency.mjs`** (Owner: perf-bench). It does not launch/drive the app; it attaches to an already-running CDP-enabled page and collects page-internal marks for decomposition. It is not a second E2E harness and must not produce competing published numbers.

### 10.3 Method A — page-mark collector (read-only)

1. Launch the Tauri app with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` (environment variable only, no code change, not enabled in shipped builds).
2. `tools/ui-latency/cdp-latency.mjs` connects to the DevTools endpoint, attaches to the launcher page, and installs capture listeners via `Runtime.evaluate` (no app edits): `keydown` marks, a `MutationObserver` on the results container, double-rAF paint. It types the query with real `Input.dispatchKeyEvent` events.
3. For each take: reset, dispatch the query, wait for the result mutation + paint, collect marks; at least 30 takes.
4. The same script validates against a browser-mode Vite page, but those numbers are labelled **MOCK-BRIDGE** and are not product latency.

### 10.3 Method B — shell-owned marks (requires Lead/UI owner change, not done by perf-bench)

An opt-in instrumented build adds `window.__PS_PERF__` ring buffer + `performance.mark(...)` at the eight names above, gated by a build flag, zero cost when off. perf-bench does not own `apps/desktop/src/**`; this is specified so UI owners can implement it without redesigning the method.

### 10.4 Also required for a defensible UI claim

- Cold vs warm launcher (prewarmed hidden window vs first summon).
- Fast typing while indexing runs: no debounce loss, stale request ids dropped.
- WebView2 process-tree memory at idle (all msedgewebview2 children) and while showing results.
- Frame time/FPS during scroll and selection changes; idle CPU after 60 s of no interaction; zero Python agent processes when unused.

## 11. Relevance evaluation

- Fixtures under `tests/golden/`: `synthetic-golden.json` (deterministic, planted anchors, portable) and `real-corpus-golden.json` (machine-specific evidence generated from the read-only legacy export; **not** a portable CI test).
- Metrics fixed at top-10: **MRR@10** (mean reciprocal rank of the first expected hit), **recall@10** (fraction of the expected set found in top-10; expected set = `expectedTop1 + expectedTop5`), **exact-match@1** (top-1 path equals `expectedTop1`; scored only when the fixture asserts a unique top-1), and `expectEmpty` queries scored separately (correct when zero results).
- Duplicate-name queries intentionally do not assert top-1 identity: tie ordering is engine-defined; they assert that all listed copies appear in top-10.
- CI smoke test: `cargo test -p ps-bench --test relevance_smoke` builds a deterministic 10k corpus in memory and evaluates the committed synthetic fixture. If `ps-index::search` is a stub the test prints `ENGINE_NOT_READY` and skips quality assertions (structural checks still run); once the engine serves results the same test asserts documented regression gates.

## 12. Known confounds (recorded with every run)

- **Other agents/build jobs on the same machine:** several agents develop this repo in parallel; a compiler can be using CPU/disk during a run. Result files record host and timestamps; benchmarks are re-run when a number is used for a claim.
- Windows Defender real-time scanning affects file I/O and SQLite reads; it is never disabled.
- OS file/page cache state is not controlled; "cold process restart" does not mean cold disk cache. Warm = warmed within the process/OS, not a cache-flush guarantee.
- CPU frequency scaling/power plan and background Windows services are not controlled.
- The legacy DB is WAL-mode with an un-checkpointed WAL; immutable reads intentionally skip it (documented in each export meta).
- Rust debug builds are not representative; published numbers come from the release profile named in the result JSON.
- Timing resolution: `Instant` on Windows is QPC-backed (~100 ns), so microsecond differences at 1M scale are noise-dominated; distributions, not single runs, are reported.
- **Stale build artifacts:** a dependency change is not always reflected in a workspace binary. We caught `ps-bench` linking pre-optimization `ps-content` after `cargo clean -p ps-content`: the binary still contained the old FTS `snippet(` SQL string and reported 40 ms p95 while a freshly-built probe of the same function measured 9-11 ms. Before accepting a benchmark after a dependency change, verify a distinctive string/symbol in the binary, or delete `target/release/<bin>.exe` plus `*<crate>*` deps artifacts and rebuild; record the binary mtime in the result JSON (done by tools/benchmark harnesses).

## 13. What we did NOT measure (this pass)

- (superseded) v2 content retrieval is now measured via `ps-bench content-v2`; see BENCHMARKS.md section 4.1.
- v2 Tauri app: hotkey -> visible, input -> first result, WebView2 memory, FPS, idle CPU. No UI numbers are published or inferred.
- (superseded) OCR availability is now measured via `ps-bench ocr` on this host (en-GB/en-US recognizers); synthetic corpus only, CJK unverified.
- NTFS scan throughput, USN-journal catch-up, incremental updates, rename handling, removable/network roots.
- Battery/power impact.
- Competitor query latency: installed, but no scriptable query interface without changing competitor settings (see BENCHMARKS.md section on competitors for the exact reasons).
- Index corruption/repair paths, crash recovery, migration end-to-end timing, IPC overhead.

## 14. Reproduce everything

From the repository root (Windows PowerShell, release profile):

    cargo build -p ps-bench --release
    cargo run -p ps-bench --release -- env
    cargo run -p ps-bench --release -- gen --count 10000 --out .bench/dataset-10k.jsonl
    cargo run -p ps-bench --release -- gen --count 100000 --out .bench/dataset-100k.jsonl
    cargo run -p ps-bench --release -- gen --count 1000000 --out .bench/dataset-1m.jsonl
    cargo run -p ps-bench --release -- queries --out .bench/queries.jsonl
    cargo run -p ps-bench --release -- real-export --out .bench/real-paths.jsonl
    cargo run -p ps-bench --release -- content-queries --out .bench/queries-content.jsonl
    cargo run -p ps-bench --release -- content --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 3 --out docs/performance/results/content-legacy-fts5.json
    cargo run -p ps-bench --release -- content-v2 --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 2 --out docs/performance/results/content-v2-pscontent.json
    cargo run -p ps-bench --release -- filename --dataset .bench/dataset-100k.jsonl --queries .bench/queries.jsonl --iters 30 --cold-restarts 3 --save-index .bench/index-100k.bin --out docs/performance/results/filename-100k.json
    cargo run -p ps-bench --release -- golden-synth
    cargo run -p ps-bench --release -- golden-real --paths .bench/real-paths.jsonl
    cargo run -p ps-bench --release -- relevance --dataset .bench/real-paths.jsonl --golden tests/golden/real-corpus-golden.json --out docs/performance/results/relevance-real.json
    cargo run -p ps-bench --release -- ocr-corpus
    cargo run -p ps-bench --release -- ocr --passes 2 --out docs/performance/results/ocr-windows.json

The `.bench/` directory is gitignored scratch (datasets are large); regenerate it with the commands above. Result JSON under `docs/performance/results/` is committed so every published table can be traced to a file with host, command line and corpus hash.
