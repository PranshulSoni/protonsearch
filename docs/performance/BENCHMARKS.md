# ProtonSearch v2 — Measured Benchmarks (Gate 2 / Gate 7 evidence)

- **Owner:** L1-C Performance & Search Quality (perf-bench)
- **Measured:** 2026-11-09, ~11:33-12:00 UTC (17:03-17:30 IST) on the user workstation, release profile; the filename table is the committed-hash re-run.
- **Engine revision:** **commit 3906836c492f6bb3f3b5e07c9af08874cad69b2e** (`feat(g2/g6): frozen ps-index, engine parser/ranker wiring, verified migration, agent shell wiring`). The filename table below was re-measured on this exact committed tree; ps-index is frozen at it.
- **Method:** [METHODOLOGY.md](./METHODOLOGY.md). Raw JSON: [results/](./results/). Every number below was printed by a command in this repository on this machine. Nothing is projected or copied from vendors.
- **Rule:** a target is "MET" only when a measured number is below it. Otherwise **NOT MET** or **NOT MEASURED**. Single-run numbers are labelled as such.

## 0. Read this before quoting anything

1. The repo is developed by several agents in parallel. During these runs the box showed **19-58% total CPU** from other agent/electron processes and compiler bursts. **from_entries build time is used as the contention proxy** (same code path: 1.4 s is normal, 5.9-6.8 s means the machine was busy).
2. Repeat runs are published, not hidden: the filename table shows the primary run plus repeat ranges. A single p95 is an estimate (n=660 samples over 22 queries x 30 iterations).
3. **100k and 1M filename targets were met in the final engine revision on this machine**; the earlier, pre-perf-fix pass did not meet 100k (5.64 ms) and is kept in the results directory as first-pass evidence. The 100k result moved by 3.6x across engine revisions, so a gate claim must name the revision.
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
| 10k | 0.070 ms | 0.545 ms | **0.584 ms** | 0.687 ms | 1.674 ms | 0.149 ms |
| 100k | 0.600 ms | 1.445 ms | **2.195 ms** | 5.108 ms | 6.728 ms | 0.728 ms |
| 1M | 2.713 ms | 7.251 ms | **7.583 ms** | 8.007 ms | 10.167 ms | 2.732 ms |

Raw (committed hash 3906836): [filename-10k](./results/filename-10k-3906836-20261109T1200Z.json), [filename-100k](./results/filename-100k-3906836-20261109T1200Z.json), [filename-1m](./results/filename-1m-3906836-20261109T1200Z.json).

### 3.1 Repeat-run variance (published, not hidden)

| Corpus | Engine revision | p95 | Build time | Note |
|---|---:|---:|---:|---|
| 10k | pre-perf-fix | 2.882 ms | 13.8 ms | first pass, loaded machine |
| 100k | pre-perf-fix | 5.636 ms | 219 ms | first pass; did **not** meet the 5 ms target |
| 1M | pre-perf-fix | 14.933 ms | 4,084 ms | first pass |
| 100k | pre-freeze (17:16) | 1.570 / 1.676 / 2.194 ms | 191-250 ms | three runs; all meet target |
| 1M | pre-freeze (17:16) | 8.297 / 9.504 / 12.666 ms | 2,793-6,200 ms | three runs; all meet target |
| 100k | **committed 3906836** | **2.195 ms** | 220 ms | final primary; meets target |
| 1M | **committed 3906836** | **7.583 ms** | 3,529 ms | final primary; meets target |
| 1M | final, contended | 28.425 ms | 5,963 ms | outlier while other agents compiled; disclosed, not used as headline |

First-pass raw files are kept: [10k-firstpass](./results/filename-10k-firstpass-20261109T1133Z.json), [100k-firstpass](./results/filename-100k-firstpass-20261109T1133Z.json), [1m-firstpass](./results/filename-1m-firstpass-20261109T1134Z.json). The 1M contended outlier: [filename-1m-contended](./results/filename-1m-contended-20261109T1148Z.json).

### 3.2 Per-category p50 / p95 / p99 (primary runs)

| Query category | 100k p50 | 100k p95 | 100k p99 | 1M p50 | 1M p95 | 1M p99 |
|---|---:|---:|---:|---:|---:|---:|
| exact | 0.03 | 0.98 | 1.26 | 0.08 | 3.76 | 3.81 |
| exact_duplicate | 0.03 | 0.03 | 0.06 | 0.07 | 0.10 | 0.15 |
| prefix | 0.57 | 0.91 | 1.03 | 1.74 | 2.87 | 2.91 |
| multi_term | 0.81 | 1.07 | 2.12 | 3.21 | 3.78 | 3.97 |
| typo | 0.71 | 1.30 | 1.71 | 2.77 | 3.31 | 3.47 |
| fuzzy | 0.98 | 5.05 | 6.32 | 5.26 | 5.57 | 5.58 |
| cjk | 0.06 | 0.08 | 0.14 | 0.08 | 0.12 | 0.12 |
| unicode | 0.47 | 0.61 | 0.68 | 1.98 | 2.34 | 2.47 |
| hidden | 0.05 | 0.06 | 0.09 | 0.08 | 0.10 | 0.13 |
| long_path | 2.65 | 6.07 | 6.73 | 3.86 | 4.20 | 4.23 |
| extension_like | 1.03 | 1.47 | 1.84 | 7.45 | 8.35 | 10.17 |
| folder | 1.51 | 2.04 | 2.13 | 7.42 | 7.97 | 7.98 |
| zero_match | 0.56 | 0.67 | 0.96 | 2.99 | 3.41 | 3.69 |

Consistent with the engine owner's note: at 1M the remaining cost is in queries that reach the **path/folder phase** (folder-only names, extension-shaped queries) — `8-10 ms p95 — and the full-scan fuzzy pass (`4.9 ms p95). Name-prefix/hidden/duplicate/CJK queries are sub-millisecond thanks to bloom filtering. Queries returning results: 19/22 at 10k, 20/22 at 100k, 19/22 at 1M. The zero-result queries are the two zero-match controls plus (10k only) the hard-fuzzy `ppjbudget` shape and (1M only) the full long-path query, which ranks #1 at 10k/100k - flagged to engine-core as a 1M-only path-phase issue. Extension-shaped queries now return results after the frozen relevance fixes, but still pay scan cost: a relevance, not a latency, item.

### 3.3 Index construction, persistence, memory

| Corpus | Dataset parse | `from_entries` build | `memory_bytes()` | Bytes/entry | Steady RSS | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| 10k | 10.2 ms | 15.0 ms | 1,245,513 B (1.19 MiB) | ~124 B | 11.5 MB | 14.1 MB |
| 100k | 113.8 ms | 220.0 ms | 12,063,032 B (11.5 MiB) | ~121 B | 22.6 MB | 43.8 MB |
| 1M | 915.4 ms | 3,529.1 ms | 115,491,574 B (110.1 MiB) | ~115 B | 126.6 MB | 319.0 MB |

- Binary index save: 1,205,581 B / 3.8 ms (10k); 11,663,100 B / 25.7 ms (100k); 111,491,642 B / 255.4 ms (1M).
- COLD = fresh process loading the saved binary (no dataset parse): load 2.7-3.3 ms (10k), 44.7-72.4 ms (100k), 505-525 ms (1M, measured while other agents were active; the pre-freeze run was 240-254 ms, so this is contention-inflated - label); first query after load 0.12-0.14 ms (10k), 1.37-2.14 ms (100k), 3.31-3.56 ms (1M).
- Cold process wall (spawn + load + 5 queries, RSS-probe time subtracted by the harness): 22-26 ms (10k), 74-107 ms (100k), 546-553 ms (1M busy).
- Memory is consistent with ADR-0002's investigated envelope (~100-150 MB at 1M; measured 110.1 MiB engine, 120.8 MB steady RSS). Peak RSS at 1M (304 MB) is dominated by the harness's transient `Vec<IndexEntry>` during load; a production streaming loader must be measured separately before quoting a 1M memory profile to users.
- Cold details are inside the primary JSON files above; pre-freeze cold runs are kept as [100k-cold-prefreeze](./results/filename-100k-cold-prefreeze-20261109T1150Z.json) and [1m-cold-prefreeze](./results/filename-1m-cold-prefreeze-20261109T1150Z.json).

## 4. Content retrieval

### 4.1 v2 ps-content FTS5 (current schema/query path)

Scope: `v2 ps-content FTS5 (current schema/query path)`. Corpus imported from the same 14,609-doc legacy `files_fts` via `ps_content::import_legacy` (read-only ATTACH; never mutates the legacy DB). Import also copies the 89,813 legacy filename rows.

| Fact | Measured |
|---|---:|
| Import (89,813 files / 13,296 content docs) | 6,601 ms wall (summary reported 6,572 ms, 0 errors) |
| v2 DB file | 136,097,792 B |
| Indexed text bytes | 72,072,254 B |
| documents / errors | 13,296 / 0 |
| Estimated FTS shadow bytes | 25,302,350 B |
| Open time (index already warm) | 1.72 ms |
| Cold first query (fresh process, 2 restarts) | 46.19 ms p50 |
| Warm search (30 iters x 32 queries, n=960) | **p50 10.860 ms, p95 61.095 ms, p99 110.730 ms**, max 153.925 ms |
| Queries with results | 25/32 |

By category (warm, snippet + snippet_line included by the API): single_high_df p50 28.743 / p95 106.600 / p99 124.815 ms; single_medium_df 11.330 / 18.797 / 20.610; prefix 0.135 / 36.627 / 61.270; multi_term_and 6.670 / 24.090 / 26.866; phrase 4.860 / 6.150 / 6.687; cjk 0.038 / 3.412 / 4.042; zero_match 0.031 / 0.045 / 0.082.

**Verdict vs target: "Indexed document content retrieval p95 < 30 ms" is NOT MET on the v2 query path (61.1 ms p95, `2x target, `10x the legacy baseline below).** The p50 single-term cost (28.7 ms) points at per-hit snippet/line-offset work or FTS ranking; that is sources-content's call to profile. Raw: [content-v2-pscontent](./results/content-v2-pscontent-20261109T1146Z.json).

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

By category (snippet path): single_high_df p95 23.290 ms, prefix p95 7.595 ms, multi_term_and p95 3.689 ms, cjk p95 2.911 ms, phrase p95 2.403 ms, single_medium_df p95 2.747 ms, zero_match p95 0.051 ms. Highlight generation is ~4x p95 versus rank-only, so snippet rendering must be budgeted explicitly in the UI. Raw: [content-legacy-fts5](./results/content-legacy-fts5-20261109T1139Z.json).

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

This includes multi-token partial matching and multi-token typo tolerance, which failed in the earlier engine revision and now rank #1. Raw: [relevance-synthetic](./results/relevance-synthetic-10k-frozen-20261109T1154Z.json).

### 6.2 Real legacy corpus, 89,813 entries, 65 queries (machine-specific evidence)

| Metric | Measured |
|---|---:|
| MRR@10 | 0.928 |
| recall@10 | 0.983 |
| exact-match@1 | 1.000 |
| pass rate | 0.985 (64/65) |
| zero-match queries correct | 5/5 |

One failure: prefix `Pasted image` did not surface the expected file in top-10 (other results with that prefix ranked above it). Categories: exact 25/25 rank 1; duplicate names 10 (all listed copies found); prefix 6/7; typo 8/8 rank 1; hidden 5/5; long paths 5/5; zero-match 5/5. No CJK queries possible (real corpus has 0 CJK paths). Raw: [relevance-real](./results/relevance-real-frozen-20261109T1154Z.json). Fixture: [tests/golden/real-corpus-golden.json](../../tests/golden/real-corpus-golden.json).

CI smoke test: `cargo test -p ps-bench` — 2 tests pass. The relevance test builds the deterministic 10k corpus in memory, evaluates the committed fixture and asserts MRR@10 >= 0.80 / exact@1 >= 0.90 when results exist (regression gates, not product targets; `ENGINE_NOT_READY` skips quality assertions while the engine is a stub).

## 7. Targets vs achieved (master prompt section VIII)

| Target | Condition | Status | Evidence |
|---|---|---|---|
| Hotkey -> visible + input-ready p95 < 50 ms | prewarmed launcher | **NOT MEASURED** | no hotkey-to-visible instrumentation yet; UI sample below covers input->visible only |
| 100k warm filename search p95 < 5 ms | representative names, engine query | **MET** | 2.195 ms p95 on committed hash 3906836 (220 ms build); pre-freeze 1.570-2.194 ms; earlier pre-perf-fix revision 5.636 ms (kept as evidence) |
| 1M warm filename search p95 < 15 ms | realistic names/paths, indexed | **MET** | 7.583 ms p95 on committed hash 3906836 (3,529 ms build); pre-freeze 8.297-12.666 ms; one contended outlier 28.425 ms disclosed |
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

Scope mismatch is stated openly, therefore **no cross-tool latency table is published**. Kensaku's architecture notes (four tiers, ~128 B/entry arena, 64-bit bloom) are cited in docs/research/RESEARCH.md as design input, not as our measurements.

## 10. What the numbers mean for engineering priorities

1. **v2 content p95 is 61 ms vs the 30 ms target and ~10x the legacy v1 FTS5 baseline (6 ms).** This is the top performance regression in the current tree; profile the ps-content snippet/line-offset and ranking path. (Legacy baseline is not a v2 accomplishment either — it is what v2 should at least match.)
2. **Filename targets are met on the final engine revision** (100k 1.57 ms, 1M 8.30 ms p95) but the 1M cold load is 240-254 ms for a 111 MB binary. A launcher that loads the index on demand cannot meet hotkey->visible 50 ms; pre-warm/memory-map/streaming load must be measured next.
3. **1M memory** ~115 B/entry (110.1 MiB engine) is inside the investigated envelope; the transient loader peak (304 MB RSS) is a harness artifact that a production streaming loader should beat.
4. **Path/folder-phase queries cost ~8-10 ms p95 at 1M** (folder-only names, extension-shaped); bounding that phase is the remaining filename push (engine-core is aware).
5. **OCR**: English UI text is fast (3.6 ms/image, 92.4% char accuracy on synthetic renders); CJK is unverified on this host (no language pack). Real-scan accuracy needs a real corpus before any product claim.
6. **Relevance** is strong (synthetic 1.000 MRR, real 0.928); the single real-corpus prefix miss is worth a look but is not a broad failure.
7. **UI latency** needs the release build and >=30 takes through e2e-drive before any claim; the current 4-query debug sample is only a sanity check.

## 11. Raw result files (this report)

| File | Contents |
|---|---|
| [filename-10k-3906836](./results/filename-10k-3906836-20261109T1200Z.json) | committed hash, 10k warm/cold, build, memory, per-query samples |
| [filename-100k-3906836](./results/filename-100k-3906836-20261109T1200Z.json) | committed hash, 100k |
| [filename-1m-3906836](./results/filename-1m-3906836-20261109T1200Z.json) | committed hash, 1M |
| [filename-*-firstpass](./results/filename-100k-firstpass-20261109T1133Z.json) | pre-perf-fix engine, loaded machine (variance evidence) |
| [filename-1m-contended](./results/filename-1m-contended-20261109T1148Z.json) | contended outlier (p95 28.4 ms), disclosed |
| [filename-*-firstpass](./results/filename-100k-firstpass-20261109T1133Z.json) | pre-perf-fix engine, loaded machine (variance evidence) |
| [filename-1m-contended](./results/filename-1m-contended-20261109T1148Z.json) | contended outlier (p95 28.4 ms), disclosed |
| [content-legacy-fts5](./results/content-legacy-fts5-20261109T1139Z.json) | legacy v1 FTS5 baseline (snippet vs rank-only, cold) |
| [content-v2-pscontent](./results/content-v2-pscontent-20261109T1146Z.json) | v2 ps-content FTS5 (import, index facts, warm/cold) |
| [ocr-windows](./results/ocr-windows-20261109T1142Z.json) | OCR speed + accuracy, per-image evidence |
| [relevance-synthetic-10k-frozen](./results/relevance-synthetic-10k-frozen-20261109T1154Z.json) | synthetic golden evaluation (committed hash) |
| [relevance-real-frozen](./results/relevance-real-frozen-20261109T1154Z.json) | real-corpus golden evaluation (committed hash) |
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

- "Sub-5 ms at 100k" is true on the frozen committed revision (2.195 ms p95); it was 5.64 ms on the earlier pre-perf-fix revision. Any claim must name the commit (3906836) and machine state.
- "Content search is fast" without the snippet/rank-only split is not supported: v2 p95 is 61 ms with snippet.
- "Everything/Kensaku-class speed" — no cross-tool measurement was run (section 9 explains why); no comparison claim is valid.
- "UI is under 30 ms p95" — only a 4-query debug-build sample exists; it is not a p95.
- "OCR works for CJK" — unverified on this host (no CJK language pack); only English rendered text was measured.
- Any 1M memory claim based on peak RSS 304 MB — that includes the harness's transient entry vector; use the engine's 110.1 MiB / steady 120.8 MB with the stated caveat.
