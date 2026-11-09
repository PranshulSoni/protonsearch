# ProtonSearch v2 — Benchmark Plan (v10)

Owner: L1-C (perf-bench) with L1-E release acceptance. Status: Gate-0 plan; baseline evidence is in
docs/performance/BENCHMARKS.md and docs/performance/results/*.json. No new benchmark numbers are
invented here; this is a plan plus the verified baseline status.

## 1. Goals (master prompt section 9; stretch targets, not facts)

| Metric | Target | Current baseline status |
|---|---|---|
| Warm filename lookup @100k | p95 <= 5 ms | MET: 1.047 ms p95 (committed a28042d, quiet) |
| Warm filename lookup @1M | p95 <= 15 ms | MET: 7.228 ms p95 (a28042d, quiet) |
| Warm indexed text retrieval | p95 <= 30 ms | NOT MET: v2 content 43.0 ms p95 at HEAD 8692997 (legacy v1 FTS5 baseline 6.07 ms) |
| Warm indexed OCR retrieval | p95 <= 30 ms | NOT MEASURED: OCR text not indexed yet |
| Launcher visible after global hotkey | p95 <= 50 ms | NOT MEASURED: no hotkey->visible instrumentation |
| Input-to-first-visible-result, warm | p95 <= 30 ms | NOT MEASURED as p95; only 4-query debug sample (3.9-7.8 ms) |
| Frame pacing | 60 fps stable; pursue 120 fps on supported displays | NOT MEASURED (no frame-time harness yet) |
| Idle CPU | near zero when truly idle | NOT MEASURED for v10 |
| 1M memory | bounded, documented | Engine 110.1 MiB; steady RSS 120.8 MB; peak 304 MB is harness artifact (caveat) |

Evidence: docs/performance/BENCHMARKS.md sections 3 (filename), 4 (content), 5 (OCR), 7 (targets),
8 (UI sample), 13 (veto list); raw JSON under docs/performance/results/ (filename a28042d, content
8692997, OCR windows, relevance, latency-real-app.json). Environment is captured inside each JSON.

## 2. v10 benchmark workstreams

1. **Launcher activation and interaction latency (both modes).**
   - Hotkey->visible: instrument native show path (timestamp before RegisterHotKey callback through
     paint/first frame) on a pre-warmed hidden launcher; p50/p95/p99, n>=30 takes, warm and cold.
   - Input->first visible result: real IPC + release build; n>=30 per query class; separate
     no-result/result states; record app/engine memory.
   - Normal vs Dynamic Island parity where meaningful; animation start vs first painted frame split.
2. **V9 frame pacing and motion fidelity.**
   - Harness: CDP screenshot/video + frame timestamps, or Windows ETW/DWM frame data; capture at 60 and
     120 Hz; measure p50/p95/p99 frame time, long tasks, dropped frames, layout shifts; specific
     transitions: compact->expanded search, card switch (wheel/swipe/dot), detail drill-down
     (Wi-Fi/Bluetooth/audio), expanded tall cards, reduce-motion path.
   - Acceptance: no visible jank/overshoot under V9 choreography; record real numbers rather than
     claiming identical hardware-independent timing.
3. **Content retrieval (top priority gap).**
   - Profile ps-content FTS/snippet/line-offset path; targets p95 <=30 ms; separate rank-only vs
     snippet/highlight cost; document per-query-class p50/p95/p99; compare legacy FTS5 baseline.
4. **OCR.**
   - Extraction throughput/accuracy separated from indexed OCR search; English + multilingual
     (language packs available), screenshot-sized images, scanned PDFs, mixed layouts.
5. **Index lifecycle.**
   - Scan/build time, incremental update cost, cold load, memory-map/streaming loader measurement,
     crash recovery, battery/AC-DC behavior, network/removable path failures.
6. **Resource budgets.**
   - Idle CPU/memory, hidden WebView2 process tree, agent runtime absent vs running, indexing under
     load, battery impact. Report process-tree RSS and startup wall separately.
7. **Platform matrix.**
   - Required: Win11, 100/125/150/200% scaling, single+dual/mixed-DPI/disconnect, 60/120 Hz, light/dark,
     reduced motion, AC/DC. Available now: single 2880x1800 @120 Hz, effective 150% (scaling variants
     by in-session change), 100/125/200% can be set; dual/mixed/disconnect and AC/DC battery states are
     GAPS (see V10_GATE0_AUDIT) and must be marked as gaps, not tested.

## 3. Datasets and fixtures

- Existing: synthetic 10k/100k/1M (seed 20261109, committed hashes in BENCHMARKS sections 2), real
  legacy corpus export 89,813 rows (read-only immutable), golden relevance fixtures
  (tests/golden/real-corpus-golden.json), OCR corpus in .bench/ocr-corpus (91 files), content corpus.
- v10 additions: preview test corpus (ASCII/Hindi/CJK/emoji, 0-byte, 1MB, 100MB, CRLF/LF, BOM/invalid,
  PDF text/scanned, DOCX with tables, Markdown/code matches, deleted/moved/locked); clipboard image
  corpus (Paint/Snipping Tool/browser images, DIB/DIBV5/PNG, large/high-DPI/transparent, rapid
  sequences); island motion fixture states (8 cards x idle/populated/empty/detail, reordered dots);
  placement fixture monitors (only single available).
- Each dataset records provenance, hash and size; never mix synthetic and real numbers without labels.

## 4. Method and evidence format

- Release build unless a debug build is explicitly labeled; record commit hash, Rust/Node/WebView2
  versions, OS build, display config, DPI, refresh, AC/DC, antivirus state, other load.
- Warmup separate; n>=30 takes for p95 claims; publish raw per-iteration JSON, not only aggregates.
  Contended outliers are disclosed (see BENCHMARKS 3.1).
- Frame timing: same resolution side-by-side V9 HTML vs app; per-state captures; p50/p95/p99, dropped
  frames, long tasks; optional ETW traces.
- Each benchmark result file includes: schema version, benchmark id, harness commit, environment,
  dataset hash, query manifest, iteration count, raw samples, summary stats, known caveats.
- Reproducibility package: commands + dataset generation seeds + expected hashes; CI smoke tests only
  for deterministic microbenchmarks, not for hardware-dependent claims.

## 5. Regression thresholds and gate linkage

- Filename: keep p95 within 20% of a28042d baseline on same machine/dataset for 100k/1M. Regressions
  block G3/G8 unless accepted with an explicit trade-off.
- Content: p95 must improve from 43 ms toward <=30 ms; every change records before/after JSON. Not
  claimed fixed until target met on defined corpus.
- Launcher: hotkey->visible p95 <=50 ms and input->first-visible p95 <=30 ms are G4/G5 exit criteria.
- Island motion: no dropped-frame spike above agreed threshold per transition; 60 Hz stable on this
  host; 120 Hz measured and reported honestly.
- Idle CPU/memory budgets defined at G1 and enforced at G8 (no agent process when unused).
- Any competitor comparison must use equal data/OS/hardware/workload and state scope; installer-less
  GUI tools may not be comparable (BENCHMARKS section 9 policy).

## 6. Schedule and ownership

| Phase | Benchmark deliverable | Owner | Gate |
|---|---|---|---|
| G0 | baseline status + plan + existing evidence index | L1-C with L1-E | G0 |
| G2 | clipboard image capture latency + preview extraction timing for P0 fixes | L2-A/L1-A | G2 |
| G3 | filename/content/OCR baseline vs optimized + regression thresholds | L1-A/L1-C | G3 |
| G4 | Normal Launcher activation/input latency p50/p95/p99, release build | L1-C | G4 |
| G5 | island frame pacing + V9 comparison captures at 60/120 Hz | L1-C/L1-B | G5 |
| G6 | placement/card-manager interaction cost (no frame drops) | L1-B/L1-C | G6 |
| G7 | native device action latency/capability checks; agent on-demand lifecycle/resource | L1-A/L1-D | G7 |
| G8 | full benchmark package, regressions fixed, security/perf report | L1-C/L1-E | G8 |
| G9 | reproducibility package + final numbers in release docs | L1-E | G9 |

## 7. Honest constraints

- No second physical monitor or mixed-DPI/disconnect setup at Gate 0; dual-monitor claims are blocked
  until hardware is available.
- No signing certificate and no non-Pi agent credentials; these do not block benchmarks but block their
  release claims.
- Frame pacing is hardware/WebView2 dependent; publish measured numbers and gaps, never promise
  identical animation timing across hardware.
- "World's fastest" is a challenge, not a claim; competitor comparisons follow the scope rules above.
