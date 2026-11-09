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
