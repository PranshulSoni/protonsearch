# tools/benchmark — reusable performance harnesses

Owner: perf-bench (L1-C). These tools produce the raw JSON behind
docs/performance/BENCHMARKS.md and docs/performance/V10_BENCHMARK_PLAN.md.

## frame-pacing.mjs

Frame-pacing harness for the V9 Dynamic Island reference and the future React
implementation. Drives real input through Playwright, measures p50/p95/p99 frame
intervals, dropped frames, over-budget frames, layout shift, long tasks, and
measured morph duration. Runs offline against local file:// references by
default; supports attaching to a live CDP endpoint for real-app runs.

    node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/v9-island-cards.json
    node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/v9-island-cards.json --headed
    node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/app-island-cards.json --cdp http://127.0.0.1:9222

Playwright is reused from tools/visual-qa/node_modules (already installed; no
download). Writes JSON to docs/performance/results/ unless --out is given.

Caveats recorded in every JSON:
- headless Chromium uses a virtual 60 Hz vsync; --headed gives a real-compositor
  cross-check and the mode is always recorded;
- V9 reference numbers are browser-rendering targets only, not native Windows
  claims;
- dropped frame = interval > 1.5 x assumed refresh period; over-budget = > 1.05 x;
- layout shift uses PerformanceObserver layout-shift entries without recent input.

## scenarios/

- v9-island-cards.json: V9 reference card-switch transitions (Ctrl+ArrowDown).
- app-island-cards.json: template for the future React Dynamic Island (CDP attach).

## island-perf.mjs

Island-specific performance harness for task-57. Launches the debug app with CDP
and perf hooks (PROTONSEARCH_PERF_HOOKS=1), then measures:

- idle process-tree memory with only the primary launcher prewarmed (lazy main/alternate windows) with per-process WS, private bytes and command lines;
- WebView2 memory-target trim: hide the launcher/island, wait --trim-wait-ms, compare per-process WS (process retained = warm trim);
- activation trade-off: first lazy island open vs warm opens, using the real global hotkey (SendInput) plus island-window-shown / island-first-paint marks;
- open/close frame pacing (rAF intervals, p50/p95/p99, dropped/over-budget, long tasks, layout shift, page resize events, unique surface sizes);
- typing -> results latency inside the island search card;
- card-switch latency plus root-cause attribution from CDP Performance durations (Task/Script/Layout/Recalc) and page resize-event counts (native resize storm vs React);
- 20x open/close leak check with memory snapshots every 5 cycles.

Usage:

    node tools/benchmark/island-perf.mjs --exe target/debug/ProtonSearch.exe --out docs/performance/results/island-perf.json
    node tools/benchmark/island-perf.mjs --attach-only --port 9222 --no-serve ...

Requirements/caveats: debug build with CDP (WEBVIEW2 CDP is debug-only), vite preview on 5173 for the debug devUrl, PROTONSEARCH_PERF_HOOKS=1 marks in <data_dir>/v2/perf-marks.jsonl, single-instance app (the harness kills stale ProtonSearch processes before launch and leaves none running). Assumed refresh 120 Hz by default (--hz); per-trial percentiles are pooled as a mean of per-trial percentiles, so aggregate p95 is an estimate - per-trial rows are kept in the JSON.

Raw JSON includes every sample so a reviewer can re-derive the numbers.
### island-hotkey.mjs

Companion harness for the island-primary profile: start the app with `launcher.primaryMode=dynamic_island` (only the island window prewarmed, hidden), then measure the real global-hotkey path (`island-window-shown` -> `island-first-paint`, Win32 visible) and the island-primary memory profile (prewarmed hidden / visible / hidden+trim, per-process).

    node tools/benchmark/island-hotkey.mjs --exe target/release/ProtonSearch.exe --data-dir .devdata --out docs/performance/results/island-hotkey.json

Why two profiles: switching primary_mode at runtime through `update_settings` currently exits the app (it destroys the inactive window; destroying the last window exits Tauri), so island-perf.mjs keeps the normal profile and measures lazy WebView creation through the `show_island` command, while island-hotkey.mjs covers the hotkey path from a fresh island-primary launch. Both truncate perf marks per session and leave no app running.

### island-switch.mjs (+ island-probe.ps1)

Release-native (no CDP) black-box card-switch probe for before/after builds: summons the island, sends Ctrl+Down switches, and samples UI-thread WM_NULL latency, rect and pixel changes from outside, plus in-app frame and ui-stall marks (pixel rate cannot resolve 120 Hz frames; use the frame marks for pacing).

    node tools/benchmark/island-switch.mjs --exe target/release/ProtonSearch.exe --data-dir .devdata --out docs/performance/results/island-switch.json --label baseline
