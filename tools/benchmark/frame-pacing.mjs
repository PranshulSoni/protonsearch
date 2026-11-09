#!/usr/bin/env node
// Reusable frame-pacing harness for the V9 Dynamic Island reference and the future
// React/Native implementation. OWNER: perf-bench (L1-C).
//
// Measures per card-switch transition, plus an exact pooled aggregate:
//   frame intervals => p50/p95/p99/max, FPS estimate
//   dropped frames (interval > 1.5 x refresh period) and over-budget frames
//   layout shift (PerformanceObserver layout-shift, hadRecentInput=false)
//   long tasks overlapping the window
//   measured morph duration from island getBoundingClientRect() samples
//   declared CSS transition properties/durations (computed style evidence)
//
// Drives REAL input through Playwright (keyboard/click), runs fully offline against
// local file:// references by default, and writes JSON with host, browser, mode,
// refresh assumption and per-take evidence under docs/performance/results/.
//
// Usage:
//   node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/v9-island-cards.json
//   node tools/benchmark/frame-pacing.mjs --scenario ... --out ../../docs/performance/results/x.json --headed --hz 60
//   node tools/benchmark/frame-pacing.mjs --scenario tools/benchmark/scenarios/app-island-cards.json --cdp http://127.0.0.1:9222
//
// Playwright is resolved from the already-installed tools/visual-qa/node_modules
// (offline, no extra download). Headless Chromium uses a virtual 60 Hz vsync; the
// mode is always recorded, and --headed gives a real-compositor cross-check.
// Reference-HTML numbers are not product claims; app numbers must come from the
// same harness with --cdp against a real build.

import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, resolve, isAbsolute } from "node:path";
import { existsSync, mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { hostname, cpus, totalmem, release } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");

function utcTag() {
  return new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "") + "Z";
}

function loadPlaywright() {
  const candidates = [process.env.PS_PLAYWRIGHT_DIR, resolve(repoRoot, "tools", "visual-qa"), __dirname].filter(Boolean);
  for (const dir of candidates) {
    try {
      const req = createRequire(resolve(dir, "package.json"));
      const pw = req("playwright");
      if (pw && pw.chromium) return { pw, from: dir };
    } catch { /* next */ }
  }
  throw new Error("playwright not found. Expected tools/visual-qa/node_modules/playwright (already installed) or PS_PLAYWRIGHT_DIR.");
}

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith("--")) continue;
    const key = a.slice(2);
    const next = argv[i + 1];
    if (next !== undefined && !next.startsWith("--")) { out[key] = next; i++; } else { out[key] = true; }
  }
  return out;
}

function percentile(sorted, p) {
  if (!sorted.length) return 0;
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, Math.min(sorted.length - 1, idx))];
}

function summarize(values) {
  if (!values.length) return { count: 0, p50: 0, p95: 0, p99: 0, max: 0, mean: 0 };
  const s = [...values].sort((a, b) => a - b);
  return {
    count: s.length,
    p50: +percentile(s, 50).toFixed(3),
    p95: +percentile(s, 95).toFixed(3),
    p99: +percentile(s, 99).toFixed(3),
    max: +s[s.length - 1].toFixed(3),
    mean: +(s.reduce((a, b) => a + b, 0) / s.length).toFixed(3),
  };
}

function deriveMorph(rects, t0) {
  const changed = rects.filter((r) => r.t >= t0 - 1);
  if (changed.length < 2) return { measuredMs: 0, settleAfterTriggerMs: 0, changes: 0, start: changed[0] || null, end: changed[changed.length - 1] || null };
  const first = changed[0];
  let firstChange = null;
  let lastChange = null;
  let changes = 0;
  for (let i = 1; i < changed.length; i++) {
    const prev = changed[i - 1];
    const cur = changed[i];
    const differs = Math.abs(cur.w - prev.w) > 0.5 || Math.abs(cur.h - prev.h) > 0.5;
    if (differs) {
      changes++;
      if (firstChange === null) firstChange = cur.t;
      lastChange = cur.t;
    }
  }
  return {
    measuredMs: firstChange === null ? 0 : +(lastChange - firstChange).toFixed(1),
    settleAfterTriggerMs: firstChange === null ? 0 : +(lastChange - first.t).toFixed(1),
    changes,
    start: first,
    end: changed[changed.length - 1],
  };
}

async function installInstrumentation(page, selector, contentSelector) {
  return await page.evaluate((cfg) => {
    if (window.__FRAME_PACING__) {
      window.__FRAME_PACING__.selector = cfg.selector;
      window.__FRAME_PACING__.contentSelector = cfg.contentSelector;
      return "already-installed";
    }
    const st = {
      frames: [], rects: [], shifts: [], longtasks: [], active: false, t0: 0, raf: 0,
      selector: cfg.selector, contentSelector: cfg.contentSelector,
    };
    const tick = (t) => {
      if (!st.active) return;
      st.frames.push(t);
      const el = document.querySelector(st.selector || "#island");
      if (el) {
        const r = el.getBoundingClientRect();
        st.rects.push({ t, w: +r.width.toFixed(2), h: +r.height.toFixed(2), x: +r.x.toFixed(2), y: +r.y.toFixed(2) });
      }
      st.raf = requestAnimationFrame(tick);
    };
    st.start = () => {
      st.frames = []; st.rects = []; st.shifts = []; st.longtasks = [];
      st.active = true; st.t0 = performance.now();
      st.raf = requestAnimationFrame(tick);
      return st.t0;
    };
    st.stop = () => {
      st.active = false;
      if (st.raf) cancelAnimationFrame(st.raf);
      const el = document.querySelector(st.selector || "#island");
      const cs = el ? getComputedStyle(el) : null;
      return {
        t0: st.t0,
        frames: st.frames.slice(),
        rects: st.rects.slice(),
        shifts: st.shifts.slice(),
        longtasks: st.longtasks.slice(),
        heading: document.querySelector(st.contentSelector || "#panel h2") ? document.querySelector(st.contentSelector || "#panel h2").textContent.trim() : null,
        css: cs ? {
          transitionProperty: cs.transitionProperty,
          transitionDuration: cs.transitionDuration,
          transitionTimingFunction: cs.transitionTimingFunction,
        } : null,
      };
    };
    try {
      const po = new PerformanceObserver((list) => {
        for (const e of list.getEntries()) if (e.entryType === "layout-shift" && !e.hadRecentInput) st.shifts.push({ t: e.startTime, value: e.value });
      });
      po.observe({ type: "layout-shift", buffered: false });
      st.poShift = po;
    } catch { /* unsupported */ }
    try {
      const po = new PerformanceObserver((list) => {
        for (const e of list.getEntries()) st.longtasks.push({ start: e.startTime, duration: e.duration });
      });
      po.observe({ type: "longtask", buffered: false });
      st.poLong = po;
    } catch { /* unsupported */ }
    window.__FRAME_PACING__ = st;
    return "installed";
  }, { selector, contentSelector });
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) { console.log("see header of tools/benchmark/frame-pacing.mjs"); return; }

  const scenarioPath = args.scenario
    ? (isAbsolute(args.scenario) ? args.scenario : resolve(process.cwd(), args.scenario))
    : resolve(__dirname, "scenarios", "v9-island-cards.json");
  if (!existsSync(scenarioPath)) throw new Error("scenario not found: " + scenarioPath);
  const scenario = JSON.parse(readFileSync(scenarioPath, "utf8"));
  const scenarioDir = dirname(scenarioPath);

  const hz = Number(args.hz || 60);
  const periodMs = 1000 / hz;
  const headed = Boolean(args["headed"]);
  const reducedMotion = Boolean(args["reduced-motion"]);
  const measureMs = Number(args["measure-ms"] || scenario.measureMs || 650);
  const repeats = Number(args.repeats || scenario.repeats || 24);
  const warmup = Number(args.warmup || scenario.warmup || 3);
  const cdpEndpoint = args.cdp || null;
  const outPath = args.out
    ? (isAbsolute(args.out) ? args.out : resolve(process.cwd(), args.out))
    : resolve(repoRoot, "docs", "performance", "results", "frame-pacing-" + (scenario.label || "run") + "-" + utcTag() + ".json");

  function resolveUrl(kind, url) {
    if (kind === "url") return url;
    if (url.startsWith("file:")) return url;
    const p = isAbsolute(url) ? url : resolve(scenarioDir, url);
    if (!existsSync(p)) throw new Error("target file not found: " + p);
    return pathToFileURL(p).href;
  }

  const { pw, from } = loadPlaywright();
  let browser;
  let page;
  let closeBrowser = true;

  if (cdpEndpoint) {
    browser = await pw.chromium.connectOverCDP(cdpEndpoint);
    closeBrowser = false;
    const pages = browser.contexts().flatMap((c) => c.pages());
    if (!pages.length) throw new Error("no pages at CDP endpoint " + cdpEndpoint);
    if (scenario.pageWindowKind) {
      for (const candidate of pages) {
        const kind = await candidate.evaluate(() => document.documentElement.dataset.window || null).catch(() => null);
        if (kind === scenario.pageWindowKind) { page = candidate; break; }
      }
    }
    if (!page) {
      const wanted = scenario.pageUrlIncludes;
      page = (wanted && pages.find((p) => p.url().includes(wanted))) || pages[0];
    }
  } else {
    browser = await pw.chromium.launch({
      headless: !headed,
      args: ["--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--disable-features=CalculateNativeWinOcclusion"],
    });
    const context = await browser.newContext({
      viewport: scenario.viewport || { width: 1440, height: 900 },
      deviceScaleFactor: scenario.deviceScaleFactor || 1,
      reducedMotion: reducedMotion ? "reduce" : "no-preference",
    });
    page = await context.newPage();
    const targetUrl = resolveUrl(scenario.kind || "file", scenario.url);
    await page.goto(targetUrl, { waitUntil: "load" });
  }

  const externalRequests = [];
  page.on("request", (req) => {
    const u = req.url();
    if (!u.startsWith("file:") && !u.startsWith("data:") && !u.startsWith("blob:")) externalRequests.push(u);
  });

  if (scenario.setup) {
    if (scenario.setup.startsWith("click:")) await page.locator(scenario.setup.slice(6)).first().click({ timeout: 8000 });
    else if (scenario.setup.startsWith("press:")) await page.keyboard.press(scenario.setup.slice(6));
    else await page.evaluate(scenario.setup);
    await page.waitForTimeout(350);
  }

  const installed = await installInstrumentation(page, scenario.transitionSelector || "#island", scenario.contentSelector || "#panel h2");

  for (let i = 0; i < warmup; i++) {
    await page.keyboard.press(scenario.switchKey || "Control+ArrowDown");
    await page.waitForTimeout(measureMs);
  }

  const takes = [];
  const allIntervals = [];
  for (let i = 0; i < repeats; i++) {
    const t0 = await page.evaluate(() => window.__FRAME_PACING__.start());
    await page.keyboard.press(scenario.switchKey || "Control+ArrowDown");
    await page.waitForTimeout(measureMs);
    const raw = await page.evaluate(() => window.__FRAME_PACING__.stop());
    const intervals = [];
    for (let k = 1; k < raw.frames.length; k++) intervals.push(+(raw.frames[k] - raw.frames[k - 1]).toFixed(3));
    allIntervals.push(...intervals);
    const dropped = intervals.filter((x) => x > periodMs * 1.5).length;
    const overBudget = intervals.filter((x) => x > periodMs * 1.05).length;
    const morph = deriveMorph(raw.rects, raw.t0);
    const shifts = raw.shifts.filter((s) => s.t >= raw.t0);
    const longtasks = (raw.longtasks || []).filter((lt) => lt.start + lt.duration >= raw.t0);
    takes.push({
      take: i,
      heading: raw.heading,
      t0: +raw.t0.toFixed(2),
      frameCount: raw.frames.length,
      windowMs: measureMs,
      fpsEstimate: +((raw.frames.length / measureMs) * 1000).toFixed(1),
      frameMs: summarize(intervals),
      intervalsMs: intervals,
      droppedFrames: dropped,
      droppedPct: intervals.length ? +((dropped / intervals.length) * 100).toFixed(2) : 0,
      overBudgetFrames: overBudget,
      overBudgetPct: intervals.length ? +((overBudget / intervals.length) * 100).toFixed(2) : 0,
      layoutShift: { count: shifts.length, cls: +shifts.reduce((a, s) => a + s.value, 0).toFixed(4) },
      longTasks: { count: longtasks.length, totalMs: +longtasks.reduce((a, l) => a + l.duration, 0).toFixed(1) },
      morph,
      css: raw.css,
    });
    await page.waitForTimeout(120);
  }

  const aggregate = {
    takes: takes.length,
    framesTotal: allIntervals.length,
    frameMs: summarize(allIntervals),
    droppedFramesTotal: takes.reduce((a, t) => a + t.droppedFrames, 0),
    overBudgetFramesTotal: takes.reduce((a, t) => a + t.overBudgetFrames, 0),
    layoutShiftTotal: +takes.reduce((a, t) => a + t.layoutShift.cls, 0).toFixed(4),
    longTasksTotal: takes.reduce((a, t) => a + t.longTasks.count, 0),
    longTasksMsTotal: +takes.reduce((a, t) => a + t.longTasks.totalMs, 0).toFixed(1),
    morphMs: summarize(takes.map((t) => t.morph.measuredMs).filter((x) => x > 0)),
  };
  aggregate.droppedPct = aggregate.framesTotal ? +((aggregate.droppedFramesTotal / aggregate.framesTotal) * 100).toFixed(2) : 0;
  aggregate.overBudgetPct = aggregate.framesTotal ? +((aggregate.overBudgetFramesTotal / aggregate.framesTotal) * 100).toFixed(2) : 0;

  const report = {
    schema: "ps-frame-pacing/1",
    kind: "frame-pacing",
    scenario: {
      name: scenario.name,
      label: scenario.label,
      target: cdpEndpoint ? cdpEndpoint : scenario.url,
      kind: cdpEndpoint ? "cdp" : (scenario.kind || "file"),
      viewport: scenario.viewport,
      deviceScaleFactor: scenario.deviceScaleFactor || 1,
      switchKey: scenario.switchKey,
      setup: scenario.setup,
    },
    refresh: { assumedHz: hz, periodMs: +periodMs.toFixed(3), note: "rAF frame intervals; dropped = interval > 1.5 x refresh period; over-budget = > 1.05 x period" },
    mode: cdpEndpoint ? "cdp-attached" : (headed ? "headed-chromium" : "headless-chromium"),
    reducedMotion,
    browser: { playwrightFrom: from, version: browser.version(), headless: cdpEndpoint ? null : !headed },
    instrumentation: installed,
    host: { hostname: hostname(), platform: process.platform, osRelease: release(), cpus: cpus().length, totalMemBytes: totalmem() },
    capturedAt: new Date().toISOString(),
    externalRequests,
    aggregate,
    takes,
    notes: [
      "V9 reference HTML is a UI-only simulation; these numbers measure the reference's browser rendering, not native Windows composition.",
      "Headless Chromium uses a virtual 60 Hz vsync; use --headed and/or a real display for compositor cross-checks; mode is recorded.",
      "Dropped frame = frame interval > 1.5 x assumed refresh period; over-budget = > 1.05 x period.",
      "Morph duration is derived from island getBoundingClientRect() samples per frame; CSS declared durations are captured for cross-check.",
      "Layout shift counts layout-shift entries without recent input inside the measurement window.",
    ],
  };

  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(report, null, 2));

  console.log("frame pacing: scenario=" + report.scenario.name + " mode=" + report.mode + " hz=" + hz);
  console.log("  takes=" + aggregate.takes + " frames=" + aggregate.framesTotal);
  console.log("  frame interval: p50=" + aggregate.frameMs.p50 + "ms p95=" + aggregate.frameMs.p95 + "ms p99=" + aggregate.frameMs.p99 + "ms max=" + aggregate.frameMs.max + "ms");
  console.log("  dropped=" + aggregate.droppedFramesTotal + " (" + aggregate.droppedPct + "%) over-budget=" + aggregate.overBudgetPct + "%");
  console.log("  layoutShift=" + aggregate.layoutShiftTotal + " longTasks=" + aggregate.longTasksTotal + " longTasksMs=" + aggregate.longTasksMsTotal);
  console.log("  morph: p50=" + aggregate.morphMs.p50 + "ms p95=" + aggregate.morphMs.p95 + "ms");
  console.log("  results: " + outPath);

  if (closeBrowser && browser) await browser.close().catch(() => {});
  return report;
}

run().catch((err) => {
  console.error("frame-pacing: " + (err && err.stack ? err.stack : err));
  process.exit(1);
});
