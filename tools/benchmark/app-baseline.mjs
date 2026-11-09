#!/usr/bin/env node
// App-level Gate 0 baseline for ProtonSearch v10: launcher activation, input to
// first visible result, engine IPC search latency, process-tree memory and idle
// CPU. OWNER: perf-bench (L1-C).
//
// Uses the real Tauri/WebView2 debug app over CDP (same launch recipe as
// tools/visual-qa/e2e-drive.mjs) and writes JSON under docs/performance/results/.
//
// Measurements:
//   1. input -> first visible result: performance.now before a native input event
//      -> double rAF after result rows change (>=20 takes).
//   2. engine IPC: invoke("search") round trip + Rust SearchResponse.tookMs.
//   3. launcher activation: hide launcher via IPC, send the real global hotkey
//      Alt+Space (SendInput helper), Win32 visible/focused timestamps + page
//      visibility/focus/paint deltas. Reported separately (window vs paint).
//   4. process-tree memory (ProtonSearch.exe + msedgewebview2 children): idle and
//      with launcher visible.
//   5. idle CPU over a sample window with launcher hidden.
//
// Conditions recorded in JSON: exe size/mtime, git HEAD + dirty flag, dist mtime,
// devicePixelRatio, data dir, index entries, takes, and any blocked step.
// This is a debug build baseline; release-build comparison remains pending.

import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync, mkdirSync, writeFileSync, statSync, readFileSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");

function loadPlaywright() {
  const candidates = [process.env.PS_PLAYWRIGHT_DIR, resolve(repoRoot, "tools", "visual-qa"), __dirname].filter(Boolean);
  for (const dir of candidates) {
    try {
      const req = createRequire(resolve(dir, "package.json"));
      const pw = req("playwright");
      if (pw && pw.chromium) return pw;
    } catch { /* next */ }
  }
  throw new Error("playwright not found (expected tools/visual-qa/node_modules/playwright)");
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

function utcTag() {
  return new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "") + "Z";
}
function now() { return new Date().toISOString(); }
function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }
const args = parseArgs(process.argv.slice(2));
if (args.help) { console.log("see header of tools/benchmark/app-baseline.mjs"); process.exit(0); }

const exe = resolve(args.exe || resolve(repoRoot, "target", "debug", "ProtonSearch.exe"));
const outPath = resolve(args.out || resolve(repoRoot, "docs", "performance", "results", "app-baseline-" + utcTag() + ".json"));
const dataDir = resolve(args["data-dir"] || resolve(repoRoot, ".devdata"));
const port = Number(args.port || 9222);
const takesHotkey = Number(args["hotkey-takes"] || 10);
const takesInput = Number(args["input-takes"] || 20);
const idleSeconds = Number(args["idle-seconds"] || 10);
const skipLaunch = Boolean(args["attach-only"]);
const noServe = Boolean(args["no-serve"]);
const exeArgs = (args["exe-args"] || "").split(" ").filter(Boolean);

const HOTKEY_PS = resolve(__dirname, "hotkey-activate.ps1");
const METRICS_PS = resolve(__dirname, "process-metrics.ps1");
const QUERIES = ["arch", "readme", "proton", "index", "report", "screenshot", "会議", "café", "zzzz-no-match", "pdf"];
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");
let marksOffset = 0;

function truncateMarks() {
  try { writeFileSync(MARKS_FILE, ""); marksOffset = 0; } catch { /* ignore */ }
}

// Read perf-marks.jsonl entries appended since the previous call. Written by the
// Rust shell when PROTONSEARCH_PERF_HOOKS=1: launcher-window-shown (Rust epoch at
// show), launcher-first-paint (frontend double-rAF after perf:show), and
// launcher-selection-painted (frontend one-rAF after ArrowUp/Down/Enter).
function readMarksSince() {
  let text = "";
  try { text = readFileSync(MARKS_FILE, "utf8"); } catch { return { marks: [], offset: marksOffset }; }
  const buf = Buffer.from(text);
  if (buf.length < marksOffset) marksOffset = 0;
  const slice = buf.slice(marksOffset).toString("utf8");
  const marks = [];
  for (const line of slice.split(/\r?\n/)) {
    if (!line.trim()) continue;
    try { marks.push(JSON.parse(line)); } catch { /* partial line */ }
  }
  marksOffset = buf.length;
  return { marks, offset: marksOffset };
}
const results = {
  schema: "ps-app-baseline/1",
  kind: "app-launcher-baseline",
  status: "running",
  capturedAt: now(),
  host: { hostname: hostname(), platform: process.platform },
  conditions: {},
  measurements: {},
  blocked: [],
  limitations: [
    "Debug Rust build + production frontend via vite preview; release-build comparison pending.",
    "Windows display used for hotkey activation test; DPI/refresh recorded per run.",
    "Other agents may be active on the machine; build/CPU contention is not controlled.",
    "Playwright/CDP only measures the WebView page; native window visibility comes from Win32 polling in PowerShell.",
    "Perf marks are opt-in (PROTONSEARCH_PERF_HOOKS=1) and written to <data_dir>/perf-marks.jsonl: launcher-window-shown (Rust), launcher-first-paint and launcher-selection-painted (frontend, via perf_mark). Epochs are same-machine wall clock (ms resolution).",
  ],
};

let app = null;
let server = null;
let browser = null;

function runPs(script, scriptArgs, timeoutMs) {
  return new Promise((resolvePromise, reject) => {
    execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script, ...scriptArgs], { windowsHide: true, timeout: timeoutMs || 60000 }, (error, stdout, stderr) => {
      if (error) return reject(new Error("PS helper failed: " + (stderr || error.message)));
      const line = String(stdout).trim().split(/\r?\n/).filter(Boolean).pop();
      try { resolvePromise(JSON.parse(line)); } catch (e) { reject(new Error("bad PS JSON: " + line + " :: " + e.message)); }
    });
  });
}

function killTree(pid) {
  if (!pid) return;
  try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* gone */ }
}

async function waitForFrontend() {
  const deadline = Date.now() + 40000;
  while (Date.now() < deadline) {
    try {
      const res = await fetch("http://127.0.0.1:5173/index.html");
      if (res.ok) return true;
    } catch { /* retry */ }
    await sleep(400);
  }
  return false;
}

async function connectCdp(pw) {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) {
    try {
      const res = await fetch("http://127.0.0.1:" + port + "/json/version");
      if (res.ok) return await pw.chromium.connectOverCDP("http://127.0.0.1:" + port);
    } catch { /* retry */ }
    await sleep(500);
  }
  throw new Error("could not connect to WebView2 CDP on port " + port);
}

async function findPages(browser) {
  const pages = browser.contexts().flatMap((c) => c.pages());
  const kinds = [];
  for (const page of pages) {
    let kind = null;
    try { kind = await page.evaluate(() => document.documentElement.dataset.window || null); } catch { /* navigating */ }
    kinds.push({ page, kind, url: page.url() });
  }
  return kinds;
}

function percentile(sorted, p) {
  if (!sorted.length) return 0;
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, Math.min(sorted.length - 1, idx))];
}
function summarize(values) {
  if (!values.length) return { count: 0, p50: null, p95: null, p99: null, min: null, max: null, mean: null };
  const s = [...values].sort((a, b) => a - b);
  return {
    count: s.length,
    p50: +percentile(s, 50).toFixed(3),
    p95: +percentile(s, 95).toFixed(3),
    p99: +percentile(s, 99).toFixed(3),
    min: +s[0].toFixed(3),
    max: +s[s.length - 1].toFixed(3),
    mean: +(s.reduce((a, b) => a + b, 0) / s.length).toFixed(3),
  };
}

async function waitIndexReady(launcher) {
  return await launcher.evaluate(async () => {
    const deadline = Date.now() + 180000;
    while (Date.now() < deadline) {
      try {
        const status = await window.__TAURI_INTERNALS__.invoke("get_status");
        if (status && (status.state === "ready" || status.state === "paused") && status.filenameEntries > 0) {
          return { state: status.state, entries: status.filenameEntries, contentDocuments: status.contentDocuments };
        }
      } catch { /* engine starting */ }
      await new Promise((r) => setTimeout(r, 500));
    }
    return { state: "timeout", entries: 0 };
  }).catch(() => ({ state: "unavailable", entries: 0 }));
}

// Wrap the Tauri invoke surface so the in-app search call made by a real input
// event can be timed: engine Wall = IPC + engine; tookMs = engine-only (from the
// response); the remainder of input->visible is React commit + paint.
async function installSearchTimingHook(launcher) {
  return await launcher.evaluate(() => {
    if (window.__SEARCH_TIMING__) return "already-installed";
    const internals = window.__TAURI_INTERNALS__;
    if (!internals || typeof internals.invoke !== "function") return "no-tauri-internals";
    const original = internals.invoke.bind(internals);
    const state = { last: null };
    internals.invoke = function (cmd, payload) {
      if (cmd !== "search") return original(cmd, payload);
      const started = performance.now();
      return original(cmd, payload).then(
        (response) => {
          state.last = {
            wallMs: performance.now() - started,
            tookMs: response && typeof response.tookMs === "number" ? response.tookMs : null,
            requestId: response ? response.requestId : null,
          };
          return response;
        },
        (error) => { state.last = { wallMs: performance.now() - started, error: String(error) }; throw error; },
      );
    };
    window.__SEARCH_TIMING__ = state;
    return "installed";
  }).catch((error) => "hook-error: " + String(error));
}

async function measureInputToVisible(launcher, takes, warmup) {
  const samples = [];
  for (let i = 0; i < warmup + takes; i++) {
    const query = QUERIES[i % QUERIES.length];
    const r = await launcher.evaluate(async (q) => {
      const input = document.querySelector("input.search-input") || document.querySelector("input");
      if (!input) return { error: "no input" };
      const listSelector = '[role="option"], .result, [role="listbox"] [aria-selected]';
      const signature = () => {
        const rows = document.querySelectorAll(listSelector);
        const first = rows[0] ? rows[0].textContent : "";
        const count = document.querySelector(".count")?.textContent || "";
        const empty = document.querySelector(".launcher-empty") ? "empty" : "rows";
        return count + "|" + rows.length + "|" + first + "|" + empty;
      };
      input.focus();
      const baseline = signature();
      const start = performance.now();
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
      setter.call(input, q);
      input.dispatchEvent(new Event("input", { bubbles: true }));
      const deadline = performance.now() + 4000;
      while (performance.now() < deadline) {
        if (signature() !== baseline) {
          await new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res)));
          const timing = window.__SEARCH_TIMING__ ? window.__SEARCH_TIMING__.last : null;
          return {
            start,
            visibleAt: performance.now(),
            rows: document.querySelectorAll(listSelector).length,
            engineWallMs: timing && timing.wallMs != null ? timing.wallMs : null,
            engineTookMs: timing && timing.tookMs != null ? timing.tookMs : null,
          };
        }
        await new Promise((res) => setTimeout(res, 4));
      }
      return { start, visibleAt: null, rows: document.querySelectorAll(listSelector).length };
    }, query);
    samples.push({ take: i, warmup: i < warmup, query, ...r, ms: r.visibleAt != null ? +(r.visibleAt - r.start).toFixed(2) : null });
  }
  const measuredRows = samples.filter((s) => !s.warmup && s.ms != null);
  const measured = measuredRows.map((s) => s.ms);
  const engineWall = measuredRows.map((s) => s.engineWallMs).filter((v) => v != null);
  const engineTook = measuredRows.map((s) => s.engineTookMs).filter((v) => v != null);
  const ipcOverhead = measuredRows.filter((s) => s.engineWallMs != null && s.engineTookMs != null).map((s) => s.engineWallMs - s.engineTookMs);
  const uiAfterEngine = measuredRows.filter((s) => s.ms != null && s.engineWallMs != null).map((s) => s.ms - s.engineWallMs);
  return {
    takes,
    warmup,
    measured: measured.length,
    ms: summarize(measured),
    split: {
      engineWallMs: summarize(engineWall),
      engineTookMs: summarize(engineTook),
      ipcOverheadMs: summarize(ipcOverhead),
      postEngineUiMs: summarize(uiAfterEngine),
      note: "engineWall = Tauri invoke wall for the app's own search call (IPC + engine); engineTook = Rust SearchResponse.took_ms; ipcOverhead = engineWall - engineTook; postEngineUi = input->visible DOM paint - engineWall (React commit + paint + observer/polling).",
    },
    samples,
  };
}

async function measureEngineIpc(launcher, takes) {
  const samples = [];
  for (let i = 0; i < takes; i++) {
    const query = QUERIES[i % QUERIES.length];
    const r = await launcher.evaluate(async (q) => {
      const started = performance.now();
      const req = { protocol: 1, requestId: Date.now(), query: q, scope: "all", filters: {}, limit: 50 };
      const resp = await window.__TAURI_INTERNALS__.invoke("search", { req });
      return {
        wallMs: performance.now() - started,
        tookMs: resp && resp.tookMs != null ? resp.tookMs : null,
        results: resp && resp.results ? resp.results.length : null,
        state: resp && resp.status ? resp.status.state : null,
      };
    }, query);
    samples.push({ take: i, query, ...r });
  }
  return {
    takes: samples.length,
    wallMs: summarize(samples.map((s) => s.wallMs)),
    engineTookMs: summarize(samples.filter((s) => s.tookMs != null).map((s) => s.tookMs)),
    samples,
  };
}

async function installActivationMarks(launcher) {
  await launcher.evaluate(() => {
    if (window.__ACT__ && window.__ACT__.installed) { window.__ACT__.reset(); return "reset"; }
    const st = { installed: true, marks: [] };
    st.reset = () => { st.marks = []; };
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible") {
        st.marks.push({ name: "visible", t: performance.now() });
        requestAnimationFrame(() => requestAnimationFrame(() => st.marks.push({ name: "paint", t: performance.now() })));
      }
    });
    window.addEventListener("focus", () => st.marks.push({ name: "focus", t: performance.now() }));
    window.__ACT__ = st;
    return "installed";
  });
}

async function measureHotkey(launcher, appPid, takes) {
  const out = [];
  for (let i = 0; i < takes; i++) {
    try { await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_launcher")); } catch { /* ignore */ }
    await sleep(700);
    await installActivationMarks(launcher);
    let win = null;
    try {
      win = await runPs(HOTKEY_PS, ["-ProcessId", String(appPid), "-TimeoutMs", "3000"], 15000);
    } catch (e) {
      out.push({ take: i, error: String(e.message) });
      continue;
    }
    await sleep(120);
    const since = readMarksSince();
    const shownEpoch = since.marks.filter((m) => m.label === "launcher-window-shown" && m.epochMs >= (win.sendAtUtc ?? 0) - 300).map((m) => m.epochMs).sort((a, b) => a - b).slice(-1)[0] ?? null;
    const paintEpoch = since.marks.filter((m) => m.label === "launcher-first-paint" && (shownEpoch == null || m.epochMs >= shownEpoch - 50)).map((m) => m.epochMs).sort((a, b) => a - b).slice(-1)[0] ?? null;
    const markDeltas = {
      sendToShownMs: shownEpoch != null ? +(shownEpoch - win.sendAtUtc).toFixed(2) : null,
      shownToPaintMs: shownEpoch != null && paintEpoch != null ? +(paintEpoch - shownEpoch).toFixed(2) : null,
      sendToPaintMs: paintEpoch != null ? +(paintEpoch - win.sendAtUtc).toFixed(2) : null,
      labels: since.marks.map((m) => m.label),
    };
    const pageMarks = await launcher.evaluate(() => {
      const st = window.__ACT__;
      const visible = st.marks.find((m) => m.name === "visible");
      const paint = st.marks.find((m) => m.name === "paint");
      const focus = st.marks.find((m) => m.name === "focus");
      return {
        visible: visible ? +visible.t.toFixed(2) : null,
        paint: paint ? +paint.t.toFixed(2) : null,
        focus: focus ? +focus.t.toFixed(2) : null,
        paintAfterVisibleMs: visible && paint ? +(paint.t - visible.t).toFixed(2) : null,
        marks: st ? st.marks : [],
      };
    }).catch(() => null);
    out.push({
      take: i,
      win32: win,
      page: pageMarks,
      markDeltas,
      activationMs: win && win.visibleMs != null ? win.visibleMs : null,
      focusMs: win && win.focusedMs != null ? win.focusedMs : null,
      paintTotalMs: win && win.visibleMs != null && pageMarks && pageMarks.paintAfterVisibleMs != null
        ? +(win.visibleMs + pageMarks.paintAfterVisibleMs).toFixed(2)
        : null,
    });
    try { await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_launcher")); } catch { /* ignore */ }
    await sleep(300);
  }
  const activation = out.map((o) => o.activationMs).filter((v) => v != null);
  const focus = out.map((o) => o.focusMs).filter((v) => v != null);
  const paint = out.map((o) => o.paintTotalMs).filter((v) => v != null);
  const sendToShown = out.map((o) => o.markDeltas?.sendToShownMs).filter((v) => v != null);
  const shownToPaint = out.map((o) => o.markDeltas?.shownToPaintMs).filter((v) => v != null);
  const sendToPaint = out.map((o) => o.markDeltas?.sendToPaintMs).filter((v) => v != null);
  return {
    takes: takes,
    measured: activation.length,
    visibleMs: summarize(activation),
    focusedMs: summarize(focus),
    paintTotalMs: summarize(paint),
    marks: {
      measuredSendToShown: sendToShown.length,
      measuredShownToPaint: shownToPaint.length,
      measuredSendToPaint: sendToPaint.length,
      sendToShownMs: summarize(sendToShown),
      shownToPaintMs: summarize(shownToPaint),
      sendToPaintMs: summarize(sendToPaint),
      note: "Rust launcher-window-shown (SystemTime epoch) -> frontend launcher-first-paint (double rAF epoch). t0 is the PowerShell SendInput timestamp on the same wall clock (ms resolution).",
    },
    samples: out,
  };
}

// Input -> selection-highlight paint, using the debug-only frontend mark
// "launcher-selection-painted" (fired one rAF after ArrowUp/Down/Enter when
// PROTONSEARCH_PERF_HOOKS=1). t0 is Date.now() in the page immediately before
// Playwright dispatches the key; the mark epoch is also Date.now(), same clock.
async function measureSelection(launcher, takes) {
  try { await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher")); } catch { /* ignore */ }
  await sleep(300);
  const seeded = await launcher.evaluate(() => {
    const input = document.querySelector("input.search-input") || document.querySelector("input");
    if (!input) return false;
    input.focus();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
    setter.call(input, "arch");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  }).catch(() => false);
  if (!seeded) return { takes, measured: 0, ms: summarize([]), samples: [], error: "no launcher input" };
  await sleep(500);
  const samples = [];
  for (let i = 0; i < takes; i++) {
    const startEpoch = await launcher.evaluate(() => Date.now());
    readMarksSince(); // drain any marks from the seed/render phase
    try { await launcher.keyboard.press(i % 2 === 0 ? "ArrowDown" : "ArrowUp"); } catch { /* ignore */ }
    await sleep(220);
    const since = readMarksSince();
    const markEpoch = since.marks
      .filter((m) => m.label === "launcher-selection-painted" && m.epochMs >= startEpoch - 250)
      .map((m) => m.epochMs)
      .sort((a, b) => a - b)
      .slice(-1)[0] ?? null;
    samples.push({ take: i, startEpoch, markEpoch, ms: markEpoch != null ? +(markEpoch - startEpoch).toFixed(2) : null, labels: since.marks.map((m) => m.label) });
  }
  const ms = samples.map((s) => s.ms).filter((v) => v != null);
  return {
    takes,
    measured: ms.length,
    ms: summarize(ms),
    note: "t0 = page Date.now() immediately before Playwright ArrowUp/Down; mark = launcher-selection-painted epoch (Date.now at one rAF after the keydown handler). Includes CDP key dispatch and IPC/file write are excluded (epoch captured in page).",
    samples,
  };
}

async function memorySnapshot(appPid, label) {
  try {
    const m = await runPs(METRICS_PS, ["-ProcessId", String(appPid), "-Mode", "memory"], 30000);
    return { label, ...m, totalWorkingSetMiB: +(m.totalWorkingSetBytes / (1024 * 1024)).toFixed(1), totalPrivateMiB: +(m.totalPrivateBytes / (1024 * 1024)).toFixed(1) };
  } catch (e) {
    return { label, error: String(e.message) };
  }
}

async function main() {
  const pw = loadPlaywright();
  if (!existsSync(exe)) throw new Error("exe not found: " + exe);
  mkdirSync(dirname(outPath), { recursive: true });
  mkdirSync(dataDir, { recursive: true });
  mkdirSync(resolve(dataDir, "webview2"), { recursive: true });

  const distIndex = resolve(repoRoot, "apps", "desktop", "dist", "index.html");
  results.conditions = {
    exe,
    exeBytes: statSync(exe).size,
    exeMtime: statSync(exe).mtime.toISOString(),
    frontendDistIndex: existsSync(distIndex) ? distIndex : null,
    frontendDistMtime: existsSync(distIndex) ? statSync(distIndex).mtime.toISOString() : null,
    dataDir,
    cdpPort: port,
    launchArgs: exeArgs,
    node: process.version,
  };
  try {
    results.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim();
    results.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length;
  } catch { /* git unavailable */ }

  try {
    if (!noServe) {
      server = spawn(process.platform === "win32" ? "npx.cmd" : "npx", ["vite", "preview", "--port", "5173", "--strictPort"], {
        cwd: resolve(repoRoot, "apps", "desktop"), stdio: "ignore", shell: process.platform === "win32", windowsHide: true,
      });
      const up = await waitForFrontend();
      if (!up) throw new Error("frontend preview server did not start on 5173");
    }
    if (!skipLaunch) {
      truncateMarks();
      app = spawn(exe, exeArgs, {
        env: {
          ...process.env,
          PROTONSEARCH_DATA_DIR: dataDir,
          PROTONSEARCH_PERF_HOOKS: "1",
          PROTONSEARCH_CDP_PORT: String(port),
          WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2"),
        },
        stdio: "ignore",
      });
      results.conditions.appPid = app.pid;
    }
    browser = await connectCdp(pw);
    let kinds = [];
    const pageDeadline = Date.now() + 60000;
    while (Date.now() < pageDeadline) {
      kinds = await findPages(browser);
      if (kinds.filter((k) => k.kind).length >= 2) break;
      await sleep(500);
    }
    const launcher = kinds.find((k) => k.kind === "launcher")?.page || kinds[0]?.page;
    if (!launcher) throw new Error("no launcher page found");
    results.conditions.pages = kinds.map((k) => ({ kind: k.kind, url: k.url }));
    results.conditions.devicePixelRatio = await launcher.evaluate(() => window.devicePixelRatio).catch(() => null);
    results.conditions.windowKind = kinds.find((k) => k.page === launcher)?.kind || null;

    results.measurements.indexReady = await waitIndexReady(launcher);

    // --- memory: idle (hidden launcher) ---
    try { await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_launcher")); } catch { /* ignore */ }
    await sleep(1200);
    results.measurements.memoryIdle = await memorySnapshot(results.conditions.appPid || app?.pid, "idle-hidden");
    results.measurements.idleCpu = await runPs(METRICS_PS, ["-ProcessId", String(results.conditions.appPid || app?.pid), "-Mode", "cpu", "-SampleSeconds", String(idleSeconds)], (idleSeconds + 30) * 1000).catch((e) => ({ error: String(e.message) }));

    // --- input -> first visible result ---
    try { await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher")); } catch { /* ignore */ }
    await sleep(500);
    results.measurements.searchTimingHook = await installSearchTimingHook(launcher);
    await launcher.evaluate(() => document.querySelector("input")?.focus()).catch(() => {});
    results.measurements.inputToVisible = await measureInputToVisible(launcher, takesInput, 3);
    results.measurements.engineIpc = await measureEngineIpc(launcher, Math.max(10, Math.floor(takesInput / 2)));
    results.measurements.selectionPaint = await measureSelection(launcher, Math.max(10, Math.floor(takesInput)));
    results.measurements.memoryVisibleResults = await memorySnapshot(results.conditions.appPid || app?.pid, "visible-results");

    // --- launcher activation via real global hotkey ---
    const appPid = results.conditions.appPid || (app ? app.pid : null);
    if (appPid) {
      results.measurements.launcherActivation = await measureHotkey(launcher, appPid, takesHotkey);
    } else {
      results.blocked.push("launcher activation: no app PID (attach-only mode)");
    }

    results.status = "ok";
    results.limitations.push("Hotkey activation uses SendInput via PowerShell and Win32 window polling at ~1ms; page paint delta comes from WebView visibilitychange + double rAF.");
  } catch (e) {
    results.status = "blocked";
    results.blocked.push(String(e && e.stack ? e.stack : e));
  } finally {
    try { if (browser) await browser.close(); } catch { /* ignore */ }
    killTree(app ? app.pid : results.conditions.appPid);
    if (server) killTree(server.pid);
    writeFileSync(outPath, JSON.stringify(results, null, 2));
  }

  const i = results.measurements.inputToVisible;
  const a = results.measurements.launcherActivation;
  console.log("app baseline: status=" + results.status);
  if (i) {
    console.log("  input->first visible result: p50=" + i.ms.p50 + "ms p95=" + i.ms.p95 + "ms n=" + i.measured + "/" + i.takes);
    if (i.split) {
      console.log("  split: engine-only p50=" + i.split.engineTookMs.p50 + " p95=" + i.split.engineTookMs.p95 + "ms | invoke wall p50=" + i.split.engineWallMs.p50 + " p95=" + i.split.engineWallMs.p95 + "ms | IPC p50=" + i.split.ipcOverheadMs.p50 + " p95=" + i.split.ipcOverheadMs.p95 + "ms | post-engine UI p50=" + i.split.postEngineUiMs.p50 + " p95=" + i.split.postEngineUiMs.p95 + "ms");
    }
  }
  if (a) console.log("  launcher activation (hotkey): visible p50=" + a.visibleMs.p50 + "ms p95=" + a.visibleMs.p95 + "ms focused p50=" + a.focusedMs.p50 + "ms paint-total p50=" + a.paintTotalMs.p50 + "ms n=" + a.measured + "/" + a.takes);
  const sp = results.measurements.selectionPaint;
  if (sp) console.log("  selection highlight (marks): p50=" + sp.ms.p50 + "ms p95=" + sp.ms.p95 + "ms n=" + sp.measured + "/" + sp.takes);
  const ah = a && a.marks;
  if (ah && ah.measuredSendToShown > 0) {
    console.log("  debug marks send->shown: p50=" + ah.sendToShownMs.p50 + "ms; shown->first-paint: p50=" + ah.shownToPaintMs.p50 + "ms p95=" + ah.shownToPaintMs.p95 + "ms; send->paint p95=" + ah.sendToPaintMs.p95 + "ms (n=" + ah.measuredSendToPaint + ")");
  }
  const m = results.measurements.memoryVisibleResults || results.measurements.memoryIdle;
  if (m && m.totalWorkingSetMiB != null) console.log("  process-tree memory: " + m.totalWorkingSetMiB + " MiB WS (" + m.processCount + " processes) [" + m.label + "]");
  const c = results.measurements.idleCpu;
  if (c && c.cpuPctOfMachine != null) console.log("  idle CPU: " + c.cpuPctOfMachine + "% of machine, " + c.cpuPctOfOneCore + "% of one core over " + c.sampleSeconds + "s");
  if (results.blocked.length) console.log("  blocked: " + results.blocked.join(" | "));
  console.log("  results: " + outPath);
  return results;
}

main().catch((e) => {
  console.error("app-baseline: " + (e && e.stack ? e.stack : e));
  process.exit(1);
});
