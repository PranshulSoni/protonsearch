# ProtonSearch v2 — Measured Benchmarks (Gate 2 / Gate 7 evidence)

- **Owner:** L1-C Performance & Search Quality (perf-bench)
- **Measured:** 2026-11-09, `11:33-12:20 UTC (17:03-17:50 IST) on the user workstation, release profile; the filename table is the committed-hash re-run.
- **Engine revision:** **commit a28042d1282e4c9630a952bd5b02894f66850037** (`fix(ps-index): full-path queries anchor basename + uncapped fallback (1M long-path regression)`). The filename table below was re-measured on this exact committed tree, on a quiet machine (from_entries build 12/171/2,867 ms). ps-index was unchanged between a28042d and HEAD 8692997 (only ps-core/ps-content/UI/docs changed), so the filename table remains the ps-index measurement for HEAD 8692997. The previous committed revision 3906836 table is kept in results/ and its 1M long-path regression is documented in section 3.4; **a28042d supersedes it**. The v2 content table was re-measured at HEAD 8692997 after the ps-content perf change (section 4.1).
- **Method:** [METHODOLOGY.md](./METHODOLOGY.md). Raw JSON: [results/](./results/). Every number below was printed by a command in this repository on this machine. Nothing is projected or copied from vendors.
- **Rule:** a target is "MET" only when a measured number is below it. Otherwise **NOT MET** or **NOT MEASURED**. Single-run numbers are labelled as such.

## 0. Read this before quoting anything

1. The repo is developed by several agents in parallel. During these runs the box showed **19-58% total CPU** from other agent/electron processes and compiler bursts. **from_entries build time is used as the contention proxy** (same code path: 1.4 s is normal, 5.9-6.8 s means the machine was busy).
2. Repeat runs are published, not hidden: the filename table shows the primary run plus repeat ranges. A single p95 is an estimate (n=660 samples over 22 queries x 30 iterations).
3. **100k and 1M filename targets were met on committed hashes 3906836 and a28042d on this machine**; the earlier pre-perf-fix pass did not meet 100k (5.64 ms) and is kept as first-pass evidence. The 100k p95 moved 5.64 -> 2.20 -> 1.05 ms across engine revisions, so any gate claim must name the commit. a28042d also fixes the 1M full-path regression found in 3906836 (section 3.4).
4. UI latency exists only as a 4-query debug-build exploratory sample (section 8); it is **not** a p50/p95 claim.
5. The v2 content index currently misses the 30 ms retrieval target (61 ms p95) while the legacy v1 FTS5 baseline is 6 ms p95. That is the most important finding in this report (section 4).

## 1. Environment (captured by `ps-bench env` inside every result file)

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 Home Single Language 10.0.26300 build 26300, 64-bit |
| CPU | AMD Ryzen AI 9 465 w/ Radeon 880M — 10 cores / 20 logical processors |
| RAM | 33,447,858,176 bytes (31.2 GiB) |
| Disk | C: 445.3 GB used / 367.9 GB free at measurement time |
| Hostname | PRANSHULASUS |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), cargo 1.99.0 |
| Build profile | `release` (opt-level 3, lto thin, codegen-units 4) |
| Harness | `crates/ps-bench` (dev-only, not shipped); no network calls, no telemetry |
| Defender / page cache | unchanged / not controlled (recorded as confounds, never disabled) |

## 2. Corpora

| Corpus | Entries | Files / folders | Bytes | FNV-1a 64 | SHA-256 |
|---|---:|---|---:|---|---|
| synthetic 10k (seed 20261109) | 10,000 | 9,179 / 807 (+14 anchors) | 1,858,494 | b4025e64e2a1fb0a | 5c53593380543f456907b03147b540597574dfeacd5a8a93b191004c3d1d7d8e |
| synthetic 100k (seed 20261109) | 100,000 | 92,032 / 7,954 (+14 anchors) | 18,603,792 | e648c61e4cf57ac1 | ca5e8d781384a23058ef073b625305ca28924e53a3ee6b9df5570421026f1c62 |
| synthetic 1M (seed 20261109) | 1,000,000 | 920,260 / 79,726 (+14 anchors) | 186,145,379 | d3092bcbbb84f6a3 | f497eb88fd03b2e716cae90fd1fe36757ddc732e532fabae848c5505d4d43d96 |
| real legacy export (read-only) | 89,813 | 59,526 / 30,287 | 20,078,459 | 6457750a37badf7d | a86f0ae2a7497a5fb1294f2f66db1ab1bd5a7c18744fd37b032cfc61b1a732f8 |

Real export = exactly 89,813 rows (Gate-0 baseline), read via `?immutable=1`. It contains **0 CJK paths** and 6 non-ASCII paths, which is why real-corpus relevance has no CJK queries. Synthetic corpora at 1M include 41,152 hidden names, 53,993 CJK paths, 125,885 non-ASCII paths and 1,737 paths over 200 chars.

## 3. Filename engine — warm search (release)

Harness: `ps-bench filename --iters 30 --limit 50 --save-index ...`. Warm = 2 untimed warmups + 30 timed iterations per query over the fixed 22-query suite (n=660), in-process, engine only. Deterministic ordering verified for every query in every run.

| Corpus | Warm p50 | Warm p90 | Warm p95 | Warm p99 | Max | Mean |
|---|---:|---:|---:|---:|---:|---:|
| 10k | 0.059 ms | 0.243 ms | **0.451 ms** | 0.488 ms | 0.670 ms | 0.103 ms |
| 100k | 0.471 ms | 0.874 ms | **1.047 ms** | 1.552 ms | 1.787 ms | 0.453 ms |
| 1M | 2.433 ms | 6.687 ms | **7.228 ms** | 8.545 ms | 9.572 ms | 2.525 ms |

Raw (committed hash a28042d): [filename-10k](./results/filename-10k-a28042d-20261109T1217Z.json), [filename-100k](./results/filename-100k-a28042d-20261109T1217Z.json), [filename-1m](./results/filename-1m-a28042d-20261109T1217Z.json). Prior revision: [3906836 1M](./results/filename-1m-3906836-20261109T1200Z.json).

### 3.1 Repeat-run variance (published, not hidden)

| Corpus | Engine revision | p95 | Build time | Note |
|---|---:|---:|---:|---|
| 10k | pre-perf-fix | 2.882 ms | 13.8 ms | first pass, loaded machine |
| 100k | pre-perf-fix | 5.636 ms | 219 ms | first pass; did **not** meet the 5 ms target |
| 1M | pre-perf-fix | 14.933 ms | 4,084 ms | first pass |
| 100k | pre-freeze (17:16) | 1.570 / 1.676 / 2.194 ms | 191-250 ms | three runs; all meet target |
| 1M | pre-freeze (17:16) | 8.297 / 9.504 / 12.666 ms | 2,793-6,200 ms | three runs; all meet target |
| 100k | committed 3906836 | 2.195 ms | 220 ms | prior committed revision; meets target |
| 1M | committed 3906836 | 7.583 ms | 3,529 ms | prior committed revision; meets target |
| 100k | **committed a28042d (quiet)** | **1.047 ms** | 171 ms | final primary; meets target |
| 1M | **committed a28042d (quiet)** | **7.228 ms** | 2,867 ms | final primary; meets target |
| 1M | final, contended | 28.425 ms | 5,963 ms | outlier while other agents compiled; disclosed, not used as headline |

First-pass raw files are kept: [10k-firstpass](./results/filename-10k-firstpass-20261109T1133Z.json), [100k-firstpass](./results/filename-100k-firstpass-20261109T1133Z.json), [1m-firstpass](./results/filename-1m-firstpass-20261109T1134Z.json). The 1M contended outlier: [filename-1m-contended](./results/filename-1m-contended-20261109T1148Z.json).

### 3.2 Per-category p50 / p95 / p99 (primary runs)

| Query category | 100k p50 | 100k p95 | 100k p99 | 1M p50 | 1M p95 | 1M p99 |
|---|---:|---:|---:|---:|---:|---:|
| exact | 0.10 | 0.73 | 0.84 | 0.11 | 3.79 | 4.19 |
| exact_duplicate | 0.03 | 0.03 | 0.07 | 0.06 | 0.08 | 0.17 |
| prefix | 0.46 | 0.68 | 0.72 | 1.61 | 2.83 | 3.01 |
| multi_term | 0.71 | 0.87 | 1.05 | 3.47 | 4.46 | 5.12 |
| typo | 0.52 | 0.72 | 0.92 | 2.41 | 2.66 | 2.75 |
| fuzzy | 0.87 | 1.27 | 1.28 | 4.83 | 5.14 | 5.37 |
| cjk | 0.05 | 0.06 | 0.12 | 0.10 | 0.12 | 0.17 |
| unicode | 0.33 | 0.80 | 0.97 | 1.77 | 2.53 | 3.45 |
| hidden | 0.03 | 0.03 | 0.04 | 0.05 | 0.06 | 0.07 |
| long_path | 0.17 | 0.22 | 0.24 | 1.07 | 1.21 | 1.24 |
| extension_like | 0.78 | 0.99 | 1.49 | 6.98 | 8.60 | 9.57 |
| folder | 1.33 | 1.70 | 1.79 | 7.05 | 7.76 | 8.74 |
| zero_match | 0.45 | 0.70 | 0.89 | 2.95 | 3.21 | 3.58 |

Consistent with the engine owner's note: at 1M the remaining cost is in queries that reach the **path/folder phase** (folder-only names, extension-shaped queries) — `8-10 ms p95 — and the full-scan fuzzy pass (`4.9 ms p95). Name-prefix/hidden/duplicate/CJK queries are sub-millisecond thanks to bloom filtering. Queries returning results: 19/22 at 10k, **20/22 at 100k**, **20/22 at 1M** (long_path fixed, section 3.4). At 100k/1M only the two zero-match controls return nothing; at 10k the hard-fuzzy `ppjbudget` shape also returns nothing (dataset-size dependent, not a correctness bug). Extension-shaped queries return results after the frozen relevance fixes but still pay scan cost: a relevance, not a latency item.

### 3.3 Index construction, persistence, memory

| Corpus | Dataset parse | `from_entries` build | `memory_bytes()` | Bytes/entry | Steady RSS | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| 10k | 7.9 ms | 11.6 ms | 1,245,513 B (1.19 MiB) | `124 B | 11.5 MB | 15.2 MB |
| 100k | 85.8 ms | 170.8 ms | 12,063,032 B (11.5 MiB) | `121 B | 22.5 MB | 43.8 MB |
| 1M | 803.8 ms | 2,867.3 ms | 115,491,574 B (110.1 MiB) | `115 B | 126.6 MB | 319.3 MB |

- Binary index save: 1,205,581 B / 3.6 ms (10k); 11,663,100 B / 21.3 ms (100k); 111,491,642 B / 239.5 ms (1M).
- COLD = fresh process loading the saved binary (no dataset parse): load 2.9-3.2 ms (10k), 52.0-64.7 ms (100k), 442.0-447.7 ms (1M); first query after load 0.13 ms (10k), 1.30-2.07 ms (100k), 3.22-4.01 ms (1M). The 1M cold load is still high; see section 10 item 2.
- Cold process wall (spawn + load + 5 queries, RSS-probe time subtracted by the harness): 20-26 ms (10k), 76-100 ms (100k), 467-476 ms (1M).
- Memory is consistent with ADR-0002's investigated envelope (`100-150 MB at 1M; measured 110.1 MiB engine, 120.8 MB steady RSS). Peak RSS at 1M (304 MB) is dominated by the harness's transient `Vec<IndexEntry>` during load; a production streaming loader must be measured separately before quoting a 1M memory profile to users.
- Cold details are inside the primary JSON files above; pre-freeze cold runs are kept as [100k-cold-prefreeze](./results/filename-100k-cold-prefreeze-20261109T1150Z.json) and [1m-cold-prefreeze](./results/filename-1m-cold-prefreeze-20261109T1150Z.json).

### 3.4 Regression fixed: 1M full-path query (3906836 -> a28042d)

3906836 scanned only the first `250k entries for separator queries; the planted long-path anchor is at entry `999,987 at 1M, so the full-path query returned **0 results** while it ranked #1 at 10k/100k. a28042d anchors a basename bloom name scan, verifies the full path (exact full path = tier 1.0), adds a dir-anchored pass and an uncapped complete fallback, and skips the dir pass when an exact hit exists.

| Revision | long_path result_count at 1M | top-1 | long_path p50 / p95 | queries with results (1M) |
|---|---:|---|---:|---:|
| 3906836 (prior commit) | **0** | null | 3.861 / 4.196 ms | 19/22 |
| **a28042d (final commit)** | **1** | the long `.json` anchor | **1.073 / 1.215 ms** | **20/22** |

Raw before/after: [3906836 1M](./results/filename-1m-3906836-20261109T1200Z.json) and [a28042d 1M](./results/filename-1m-a28042d-20261109T1217Z.json). Deterministic order held and zero-result queries are exactly the two zero-match controls at 1M.

## 4. Content retrieval

### 4.1 v2 ps-content FTS5 (current schema/query path)

Scope: `v2 ps-content FTS5 (current schema/query path)`. Corpus imported from the same 14,609-doc legacy `files_fts` via `ps_content::import_legacy` (read-only ATTACH; never mutates the legacy DB). Import also copies the 89,813 legacy filename rows.

| Fact | Measured |
|---|---:|
| Import (89,813 files / 13,296 content docs) | 3,722 ms wall (summary reported 3,695 ms, 0 errors) at HEAD 8692997 |
| v2 DB file | 136,097,792 B |
| Indexed text bytes | 72,072,254 B |
| documents / errors | 13,296 / 0 |
| Estimated FTS shadow bytes | 25,302,350 B |
| Open time (index already warm) | 1.76 ms |
| Cold first query (fresh process, 2 restarts) | 50.05 ms p50 |
| Warm search (30 iters x 32 queries, n=960) | **p50 9.406 ms, p95 42.998 ms, p99 67.547 ms**, max 78.161 ms (HEAD 8692997) |
| Queries with results | 25/32 |

By category (warm, snippet + snippet_line included by the API, HEAD 8692997): single_high_df p50 27.347 / p95 67.073 / p99 76.538 ms; single_medium_df 9.877 / 18.649 / 20.280; prefix 0.183 / 30.036 / 36.876; multi_term_and 5.697 / 14.307 / 15.520; phrase 3.905 / 4.321 / 4.489; cjk 0.033 / 2.092 / 3.210; zero_match 0.031 / 0.032 / 0.057.

**Verdict vs target: "Indexed document content retrieval p95 < 30 ms" is NOT MET on the v2 query path (43.0 ms p95 at HEAD 8692997, `1.4x target and `7x the legacy baseline below; it was 61.1 ms before the ps-content perf change).** The p50 single-term cost (27.3 ms) still points at per-hit snippet/line-offset work or FTS ranking; that is sources-content's call to profile. Raw (HEAD 8692997): [content-v2-pscontent-8692997](./results/content-v2-pscontent-8692997-20261109T1220Z.json); earlier revision: [content-v2-pscontent-1146Z](./results/content-v2-pscontent-20261109T1146Z.json).

### 4.2 BASELINE-FOR-MIGRATION: legacy v1 FTS5 (read-only)

Scope: the user's historical v1 SQLite FTS5 index (`files_fts`, 14,609 docs), opened `?immutable=1`. Not a v2 result; it is the baseline ADR-0003 must beat.

| Index fact | Measured |
|---|---:|
| Documents | 14,609 |
| Extracted text bytes | 72,216,618 (avg 4,943 B/doc, max 51,200 B/doc) |
| FTS shadow-byte estimate | 101,802,788 B (97.1 MiB) |
| Whole DB file | 149,372,928 B (142.4 MiB, includes non-FTS tables) |
| Open time (warm OS cache) | 0.15-0.19 ms |
| Cold first query after fresh process | 27.2 / 23.8 / 45.1 ms (3 restarts) |

Warm retrieval, 30 iterations x 32 queries (n=960), prepared statement reused:

| Path | p50 | p90 | p95 | p99 | Mean |
|---|---:|---:|---:|---:|---:|
| With `snippet()` (highlighting included) | 2.056 ms | 4.442 ms | **6.066 ms** | 24.976 ms | 2.822 ms |
| Rank-only (no snippet) | 0.568 ms | 1.959 ms | 2.221 ms | 2.865 ms | 0.855 ms |

By category (snippet path): single_high_df p95 23.290 ms, prefix p95 7.595 ms, multi_term_and p95 3.689 ms, cjk p95 2.911 ms, phrase p95 2.403 ms, single_medium_df p95 2.747 ms, zero_match p95 0.051 ms. Highlight generation is `4x p95 versus rank-only, so snippet rendering must be budgeted explicitly in the UI. Raw: [content-legacy-fts5](./results/content-legacy-fts5-20261109T1139Z.json).

## 5. OCR — Windows.Media.Ocr (speed and accuracy reported separately)

Corpus: 90 synthetic GDI+ rendered images across 9 scenarios (ui_light, ui_dark, small_text 11px, heading 30px, rotated doc_scan, monospace code, numbers, low_contrast, CJK font) x 10 known strings each. Ground truth = the exact drawn string (manifest). Engine: `Windows.Media.Ocr` through the ps-bench WinRT adapter (ps-ocr was still a stub); language = en-GB, installed recognizer languages = [en-GB, en-US] only.

| Metric | Pass 1 (cold session) | Pass 2 (warm) |
|---|---:|---:|
| Images | 90 | 90 |
| Wall | 370 ms | 368 ms |
| Throughput | 243.4 images/sec | 244.4 images/sec |
| Per-image p50 / p95 / p99 | 3.6 / 7.4 / 14.4 ms | 3.6 / 7.3 / 9.2 ms |
| Session open | 0.2 ms | (same session) |

Accuracy, warm pass (never combined with throughput above):

| Subset | n | Mean CER | Median CER | Image exact-match | Char accuracy |
|---|---:|---:|---:|---:|---:|
| All images | 90 | 0.1834 | 0.0227 | 47.8% | 87.7% |
| English-only text | 72 | **0.0556** | - | **59.7%** | **92.4%** |
| CJK text | 18 | 0.6944 | - | 0.0% | 33.7% |

Interpretation: English UI/document-style text is recognized quickly and mostly exactly at this size (p50 3.6 ms/image, char accuracy 92.4%). CJK text fails because **no CJK OCR language pack is installed** on this host — this is a language-pack limitation, not evidence about Windows OCR's CJK capability on a machine with those packs. By-scenario mean CER (warm): cjk 0.1375, heading 0.1416, doc_scan 0.1865, ui_light 0.1866, ui_dark 0.1866, numbers 0.1916, low_contrast 0.1927, small_text 0.1968, code 0.2303. Synthetic rendered text is not photographs/scans; real-scan accuracy will differ. Raw: [ocr-windows](./results/ocr-windows-20261109T1142Z.json).

## 6. Relevance quality (golden fixtures, release engine)

### 6.1 Synthetic 10k, 21 queries (portable fixture)

| Metric | Measured |
|---|---:|
| MRR@10 | 1.000 |
| recall@10 | 1.000 |
| exact-match@1 | 1.000 (13/13 asserted top-1 queries) |
| pass rate | 1.000 (19/19 scored + 2/2 zero-match) |

This includes multi-token partial matching and multi-token typo tolerance, which failed in the earlier engine revision and now rank #1. Raw: [relevance-synthetic (a28042d)](./results/relevance-synthetic-10k-a28042d-20261109T1217Z.json).

### 6.2 Real legacy corpus, 89,813 entries, 65 queries (machine-specific evidence)

| Metric | Measured |
|---|---:|
| MRR@10 | 0.928 |
| recall@10 | 0.983 |
| exact-match@1 | 1.000 |
| pass rate | 0.985 (64/65) |
| zero-match queries correct | 5/5 |

One failure: prefix `Pasted image` did not surface the expected file in top-10 (other results with that prefix ranked above it). Categories: exact 25/25 rank 1; duplicate names 10 (all listed copies found); prefix 6/7; typo 8/8 rank 1; hidden 5/5; long paths 5/5; zero-match 5/5. No CJK queries possible (real corpus has 0 CJK paths). Raw: [relevance-real (a28042d)](./results/relevance-real-a28042d-20261109T1217Z.json). Fixture: [tests/golden/real-corpus-golden.json](../../tests/golden/real-corpus-golden.json).

CI smoke test: `cargo test -p ps-bench` — 2 tests pass. The relevance test builds the deterministic 10k corpus in memory, evaluates the committed fixture and asserts MRR@10 >= 0.80 / exact@1 >= 0.90 when results exist (regression gates, not product targets; `ENGINE_NOT_READY` skips quality assertions while the engine is a stub).

## 7. Targets vs achieved (master prompt section VIII)

| Target | Condition | Status | Evidence |
|---|---|---|---|
| Hotkey -> visible + input-ready p95 < 50 ms | prewarmed launcher | **NOT MEASURED** | no hotkey-to-visible instrumentation yet; UI sample below covers input->visible only |
| 100k warm filename search p95 < 5 ms | representative names, engine query | **MET** | **1.047 ms p95 on committed hash a28042d (quiet)**; 2.195 ms on 3906836; 1.570-2.194 ms pre-freeze; pre-perf-fix 5.636 ms |
| 1M warm filename search p95 < 15 ms | realistic names/paths, indexed | **MET** | **7.228 ms p95 on committed hash a28042d (quiet, build 2,867 ms)**; 7.583 ms on 3906836; 8.297-12.666 ms pre-freeze; contended outlier 28.425 ms disclosed |
| First visible results from input p95 < 30 ms | UI+IPC+Rust+paint | **NOT MEASURED as p95** | exploratory n=4 debug-build sample: 3.8-32.3 ms input->visible (section 8); no p50/p95 until release build + >=30 takes |
| Indexed document content retrieval p95 < 30 ms | fixed mixed corpus | **NOT MET (v2)** | v2 ps-content p95 61.095 ms; legacy v1 baseline p95 6.066 ms (not a v2 number) |
| Indexed OCR-text retrieval p95 < 30 ms | OCR text already indexed | **NOT MEASURED** | OCR text is not indexed yet (ps-content/ps-ocr integration pending) |
| Responsive interaction 60 FPS / 120 FPS | scroll/selection/preview | **NOT MEASURED** | needs frame-time instrumentation in the real app |
| Inactive Python agent processes = 0 | agent unused | **NOT MEASURED** | app/sidecar were not part of these runs |
| Idle CPU near 0 | truly idle | **NOT MEASURED** | no idle-CPU instrumentation; other agents active |

## 8. UI / end-to-end latency (honest first sample, NOT a p50/p95 claim)

### 8.1 Harness of record and ownership

- **Canonical real-app harness:** `tools/visual-qa/e2e-drive.mjs` (Owner: Lead / UX-QA; launches the Tauri debug binary with WebView2 CDP, serves the production frontend via a preview server, types real queries, captures real window screenshots and measures input->visible latency).
- **Auxiliary mark collector:** `tools/ui-latency/cdp-latency.mjs` (Owner: perf-bench). It does **not** launch or drive the app; it attaches to an already-running CDP-enabled page and collects page-internal marks for decomposition. It is not a second E2E harness; when a real app run is needed, e2e-drive is used.

### 8.2 Method (as implemented by e2e-drive, confirmed against its source)

1. `performance.now()` immediately before typing the query (input event time).
2. Wait until the first result row appears in the DOM, then **two requestAnimationFrame ticks** ("paint").
3. Record `visibleAt - start` on the same page clock. Rust-side `SearchResponse.took_ms` is recorded alongside for engine-vs-UI decomposition, but the reported number is end-to-end input->visible DOM paint.

### 8.3 Conditions (record exactly)

- **Debug Rust build** (`target/debug`), production JS served by Vite preview; 150% DPI; launcher + app pages; other agents may have been compiling; n = **4 queries, no warmup separation, single take each** — this is an exploratory sample, not a distribution; release-build comparison is pending.

### 8.4 Measured sample (raw: [latency-real-app.json](../../docs/qa/visual/app-captures/latency-real-app.json))

| Query | Rows | input -> visible |
|---|---:|---:|
| `arch` (broad, first query) | 50 | 32.3 ms |
| `readme` | 50 | 4.9 ms |
| `protonserch` (typo) | 50 | 7.3 ms |
| `zzzzzz-no-such-file` (zero-match shape) | 19 | 3.8 ms |

Honest notes: (a) the broad first query includes first-use warming, so it is not comparable to the rest; (b) the "zero-match" query returned **19 rows**, i.e. the app did not show an empty state for that input — worth investigating as a relevance/fallback question; (c) hotkey->visible, focus timing, WebView2 process-tree memory, FPS and idle CPU are still unmeasured. No UI number from this section may be quoted as "p95" or as release performance.

## 9. Competitor scope on this machine (no installs, no settings changes)

| Tool | Installed | Observed facts (read-only) | Latency compared? |
|---|---|---|---|
| Everything 1.4.1.1032 (voidtools) | yes, running at probe time | WS 10.9 MB, private 2.3 MB; name-only NTFS index; `Everything.ini` has `allow_http_server=0`, `allow_etp_server=0`; no `es.exe` CLI in install dir or PATH; no SDK DLL present | **No.** No scriptable query interface; enabling servers or installing CLI/SDK would change the competitor's setup (forbidden). Different scope: whole-volume names, no indexed content. |
| Kensaku 1.0.16 (Yash Jangid) | yes, not running at probe time | Tantivy `index/` 42,255,059 B / 48 files; `metadata.db` 10,784,768 B (+10.9 MB WAL); models 30.2 MB; thumbs 47.5 MB; watches 4 dirs (Documents/Downloads/Desktop/Pictures), skip_hidden=true, OCR enabled. Read-only immutable metadata: nodes 10,853; units 13,103; visual_units 0 (WAL not replayed; caveat recorded) | **No.** Only the GUI is installed (`kensaku-cli` is not); automating a GUI launcher is not an equal-workload measurement. Scope differs: 10,853 nodes / 4 watched dirs vs our 89,813-entry real corpus and 1M synthetic. |
| PowerToys (Preview) 0.102.3101.0 | yes (`%LOCALAPPDATA%\PowerToys`) | installed; Command Palette family present in this version | **No.** Not launched/measured (different product surface). |
| Flow Launcher | no | not installed | **No.** No install performed. |

Scope mismatch is stated openly, therefore **no cross-tool latency table is published**. Kensaku's architecture notes (four tiers, `128 B/entry arena, 64-bit bloom) are cited in docs/research/RESEARCH.md as design input, not as our measurements.

## 10. What the numbers mean for engineering priorities

1. **v2 content p95 is 43 ms at HEAD 8692997 vs the 30 ms target and `7x the legacy v1 FTS5 baseline (6 ms); it improved from 61 ms after the ps-content perf change.** This remains the top performance gap; profile the ps-content snippet/line-offset and ranking path. (Legacy baseline is not a v2 accomplishment either — it is what v2 should at least match.)
2. **Filename targets are met on the final engine revision** (100k 1.57 ms, 1M 8.30 ms p95) but the 1M cold load is 240-254 ms for a 111 MB binary. A launcher that loads the index on demand cannot meet hotkey->visible 50 ms; pre-warm/memory-map/streaming load must be measured next.
3. **1M memory** `115 B/entry (110.1 MiB engine) is inside the investigated envelope; the transient loader peak (304 MB RSS) is a harness artifact that a production streaming loader should beat.
4. **Path/folder-phase queries cost `8-10 ms p95 at 1M** (folder-only names, extension-shaped); bounding that phase is the remaining filename push (engine-core is aware).
5. **OCR**: English UI text is fast (3.6 ms/image, 92.4% char accuracy on synthetic renders); CJK is unverified on this host (no language pack). Real-scan accuracy needs a real corpus before any product claim.
6. **Relevance** is strong (synthetic 1.000 MRR, real 0.928); the single real-corpus prefix miss is worth a look but is not a broad failure.
7. **UI latency** needs the release build and >=30 takes through e2e-drive before any claim; the current 4-query debug sample is only a sanity check.

## 11. Raw result files (this report)

| File | Contents |
|---|---|
| [filename-10k-a28042d](./results/filename-10k-a28042d-20261109T1217Z.json) | final committed hash, 10k warm/cold, build, memory, per-query samples |
| [filename-100k-a28042d](./results/filename-100k-a28042d-20261109T1217Z.json) | final committed hash, 100k |
| [filename-1m-a28042d](./results/filename-1m-a28042d-20261109T1217Z.json) | final committed hash, 1M (long_path fixed) |
| [filename-1m-3906836](./results/filename-1m-3906836-20261109T1200Z.json) | prior committed hash, 1M (long_path regression evidence) |
| [filename-*-firstpass](./results/filename-100k-firstpass-20261109T1133Z.json) | pre-perf-fix engine, loaded machine (variance evidence) |
| [filename-1m-contended](./results/filename-1m-contended-20261109T1148Z.json) | contended outlier (p95 28.4 ms), disclosed |
| [filename-*-firstpass](./results/filename-100k-firstpass-20261109T1133Z.json) | pre-perf-fix engine, loaded machine (variance evidence) |
| [filename-1m-contended](./results/filename-1m-contended-20261109T1148Z.json) | contended outlier (p95 28.4 ms), disclosed |
| [content-legacy-fts5](./results/content-legacy-fts5-20261109T1139Z.json) | legacy v1 FTS5 baseline (snippet vs rank-only, cold) |
| [content-v2-pscontent-8692997](./results/content-v2-pscontent-8692997-20261109T1220Z.json) | v2 ps-content FTS5 at HEAD 8692997 (import, index facts, warm/cold) |
| [ocr-windows](./results/ocr-windows-20261109T1142Z.json) | OCR speed + accuracy, per-image evidence |
| [relevance-synthetic-10k-a28042d](./results/relevance-synthetic-10k-a28042d-20261109T1217Z.json) | synthetic golden evaluation (a28042d) |
| [relevance-real-a28042d](./results/relevance-real-a28042d-20261109T1217Z.json) | real-corpus golden evaluation (a28042d) |
| [latency-real-app.json](../../docs/qa/visual/app-captures/latency-real-app.json) | exploratory UI input->visible sample (e2e-drive) |
| [synthetic-golden.json](../../tests/golden/synthetic-golden.json) / [real-corpus-golden.json](../../tests/golden/real-corpus-golden.json) | committed golden fixtures |

## 12. Reproduce

See [METHODOLOGY.md](./METHODOLOGY.md) section 14 for the full sequence. Minimum filename reproduction:

    cargo build -p ps-bench --release
    cargo run -p ps-bench --release -- gen --count 100000 --out .bench/dataset-100k.jsonl
    cargo run -p ps-bench --release -- queries --out .bench/queries.jsonl
    cargo run -p ps-bench --release -- filename --dataset .bench/dataset-100k.jsonl --queries .bench/queries.jsonl --iters 30 --cold-restarts 3 --save-index .bench/index-100k.bin --out docs/performance/results/filename-100k.json
    cargo run -p ps-bench --release -- content-v2 --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 2 --out docs/performance/results/content-v2.json
    cargo run -p ps-bench --release -- ocr --passes 2 --out docs/performance/results/ocr-windows.json

## 13. Veto list (claims this report does not support)

- "Sub-5 ms at 100k" is true on the final committed revision a28042d (1.047 ms p95 quiet; 2.195 ms on 3906836); it was 5.64 ms on the earlier pre-perf-fix revision. Any claim must name the commit and machine state.
- "Content search is fast" without the snippet/rank-only split is not supported: v2 p95 is 61 ms with snippet.
- "Everything/Kensaku-class speed" — no cross-tool measurement was run (section 9 explains why); no comparison claim is valid.
- "UI is under 30 ms p95" — only a 4-query debug-build sample exists; it is not a p95.
- "OCR works for CJK" — unverified on this host (no CJK language pack); only English rendered text was measured.
- Any 1M memory claim based on peak RSS 304 MB — that includes the harness's transient entry vector; use the engine's 110.1 MiB / steady 120.8 MB with the stated caveat.

## 14. v10 Gate 0 baseline (measured 2026-11-09/10; plan: [V10_BENCHMARK_PLAN.md](./V10_BENCHMARK_PLAN.md))

Conditions: engine rows are release-profile `crates/ps-bench` on the working tree at HEAD 0e4c9ec (+ uncommitted v10 work; other agents were building/scanning during the window - contended rows are marked by build_ms/import time). App rows are the current-tree DEBUG build (exe built 20:45Z from HEAD 3747c71 + 37-50 dirty files) with production JS via `vite preview`; the release app build was blocked by concurrent `ps-core` edits, so **no release activation number is claimed**. Reference frame-pacing is Playwright Chromium against the binding V9 HTML (offline). Every row is backed by JSON in [results/](./results/).

| Row | Gate 0 measured | Verdict vs v10 stretch target | Evidence file |
|---|---|---|---|
| Filename 10k warm | p50 0.072 / p95 0.553 / p99 0.695 ms (n=660; build 21 ms) | - | v10-g0-filename-10k-0e4c9ec-20261109T2037Z.json |
| Filename 100k warm | p50 0.515 / p95 **1.153** / p99 1.817 ms (n=660; contended build 253 ms) | **MET** (<5 ms p95) | v10-g0-filename-100k-0e4c9ec-20261109T2037Z.json |
| Filename 1M warm (first Gate-0 run) | p50 2.962 / p95 **9.310** / p99 9.941 ms (n=660; build 5.4 s -> contended) | **MET** (<15 ms p95) | v10-g0-filename-1m-0e4c9ec-20261109T2037Z.json |
| Filename 1M warm (lower-contention re-run) | p50 **2.680** / p95 **7.504** / p99 8.210 ms (max 9.108, n=660; build 4.27 s, CPU avg 23.2% at start; 20/22 queries with results, deterministic) | **MET** (<15 ms p95); cold binary load 449.5 / 583.9 ms, first query 3.94 / 4.15 ms | v10-g0b-filename-1m-quiet-4c93d21-20261109T2100Z.json |
| Content v2 (ps-content FTS5) | Gate 0 pre-optimization: p50 11.788 / p95 **59.129** / p99 161.206 ms (n=960, contended); **task-42 optimized, official quiet**: p50 **1.140** / p95 **8.984** / p99 9.399 ms, max 10.294 (n=960) | **MET** (<30 ms p95) after optimization (was NOT MET: Gate 0 59.1 ms contended / 43.0 ms quiet) | v10-g0-content-v2-0e4c9ec-20261109T2037Z.json; v10-g3-content-quiet-official2-eb61b82-20261109T2122Z.json |
| OCR extraction (ps-ocr, 90 synthetic images) | cold 92 img/s (p50 9.9 ms), warm 115 img/s; English char accuracy 92.4%, CJK 61.1% (en-GB/en-US only) | baseline recorded; no product bar claimed on synthetic renders | v10-g0-ocr-baseline-03633c7-20261109T2041Z.json |
| Indexed OCR retrieval (OcrIndex FTS5) | p50 0.042 / p95 **0.072** / p99 0.081 ms (17 queries x 30 iterations); cold open 0.94 ms, first query 0.37 ms; 81 docs / 2910 text bytes + 1.79 MB WAL | **MET** on the defined small corpus (<30 ms p95); scale and real-screenshot quality unproven | v10-g0-ocr-baseline-03633c7-20261109T2041Z.json |
| Launcher activation (DEBUG, hotkey Alt+Space) | window visible p50 7.215 / p95 **9.612** / p99 10.818 ms; foreground p50 21.266 / p95 30.384 ms (n=20) | release row **PENDING** (build blocked); debug is not a release claim | v10-g0-app-debug-4c93d21.json |
| Input -> first visible result (DEBUG, n=20 + 3 warmups) | p50 14.3 / p95 **38.5** / p99 38.6 ms (slower repeatable shapes: 'readme' 38.5-38.6 ms, 'index'/'screenshot' 30-31 ms) | **NOT MET** in debug (<30 ms p95) | v10-g0-app-debug-4c93d21.json |
| V9 reference frame pacing (24 card-switch takes) | headless virtual 60 Hz: p50 16.7 / p95 16.8 / p99 33.2 ms, dropped 1.03%, CLS 0, long tasks 0; headed real 120 Hz: p50 8.3 / p95 8.5 / p99 8.8 ms, dropped 0.16%, over-budget 1.11%, CLS 0; measured morph p50 166.6-216.6 ms; reduced-motion: 0 dropped, morph 0 ms | reference baseline recorded; app island implementation not yet measured | frame-pacing-v9-reference-{headless,headed,headed120,reduced-motion}.json |
| Idle CPU (DEBUG, 60 s idle, launcher hidden) | 2.188 CPU-s = **0.18% of machine / 3.65% of one core** (9-process tree, 0 python processes) | release row **PENDING** | v10-g0-app-debug-4c93d21.json |
| Idle memory (DEBUG, launcher hidden) | **694.7 MiB tree WS** = ProtonSearch.exe 60.3 MiB + 8 msedgewebview2 processes 634.4 MiB (455.7 MiB private) | release row **PENDING**; debug renderer overhead must not be quoted as product memory | v10-g0-app-debug-4c93d21.json |
| Input-to-highlight / cold-vs-warm launch split / multi-monitor | not measured | NOT MEASURED (needs the debug-only shell marks in plan section 6; single monitor) | - |

### 14.1 Earlier gaps: status confirmation

1. **Content retrieval p95:** **now MET** after task-42 (engine-core) - official quiet run **p50 1.140 / p95 8.984 / p99 9.399 ms** (n=960, limit 20, iters 30) vs the 30 ms target; pre-optimization Gate-0 rows were 59.1 ms (contended) / 43.0 ms (quiet). Caveats: 32 KB text-prefix limitation (matches past 32 KB lose line/text-head snippet; regression-tested), and the committed formal 'after' row from engine-core (p95 40.13) is a stale-artifact row that must not be cited - see §14.3. Legacy v1 FTS5 baseline 6.07 ms p95 remains the comparison anchor.
2. **OCR search:** now measured end-to-end (ps-ocr extraction -> OcrIndex FTS5); retrieval latency is far below target on the defined 90-image corpus, but the real gap is corpus scale and real-screenshot accuracy (English synthetic char accuracy 92.4%, below a 95% product bar; CJK unverified because only en-GB/en-US recognizers exist here).
3. **Launcher activation:** debug window-visible p95 9.6 ms; foreground p95 30.4 ms; release black-box number pending due to the build blocker. These are debug-build numbers only.
4. **UI input latency:** debug p95 38.5 ms -> NOT MET vs 30 ms; repeatable slow shapes rather than one-off jitter. Needs release-build confirmation and a fix owner (ui-launcher + engine-core).
5. **Frame pacing:** the V9 reference renders at 60 Hz virtual and 120 Hz real compositor with 0.16% dropped frames and zero layout shift; this is the bar the React island must meet through the same harness once it ships.
6. **Release-build activation, input latency, memory, idle CPU:** unmeasured at Gate 0 because the release app build was blocked by concurrent ps-core edits (duplicate constants, then missing `truncate_chars`). Retried during the Gate 0 window and still blocked; these rows cannot be quoted until a clean release build exists.
7. **Instrumentation request (Lead):** debug-only marks for (a) launcher first paint after show, (b) selection highlight paint. Without them, hotkey->paint and input->highlight remain unmeasured (plan section 6).

Raw files: [results/](./results/) - v10-g0-filename-* , v10-g0-content-v2-* , v10-g0-ocr-baseline-* , v10-g0-app-debug-4c93d21.json , frame-pacing-v9-reference-*.json.

### 14.2 G8 perf-hook verification — release activation, idle/memory, debug marks (task-43)

Builds/conditions: release exe built 02:36:28 IST from commit **c7ee0a7** (9 dirty files at run time); debug exe built 02:34:56 from the same commit (18 dirty files). Perf hooks verified on this commit: `PROTONSEARCH_PERF_HOOKS=1` writes JSONL to `<data_dir>/v2/perf-marks.jsonl` (labels `launcher-window-shown`, `launcher-first-paint`, `launcher-selection-painted`; the harness truncates the file per session and kills any stale single-instance app first). Release build cannot expose CDP by design, so the release harness is black-box: SendInput Alt+Space + Win32 EnumWindows/IsWindowVisible/GetForegroundWindow at `1 ms.

**Release activation (30/30 trials, commit c7ee0a7):**

| Metric | p50 | p95 | p99 | min | max |
|---|---:|---:|---:|---:|---:|
| Window visible after hotkey | **6.974 ms** | **9.105 ms** | 10.158 ms | 6.150 ms | 10.158 ms |
| Window foreground after hotkey | 21.457 ms | **28.994 ms** | 30.802 ms | 18.902 ms | 30.802 ms |

Cold first summon after process start: visible p50 6.198 ms; warm summons: visible p50 7.029 ms (window is pre-created hidden, so "cold" is process-cold, not window-cold). **T5 target p95 <= 50 ms: MET on the release build, single 120 Hz/150% display.** Raw: [v10-g8-release-activation-c7ee0a7.json](./results/v10-g8-release-activation-c7ee0a7.json).

**Perf marks in the same release run (30/30 measured):** Rust `launcher-window-shown` is written after `win.show() + win.set_focus() + placement`, so it reads send->shown p50 46 / p95 59 ms even though Win32 already sees the window at p95 9.1 ms; the frontend `launcher-first-paint` (double rAF after `perf:show`) adds p50 8 / p95 18 ms, giving send->"post-focus paint mark" p50 56 / p95 71 ms. **This is not hotkey->first-pixel**: the Rust mark is post-focus bookkeeping and the frontend event is emitted after it. Hotkey->first-pixel remains unmeasured until a `launcher-show-begin` mark is added immediately after `win.show()` and before `set_focus()` (request to Lead, plan section 6). No hotkey->paint claim is published from these marks.

**Release idle (60 s, launcher hidden, commit c7ee0a7):**

| Metric | Measured |
|---|---:|
| Process-tree Working Set | **786.9 MiB** = ProtonSearch.exe 70.9 MiB + 8 msedgewebview2 processes 716.0 MiB |
| Process-tree private bytes | 486.0 MiB |
| Process count | 9 (1 app + 8 WebView2), no python process |
| CPU over 60 s | 2.266 CPU-seconds = **0.19% of machine / 3.78% of one core** |

The ADR-0001 investigation budget (`150 MB tree idle) is **MISSED by `5x** on this release build - reported as a genuine release miss. Idle CPU is low as a share of the machine but still `3.8% of one core while hidden; no claim of "near-zero" is made beyond the measured number. Raw: [v10-g8-release-activation-c7ee0a7.json](./results/v10-g8-release-activation-c7ee0a7.json).

**Debug build with marks (commit c7ee0a7, hooks on, production JS, dpr 1.5):**

| Measurement | p50 | p95 | p99 | n | Verdict |
|---|---:|---:|---:|---:|---|
| Input -> first-visible result | 15.2 ms | **38.7 ms** | 45.8 ms | 20 (+3 warmup) | **NOT MET** vs 30 ms p95 (debug) |
| Hotkey -> window visible (Win32) | 7.969 ms | 10.890 ms | - | 20 | debug reference |
| Hotkey -> foreground (Win32) | 25.458 ms | - | - | 20 | debug reference |
| Input -> selection painted (mark) | **5 ms** | **7 ms** | 7 ms | 20/20 | **MET** under one 120 Hz frame (8.3 ms) and one 60 Hz frame |
| shown-mark -> first-paint (mark) | 6 ms | 11 ms | 12 ms | 20 | post-focus caveat as above |

Selection mark latency uses same-clock `Date.now()` marks (captured in-page immediately before Playwright dispatches ArrowUp/Down; mark epoch is in the page at one rAF after the keydown handler), so it excludes IPC/file-write. Debug idle over 10 s: 0.27% of machine / 5.46% of one core; memory with results visible 779.7 MiB tree. Raw: [v10-g8-app-debug-marks-c7ee0a7.json](./results/v10-g8-app-debug-marks-c7ee0a7.json).

**Quiet-window 1M filename re-run (release ps-bench, commit 4c93d21 working tree):** p50 **2.680 ms**, p95 **7.504 ms**, p99 8.210 ms (max 9.108, n=660, 20/22 queries with results, deterministic); from_entries build 4.27 s with machine CPU at 23.2% at start (lower-contention, not fully idle); cold binary load 449.5 / 583.9 ms, first query 3.94 / 4.15 ms. **T2 target p95 <= 15 ms: MET.** Raw: [v10-g0b-filename-1m-quiet-4c93d21-20261109T2100Z.json](./results/v10-g0b-filename-1m-quiet-4c93d21-20261109T2100Z.json).

Read together: filename and hotkey-activation targets are met on release; selection highlight is met in debug; input->first-visible is NOT met in debug (p95 38.7 ms); idle memory misses budget; content p95 remains task-42/engine-core; multi-monitor and hotkey->first-pixel remain unmeasured (single monitor; hook ordering).

### 14.3 G8 content optimization acceptance (task-42) - official quiet row

Engine-core's task-42 optimization (FTS5 `snippet()` removal; bounded 32 KB text prefix fetched; Rust snippet + first-match line scan; engine content lock released before search) was accepted with a formal ps-bench content-v2 run on a quiet machine (CPU avg 14.6% at start), same DB/queries/limit/iters as the engine team's own command:

    target/release/ps-bench.exe content-v2 --v2-db .bench/v2-content.db --queries .bench/queries-content.jsonl --iters 30 --limit 20 --keep-db --no-import --cold-restarts 0 --out docs/performance/results/v10-g3-content-quiet-official2-<sha>-<ts>.json

| Metric | Measured |
|---|---:|
| Warm search overall | **p50 1.140 ms / p95 8.984 ms / p99 9.399 ms / max 10.294 ms** (n=960) |
| single_high_df | p50 7.081 / p95 9.336 / p99 9.768 ms |
| single_medium_df | p50 1.197 / p95 2.776 / p99 3.056 ms |
| prefix | p50 0.040 / p95 4.597 / p99 5.048 ms |
| multi_term_and | p50 0.516 / p95 1.604 / p99 1.629 ms |
| phrase | p50 0.406 / p95 0.557 / p99 0.571 ms |
| cjk | p50 0.020 / p95 0.161 / p99 0.191 ms |
| zero_match | p50 0.019 / p95 0.020 / p99 0.031 ms |
| Index facts (unchanged) | 13,296 docs / 72,072,254 text bytes / est. FTS 25,302,350 bytes; open 0.92 ms |

**Acceptance: MET (<30 ms p95).** Raw: [v10-g3-content-quiet-official2-eb61b82-20261109T2122Z.json](./results/v10-g3-content-quiet-official2-eb61b82-20261109T2122Z.json).

**Build-artifact caveat (important for reproducibility):** two earlier formal runs (mine at HEAD 0e4c9ec and engine-core's committed 'after' row v10-g3-content-after-enginecore-20261109T2116Z.json, p50 9.23 / p95 40.13) were produced by a `ps-bench` binary that still linked the **pre-optimization ps-content**. Evidence: the binary contained the old `snippet(documents_fts` SQL string; `cargo build -p ps-bench --release` reported the target up to date even after `cargo clean -p ps-content`, and only deleting `target/release/ps-bench.exe` plus all `*ps_content*`/`*ps_bench*` deps artifacts and rebuilding produced a binary without that string. The in-process A/B probe (28.66 ms p95) and this official single-process row (8.98 ms p95) agree once the artifact is fresh; the A/B number is higher because the probe shares the process with the old 128 KB-text path. Engine-core's stale 'after' JSON should not be cited. Rule recorded in METHODOLOGY §12: verify a distinctive binary string or purge artifacts before accepting a benchmark after a dependency change.

### 14.4 G8b: mark-ordering verification + engine tail-fix end-to-end (task-48)

App runs used the Lead's reordered perf hooks (mark emitted after win.show() and before set_focus). Release run: exe built 03:04:57 IST from tree at commit 562d751 (4 dirty files). Debug run: exe built 03:11:30 IST from commit 10beb90 (1 dirty file), data dir .devdata (131,199-entry real index), production JS, dpr 1.5. All app instances were stopped afterwards; none left running.

**Release hotkey activation with strict new-window detection (30/30 trials, commit 562d751):**

The earlier task-43 release row (window visible p95 9.1 ms) used any-visible-window detection and could count a pre-existing window. The helper now excludes windows already visible before the hotkey and measures only the newly-shown launcher. Superseding row:

| Metric | p50 | p95 | p99 | min | max |
|---|---:|---:|---:|---:|---:|
| New window visible after hotkey | **12.115 ms** | **14.289 ms** | 19.786 ms | 10.701 ms | 19.786 ms |
| New window foreground after hotkey | 24.697 ms | **28.790 ms** | 32.696 ms | 21.950 ms | 32.696 ms |
| cold first summon visible / warm | 12.067 ms | 12.212 ms (warm p50) | - | - | - |

**Perf marks in the same release run (30/30):** send -> Rust launcher-window-shown p50 **41** / p95 **56** ms; shown -> frontend launcher-first-paint (double rAF after perf:show, emitted before set_focus) p50 **13** / p95 **18** ms; send -> paint-mark p50 53 / p95 70 ms. The hook ordering fix is present in source and in the built binary, but the marks cannot yet pin **hotkey->first-pixel**: the hotkey-invoke epoch is carried in the `perf:show` payload but not persisted to JSONL, so send->shown still contains hotkey dispatch + show-path latency (`30 ms beyond the Win32-visible instant). Current honest bounds on hotkey->first-pixel: Win32-visible p95 is 14.3 ms and the marks' post-show paint is 70 ms p95; the true value is inside that range. One-line hook request: persist a `launcher-hotkey-invoke` mark at the global-shortcut handler (or write `invokeEpochMs` into the shown mark) to close it. Raw: [v10-g8c-release-activation-marks-562d751.json](./results/v10-g8c-release-activation-marks-562d751.json).

**Debug end-to-end after the engine tail fix (commit 10beb90, .devdata 131,199 entries, n=20 + 3 warmups):**

| Measurement | p50 | p95 | p99 | Verdict |
|---|---:|---:|---:|---|
| Input -> first-visible result | **11.0 ms** | **38.2 ms** | 40.0 ms | **NOT MET** vs 30 ms p95 (debug); p50 improved from 15.2 ms in the previous debug run, p95 unchanged (38.7 -> 38.2) |
| Selection highlight (launcher-selection-painted mark) | **5 ms** | **6 ms** | 8 ms | **MET** under one frame; recheck stable (n=20/20) |
| Hotkey -> window visible (debug, strict detection, n=10) | 13.923 ms | 32.831 ms | - | debug reference, high variance |
| shown-mark -> first-paint (debug marks) | 9 ms | 12 ms | - | debug |

Per-query input->visible samples are bimodal: fast shapes 4.2-12.8 ms (index/report/arch/proton/café once) and repeatable slow outliers 32.9-40.0 ms (readme, screenshot, zero-match shape, pdf, café once), so the remaining p95 is a UI/IPC path issue rather than a constant cost.

**Engine/IPC/UI split: UNMEASURED in the same run.** The harness wrapped `window.__TAURI_INTERNALS__.invoke` (hook reported "installed"), but the app's search path captured the original invoke at module import, so no per-call engineWall/tookMs was recorded (all null). Directional inference only, not a same-run split: engine-core's debug probe reports engine p95 8.82 ms for the slow filename shapes, and input->visible p95 is 38.2 ms, so the remainder is IPC + React commit + paint; this must be measured by persisting the app's own search timing (e.g., a `results-committed` perf mark with the SearchResponse tookMs) rather than wrapping invoke. Honest status: split NOT MEASURED.

Idle CPU/memory rows are unchanged from task-43 (window creation did not materially change): release idle tree **786.9 MiB -> NOT MET** vs the `150 MB investigation budget, idle CPU 0.19% machine / 3.78% one core over 60 s. Not re-run in this pass, per task scope. Raw files: [v10-g3b-app-debug-enginefix-10beb90.json](./results/v10-g3b-app-debug-enginefix-10beb90.json), [v10-g8c-release-activation-marks-562d751.json](./results/v10-g8c-release-activation-marks-562d751.json), content acceptance in §14.3.

### 14.6 Island card-switch lag + memory: BEFORE (acf74ce) vs AFTER (617b6df) - task-61

User report: island open and card switching are very laggy and memory is too high. Two matched measurement sets:

- **BEFORE**: commit acf74ce, debug build + CDP with the acf74ce frontend dist rebuilt in a clean git worktree (the dist is gitignored and had been overwritten by later builds). Method: island-primary profile, prewarmed island, real Ctrl+ArrowDown switches, rAF sampling + CDP Performance deltas + page resize-event counting.
- **AFTER (frame attribution)**: same method, final build 617b6df (debug + CDP; dist built by npm run build).
- **AFTER (native release memory/activation)**: release exe built 13:26 IST from 617b6df (matched frontend dist), no CDP.

**Card switch frame pacing, BEFORE vs AFTER (debug+CDP, 5 switches search->media->clipboard->agents->controls, 120 Hz display):**

| switch | BEFORE p95 frame / dropped | AFTER p95 frame / dropped | BEFORE Layout+Recalc per switch | AFTER Layout+Recalc per switch |
|---|---:|---:|---:|---:|
| 1 (media) | 8.6 ms / 3 | 8.6 ms / 4 | 22.4+12.4 ms | 16.3+10.0 ms |
| 2 (clipboard) | 9.2 ms / 4 | 8.5 ms / 2 | 21.3+13.1 ms | 12.3+9.2 ms |
| 3 (agents) | 8.8 ms / 0 | 8.4 ms / 0 | 20.7+11.4 ms | 11.0+6.0 ms |
| 4 (controls) | 8.8 ms / 1 | 8.5 ms / 1 | 27.4+12.9 ms | 8.9+7.0 ms |
| 5 (bell) | 8.5 ms / 0 | 8.5 ms / 1 | 14.9+11.4 ms | 5.7+5.3 ms |
| **avg** | **8.8 / 1.6** | **8.5 / 1.6** | **21.3+12.2** | **10.8+7.5** |

Steady-state switches run at the 120 Hz frame interval (8.3 ms p50) in both builds. The material AFTER change is **layout+recalc cost per switch (33.5 ms -> 18.3 ms, -45%)**, matching the "one geometry call per transition, dims-driven sizing" fix; ScriptDuration stayed ~10-11 ms. First two switches still drop 3-4 frames after a fresh open in both builds (card mount/data warm-up). An intermediate WIP build (12:28) was worse than BEFORE (first-switch p95 16.7 ms, 12/10 dropped frames); the prefetch/one-geometry pass (13:01 WIP) restored steady 8.3 ms / 0-1 dropped, and final 617b6df keeps that.

**Memory - native release 617b6df (island-primary profile, CIM per-process WS/private):**

| State | Measured (617b6df release) | BEFORE reference |
|---|---:|---:|
| Cold idle, island prewarmed hidden (12 s) | **399.5 MiB WS / 200.1 MiB private, 7 procs** | 392.6-405.9 MiB WS (acf74ce, task-57) |
| Island visible after hotkey | 408.2 MiB WS | 520.8 MiB WS (acf74ce) |
| Hidden after use + 6 s LOW-trim wait | **145.1 MiB WS / 352.1 MiB private** | 183.9 MiB WS (acf74ce) |
| 20x open/switch/close (snapshot every 5) | base 145.1 -> 359.2 -> 364.0 -> 379.1 -> 351.6 MiB WS | not measured |

No monotonic leak: after the first post-trim re-warm the tree plateaus ~350-380 MiB WS and ends at 351.6 MiB. The first re-open after trim costs ~215 MiB WS (WebView2 re-warm), which is expected but worth a product decision (aggressive trim vs warm latency). The original three-window v10 idle baseline was **786.9 MiB WS**, so lazy windows + trim cut the hidden idle tree by ~82% and the after-use hidden state to ~19% of the old baseline.

**Native activation and jank (617b6df release):** hotkey->island window visible p50 21.5 ms; the island-first-paint mark follows island-window-shown by ~8-17 ms; however the Rust ui-stall probe recorded **ui-stall 30-39 ms on every open** (17 marks over the session) and one **ui-stall-124ms** on the first open. That is a real main-thread stall on the summon path and is the best native evidence for the user-perceived island open lag; card-switch frame pacing in the final **release** build could not be measured (see defect below).

**Defect (report to ui-launcher):** the final 617b6df release build does NOT emit the frontend frame marks that are committed in perfProbe.ts (island-open, island-switch:*, island-close, island-typing). Only shell-side marks appear (island-window-shown, island-first-paint, ui-stall-*). The intermediate 13:01 WIP release build DID emit island-switch marks, so this is a regression in the final build (likely the island window capability/permission set now blocking the perf_mark invoke from the island WebView, or event wiring changed by the capability ACL pass). Impact: native release card-switch frame pacing is unmeasurable; AFTER card-switch attribution above is therefore debug+CDP. Requested fix: allow perf_mark from the island window and confirm one island-switch:* mark per switch in a release run.

**Root-cause note:** the user-reported card-switch lag is not engine search (engine p50 ~1.2 ms in the payload probe). In the BEFORE build the switch transition spends **~33.5 ms in layout+recalc vs ~10 ms scripting**, consistent with the old ResizeObserver->native-resize path; the AFTER fix cuts that layout+recalc by ~45% and keeps steady-state switches at one frame (8.3 ms). Remaining native open-path cost is main-thread stalls of 30-39 ms per open (first open 124 ms) plus first-two-switch frame drops after a fresh summon (card data warm-up); those are the targets for the next UI pass. Raw JSON: [island-before-debug-acf74ce.json](./results/island-before-debug-acf74ce.json), [island-after-debug-617b6df.json](./results/island-after-debug-617b6df.json), [island-native-after-617b6df.json](./results/island-native-after-617b6df.json).

### 14.7 SearchResponse payload vs IPC vs React split - task-64 (debug 617b6df, 131,199-entry real index)

One debug+CDP session (island-primary, final frontend dist) for the engine payload investigation. Shell does not log raw wire bytes, so payloadChars = JSON.stringify(response).length from an identical page-side invoke("search") (app-visible JSON size, lower bound for wire bytes).

| Measurement | p50 | p95 | n |
|---|---:|---:|---:|
| input -> first visible result (real input event + double rAF) | **15.8 ms** | **16.3 ms** | 12/13 prefixes |
| direct invoke wall (IPC return, identical request) | 7.9 ms | 9.2 ms | 13 |
| engine took_ms from response | ~1.2 ms | ~2.2 ms | 13 |
| payloadChars (app-visible response JSON) | 46,342 | 54,101 | 13 (min 41,381 / max 54,101) |

Per prefix: 'r' 54,101 chars; 're' 52,288; 'rea' 41,381; 'read' 41,699; 'readm' 41,398; 'readme' 47,082; 's' 49,977; 'sc' 47,817; 'scre' 45,179; 'screenshot' 42,249; 'p' 42,515; 'pro' 46,619; 'project' 46,342. Action JSON is ~5.0-6.1k chars and nulls ~356-386 per response (icon field was not populated in this data dir/profile, so icon chars read 0 here; the engine probe's icon totals are the authoritative icon measurement).

**Steady-state typing (the actual lag scenario), 6 chars for readme / 8 for screenshot, 50 ms spacing:** first char 15.8-16.0 ms, then every subsequent keystroke takes **~106-108 ms** (result DOM change at ~94-96 ms) with 2/14 rows showing no result-signature change within 500 ms. Engine is not the cost: engine took_ms ~1-2 ms and direct invoke wall ~8 ms, while input->first-visible is 15.8 ms (so ~8 ms is React commit+paint beyond IPC) and the steady-state tail adds ~90 ms beyond the first keystroke. That points at the island typing/debounce/async result-update path rather than IPC payload size or engine search - worth checking the island input debounce/queue and React commit work for the remaining tail.

Card open->rendered rows from this probe were a measurement artifact for press>0 (the probe waited a fixed 420 ms before sampling, so the reported ~438-441 ms is the probe's own wait, not latency); only the fresh search open (5.2 ms) is a valid row. Not published as card data; needs a follow-up probe that polls the heading change instead of waiting.

Raw JSON: [task64-payload-split-617b6df.json](./results/task64-payload-split-617b6df.json).

**Update: island debounce 80 ms -> 24 ms (commit 364be8a, direct follow-up to the ~106 ms steady-state typing tail).** Re-measured with the same debug+CDP payload probe and final minified dist (131,199-entry .devdata index):

| Measurement | BEFORE (617b6df) | AFTER (364be8a) |
|---|---:|---:|
| Single input event input->first-visible | p50 15.8 / p95 16.3 ms (n=12) | **p50 8.5 / p95 16.9 ms (n=11, min 6.0)** |
| Steady-state subsequent keystroke (50 ms spacing) | ~106-108 ms (result change ~94-96 ms) | **~42-56 ms (result change ~36-45 ms)** |
| island-typing frontend mark (typedAt -> after paint rAF) | not emitted in this build | 32.7-49.0 ms per keystroke |
| direct invoke wall / engine took_ms | 7.9 / ~1.2 ms p50 | 9.0 / ~1.2 ms p50 |
| payloadChars p50 | 46,342 | 46,342 (unchanged) |

The debounce fix **materially improved typing**: per-keystroke tail dropped from ~106-108 ms to ~42-56 ms (~50-60% faster), and single-input-event latency halved (p50 15.8 -> 8.5 ms). Remaining ~40-50 ms per steady-state keystroke is still dominated by UI-side work (24 ms debounce + IPC ~7-9 ms + full result-list commit/paint); two of fourteen rows still show no result-signature change within 500 ms when a prefix does not change the result set (measurement caveat, not a regression). Raw JSON: [task64-payload-split-364be8a.json](./results/task64-payload-split-364be8a.json). Card press>0 rows in this run are still probe artifacts (350 ms switch guard vs 300 ms spacing) - ignored per engine-core.

**Update 2: debounce removed + memoized rows + single shown listener (commit dee5617, final).** Re-measured with the final frontend dist (debug+CDP for typing; forced-fresh native release for marks/frames):

| Measurement | 364be8a (24 ms debounce) | dee5617 (no debounce, memoized rows) |
|---|---:|---:|
| Single input event input->first-visible | p50 8.5 / p95 16.9 ms | **p50 8.8 / p95 20.5 ms (n=12, min 7.7)** |
| Steady-state subsequent keystroke (50 ms spacing) | ~42-56 ms (change ~36-45 ms) | **~24-26 ms (change ~10-14 ms)**; first char 8.3-17.4 ms |
| island-typing frontend marks | 32.7-49.0 ms | **4-28.7 ms (mostly 4-12 ms after the first)** |
| direct invoke wall / engine took_ms | 9.0 / ~1.2 ms p50 | 6.7 / ~1.2 ms p50 (payload p50 46,344 chars, unchanged) |

The steady-state typing tail is now **~24-26 ms per keystroke, down from ~106-108 ms at 617b6df (-77%)** and matches the predicted IPC+render range. Two of fourteen rows still show no result-signature change within 500 ms when the prefix does not change the result set (caveat).

**Native release mark verification (dee5617, forced-fresh release build):** the earlier "no frontend marks" report was **stale embedded assets** - removing target/release/build/protonsearch-v2-* and the release exe and relinking produced a build with frontend marks. Verified in a release session: exactly **1 island-open mark per open** (n=50, p50 8.3 / p95 8.5 / p99 41.5 / drop 1 / long 0), one **island-typing** mark 17.2 ms, and one **island-switch** mark per switch: search>media p50 8.3/p95 8.5/drop 1, media>clipboard 8.3/8.5/drop 1, clipboard>agent 8.3/8.5/drop 1. No duplicate marks (the old 10-20 duplicates per open are gone). Native release open latency: win32 visible 18.2 ms, island-open frame mark p50 8.3 ms; one **ui-stall-54ms** still appears on the open path. This run's app process was torn down externally before the close/memory sections (memory snapshots returned 0 procs), so close-mark and memory numbers are not from this run - use the 617b6df memory table above (memory code path unchanged) and re-verify close mark separately.

**Release-build embedding caveat (for the installer):** a plain rebuild after a frontend change may keep STALE embedded assets (we reproduced the missing-marks state twice); forcing removal of target/release/build/protonsearch-v2-* and the release exe (plus a main.rs touch / clean of the app crate) was required before frontend marks appeared. The installer build should force that or verify a frontend mark in a release run.
