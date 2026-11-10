#!/usr/bin/env node
// Island performance harness: open/close frame pacing, typing + card-switch
// latency, lazy-window memory breakdown, activation trade-off and 20x
// open/close leak check. OWNER: perf-bench (L1-C).
//
// Debug build + CDP (Playwright) drives the real island WebView. Perf hooks
// (PROTONSEARCH_PERF_HOOKS=1) provide island-window-shown / island-first-paint
// marks. Process-tree memory comes from PowerShell/CIM (per-process WS, private
// bytes, command line). Every section is optional and recorded independently so
// a partial run still yields honest JSON.
//
// Usage:
//   node tools/benchmark/island-perf.mjs --exe target/debug/ProtonSearch.exe --out docs/performance/results/island-perf.json
//   node tools/benchmark/island-perf.mjs --attach-only --port 9222 ... (app already running)
//
// Leaves NO app process running (unless --attach-only). Reads marks from
// <data_dir>/v2/perf-marks.jsonl and truncates it at session start.

import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync, mkdirSync, writeFileSync, readFileSync, statSync, openSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");

function loadPlaywright() {
  const candidates = [process.env.PS_PLAYWRIGHT_DIR, resolve(repoRoot, "tools", "visual-qa"), __dirname].filter(Boolean);
  for (const dir of candidates) {
    try { const req = createRequire(resolve(dir, "package.json")); const pw = req("playwright"); if (pw && pw.chromium) return pw; } catch { /* next */ }
  }
  throw new Error("playwright not found (expected tools/visual-qa/node_modules/playwright)");
}
function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]; if (!a.startsWith("--")) continue;
    const key = a.slice(2); const next = argv[i + 1];
    if (next !== undefined && !next.startsWith("--")) { out[key] = next; i++; } else out[key] = true;
  }
  return out;
}
function utcTag() { return new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "") + "Z"; }
function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }
function percentile(s, p) { if (!s.length) return null; const i = Math.ceil((p / 100) * s.length) - 1; return +s[Math.max(0, Math.min(s.length - 1, i))].toFixed(3); }
function summarize(values) {
  if (!values.length) return { count: 0, p50: null, p95: null, p99: null, min: null, max: null, mean: null };
  const s = [...values].sort((a, b) => a - b);
  return { count: s.length, p50: percentile(s, 50), p95: percentile(s, 95), p99: percentile(s, 99), min: +s[0].toFixed(3), max: +s[s.length - 1].toFixed(3), mean: +(s.reduce((a, b) => a + b, 0) / s.length).toFixed(3) };
}
function runPs(script, scriptArgs, timeoutMs) {
  return new Promise((res, rej) => {
    execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script, ...scriptArgs], { windowsHide: true, timeout: timeoutMs || 60000 }, (err, stdout, stderr) => {
      if (err) return rej(new Error("PS helper failed: " + (stderr || err.message)));
      const line = String(stdout).trim().split(/\r?\n/).filter(Boolean).pop();
      try { res(JSON.parse(line)); } catch (e) { rej(new Error("bad PS JSON: " + line)); }
    });
  });
}
function killTree(pid) { if (!pid) return; try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* gone */ } }
function shotScreen(name) {
  return new Promise((resolveShot) => {
    const target = resolve(repoRoot, ".bench", name + ".png");
    execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", resolve(repoRoot, ".bench", "shot.ps1"), "-Path", target], { windowsHide: true }, (err) => resolveShot({ path: target, ok: !err }));
  });
}

const args = parseArgs(process.argv.slice(2));
const exe = resolve(args.exe || resolve(repoRoot, "target", "debug", "ProtonSearch.exe"));
const outPath = resolve(args.out || resolve(repoRoot, "docs", "performance", "results", "island-perf-" + utcTag() + ".json"));
const dataDir = resolve(args["data-dir"] || resolve(repoRoot, ".devdata"));
const port = Number(args.port || 9222);
const hz = Number(args.hz || 120);
const periodMs = 1000 / hz;
const cycles = Number(args.cycles || 20);
const takes = Number(args.takes || 20);
const openTakes = Number(args["open-takes"] || 8);
const attachOnly = Boolean(args["attach-only"]);
const noServe = Boolean(args["no-serve"]);
const idleSeconds = Number(args["idle-seconds"] || 20);
const trimWaitMs = Number(args["trim-wait-ms"] || 8000);
const memoryOnly = Boolean(args["memory-only"]);
const settingsPrimary = args["settings-primary"] === "island" ? "dynamic_island" : (args["settings-primary"] || "normal");
const HOTKEY_PS = resolve(__dirname, "hotkey-activate.ps1");
const METRICS_PS = resolve(__dirname, "process-metrics.ps1");
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");
let marksOffset = 0;

function truncateMarks() { try { writeFileSync(MARKS_FILE, ""); marksOffset = 0; } catch { /* ignore */ } }
function readMarksSince() {
  let text = ""; try { text = readFileSync(MARKS_FILE, "utf8"); } catch { return { marks: [], offset: marksOffset }; }
  const buf = Buffer.from(text); if (buf.length < marksOffset) marksOffset = 0;
  const slice = buf.slice(marksOffset).toString("utf8"); const marks = [];
  for (const line of slice.split(/\r?\n/)) { if (!line.trim()) continue; try { marks.push(JSON.parse(line)); } catch { /* partial */ } }
  marksOffset = buf.length; return { marks, offset: marksOffset };
}
function markEpochs(marks, label) { return marks.filter((m) => m.label === label).map((m) => m.epochMs).sort((a, b) => a - b); }

// The harness switches primary_mode at runtime; a previous run persists
// dynamic_island in the scratch data dir. Normalise the scratch profile before
// launch so the run always starts from the "primary launcher prewarmed" state
// that section 1 is defined against.
function normalizeScratchSettings(primaryMode) {
  const p = resolve(dataDir, "v2", "settings.json");
  const mode = primaryMode || "normal";
  try {
    if (!existsSync(p)) return "no-settings";
    const data = JSON.parse(readFileSync(p, "utf8"));
    if (data && data.launcher) {
      data.launcher.primaryMode = mode;
      writeFileSync(p, JSON.stringify(data, null, 2));
      return "normalised-to-" + mode;
    }
    return "no-launcher-settings";
  } catch (e) { return "normalise-error: " + String(e); }
}

const report = {
  schema: "ps-island-perf/1",
  kind: "island-performance",
  status: "running",
  capturedAt: new Date().toISOString(),
  host: { hostname: hostname(), platform: process.platform },
  conditions: {},
  measurements: {},
  blocked: [],
  limitations: [
    "Debug build + CDP drive the real island WebView; headless frame counting is not used - WebView2 runs on the real display.",
    "Frame interval p50/p95/p99 are rAF intervals inside the island page during a 900-1000 ms window; dropped = interval > 1.5 x assumed refresh period (default 120 Hz for this host, period 8.333 ms).",
    "Activation first-open vs warm-open is measured with the real global hotkey (SendInput) and perf marks; the lazy window creation cost is isolated by whether the island page existed before the hotkey.",
    "Process-tree memory is PowerShell/CIM Working Set + private bytes with command lines; WebView2 child roles are inferred from command lines, not guessed.",
    "Root-cause attribution is evidence-based (CDP Performance Layout/Recalc/Script/Task durations + page resize events + long tasks), not a visual guess.",
    "KNOWN v10 BUG (found by this harness): switching launcher.primary_mode at runtime through update_settings exits the app. update_settings destroys the inactive window with win.destroy(); when the launcher is the only window this is the last window, so Tauri exits (observed code 0 plus 'Error removing system tray icon'). Lazy island activation is therefore measured through the show_island command (same ensure_island_window path); hotkey->island is measured in island-hotkey.mjs from an island-primary profile.",
  ],
};
let app = null; let server = null; let browser = null; let cdpClient = null;
function step(name) { try { console.error("[island-perf] " + name + " @ " + ((Date.now() / 1000) % 100000).toFixed(1)); } catch { /* ignore */ } }
function appLogFiles() {
  try {
    mkdirSync(resolve(repoRoot, ".bench"), { recursive: true });
    const outFd = openSync(resolve(repoRoot, ".bench", "island-app-stdout.log"), "w");
    const errFd = openSync(resolve(repoRoot, ".bench", "island-app-stderr.log"), "w");
    return { outFd, errFd };
  } catch { return null; }
}

async function waitForFrontend() {
  const deadline = Date.now() + 40000;
  while (Date.now() < deadline) { try { const res = await fetch("http://127.0.0.1:5173/index.html"); if (res.ok) return true; } catch { /* retry */ } await sleep(400); }
  return false;
}
async function connectCdp(pw) {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) { try { const res = await fetch("http://127.0.0.1:" + port + "/json/version"); if (res.ok) return await pw.chromium.connectOverCDP("http://127.0.0.1:" + port); } catch { /* retry */ } await sleep(500); }
  throw new Error("could not connect to WebView2 CDP on port " + port);
}
async function collectPages() { return browser.contexts().flatMap((c) => c.pages()); }
async function pageKind(page) { try { return await page.evaluate(() => document.documentElement.dataset.window || null); } catch { return null; } }
async function findWindow(pages, kind) { for (const page of pages) { if ((await pageKind(page)) === kind) return page; } return null; }
async function waitWindow(kind, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const page = await findWindow(await collectPages(), kind);
    if (page) { try { await page.waitForLoadState("domcontentloaded", { timeout: 3000 }).catch(() => {}); return page; } catch { /* retry */ } }
    await sleep(250);
  }
  return null;
}
async function memorySnapshot(label) {
  const pid = report.conditions.appPid;
  if (!pid) return { label, error: "no app pid" };
  try {
    const m = await runPs(METRICS_PS, ["-ProcessId", String(pid), "-Mode", "memory"], 40000);
    return { label, ...m, totalWorkingSetMiB: +(m.totalWorkingSetBytes / 1048576).toFixed(1), totalPrivateMiB: +(m.totalPrivateBytes / 1048576).toFixed(1) };
  } catch (e) { return { label, error: String(e.message) }; }
}
function compareSnapshots(before, after) {
  if (!before || !after || !before.processes || !after.processes) return null;
  const map = new Map();
  for (const p of before.processes) map.set(p.pid, { name: p.name, wsBefore: p.wsBytes, privBefore: p.privateBytes });
  const rows = [];
  for (const p of after.processes) {
    const prev = map.get(p.pid);
    if (!prev) { rows.push({ pid: p.pid, name: p.name, wsBeforeMiB: null, wsAfterMiB: +(p.wsBytes / 1048576).toFixed(1), deltaMiB: null, trimmed: null }); continue; }
    const deltaMiB = +((p.wsBytes - prev.wsBefore) / 1048576).toFixed(1);
    rows.push({ pid: p.pid, name: p.name, wsBeforeMiB: +(prev.wsBefore / 1048576).toFixed(1), wsAfterMiB: +(p.wsBytes / 1048576).toFixed(1), deltaMiB, trimmed: deltaMiB <= -5 });
  }
  return {
    rows,
    trimmedCount: rows.filter((r) => r.trimmed).length,
    totalDeltaMiB: +((after.totalWorkingSetBytes - before.totalWorkingSetBytes) / 1048576).toFixed(1),
  };
}

async function installIslandRecorder(page) {
  return await page.evaluate(() => {
    if (window.__ISL__) return "already-installed";
    const st = { frames: [], shifts: [], longtasks: [], resizeEvents: 0, surfaceSizes: [], active: false, raf: 0 };
    const surface = () => document.querySelector(".island-shell, .island-surface, .island-host");
    const tick = (t) => { if (!st.active) return; st.frames.push(t); st.raf = requestAnimationFrame(tick); };
    st.start = () => {
      st.frames = []; st.shifts = []; st.longtasks = []; st.resizeEvents = 0; st.surfaceSizes = []; st.active = true;
      window.addEventListener("resize", () => { st.resizeEvents += 1; });
      const el = surface();
      if (el) {
        const ro = new ResizeObserver(() => {
          const r = el.getBoundingClientRect();
          const key = Math.ceil(r.width) + "x" + Math.ceil(r.height);
          if (st.surfaceSizes[st.surfaceSizes.length - 1] !== key) st.surfaceSizes.push(key);
        });
        ro.observe(el);
        st._ro = ro;
      }
      st.raf = requestAnimationFrame(tick);
      return performance.now();
    };
    st.stop = () => {
      st.active = false; if (st.raf) cancelAnimationFrame(st.raf); if (st._ro) st._ro.disconnect();
      return { frames: st.frames.slice(), shifts: st.shifts.slice(), longtasks: st.longtasks.slice(), resizeEvents: st.resizeEvents, surfaceSizes: st.surfaceSizes.slice() };
    };
    try {
      const po = new PerformanceObserver((list) => { for (const e of list.getEntries()) if (e.entryType === "layout-shift" && !e.hadRecentInput) st.shifts.push({ t: e.startTime, value: e.value }); });
      po.observe({ type: "layout-shift", buffered: false });
      const po2 = new PerformanceObserver((list) => { for (const e of list.getEntries()) st.longtasks.push({ start: e.startTime, duration: e.duration }); });
      po2.observe({ type: "longtask", buffered: false });
    } catch { /* unsupported */ }
    window.__ISL__ = st;
    return "installed";
  });
}
function framesSummary(raw) {
  const intervals = [];
  for (let i = 1; i < raw.frames.length; i++) intervals.push(+(raw.frames[i] - raw.frames[i - 1]).toFixed(3));
  const dropped = intervals.filter((x) => x > periodMs * 1.5).length;
  const over = intervals.filter((x) => x > periodMs * 1.05).length;
  const shifts = raw.shifts || [];
  const longtasks = raw.longtasks || [];
  return {
    frames: raw.frames.length,
    frameMs: summarize(intervals),
    intervalsMs: intervals,
    droppedFrames: dropped,
    droppedPct: intervals.length ? +((dropped / intervals.length) * 100).toFixed(2) : 0,
    overBudgetFrames: over,
    overBudgetPct: intervals.length ? +((over / intervals.length) * 100).toFixed(2) : 0,
    layoutShift: { count: shifts.length, cls: +shifts.reduce((a, s) => a + s.value, 0).toFixed(4) },
    longTasks: { count: longtasks.length, totalMs: +longtasks.reduce((a, l) => a + l.duration, 0).toFixed(1) },
    resizeEvents: raw.resizeEvents,
    uniqueSurfaceSizes: (raw.surfaceSizes || []).length,
    surfaceSizes: (raw.surfaceSizes || []).slice(0, 25),
  };
}

async function memoryOnlyRun() {
  // Release-friendly memory run: no CDP required (release builds do not expose it).
  // Measures idle tree with only the prewarmed primary launcher, then a real
  // hotkey show/hide cycle plus trim wait.
  if (!existsSync(exe)) throw new Error("exe not found: " + exe);
  mkdirSync(dirname(outPath), { recursive: true }); mkdirSync(dataDir, { recursive: true }); mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
  report.conditions = { exe, exeBytes: statSync(exe).size, exeMtime: statSync(exe).mtime.toISOString(), dataDir, memoryOnly: true, assumedRefreshHz: hz };
  try { report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim(); report.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length; } catch { /* ignore */ }
  try {
    report.measurements.scratchSettings = normalizeScratchSettings(settingsPrimary);
    truncateMarks();
    app = spawn(exe, [], { env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") }, stdio: "ignore" });
    report.conditions.appPid = app.pid;
    await sleep(10000);
    report.measurements.memoryIdleLauncherOnly = await memorySnapshot("idle-launcher-only-after-10s");
    report.measurements.idleCpuLauncherOnly = await runPs(METRICS_PS, ["-ProcessId", String(app.pid), "-Mode", "cpu", "-SampleSeconds", String(idleSeconds)], (idleSeconds + 40) * 1000).catch((e) => ({ error: String(e.message) }));
    await sleep(trimWaitMs);
    const trimmed = await memorySnapshot("launcher-hidden-after-trim-wait");
    report.measurements.memoryTrim = { waitMs: trimWaitMs, visibleIdle: report.measurements.memoryIdleLauncherOnly, afterHideTrim: trimmed, delta: compareSnapshots(report.measurements.memoryIdleLauncherOnly, trimmed), note: "Primary launcher was never shown in this mode; this compares start-up (10 s) with start-up + trim wait to detect WebView2 memory-target trim on a hidden prewarmed window." };
    // Real hotkey show (activation) then hide + trim
    const since = readMarksSince();
    let win = null;
    try { win = await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-TimeoutMs", "5000", "-Mode", "show"], 25000); } catch (e) { report.blocked.push("hotkey show: " + String(e.message)); }
    await sleep(800);
    const visible = await memorySnapshot("launcher-visible");
    const marks = readMarksSince().marks.concat(since.marks);
    const primaryLabel = settingsPrimary === "dynamic_island" ? "island" : "launcher";
    const shown = markEpochs(marks, primaryLabel + "-window-shown").filter((v) => v >= ((win && win.sendAtUtc) || 0) - 400).slice(-1)[0] ?? null;
    const paint = markEpochs(marks, primaryLabel + "-first-paint").filter((v) => (shown == null ? true : v >= shown - 60)).slice(-1)[0] ?? null;
    if (args.screenshot) {
      report.measurements.screenshot = await shotScreen(String(args.screenshot));
    }
    report.measurements["activation_" + settingsPrimary] = {
      win32VisibleMs: win && win.visibleMs != null ? win.visibleMs : null,
      win32FocusedMs: win && win.focusedMs != null ? win.focusedMs : null,
      sendToShownMs: shown != null ? +(shown - win.sendAtUtc).toFixed(2) : null,
      shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
      sendToPaintMs: paint != null ? +(paint - win.sendAtUtc).toFixed(2) : null,
    };
    // hide again and measure trim
    try { await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-Mode", "send"], 15000); } catch { /* ignore */ }
    await sleep(1200);
    const hidden1 = await memorySnapshot("launcher-hidden-immediate");
    await sleep(trimWaitMs);
    const hiddenTrim = await memorySnapshot("launcher-hidden-trimmed");
    report.measurements.visibleVsHiddenTrim = { waitMs: trimWaitMs, visible, hiddenImmediate: hidden1, hiddenTrimmed: hiddenTrim, delta: compareSnapshots(visible, hiddenTrim), note: "Real show -> hide cycle; hiddenTrimmed is after the trim wait so a WebView2 LOW memory-target trim shows as per-process WS drop while the process stays warm." };
    report.status = "ok";
  } catch (e) { report.status = "blocked"; report.blocked.push(String(e && e.stack ? e.stack : e)); }
  finally {
    killTree(app ? app.pid : report.conditions.appPid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  console.log("island-perf memory-only: status=" + report.status);
  const idle = report.measurements.memoryIdleLauncherOnly;
  if (idle && idle.totalWorkingSetMiB != null) console.log("  idle launcher-only tree: " + idle.totalWorkingSetMiB + " MiB WS / " + idle.totalPrivateMiB + " MiB private across " + idle.processCount + " processes");
  const vt = report.measurements.visibleVsHiddenTrim;
  if (vt && vt.delta) console.log("  visible->hidden+trim: tree " + vt.visible.totalWorkingSetMiB + " -> " + vt.hiddenTrimmed.totalWorkingSetMiB + " MiB (" + vt.delta.totalDeltaMiB + " MiB), trimmed=" + vt.delta.trimmedCount);
  const actAny = report.measurements["activation_" + settingsPrimary];
  if (actAny) console.log("  " + settingsPrimary + " activation: visible=" + actAny.win32VisibleMs + "ms send->paint=" + actAny.sendToPaintMs + "ms");
  if (report.blocked.length) console.log("  blocked: " + report.blocked.join(" | "));
  console.log("  results: " + outPath);
}

async function main() {
  if (memoryOnly) return memoryOnlyRun();
  const pw = loadPlaywright();
  if (!attachOnly && !existsSync(exe)) throw new Error("exe not found: " + exe);
  mkdirSync(dirname(outPath), { recursive: true }); mkdirSync(dataDir, { recursive: true }); mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
  const distIndex = resolve(repoRoot, "apps", "desktop", "dist", "index.html");
  report.conditions = {
    exe, exeBytes: existsSync(exe) ? statSync(exe).size : null, exeMtime: existsSync(exe) ? statSync(exe).mtime.toISOString() : null,
    frontendDistMtime: existsSync(distIndex) ? statSync(distIndex).mtime.toISOString() : null,
    dataDir, port, assumedRefreshHz: hz, cycles, takes, openTakes, attachOnly,
  };
  try {
    report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim();
    report.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length;
  } catch { /* ignore */ }

  try {
    if (!noServe) {
      server = spawn(process.platform === "win32" ? "npx.cmd" : "npx", ["vite", "preview", "--port", "5173", "--strictPort"], { cwd: resolve(repoRoot, "apps", "desktop"), stdio: "ignore", shell: process.platform === "win32", windowsHide: true });
      if (!(await waitForFrontend())) throw new Error("frontend preview server did not start on 5173");
    }
    if (!attachOnly) {
      report.measurements.scratchSettings = normalizeScratchSettings("normal");
      truncateMarks();
      const logs = appLogFiles();
      app = spawn(exe, [], {
        env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", PROTONSEARCH_CDP_PORT: String(port), WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") },
        stdio: logs ? ["ignore", logs.outFd, logs.errFd] : "ignore",
      });
      report.conditions.appPid = app.pid;
    }
    step("launching");
    browser = await connectCdp(pw);
    step("cdp-connected");
    const launcher = await waitWindow("launcher", 60000);
    step("launcher-found");
    if (!launcher) throw new Error("no launcher page found");
    report.conditions.initialWindows = (await collectPages()).length;
    report.measurements.indexReady = await launcher.evaluate(async () => {
      const deadline = Date.now() + 180000;
      while (Date.now() < deadline) {
        try {
          const status = await window.__TAURI_INTERNALS__.invoke("get_status");
          if (status && (status.state === "ready" || status.state === "paused") && status.filenameEntries > 0) return { state: status.state, entries: status.filenameEntries };
        } catch { /* starting */ }
        await new Promise((r) => setTimeout(r, 500));
      }
      return { state: "timeout" };
    }).catch(() => ({ state: "unavailable" }));

    step("index-ready:" + JSON.stringify(report.measurements.indexReady));
    // --- Section 1: idle memory with only the primary launcher prewarmed ---
    await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_launcher")).catch(() => {});
    await sleep(1500);
    report.measurements.initialPages = (await collectPages()).map((p) => ({ url: p.url(), kind: null }));
    for (const p of (await collectPages())) {
      const kind = await pageKind(p);
      const entry = report.measurements.initialPages.find((x) => x.url === p.url());
      if (entry) entry.kind = kind;
    }
    step("memory-idle");
    report.measurements.memoryIdleLauncherOnly = await memorySnapshot("idle-launcher-only");
    step("cpu-idle");
    report.measurements.idleCpuLauncherOnly = await runPs(METRICS_PS, ["-ProcessId", String(report.conditions.appPid), "-Mode", "cpu", "-SampleSeconds", String(idleSeconds)], (idleSeconds + 40) * 1000).catch((e) => ({ error: String(e.message) }));
    // WebView2 memory-target trim check: the launcher window is already hidden; wait
    // for any SetMemoryUsageTargetLevel(LOW) trim and compare per-process memory.
    const launcherIdleSnapshot = report.measurements.memoryIdleLauncherOnly;
    await sleep(trimWaitMs);
    const launcherTrimSnapshot = await memorySnapshot("launcher-hidden-after-trim-wait");
    report.measurements.memoryTrim = {
      waitMs: trimWaitMs,
      visibleIdle: launcherIdleSnapshot,
      afterHideTrim: launcherTrimSnapshot,
      delta: compareSnapshots(launcherIdleSnapshot, launcherTrimSnapshot),
      note: "Launcher is hidden in both snapshots; afterHideTrim is taken " + trimWaitMs + " ms later so any WebView2 SetMemoryUsageTargetLevel(LOW) trim is visible as a per-process WS drop. A process with >=5 MiB drop is marked trimmed; warm-process retention is checked later by reopening.",
    };

    // --- Section 2: lazy island window creation (no runtime mode switch) ---
    // The runtime primary-mode switch through update_settings is currently a hard
    // exit: update_settings destroys the inactive WebView with win.destroy(), and
    // when that is the only window Tauri exits (observed: exit code 0 + "Error
    // removing system tray icon"). That bug is reported to the shell owner, and
    // this harness keeps the normal profile and exercises the lazy creation path
    // through show_island, which calls ensure_island_window exactly like the
    // hotkey path does. Real hotkey->island activation is measured separately by
    // island-hotkey.mjs with an island-primary profile.
    step("mode-switch-skipped-runtime-bug");
    const activationTrials = [];
    for (let i = 0; i < openTakes; i++) {
      let islandPage = await findWindow(await collectPages(), "island");
      const existedBefore = islandPage != null;
      if (islandPage) { try { await islandPage.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ } await sleep(450); }
      const since = readMarksSince();
      const invokeResult = await launcher.evaluate(async () => {
        const t0 = Date.now();
        const perfStart = performance.now();
        await window.__TAURI_INTERNALS__.invoke("show_island");
        return { t0, invokeMs: performance.now() - perfStart };
      }).catch((e) => ({ error: String(e) }));
      await sleep(250);
      const marks = readMarksSince().marks.concat(since.marks);
      const shown = markEpochs(marks, "island-window-shown").slice(-1)[0] ?? null;
      const paint = markEpochs(marks, "island-first-paint").filter((v) => (shown == null ? true : v >= shown - 60)).slice(-1)[0] ?? null;
      islandPage = await waitWindow("island", 8000);
      activationTrials.push({
        take: i,
        islandExistedBefore: existedBefore,
        invokeMs: invokeResult && invokeResult.invokeMs != null ? +invokeResult.invokeMs.toFixed(2) : null,
        createShowMs: shown != null && invokeResult.t0 != null ? +(shown - invokeResult.t0).toFixed(2) : null,
        shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
        commandToPaintMs: paint != null && invokeResult.t0 != null ? +(paint - invokeResult.t0).toFixed(2) : null,
        islandPageFound: islandPage != null,
      });
      if (islandPage) { try { await islandPage.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ } }
      readMarksSince();
      await sleep(450);
    }
    const firstActivation = activationTrials[0] || null;
    const warmActivation = activationTrials.slice(1);
    report.measurements.activation = {
      lazyFirstOpen: firstActivation,
      warmTrials: warmActivation.length,
      warmInvokeMs: summarize(warmActivation.map((s) => s.invokeMs).filter((v) => v != null)),
      warmCreateShowMs: summarize(warmActivation.map((s) => s.createShowMs).filter((v) => v != null)),
      warmCommandToPaintMs: summarize(warmActivation.map((s) => s.commandToPaintMs).filter((v) => v != null)),
      warmShownToPaintMs: summarize(warmActivation.map((s) => s.shownToPaintMs).filter((v) => v != null)),
      samples: activationTrials,
      note: "First show_island after launch includes lazy WebView creation; later shows are warm. Command path is measured here because the runtime mode switch exits the app; hotkey->island is measured in island-hotkey.mjs.",
    };

    // Ensure the island exists and is visible for memory/pacing sections.
    let island = await waitWindow("island", 15000);
    if (!island) { try { await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ } await sleep(700); island = await waitWindow("island", 15000); }
    if (!island) throw new Error("island page never appeared after show_island (lazy creation path)");
    try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ }
    await sleep(400);
    report.measurements.memoryAfterIslandCreated = await memorySnapshot("island-created-visible");
    // Island hide -> memory-target trim check (visible warm island vs hidden after wait).
    try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ }
    await sleep(trimWaitMs);
    const islandHiddenTrim = await memorySnapshot("island-hidden-after-trim-wait");
    report.measurements.islandTrim = {
      waitMs: trimWaitMs,
      visible: report.measurements.memoryAfterIslandCreated,
      hiddenAfterTrim: islandHiddenTrim,
      delta: compareSnapshots(report.measurements.memoryAfterIslandCreated, islandHiddenTrim),
      islandPageStillPresent: (await findWindow(await collectPages(), "island")) != null,
      note: "Island hidden for " + trimWaitMs + " ms; a WS drop with the page still present indicates memory-target trim without process teardown (warm).",
    };
    // Warm reopen after the trim window via show_island (primary mode is normal, so
    // the hotkey would summon the launcher; island hotkey is measured separately).
    try {
      const since = readMarksSince();
      const reopen = await launcher.evaluate(async () => {
        const t0 = Date.now();
        const perfStart = performance.now();
        await window.__TAURI_INTERNALS__.invoke("show_island");
        return { t0, invokeMs: performance.now() - perfStart };
      });
      await sleep(300);
      const marks = readMarksSince().marks.concat(since.marks);
      const shown = markEpochs(marks, "island-window-shown").slice(-1)[0] ?? null;
      const paint = markEpochs(marks, "island-first-paint").filter((v) => (shown == null ? true : v >= shown - 60)).slice(-1)[0] ?? null;
      report.measurements.reopenAfterTrim = {
        invokeMs: +reopen.invokeMs.toFixed(2),
        createShowMs: shown != null ? +(shown - reopen.t0).toFixed(2) : null,
        shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
        commandToPaintMs: paint != null ? +(paint - reopen.t0).toFixed(2) : null,
        note: "Warm show_island reopen after the trim wait; compare with the warm activation percentiles.",
      };
    } catch (e) { report.measurements.reopenAfterTrim = { error: String(e.message) }; }
    // Ensure the island is visible again for pacing/typing sections.
    try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ }
    await sleep(400);
    report.measurements.installed = await installIslandRecorder(island);
    const cdp = await island.context().newCDPSession(island);
    cdpClient = cdp;
    await cdp.send("Performance.enable").catch(() => {});

    step("activation-done");
    // --- Section 4: open/close frame pacing ---
    const openSeries = [];
    const closeSeries = [];
    for (let i = 0; i < openTakes; i++) {
      try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ }
      await sleep(500);
      await island.evaluate(() => window.__ISL__.start());
      await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island"));
      await sleep(950);
      const raw = await island.evaluate(() => window.__ISL__.stop());
      openSeries.push(framesSummary(raw));
      // close: dispatch Escape to the frontend (its keydown handler plays the
      // exit animation and then calls hide_island; Playwright keyboard needs
      // window focus, which is not guaranteed after programmatic show).
      await island.evaluate(() => window.__ISL__.start());
      try {
        await island.evaluate(() => {
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape", bubbles: true, cancelable: true }));
        });
      } catch { /* ignore */ }
      await sleep(1000);
      const rawClose = await island.evaluate(() => window.__ISL__.stop()).catch(() => null);
      if (rawClose) closeSeries.push(framesSummary(rawClose));
      await sleep(300);
    }
    report.measurements.openFramePacing = { trials: openSeries.length, aggregate: poolFrameSeries(openSeries), series: openSeries };
    report.measurements.closeFramePacing = { trials: closeSeries.length, aggregate: poolFrameSeries(closeSeries), series: closeSeries };

    step("pacing-done");
    // --- Section 5: typing -> results latency inside the island search card ---
    if (report.measurements.indexReady && report.measurements.indexReady.entries > 0) {
      try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ }
      await sleep(500);
      const typingSamples = [];
      const queries = ["arch", "readme", "proton", "index", "报告", "café", "zzzz-no-match", "pdf"];
      for (let i = 0; i < takes; i++) {
        const q = queries[i % queries.length];
        const r = await island.evaluate(async (query) => {
          const input = document.querySelector(".island-search-input") || document.querySelector("input");
          if (!input) return { error: "no island search input" };
          input.focus();
          const results = () => document.querySelectorAll("#island-search-results [role=option], .island-results [role=option]").length;
          const signature = () => results() + "|" + (document.querySelector(".island-results")?.textContent || "").slice(0, 40);
          const baseline = signature();
          const start = performance.now();
          const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
          setter.call(input, query);
          input.dispatchEvent(new Event("input", { bubbles: true }));
          const deadline = performance.now() + 4000;
          while (performance.now() < deadline) {
            if (signature() !== baseline) { await new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res))); return { start, visibleAt: performance.now(), rows: results() }; }
            await new Promise((res) => setTimeout(res, 4));
          }
          return { start, visibleAt: null, rows: results() };
        }, q).catch((e) => ({ error: String(e) }));
        typingSamples.push({ take: i, query: q, ...r, ms: r.visibleAt != null ? +(r.visibleAt - r.start).toFixed(2) : null });
      }
      const ms = typingSamples.map((s) => s.ms).filter((v) => v != null);
      report.measurements.typingLatency = { takes, measured: ms.length, ms: summarize(ms), samples: typingSamples };
    }

    // --- Section 6: card-switch latency + root-cause attribution ---
    const switchSamples = [];
    const metricNames = ["TaskDuration", "ScriptDuration", "LayoutDuration", "RecalcStyleDuration"];
    const getMetrics = async () => {
      const res = await cdp.send("Performance.getMetrics").catch(() => null);
      const out = {};
      if (res && res.metrics) for (const m of res.metrics) out[m.name] = m.value;
      return out;
    };
    for (let i = 0; i < takes; i++) {
      try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ }
      await sleep(250);
      const before = await getMetrics();
      const startInfo = await island.evaluate(() => {
        const heading = document.querySelector(".island-card-heading")?.textContent?.trim() || null;
        const t0 = performance.now();
        window.__ISL__.start();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", code: "ArrowDown", ctrlKey: true, bubbles: true, cancelable: true }));
        return { t0, heading };
      });
      const changed = await island.evaluate(async (beforeHeading) => {
        const deadline = performance.now() + 4000;
        const heading = () => document.querySelector(".island-card-heading")?.textContent?.trim() || null;
        while (performance.now() < deadline) {
          if (heading() !== beforeHeading) { await new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res))); return { at: performance.now(), heading: heading() }; }
          await new Promise((res) => setTimeout(res, 4));
        }
        return { at: null, heading: heading() };
      }, startInfo.heading).catch(() => ({ at: null }));
      const raw = await island.evaluate(() => window.__ISL__.stop()).catch(() => null);
      const after = await getMetrics();
      const deltas = {};
      for (const name of metricNames) deltas[name + "Sec"] = +(((after[name] || 0) - (before[name] || 0))).toFixed(4);
      switchSamples.push({
        take: i,
        switchMs: changed.at != null ? +(changed.at - startInfo.t0).toFixed(2) : null,
        headingBefore: startInfo.heading, headingAfter: changed.heading || null,
        frames: raw ? framesSummary(raw) : null,
        perfDeltas: deltas,
      });
      await sleep(150);
    }
    const switchMs = switchSamples.map((s) => s.switchMs).filter((v) => v != null);
    const layout = switchSamples.map((s) => s.perfDeltas.LayoutDurationSec);
    const recalc = switchSamples.map((s) => s.perfDeltas.RecalcStyleDurationSec);
    const script = switchSamples.map((s) => s.perfDeltas.ScriptDurationSec);
    const task = switchSamples.map((s) => s.perfDeltas.TaskDurationSec);
    const avg = (a) => (a.length ? +(a.reduce((x, y) => x + y, 0) / a.length).toFixed(4) : null);
    const resizeEvents = switchSamples.map((s) => (s.frames ? s.frames.resizeEvents : 0));
    const avgLayout = avg(layout), avgRecalc = avg(recalc), avgScript = avg(script);
    let attribution = "inconclusive";
    if (avgLayout != null && avgScript != null) {
      const nativeish = (avgLayout || 0) + (avgRecalc || 0) > (avgScript || 0) * 1.25 && resizeEvents.reduce((a, b) => a + (b || 0), 0) / Math.max(1, resizeEvents.length) > 3;
      if (nativeish) attribution = "native window-resize path dominates (high Layout/Recalc + many page resize events per switch)";
      else if ((avgScript || 0) > ((avgLayout || 0) + (avgRecalc || 0))) attribution = "React scripting dominates (ScriptDuration > Layout+Recalc)";
      else attribution = "mixed (layout/recalc and scripting are comparable)";
    }
    report.measurements.cardSwitch = {
      takes,
      measured: switchMs.length,
      switchMs: summarize(switchMs),
      frames: poolFrameSeries(switchSamples.map((s) => s.frames).filter(Boolean)),
      perfAverages: { taskSec: avg(task), scriptSec: avg(script), layoutSec: avg(layout), recalcSec: avg(recalc) },
      avgResizeEventsPerSwitch: resizeEvents.length ? +(resizeEvents.reduce((a, b) => a + (b || 0), 0) / resizeEvents.length).toFixed(2) : 0,
      attribution,
      samples: switchSamples,
    };

    step("switch-done attribution=" + (report.measurements.cardSwitch && report.measurements.cardSwitch.attribution));
    // --- Section 7: 20x open/close leak check ---
    const leak = [];
    try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ }
    await sleep(400);
    const base = await memorySnapshot("leak-base");
    leak.push({ cycle: 0, ...base });
    for (let c = 1; c <= cycles; c++) {
      try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ }
      await sleep(220);
      try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")); } catch { /* ignore */ }
      await sleep(220);
      if (c % 5 === 0 || c === cycles) leak.push({ cycle: c, ...(await memorySnapshot("cycle-" + c)) });
    }
    const pagesAfter = (await collectPages()).length;
    const first = leak[0] && leak[0].totalWorkingSetBytes;
    const last = leak[leak.length - 1] && leak[leak.length - 1].totalWorkingSetBytes;
    report.measurements.openCloseLeak = {
      cycles, snapshots: leak, pagesAfter,
      growthMiB: first != null && last != null ? +((last - first) / 1048576).toFixed(1) : null,
      note: "Working-set snapshot every 5 cycles; a leak is only declared if growth is monotonic and >20 MiB over 20 cycles after the first snapshot.",
    };
    // --- Section 8: runtime primary-mode switch verification (bug-fix probe) ---
    // The pre-fix build exited when update_settings destroyed the inactive window.
    // The fix creates the target window first; this probe records whether the app
    // survives a runtime switch and whether the hotkey then opens the island.
    const switchProbe = { attempted: true };
    try {
      switchProbe.updateSettings = await launcher.evaluate(async () => {
        const settings = await window.__TAURI_INTERNALS__.invoke("get_settings");
        settings.launcher.primaryMode = "dynamic_island";
        await window.__TAURI_INTERNALS__.invoke("update_settings", { patch: { launcher: settings.launcher } });
        return "ok";
      });
      await sleep(600);
      const since = readMarksSince();
      let win = null;
      try { win = await runPs(HOTKEY_PS, ["-ProcessId", String(report.conditions.appPid), "-TimeoutMs", "8000", "-Mode", "show"], 30000); } catch (e) { switchProbe.hotkeyError = String(e.message); }
      await sleep(300);
      const marks = readMarksSince().marks.concat(since.marks);
      const shown = win ? (markEpochs(marks, "island-window-shown").filter((v) => v >= (win.sendAtUtc || 0) - 400).slice(-1)[0] ?? null) : null;
      const paint = markEpochs(marks, "island-first-paint").filter((v) => (shown == null ? true : v >= shown - 60)).slice(-1)[0] ?? null;
      const islandPage = await waitWindow("island", 5000);
      switchProbe.hotkey = {
        win32VisibleMs: win && win.visibleMs != null ? win.visibleMs : null,
        sendToShownMs: shown != null && win ? +(shown - win.sendAtUtc).toFixed(2) : null,
        shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
        sendToPaintMs: shown != null && paint != null && win ? +(paint - win.sendAtUtc).toFixed(2) : null,
        islandPageFound: islandPage != null,
        note: "Runtime switch to island-primary then real hotkey; pre-fix this exited the app before show_island could run.",
      };
    } catch (e) {
      switchProbe.error = String(e && e.message ? e.message : e);
    }
    report.measurements.runtimeModeSwitch = switchProbe;
    report.measurements.memoryFinal = await memorySnapshot("final-visible-results");
    report.status = "ok";
  } catch (e) {
    report.status = "blocked";
    report.blocked.push(String(e && e.stack ? e.stack : e));
  } finally {
    try { if (browser) await browser.close(); } catch { /* ignore */ }
    if (!attachOnly) killTree(app ? app.pid : report.conditions.appPid);
    if (server) killTree(server.pid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  console.log("island-perf: status=" + report.status);
  const idle = report.measurements.memoryIdleLauncherOnly;
  if (idle && idle.totalWorkingSetMiB != null) console.log("  idle launcher-only tree: " + idle.totalWorkingSetMiB + " MiB WS / " + idle.totalPrivateMiB + " MiB private across " + idle.processCount + " processes");
  const trim = report.measurements.memoryTrim;
  if (trim && trim.delta) console.log("  hide-trim: tree " + trim.visibleIdle.totalWorkingSetMiB + " -> " + trim.afterHideTrim.totalWorkingSetMiB + " MiB (" + trim.delta.totalDeltaMiB + " MiB), processes trimmed=" + trim.delta.trimmedCount);
  const itrim = report.measurements.islandTrim;
  if (itrim && itrim.delta) console.log("  island hide-trim: tree " + itrim.visible.totalWorkingSetMiB + " -> " + itrim.hiddenAfterTrim.totalWorkingSetMiB + " MiB (" + itrim.delta.totalDeltaMiB + " MiB), trimmed=" + itrim.delta.trimmedCount + ", pageWarm=" + itrim.islandPageStillPresent);
  const act = report.measurements.activation;
  if (act) {
    if (act.lazyFirstOpen) console.log("  lazy first island open (show_island cmd): invoke=" + act.lazyFirstOpen.invokeMs + "ms create+show=" + act.lazyFirstOpen.createShowMs + "ms shown->paint=" + act.lazyFirstOpen.shownToPaintMs + "ms total=" + act.lazyFirstOpen.commandToPaintMs + "ms pageFound=" + act.lazyFirstOpen.islandPageFound);
    console.log("  warm island opens (show_island cmd): invoke p50=" + act.warmInvokeMs.p50 + "ms create+show p50=" + act.warmCreateShowMs.p50 + "ms command->paint p50=" + act.warmCommandToPaintMs.p50 + "ms p95=" + act.warmCommandToPaintMs.p95 + "ms n=" + act.warmTrials);
  }
  const ofp = report.measurements.openFramePacing;
  if (ofp) console.log("  open frames: p50=" + ofp.aggregate.frameMs.p50 + "ms p95=" + ofp.aggregate.frameMs.p95 + "ms p99=" + ofp.aggregate.frameMs.p99 + " dropped=" + ofp.aggregate.droppedPct + "% longTasks=" + ofp.aggregate.longTasksTotal + " n=" + ofp.trials);
  const cfp = report.measurements.closeFramePacing;
  if (cfp) console.log("  close frames: p50=" + cfp.aggregate.frameMs.p50 + "ms p95=" + cfp.aggregate.frameMs.p95 + "ms dropped=" + cfp.aggregate.droppedPct + "% n=" + cfp.trials);
  const typing = report.measurements.typingLatency;
  if (typing) console.log("  island typing->results: p50=" + typing.ms.p50 + "ms p95=" + typing.ms.p95 + "ms n=" + typing.measured + "/" + typing.takes);
  const sw = report.measurements.cardSwitch;
  if (sw) console.log("  card switch: p50=" + sw.switchMs.p50 + "ms p95=" + sw.switchMs.p95 + "ms | layout=" + sw.perfAverages.layoutSec + "s recalc=" + sw.perfAverages.recalcSec + "s script=" + sw.perfAverages.scriptSec + "s resizeEvents/switch=" + sw.avgResizeEventsPerSwitch + " | attribution: " + sw.attribution);
  const ms = report.measurements.runtimeModeSwitch;
  if (ms) console.log("  runtime mode switch: updateSettings=" + ms.updateSettings + " hotkey visible=" + (ms.hotkey && ms.hotkey.win32VisibleMs) + "ms send->paint=" + (ms.hotkey && ms.hotkey.sendToPaintMs) + "ms islandPage=" + (ms.hotkey && ms.hotkey.islandPageFound) + (ms.error ? (" ERROR=" + ms.error) : ""));
  const lk = report.measurements.openCloseLeak;
  if (lk) console.log("  open/close leak: " + lk.cycles + "x growth=" + lk.growthMiB + " MiB, pages=" + lk.pagesAfter);
  if (report.blocked.length) console.log("  blocked: " + report.blocked.join(" | "));
  console.log("  results: " + outPath);
  if (report.status !== "ok") process.exitCode = 0;
}

function poolFrameSeries(series) {
  const withFrames = series.filter((s) => s && s.frames > 0);
  if (!withFrames.length) return { trials: series.length, frameMs: summarize([]), droppedFramesTotal: 0, droppedPct: 0, longTasksTotal: 0, layoutShiftTotal: 0, resizeEventsAvg: 0 };
  // Exact pooling when raw intervals are present; otherwise fall back to the mean
  // of per-trial percentiles (labelled).
  const pooled = [];
  for (const s of withFrames) { if (Array.isArray(s.intervalsMs)) pooled.push(...s.intervalsMs); }
  let frameMs;
  if (pooled.length) {
    frameMs = summarize(pooled);
  } else {
    const p50 = withFrames.map((s) => s.frameMs.p50).filter((v) => v != null);
    const p95 = withFrames.map((s) => s.frameMs.p95).filter((v) => v != null);
    const p99 = withFrames.map((s) => s.frameMs.p99).filter((v) => v != null);
    frameMs = { p50: +(p50.reduce((a, b) => a + b, 0) / Math.max(1, p50.length)).toFixed(3), p95: +(p95.reduce((a, b) => a + b, 0) / Math.max(1, p95.length)).toFixed(3), p99: +(p99.reduce((a, b) => a + b, 0) / Math.max(1, p99.length)).toFixed(3), note: "mean of per-trial percentiles (raw intervals unavailable)" };
  }
  const totalFrames = withFrames.reduce((a, s) => a + (s.frames || 0), 0);
  const droppedFramesTotal = withFrames.reduce((a, s) => a + (s.droppedFrames || 0), 0);
  return {
    trials: series.length,
    frames: totalFrames,
    frameMs,
    droppedFramesTotal,
    droppedPct: pooled.length ? +((droppedFramesTotal / Math.max(1, pooled.length)) * 100).toFixed(2) : +(withFrames.reduce((a, s) => a + (s.droppedPct || 0), 0) / withFrames.length).toFixed(2),
    overBudgetFramesTotal: withFrames.reduce((a, s) => a + (s.overBudgetFrames || 0), 0),
    overBudgetPct: pooled.length ? +((withFrames.reduce((a, s) => a + (s.overBudgetFrames || 0), 0) / Math.max(1, pooled.length)) * 100).toFixed(2) : null,
    longTasksTotal: withFrames.reduce((a, s) => a + ((s.longTasks && s.longTasks.count) || 0), 0),
    layoutShiftTotal: +withFrames.reduce((a, s) => a + ((s.layoutShift && s.layoutShift.cls) || 0), 0).toFixed(4),
    resizeEventsAvg: +(withFrames.reduce((a, s) => a + (s.resizeEvents || 0), 0) / withFrames.length).toFixed(2),
  };
}

main().catch((e) => { console.error("island-perf: " + (e && e.stack ? e.stack : e)); process.exit(1); });
