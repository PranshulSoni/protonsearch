# ProtonSearch v2 — Measured Benchmarks (Gate 2, first measured pass)

- **Owner:** L1-C Performance & Search Quality (perf-bench)
- **Measured:** 2026-11-09 `11:38-11:48 UTC (17:08-17:18 IST) on the user workstation
- **Method:** [METHODOLOGY.md](./METHODOLOGY.md). Raw JSON: [results/](./results/). Every number below came from a command printed in this section; nothing is projected, extrapolated or copied from vendors.
- **Rule:** a target is only "MET" when a measured number is below it. Otherwise it is "NOT MET" or "NOT MEASURED". No composite scores.

## 1. Environment (recorded by `ps-bench env` inside the result files)

| Item | Value |
|---|---|
| OS | Microsoft Windows 11 Home Single Language 10.0.26300 build 26300, 64-bit |
| CPU | AMD Ryzen AI 9 465 w/ Radeon 880M — 10 cores / 20 logical processors |
| RAM | 33,447,858,176 bytes (31.2 GiB) |
| Disk | C: 445.3 GB used / 367.9 GB free at measurement time |
| Hostname | PRANSHULASUS |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28), cargo 1.99.0 |
| Build profile | `release` (workspace opt-level 3, lto thin, codegen-units 4) |
| Harness | `crates/ps-bench` (dev-only, not shipped), engine = `crates/ps-index` as landed at measurement time |

Confound recorded: the repo is developed by several agents in parallel; at measurement time the machine showed `17-47% total CPU from other agent/electron processes and had recent compiler activity. This can inflate p95/p99. The numbers are published as measured under those conditions, and will be re-run on an idle machine before any release claim. Defender was never disabled; page-cache state was not controlled.

## 2. Corpora

| Corpus | Entries | Files/folders | Bytes | FNV-1a 64 | SHA-256 |
|---|---:|---|---:|---|---|
| synthetic 10k (seed 20261109) | 10,000 | 9,179 / 807 (+14 anchors) | 1,858,494 | b4025e64e2a1fb0a | 5c53593380543f456907b03147b540597574dfeacd5a8a93b191004c3d1d7d8e |
| synthetic 100k (seed 20261109) | 100,000 | 92,032 / 7,954 (+14 anchors) | 18,603,792 | e648c61e4cf57ac1 | ca5e8d781384a23058ef073b625305ca28924e53a3ee6b9df5570421026f1c62 |
| synthetic 1M (seed 20261109) | 1,000,000 | 920,260 / 79,726 (+14 anchors) | 186,145,379 | d3092bcbbb84f6a3 | f497eb88fd03b2e716cae90fd1fe36757ddc732e532fabae848c5505d4d43d96 |
| real legacy export (read-only) | 89,813 | 59,526 / 30,287 | 20,078,459 | 6457750a37badf7d | a86f0ae2a7497a5fb1294f2f66db1ab1bd5a7c18744fd37b032cfc61b1a732f8 |

The real export has exactly 89,813 rows — equal to the Gate 0 baseline — so the immutable read-only export did not silently lose committed data. It contains 0 CJK paths and 6 non-ASCII paths (a fact relevant to Japanese/CJK relevance expectations: the real corpus cannot test them).

Synthetic corpora include 41,152 hidden names, 53,993 CJK paths, 125,885 non-ASCII paths and 1,737 paths over 200 chars at 1M; planted anchors make all sizes directly comparable.

## 3. Filename engine — warm search (release)

Harness: `ps-bench filename --iters 30 --limit 50 --cold-restarts 3 --save-index ...`. Warm = 2 untimed warmups + 30 timed iterations per query, 22 fixed queries (n=660 samples), in-process, engine only (dataset parse and build excluded from the timed loop). Deterministic ordering verified for every query in every run.

| Corpus | Warm p50 | Warm p90 | Warm p95 | Warm p99 | Max | Verdict vs target |
|---|---:|---:|---:|---:|---:|---|
| 10k | 1.020 ms | 2.472 ms | 2.882 ms | 3.543 ms | 7.662 ms | — |
| 100k | 2.460 ms | 5.056 ms | **5.636 ms** | 6.961 ms | 11.498 ms | target p95 < 5 ms: **NOT MET** (+0.636 ms, +12.7%) |
| 1M | 9.048 ms | 13.768 ms | **14.933 ms** | 16.688 ms | 24.974 ms | target p95 < 15 ms: **MET** (0.45 ms margin, single run) |

Raw: [10k result](./results/filename-10k-20261109T1138Z.json), [100k result](./results/filename-100k-20261109T1139Z.json), [1M result](./results/filename-1m-20261109T1140Z.json).

### 3.1 Per-category p50 / p95 / p99 (100k and 1M)

| Query category | 100k p50 | 100k p95 | 100k p99 | 1M p50 | 1M p95 | 1M p99 |
|---|---:|---:|---:|---:|---:|---:|
| exact | 0.055 | 6.050 | 11.498 | 0.132 | 10.781 | 11.462 |
| exact_duplicate | 0.036 | 0.076 | 0.157 | 0.093 | 0.113 | 0.192 |
| prefix | 3.286 | 5.630 | 5.984 | 0.063 | 12.178 | 13.833 |
| multi_term | 4.455 | 6.961 | 7.469 | 10.040 | 12.101 | 14.933 |
| typo | 4.089 | 5.999 | 7.110 | 10.795 | 14.428 | 15.777 |
| fuzzy | 4.856 | 6.525 | 6.546 | 15.740 | 20.546 | 24.974 |
| cjk | 0.049 | 0.051 | 0.068 | 0.128 | 0.191 | 0.416 |
| unicode | 3.716 | 5.142 | 5.337 | 12.260 | 13.950 | 15.129 |
| hidden | 0.034 | 0.055 | 0.091 | 0.064 | 0.104 | 0.115 |
| long_path | 2.147 | 2.566 | 2.737 | 3.099 | 3.580 | 3.771 |
| extension_like | 1.993 | 2.398 | 2.588 | 10.740 | 12.495 | 16.869 |
| folder | 4.122 | 5.278 | 5.324 | 13.984 | 16.331 | 16.688 |
| zero_match | 2.569 | 3.938 | 4.781 | 9.111 | 10.369 | 11.531 |

Interpretation, stated conservatively: queries with strong prefilter selectivity (hidden dotfiles, CJK prefixes, duplicates) are fast even at 1M; *full scans* (fuzzy /**ppjbudget**-style, extension-shaped, folder, typo, multi-term, unicode) dominate the p95 at 1M and are where the 100k target is missed. The engine currently returns zero results for the extension-shaped queries (`.pdf`, `xlsx`) and the hard fuzzy query `ppjbudget` at every size while still paying the scan cost — a relevance finding, not a latency win.

### 3.2 Index construction, persistence, memory

| Corpus | Dataset parse | Index build (`from_entries`) | Engine `memory_bytes()` | Bytes/entry | Steady RSS | Peak RSS (during load) |
|---|---:|---:|---:|---:|---:|---:|
| 10k | 9.8 ms | 13.8 ms | 1,174,257 B (1.12 MiB) | `117 B | 11 MiB | 14 MiB |
| 100k | 96.5 ms | 219.0 ms | 11,416,376 B (10.9 MiB) | `114 B | 21 MiB | 43 MiB |
| 1M | 1,073.9 ms | 4,084.3 ms | 109,791,502 B (104.7 MiB) | `110 B | 120 MiB | 315 MiB |

- Binary index save: 3.6 ms / 1.08 MiB at 10k; 23.8 ms / 10.5 MiB at 100k; 278.2 ms / 100.9 MiB at 1M.
- COLD = fresh process loading the saved binary (no dataset parse): load 3.1 ms (10k), 30.9 ms (100k), 300.1-388.5 ms (1M); first query after load 0.97 ms (10k), 4.69 ms (100k), 9.93-11.70 ms (1M).
- RSS byte/entry (`110 B) is consistent across sizes and matches the documented ADR-0002 expectation (`100-150 MB at 1M; measured 104.7 MiB engine, 120 MiB steady RSS). Peak RSS at 1M (315 MiB) is dominated by the harness's transient `Vec<IndexEntry>` during load, not by the engine; a production loader that streams into the arena should be measured separately.
- Cold process wall time in this run was inflated by the child's PowerShell RSS probe (`0.4 s); the harness now reports and subtracts probe time (`rss_probe_ms`) and the final numbers will be re-published from the corrected run.

## 4. Content retrieval — BASELINE-FOR-MIGRATION (legacy v1 FTS5, read-only)

**Scope label:** this measures the user's historical v1 SQLite FTS5 index (`files_fts`, 14,609 docs), not v2 `ps-content`. It is the baseline ADR-0003 must beat when the v2 content index lands. The DB was opened `?immutable=1` read-only; no writes, no checkpoints.

| Index fact | Measured |
|---|---:|
| Documents | 14,609 |
| Extracted text bytes | 72,216,618 (avg 4,943 B/doc, max 51,200 B/doc) |
| FTS shadow-byte estimate | 101,802,788 B (`97.1 MiB) |
| Whole DB file | 149,372,928 B (142.4 MiB; includes non-FTS tables) |
| Open time (warm OS cache) | 0.15 ms |
| Cold first query after fresh process | 27.2 / 23.8 / 45.1 ms (3 restarts) |

Warm retrieval, 30 iterations x 32 queries (n=960), prepared statement reused:

| Path | p50 | p90 | p95 | p99 | Mean | Max |
|---|---:|---:|---:|---:|---:|---:|
| With `snippet()` (highlighting included) | 1.953 ms | 4.442 ms | **6.465 ms** | 23.807 ms | 2.702 ms | 29.179 ms |
| Rank-only (no snippet) | 0.515 ms | 1.959 ms | 2.103 ms | 2.534 ms | 0.810 ms | 4.048 ms |

By category (with snippet): single_high_df p95 23.662 ms (posting-list traversal dominates), prefix p95 9.291 ms, multi_term_and p95 4.169 ms, cjk p95 2.992 ms, phrase p95 2.434 ms, single_medium_df p95 2.034 ms, zero_match p95 0.042 ms.

Raw: [content-legacy-fts5-20261109T1145Z.json](./results/content-legacy-fts5-20261109T1145Z.json).

Reading: highlight generation is the expensive part (`4x p95 versus rank-only), so a v2 UI that shows snippets pays that cost; open-at-match/snippet rendering must be budgeted explicitly. The v2 `ps-content` index has not been measured yet (crate stub at measurement time), so no v2 content target is claimed.

## 5. OCR (Windows.Media.Ocr)

Status: **pending run** in this revision (the release rebuild was blocked by an in-flight `ps-index` edit). The corpus generator, ground-truth manifest (54 PNGs, 9 scenarios) and the WinRT adapter are in place; the measurement command is:

    cargo run -p ps-bench --release -- ocr-corpus
    cargo run -p ps-bench --release -- ocr --passes 2 --out docs/performance/results/ocr-windows.json

If Windows.Media.Ocr cannot be exercised on this host the report will be `status: unverified` with no numbers, per METHODOLOGY section 9. **No OCR accuracy or throughput claim exists until this section is replaced with measured output.**

## 6. Relevance quality (golden fixtures, release)

### 6.1 Synthetic corpus, 10k, 21 golden queries (portable fixture)

| Metric | Measured |
|---|---:|
| MRR@10 | 0.895 |
| recall@10 | 0.895 |
| exact-match@1 | 1.000 (13/13 asserted top-1 queries) |
| pass rate | 0.905 (19/21) |
| 0-match queries correct | 2/2 |

Failures (genuine engine gaps, reported to engine-core): `quarterly report final` (multi-term) returns no expected hit in top-10, while the prefix `quarterly_rep` ranks the same file #1; `quaterly report final` (typo) finds no fuzzy match, while the single-token typo `screnshot 2026-11-09` ranks #1. Raw: [relevance-synthetic-10k result](./results/relevance-synthetic-10k-20261109T1147Z.json).

### 6.2 Real legacy corpus, 89,813 entries, 65 golden queries (machine-specific evidence)

| Metric | Measured |
|---|---:|
| MRR@10 | 0.928 |
| recall@10 | 0.983 |
| exact-match@1 | 1.000 |
| pass rate | 0.985 (64/65) |
| 0-match queries correct | 5/5 |

Only failure: prefix `Pasted image` did not surface the expected file in top-10. Categories covered: exact (25), duplicate names (10), prefix (7), typo (8), hidden (5), long paths (5), 0-match (5); no CJK queries are possible because the real corpus has zero CJK paths. Raw: [relevance-real result](./results/relevance-real-20261109T1148Z.json). Fixture: [tests/golden/real-corpus-golden.json](../../tests/golden/real-corpus-golden.json).

CI smoke test: `cargo test -p ps-bench --test relevance_smoke` — 2 tests pass; builds the deterministic 10k corpus in memory, evaluates the committed synthetic fixture, asserts MRR@10 >= 0.80 and exact@1 >= 0.90 when the engine serves results (gates documented in the test; they are regression gates, not product targets).

## 7. Targets vs achieved (master prompt, section VIII)

| Target | Condition | Status | Evidence |
|---|---|---|---|
| Hotkey -> visible + input-ready p95 < 50 ms | prewarmed launcher | **NOT MEASURED** | no Tauri/WebView2 measurement yet; method in METHODOLOGY section 10 |
| 100k warm filename search p95 < 5 ms | representative names, engine query | **NOT MET** | 5.636 ms p95 (n=660, release, this host) |
| 1M warm filename search p95 < 15 ms | realistic names/paths, indexed | **MET (margin 0.45 ms)** | 14.933 ms p95 (n=660, release, single run; shared-machine confound) |
| First visible results from input p95 < 30 ms | UI+IPC+Rust+paint | **NOT MEASURED** | no UI numbers published |
| Indexed document content retrieval p95 < 30 ms | fixed mixed corpus | **NOT MEASURED for v2**; legacy v1 baseline with snippet p95 = 6.465 ms | v1 baseline only; v2 `ps-content` pending |
| Indexed OCR-text retrieval p95 < 30 ms | OCR text already indexed | **NOT MEASURED** | no OCR index yet |
| Responsive interaction 60 FPS / 120 FPS | scroll/selection/preview | **NOT MEASURED** | needs real app build |
| Inactive Python agent processes = 0 | agent unused | **NOT MEASURED** | app/sidecar not launched during this pass |
| Idle CPU near 0 | truly idle | **NOT MEASURED** | no instrumentation; other agents active |

Explicitly rejected claims (the harness vetoes these): "sub-5ms at 100k" (measured 5.64 ms p95), "5ms-class at 1M" (measured 9.05 ms p50 / 14.93 ms p95), "content search is fast" without the snippet/rank-only split, and any "Everything/Kensaku-class" claim (no cross-tool measurement was run; see section 8). A single-run 1M MET result is not a guarantee under indexing load; it is a measurement.

## 8. Competitor scope on this machine (no installs, no settings changes)

| Tool | Installed | Observed facts (read-only) | Query latency compared? |
|---|---|---|---|
| Everything 1.4.1.1032 (voidtools) | yes, running at probe time | WS 10.9 MB, private 2.3 MB; name-only NTFS index; `Everything.ini` has `allow_http_server=0` and `allow_etp_server=0`; **no `es.exe` CLI in the install dir or PATH**, no SDK DLL present | **No.** No scriptable query interface; enabling ETP/HTTP or installing the CLI/SDK would change the competitor's setup (forbidden). Different scope: whole-volume names, no indexed content. |
| Kensaku 1.0.16 (Yash Jangid) | yes, not running at probe time | Tantivy `index/` 42,255,059 B across 48 files; `metadata.db` 10,784,768 B (+10.9 MB WAL); models 30.2 MB safetensors; thumbs 47.5 MB; config watches 4 dirs (Documents/Downloads/Desktop/Pictures), skip_hidden=true, OCR enabled. Read-only immutable `metadata.db`: nodes=10,853; units=13,103; visual_units=0 (WAL not replayed; caveat recorded) | **No.** Only the GUI is installed (`kensaku-cli` is not); automating a GUI launcher is not an equal-workload measurement. Scope differs: 10,853 nodes over 4 watched dirs vs our 89,813-entry real corpus / 1M synthetic. |
| PowerToys (Preview) 0.102.3101.0 | yes (`%LOCALAPPDATA%\PowerToys`) | installed; Command Palette component present in this version family | **No.** Not launched; it is a different product surface and no benchmark was run. |
| Flow Launcher | no | not installed | **No.** Not installed; no install performed. |

Scope mismatch is stated openly: our filename corpus has 89,813 real entries and up to 1M synthetic; Kensaku's observed metadata covers 10,853 nodes; Everything indexes names for entire volumes. A cross-tool latency table would compare different corpora, scopes and cache states, so none is published. Kensaku's architecture notes (four tiers, `128 B/entry arena, 64-bit bloom) are cited in docs/research/RESEARCH.md as design input, not as our measurements.

## 9. What the numbers mean for engineering priorities

1. **100k target is 0.64 ms off**; the 1M MET result rests on selective prefiltering, while full-scan query shapes (fuzzy/typo/folder/unicode/multi-term/extension-shaped) cost 10-20 ms at 1M. Reducing full scans (better prefilter, trigram/prefix index, or bounded early-exit) is the highest-impact filename work.
2. **Cold load at 1M is 300-388 ms** for a 100.9 MiB binary. That directly threatens "hotkey -> visible" if the launcher loads the index on demand; a pre-warmed hidden window or memory-mapped/streamed load is required, and that tradeoff must be measured (not assumed).
3. **Memory is within the investigated envelope** (`110 B/entry; 104.7 MiB engine at 1M), but the current loader's transient peak (315 MiB) needs a streaming path before claiming a 1M memory profile.
4. **Content snippet generation costs `4x p95** vs rank-only; the UI must not render snippets for every keystroke and needs a measured budget.
5. **Relevance is strong for exact/prefix/duplicate/hidden/long-path/unicode/CJK** (rank 1 in almost all synthetic cases, MRR 0.928 on the real corpus) but has two concrete gaps: multi-token partial matching and multi-token typo tolerance. Both are testable with the committed fixtures.

## 10. Raw result files

| File | Contents |
|---|---|
| results/filename-10k-20261109T1138Z.json | warm/cold filename, build, memory, per-query samples |
| results/filename-100k-20261109T1139Z.json | same at 100k |
| results/filename-1m-20261109T1140Z.json | same at 1M |
| results/content-legacy-fts5-20261109T1145Z.json | legacy v1 FTS5 latency, index size, cold |
| results/relevance-synthetic-10k-20261109T1147Z.json | golden relevance, portable fixture |
| results/relevance-real-20261109T1148Z.json | golden relevance, real corpus |
| results/ocr-windows-*.json | OCR speed/accuracy (pending run) |
| ../tests/golden/synthetic-golden.json | committed golden queries (synthetic) |
| ../tests/golden/real-corpus-golden.json | committed golden queries (real corpus, machine-specific) |

## 11. Reproduce

See [METHODOLOGY.md](./METHODOLOGY.md) section 14 for the complete command sequence. Minimum reproduction for the filename table:

    cargo build -p ps-bench --release
    cargo run -p ps-bench --release -- gen --count 100000 --out .bench/dataset-100k.jsonl
    cargo run -p ps-bench --release -- queries --out .bench/queries.jsonl
    cargo run -p ps-bench --release -- filename --dataset .bench/dataset-100k.jsonl --queries .bench/queries.jsonl --iters 30 --cold-restarts 3 --save-index .bench/index-100k.bin --out docs/performance/results/filename-100k.json
