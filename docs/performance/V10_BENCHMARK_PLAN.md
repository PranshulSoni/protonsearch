# ProtonSearch v10 — Benchmark & Acceptance Plan (Gate 0 baseline + targets)

- **Owner:** L1-C Search Quality / Performance Lead (perf-bench)
- **Status:** Gate 0 baseline published, plan active for the v10 Dynamic Island programme
- **Source of truth for targets:** docs/reference/v10-handoff/PROTONSEARCH_V2_FINAL_MASTER_IMPLEMENTATION_PROMPT.md sections 4.2, 9 (Benchmark goals) and 13
- **Raw evidence:** docs/performance/results/*.json (every row below names its file or command)
- **Rules:** a target is MET only when a measured number is below it on this machine; otherwise NOT MET or NOT MEASURED. Stretch targets are not facts. Engine results are release-profile unless labelled debug. UI results are labelled debug/release explicitly. No fabricated numbers, no cross-tool claims without equal work.

## 1. Environment for all measurements

| Item | Value |
|---|---|
| OS | Windows 11 Home Single Language 10.0.26300 build 26300, 64-bit |
| CPU / RAM | AMD Ryzen AI 9 465 w/ Radeon 880M, 10C/20T / 31.2 GiB |
| Display | single 2880x1800 panel at 120 Hz, 150% scale (logical 1920x1200) - no second monitor available |
| Rust / Node | rustc 1.99.0 / Node 26.7.0 |
| WebView2 | Edge 157.x runtime (seen via CDP); Playwright Chromium 158 for reference HTML |
| Benchmark harness | crates/ps-bench (engine), tools/benchmark (frame pacing + app activation), tools/visual-qa/e2e-drive.mjs (canonical real-app E2E) |
| Caveat | other agents build/run on the same box; each result JSON records git HEAD, dirty count, command and timestamps. Contended runs are labelled. |

## 2. Stretch targets to scenario/threshold/owner map

Thresholds marked "stretch" come from the master prompt; the acceptance threshold is the number a gate will actually enforce (stretch values are used directly unless a measurement method cannot support them).

| # | Target (stretch) | Reproducible scenario | Acceptance threshold | Owner | Current status / evidence |
|---|---|---|---|---|---|
| T1 | Warm filename lookup 100k files p95 <= 5 ms | `cargo run -p ps-bench --release -- filename --dataset .bench/dataset-100k.jsonl --queries .bench/queries.jsonl --iters 30 --cold-restarts 3` (release) | p95 <= 5 ms, n >= 660 samples, deterministic order | engine-core (engine), perf-bench (harness) | **MET** 1.15 ms p95 Gate 0 (docs/performance/results/v10-g0-filename-100k-*.json); 1.05 ms quiet run earlier |
| T2 | Warm filename lookup 1M files p95 <= 15 ms | same with dataset-1m | p95 <= 15 ms | engine-core | **MET** 7.50 ms p95 in the lower-contention Gate-0 re-run (p50 2.68, p99 8.21, n=660, build 4.27 s); first run 9.31 ms under heavier contention |
| T3 | Warm indexed text retrieval p95 <= 30 ms | `ps-bench content-v2 --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 2` | p95 <= 30 ms with snippet + snippet_line included | sources-content (ps-content), perf-bench | **NOT MET** 59.1 ms p95 Gate 0 (contended; 43.0 ms earlier quiet) - top performance gap |
| T4 | Warm indexed OCR retrieval p95 <= 30 ms | `ps-bench ocr-baseline --manifest .bench/ocr-corpus/manifest.jsonl` (ps-ocr extraction + OcrIndex FTS5) | p95 <= 30 ms on a defined corpus; extraction accuracy reported separately | sources-content, perf-bench | **MET on defined corpus** 0.072 ms p95 (81 docs, 90-image synthetic corpus). Not yet proven at screenshot-corpus scale (G5 needs a >=2k image corpus) |
| T5 | Warm launcher visible after global hotkey p95 <= 50 ms | release build (CDP disabled by design) + SendInput Alt+Space + Win32 EnumWindows polling, 30 trials: `node tools/benchmark/release-activation.mjs` | p50/p95/p99 visible+foreground <= 50 ms p95 | ui-launcher (window/hotkey), L0 (shell), perf-bench (harness) | **DEBUG MEASURED (n=20, current-tree debug build)**: window visible p50 7.215 / p95 9.612 / p99 10.818 ms; foreground p50 21.266 / p95 30.384 ms. **RELEASE black-box row PENDING** - release build was blocked at Gate 0 by concurrent ps-core edits (compile errors), so no release number is claimed yet. Debug is not a release claim. |
| T6 | Input -> first visible result p95 <= 30 ms (warm cache) | debug build + CDP: `node tools/benchmark/app-baseline.mjs` (>=20 takes + 3 warmup) | p95 <= 30 ms end-to-end DOM paint; engine tookMs reported separately | ui-launcher + engine-core, perf-bench | **DEBUG MEASURED (n=20 after 3 warmups): p50 14.3 / p95 38.5 / p99 38.6 ms -> NOT MET** vs 30 ms p95 in the debug build. Slow shapes: 'readme' 38.5-38.6 ms, 'index'/'screenshot' 30-31 ms; most other queries 9.9-14.9 ms. Release-build row PENDING. |
| T7 | 60 FPS ordinary monitors, pursue 120 FPS; measure p95/p99 frame pacing | V9 reference: `node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/v9-island-cards.json`; app island: same harness with `--cdp` once shipped | >=99% of frames <= 1.5x refresh period; p95 frame <= 1.2x period; p99 <= 2x period; long tasks >50 ms = 0; CLS <= 0.02 per transition | ui-launcher (implementation), perf-bench (harness) | **REFERENCE BASELINE MEASURED** headless 60 Hz virtual: p50 16.7/p95 16.8/p99 33.2 ms, dropped 1.03%, CLS 0; headed real compositor at 120 Hz (hz=120 run): p50 8.3 / p95 8.5 / p99 8.8 ms, dropped 0.16% (3/1896 frames), over-budget 1.11%, max 83.4 ms single jank; reduced-motion: 0 dropped, morph 0 |
| T8 | Near-zero CPU idle; optional Python runtime absent when unused | `node tools/benchmark/release-activation.mjs` idle CPU 60 s; agent process check | <= 1% of machine long-run idle with no user work; 0 python sidecar processes | perf-bench + agent-sidecar | **DEBUG MEASURED (60 s idle, launcher hidden): 2.188 CPU-seconds = 3.65% of one core / 0.18% of machine** (9-process tree). Release-build row PENDING (same harness). Python sidecar not running - checked in process tree (0 python processes). |
| T9 | App + WebView2 memory budget (ADR-0001 investigation: `150 MB idle) | process-tree Working Set for ProtonSearch + msedgewebview2 children (PowerShell/CIM) | <= 150 MB tree idle on release build (stretch, report if missed) | perf-bench + perf owner (ui?) | **DEBUG MEASURED (idle, launcher hidden): 694.7 MiB tree WS** = ProtonSearch.exe 60.3 MiB + 8 msedgewebview2 processes 634.4 MiB (455.7 MiB private). This is a debug/dev-build number and must not be quoted as release memory; release-build row PENDING. |
| T10 | OCR extraction throughput + accuracy (separate from retrieval) | `ps-bench ocr-baseline` extraction passes | English char accuracy >= 95% on real screenshots before a product claim; throughput tracked (no invented threshold) | sources-content | **BASELINE MEASURED** synthetic GDI+ renders: cold 92 img/s (p50 9.9 ms), warm 115 img/s; English char accuracy 92.4%, CJK 61.1% (en-GB/en-US only) - synthetic corpus, real-scan quality unverified |
| T11 | Input-to-highlight under one rendered frame when feasible | app debug + CDP highlight mark (needs shell hook; see section 6) | p95 <= 1 frame (16.7 ms @60 Hz / 8.3 ms @120 Hz) for arrow-key selection highlight | ui-launcher | **NOT MEASURED** - no shell instrumentation mark for selection paint yet |
| T12 | Cold start / activation split | release launch to first invocation vs warm summons (T5 harness records cold first trial vs warm) | reported, no invented threshold | ui-launcher | **PENDING release run** |

Additional required v10 measurements tracked in this plan but without stretch numbers: index build time/memory (T1/T2 harness), content import time (T3 harness), OCR index size (T4 harness), long tasks and layout shift per transition (T7 harness), reduced-motion equivalence (T7 reduced-motion scenario), hotkey conflict/remap path (manual UI test, ui-launcher), WebView2 child-process memory breakdown (T9), indexing-active scroll/typing smoothness (T7 + app harness under scan), multi-monitor/DPI placement (blocked on hardware, section 6), battery/power (not measured without a supported observer).

## 3. Measurement definitions (binding for every row)

- **Warm filename/content/OCR retrieval:** engine process already started with the index/DB loaded; 2 untimed warmups per query then N timed iterations; nearest-rank p50/p95/p99 over pooled samples; deterministic ordering checked.
- **Cold filename:** fresh process loading the persisted binary index; reported separately from spawn time.
- **Warm launcher visible after hotkey:** release process started and settled; launcher window pre-created hidden; SendInput Alt+Space; Win32 EnumWindows/IsWindowVisible + GetForegroundWindow polled at `1 ms; visibleMs and focusedMs recorded per trial; first trial labelled cold (process-cold), remaining warm.
- **Input -> first visible result:** debug build (CDP enabled) with production JS; performance.now before a native input event, until result signature changes and two requestAnimationFrame ticks complete; engine-only time from SearchResponse.tookMs reported separately. Debug numbers appear only in the debug section.
- **Frame pacing:** rAF frame intervals inside a card-switch window; dropped frame = interval > 1.5x assumed refresh period; over-budget = > 1.05x; layout shift = PerformanceObserver layout-shift without recent input; long tasks from PerformanceObserver longtask; morph duration from island getBoundingClientRect samples per frame; CSS declared transitions captured for cross-check. Headless Chromium uses a virtual 60 Hz vsync; headed runs record the real compositor refresh (this host is 120 Hz). Mode is always written to JSON.
- **Idle CPU:** process-tree CPU time delta over a 60 s no-interaction window (PowerShell/CIM + Get-Process TotalProcessorTime), reported as % of machine and % of one core; pending tasks/indexing noted.
- **Memory:** sum of Working Set for ProtonSearch.exe plus msedgewebview2 descendants via Win32_Process parent chain.

## 4. V9 reference motion specification (measured, for implementation parity)

Derived from the binding reference HTML (protonsearch_dynamic_island_cards_v9_polished.html) and the frame-pacing harness:

| Property | Declared / measured |
|---|---|
| Island width transition | 0.55 s, cubic-bezier(0.22, 1, 0.36, 1) |
| Island height transition | 0.55 s, same easing |
| Island border-radius | 0.5 s |
| Island transform | 0.45 s |
| Island opacity | 0.22 s |
| Card content swap | panel gets .out (opacity 0, translateY(-11px)); innerHTML replaced after 120 ms; .in runs cardin 0.25 s ease (opacity 0->1, translateY(11px)->0) |
| Card switch guard | switchCard ignores a new switch for 350 ms |
| Reduced motion | all animations/transitions disabled via prefers-reduced-motion |
| Reference frame pacing (headless 60 Hz virtual) | p50 16.7 / p95 16.8 / p99 33.2 ms; dropped 1.03% of intervals; CLS 0 |
| Reference frame pacing (headed real 120 Hz compositor) | p50 8.3 / p95 8.6 / p99 13.8 ms; dropped 0.55%; max 99.9 ms single jank |
| Morph duration from rect sampling | headless p50 216.6 / p95 383.4 ms; headed p50 166.7 / p95 341.8 ms (multi-property transitions overlap; CSS declared duration is the source of truth for parity) |
| Reduced motion | 0 dropped frames, morph 0 ms |

Implementation acceptance for the React island: same declared durations/easing for width/height/radius/opacity; one composited content transition per card change; no queued stale transitions; prefers-reduced-motion yields zero animation and no dropped frames; layout shift per transition <= 0.02.

## 5. Harness catalog (reusable, offline)

| Harness | Command | Output |
|---|---|---|
| Filename engine | `cargo run -p ps-bench --release -- filename ...` | docs/performance/results/filename-*.json |
| Content retrieval | `cargo run -p ps-bench --release -- content-v2 ...` | docs/performance/results/content-v2-*.json |
| OCR extraction + retrieval | `cargo run -p ps-bench --release -- ocr-core ... / ocr-baseline ...` | docs/performance/results/ocr-*.json |
| Relevance | `cargo run -p ps-bench --release -- relevance ...` | golden fixtures + results JSON |
| V9 / React frame pacing | `node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/v9-island-cards.json` (or `--cdp` for app) | docs/performance/results/frame-pacing-*.json |
| App debug baseline (CDP) | `node tools/benchmark/app-baseline.mjs` | docs/performance/results/app-baseline-*.json |
| Release hotkey activation (black box) | `node tools/benchmark/release-activation.mjs` | docs/performance/results/release-activation-*.json |
| Canonical real-app E2E (visual QA) | `node tools/visual-qa/e2e-drive.mjs` (Lead/UX-QA) | docs/qa/visual/app-captures/ |

## 6. What needs a shell/UI instrumentation hook (request to Lead)

- A debug-only mark for **selection highlight paint** (T11): e.g. window.__PS_PERF__.mark("selection-paint") after the selected row's paint, and a mark after SearchResponse commit. Without it, input-to-highlight cannot be measured and stays NOT MEASURED.
- A debug-only mark for **launcher first-paint after show** (T5 paint decomposition): the WebView does not emit visibilitychange when the Tauri window is shown from hidden, so page-side paint after hotkey is currently unmeasurable. A Rust-side "window shown at Instant" + a WebView mark on first rAF after show, both debug-only, would let us split Win32 visible vs first paint.
- Both hooks must be build-flag gated and zero-cost off. Until then T5 reports window visible+foreground only, and T6 reports page DOM+double-rAF from input.

## 7. Gate plan

| Gate | Performance deliverable | Blocking evidence |
|---|---|---|
| G0 (now) | baseline JSON for T1-T4, T7 reference, T5/T6/T8/T9 pending current build; gap list below | this plan + results/*.json + BENCHMARKS.md v10 section |
| G1 | React island frame-pacing run on first implemented island build; activation + input->visible on current build | frame-pacing app scenario JSON, app-baseline JSON |
| G4 | Normal Launcher v2 + island parity frame/activation measurements; 120 Hz validation on this host | same harnesses, >=30 takes |
| G5 | OCR corpus scale-up (>=2k real screenshots) + indexed retrieval at scale; content p95 <=30 ms | ocr-baseline + content-v2 JSON |
| G7 | Release-candidate performance report: all rows measured on release build, honest misses documented | BENCHMARKS.md final + results JSON |

## 8. Gate 0 measured baseline snapshot (2026-11-09/10, current tree)

Engine rows are release-profile crates/ps-bench; app rows are the current-tree DEBUG build (release build blocked, see row T5); reference frame-pacing is browser Chromium. Every row is backed by a JSON file in docs/performance/results/.

| Row | Measured Gate 0 | Verdict | Evidence |
|---|---|---|---|
| T1 100k filename p95 | 1.15 ms (contended window; 1.05 ms earlier quiet) | MET (<5 ms) | v10-g0-filename-100k-0e4c9ec-20261109T2037Z.json |
| T2 1M filename p95 | 9.31 ms (build 5.4 s -> contended; 7.23 ms quiet earlier) | MET (<15 ms) | v10-g0-filename-1m-0e4c9ec-20261109T2037Z.json |
| T3 content v2 p95 | 59.1 ms with snippet (43.0 ms earlier quiet); import 26.9 s contended | NOT MET (<30 ms) | v10-g0-content-v2-0e4c9ec-20261109T2037Z.json |
| T4 indexed OCR retrieval p95 | 0.072 ms (81 docs, 90-image synthetic corpus); cold open 0.94 ms, first query 0.37 ms | MET on defined corpus (<30 ms); scale unproven | v10-g0-ocr-baseline-03633c7-20261109T2041Z.json |
| T10 OCR extraction | cold 92 img/s (p50 9.9 ms), warm 115 img/s; English char acc 92.4%, CJK 61.1% (no CJK pack) | baseline recorded; product bar not yet claimed | same OCR JSON |
| T5 launcher activation (DEBUG) | visible p50 7.215 / p95 9.612 / p99 10.818 ms; focused p50 21.266 / p95 30.384 ms (n=20) | release PENDING; debug only | v10-g0-app-debug-4c93d21.json |
| T6 input -> first visible (DEBUG) | p50 14.3 / p95 38.5 / p99 38.6 ms (n=20, 3 warmups) | NOT MET in debug (<30 ms p95) | v10-g0-app-debug-4c93d21.json |
| T7 V9 reference frame pacing | headless 60 Hz virtual: p50 16.7 / p95 16.8 / p99 33.2 ms, dropped 1.03%, CLS 0; headed real 120 Hz: p50 8.3 / p95 8.5 / p99 8.8 ms, dropped 0.16%, over-budget 1.11%, CLS 0; reduced-motion: 0 dropped, morph 0; measured morph p50 166.6-216.6 ms | REFERENCE BASELINE (app island pending) | frame-pacing-v9-reference-{headless,headed,headed120,reduced-motion}.json |
| T8 idle CPU (DEBUG) | 0.18% of machine / 3.65% of one core over 60 s | release PENDING | v10-g0-app-debug-4c93d21.json |
| T9 idle memory (DEBUG) | 694.7 MiB tree WS (60.3 app + 634.4 WebView2 across 8 processes) | release PENDING; debug only | v10-g0-app-debug-4c93d21.json |
| T11 input-to-highlight | not measured | NOT MEASURED (needs shell hook, section 6) | - |
| T12 cold start/activation split | not measured yet | PENDING (in release harness) | - |

Gate 0 blockers (exact): release app build (`cargo build --release -p protonsearch-v2`) failed while engine/sources crates were mid-edit (ps-core duplicate constants, then missing `truncate_chars`; then ps-sources ClipboardEntry initializer missing fields). Two retries failed for reasons outside the perf harness; no release activation number is claimed at publish time. Single 120 Hz/150% monitor means multi-monitor placement stays unmeasured. WebView page paint after show is not observable without a debug-only shell mark (section 6), so T5 currently reports Win32 visible/foreground only.

## 8b. Baseline gap confirmation (Gate 0, honest)

Earlier-v9 gaps from docs/performance/BENCHMARKS.md remain, updated with the v10 Gate-0 runs:

1. **Content retrieval (T3):** still NOT MET - 59.1 ms p95 Gate 0 contended (43.0 ms quiet) vs 30 ms target; legacy v1 FTS5 baseline 6.07 ms p95. Top blocker for "fast search" claims.
2. **OCR search (T4):** now measured end-to-end through ps-ocr + OcrIndex: retrieval itself is far under target on the defined 90-image corpus (0.072 ms p95); the gap is **scale and real-screenshot quality**, not retrieval latency. Extraction English accuracy 92.4% (synthetic) is below a 95% product bar and needs real-corpus validation.
3. **Launcher activation (T5):** debug build (n=20) window visible p50 7.2 / p95 9.6 ms; foreground p50 21.3 / p95 30.4 ms. Release black-box number still PENDING because the release build was blocked by concurrent ps-core edits during Gate 0; paint-after-show is not measurable without the shell hook in section 6.
4. **UI latency (T6):** debug build (n=20, settled engine, real 131,199-entry index): p50 14.3 / p95 38.5 / p99 38.6 ms -> **NOT MET** vs 30 ms p95 in debug; release-build and warm/cold split pending. The slow shapes are repeatable ('readme' 38.5-38.6 ms), so this looks real rather than noise.
5. **Memory/idle CPU (T8/T9):** debug tree 694.7 MiB idle (60.3 app + 634.4 WebView2) and 0.18% of machine idle CPU over 60 s; release-build numbers pending. Debug memory must not be quoted as product memory (renderer overhead/dev build).
6. **Multi-monitor / mixed DPI placement:** blocked - single monitor on this host; only 150% DPI normal-launcher behaviour can be measured.
7. **Input-to-highlight (T11):** NOT MEASURED pending the shell hook in section 6.

## 9. Reproduction (minimum)

    cargo build -p ps-bench --release
    cargo run -p ps-bench --release -- filename --dataset .bench/dataset-100k.jsonl --queries .bench/queries.jsonl --iters 30 --cold-restarts 3 --out docs/performance/results/filename-100k.json
    cargo run -p ps-bench --release -- content-v2 --queries .bench/queries-content.jsonl --iters 30 --cold-restarts 2 --out docs/performance/results/content-v2.json
    cargo run -p ps-bench --release -- ocr-baseline --manifest .bench/ocr-corpus/manifest.jsonl --out docs/performance/results/ocr-baseline.json
    node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/v9-island-cards.json --out docs/performance/results/frame-pacing-v9.json
    cd apps/desktop; npm run build; cd ../..
    cargo build -p protonsearch-v2
    node tools/benchmark/app-baseline.mjs --out docs/performance/results/app-baseline.json
    cargo build --release -p protonsearch-v2
    node tools/benchmark/release-activation.mjs --out docs/performance/results/release-activation.json

All datasets are generated by `ps-bench gen` (deterministic seed 20261109) and validated by content hash; see docs/performance/METHODOLOGY.md for the full protocol and confounds.
