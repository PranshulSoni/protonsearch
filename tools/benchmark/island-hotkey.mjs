#!/usr/bin/env node
// Island-primary hotkey activation harness (task-57 companion to island-perf.mjs).
//
// The main island-perf suite runs an app in the normal primary-mode profile
// because switching launcher.primary_mode at runtime currently exits the app
// (update_settings destroys the inactive window; destroying the last window exits
// Tauri). This harness starts the app in an island-primary profile so the island
// window is prewarmed at startup, then measures the REAL global-hotkey path:
//   hotkey -> island-window-shown -> island-first-paint (plus Win32 visible),
// and the island-primary memory profile (prewarmed hidden / visible / hidden+trim),
// per-process breakdown.
//
// Usage:
//   node tools/benchmark/island-hotkey.mjs --exe target/release/ProtonSearch.exe --data-dir .devdata --out docs/performance/results/island-hotkey.json
// Leaves NO app running.

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
  throw new Error("playwright not found");
}
function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) { const a = argv[i]; if (!a.startsWith("--")) continue; const key = a.slice(2); const next = argv[i + 1]; if (next !== undefined && !next.startsWith("--")) { out[key] = next; i++; } else out[key] = true; }
  return out;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
function summarize(values) {
  if (!values.length) return { count: 0, p50: null, p95: null, p99: null, min: null, max: null, mean: null };
  const s = [...values].sort((a, b) => a - b);
  const pct = (p) => +s[Math.max(0, Math.min(s.length - 1, Math.ceil((p / 100) * s.length) - 1))].toFixed(3);
  return { count: s.length, p50: pct(50), p95: pct(95), p99: pct(99), min: +s[0].toFixed(3), max: +s[s.length - 1].toFixed(3), mean: +(s.reduce((a, b) => a + b, 0) / s.length).toFixed(3) };
}
function runPs(script, scriptArgs, timeoutMs) {
  return new Promise((res, rej) => {
    execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script, ...scriptArgs], { windowsHide: true, timeout: timeoutMs || 60000 }, (err, stdout, stderr) => {
      if (err) return rej(new Error("PS helper failed: " + (stderr || err.message)));
      const line = String(stdout).trim().split(/\r?\n/).filter(Boolean).pop();
      try { res(JSON.parse(line)); } catch { rej(new Error("bad PS JSON: " + line)); }
    });
  });
}
function killTree(pid) { if (!pid) return; try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* gone */ } }

const args = parseArgs(process.argv.slice(2));
const exe = resolve(args.exe || resolve(repoRoot, "target", "debug", "ProtonSearch.exe"));
const outPath = resolve(args.out || resolve(repoRoot, "docs", "performance", "results", "island-hotkey-" + new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "") + "Z.json"));
const dataDir = resolve(args["data-dir"] || resolve(repoRoot, ".devdata"));
const port = Number(args.port || 9222);
const takes = Number(args.takes || 10);
const trimWaitMs = Number(args["trim-wait-ms"] || 8000);
const idleSeconds = Number(args["idle-seconds"] || 10);
const noCdp = Boolean(args["no-cdp"]);
const HOTKEY_PS = resolve(__dirname, "hotkey-activate.ps1");
const METRICS_PS = resolve(__dirname, "process-metrics.ps1");
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");
let marksOffset = 0;
function truncateMarks() { try { writeFileSync(MARKS_FILE, ""); marksOffset = 0; } catch { /* ignore */ } }
function readMarksSince() {
  let text = ""; try { text = readFileSync(MARKS_FILE, "utf8"); } catch { return { marks: [] }; }
  const buf = Buffer.from(text); if (buf.length < marksOffset) marksOffset = 0;
  const slice = buf.slice(marksOffset).toString("utf8"); const marks = [];
  for (const line of slice.split(/\r?\n/)) { if (!line.trim()) continue; try { marks.push(JSON.parse(line)); } catch { /* partial */ } }
  marksOffset = buf.length; return { marks };
}
function markEpochs(marks, label) { return marks.filter((m) => m.label === label).map((m) => m.epochMs).sort((a, b) => a - b); }
function setPrimaryMode(mode) {
  const p = resolve(dataDir, "v2", "settings.json");
  const data = JSON.parse(readFileSync(p, "utf8"));
  data.launcher.primaryMode = mode;
  writeFileSync(p, JSON.stringify(data, null, 2));
}
async function memorySnapshot(pid, label) {
  if (!pid) return { label, error: "no pid" };
  try {
    const m = await runPs(METRICS_PS, ["-ProcessId", String(pid), "-Mode", "memory"], 40000);
    return { label, ...m, totalWorkingSetMiB: +(m.totalWorkingSetBytes / 1048576).toFixed(1), totalPrivateMiB: +(m.totalPrivateBytes / 1048576).toFixed(1) };
  } catch (e) { return { label, error: String(e.message) }; }
}
function compareSnapshots(before, after) {
  if (!before || !after || !before.processes || !after.processes) return null;
  const map = new Map();
  for (const p of before.processes) map.set(p.pid, p.wsBytes);
  const rows = [];
  for (const p of after.processes) {
    const prev = map.get(p.pid);
    if (prev == null) { rows.push({ pid: p.pid, name: p.name, wsBeforeMiB: null, wsAfterMiB: +(p.wsBytes / 1048576).toFixed(1), deltaMiB: null, trimmed: null }); continue; }
    const deltaMiB = +((p.wsBytes - prev) / 1048576).toFixed(1);
    rows.push({ pid: p.pid, name: p.name, wsBeforeMiB: +(prev / 1048576).toFixed(1), wsAfterMiB: +(p.wsBytes / 1048576).toFixed(1), deltaMiB, trimmed: deltaMiB <= -5 });
  }
  return { rows, trimmedCount: rows.filter((r) => r.trimmed).length, totalDeltaMiB: +((after.totalWorkingSetBytes - before.totalWorkingSetBytes) / 1048576).toFixed(1) };
}

const report = { schema: "ps-island-hotkey/1", kind: "island-hotkey-activation", status: "running", capturedAt: new Date().toISOString(), host: { hostname: hostname() }, conditions: {}, measurements: {}, blocked: [], limitations: [
  "App is started with launcher.primaryMode=dynamic_island so only the island window is created at startup (lazy main/alternate launcher). The island window exists but is hidden.",
  "Hotkey path is the real global shortcut (SendInput Alt+Space); marks give island-window-shown and island-first-paint on the same wall clock; Win32 visible uses EnumWindows and excludes already-visible windows.",
  "This profile cannot also measure the launcher-only idle tree; island-perf.mjs runs the normal profile for that.",
] };
let app = null;

// Release-build path: release WebView2 CDP is disabled by design, so this mode
// drives the real hotkey only (marks + Win32) and reads process memory via CIM.
async function releaseHotkeyRun() {
  await sleep(12000); // > delayed LOW trim (2.5 s) + startup settle
  report.measurements.memoryIdleIslandPrewarmedHidden = await memorySnapshot(app.pid, "island-prewarmed-hidden-after-12s");
  const trials = [];
  let visibleSnapshot = null;
  for (let i = 0; i < takes; i++) {
    const since = readMarksSince();
    let win = null;
    try { win = await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-TimeoutMs", "8000", "-Mode", "show"], 30000); } catch (e) { trials.push({ take: i, error: String(e.message) }); continue; }
    await sleep(300);
    const marks = readMarksSince().marks.concat(since.marks);
    const shown = markEpochs(marks, "island-window-shown").filter((v) => v >= ((win && win.sendAtUtc) || 0) - 400).slice(-1)[0] ?? null;
    const paint = markEpochs(marks, "island-first-paint").filter((v) => (shown == null ? true : v >= shown - 60)).slice(-1)[0] ?? null;
    trials.push({
      take: i,
      win32VisibleMs: win && win.visibleMs != null ? win.visibleMs : null,
      win32FocusedMs: win && win.focusedMs != null ? win.focusedMs : null,
      sendToShownMs: shown != null && win ? +(shown - win.sendAtUtc).toFixed(2) : null,
      shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
      sendToPaintMs: shown != null && paint != null && win ? +(paint - win.sendAtUtc).toFixed(2) : null,
      timedOut: win ? win.timedOut : null,
    });
    if (i === 0) { await sleep(600); visibleSnapshot = await memorySnapshot(app.pid, "island-visible-after-hotkey"); }
    try { await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-TimeoutMs", "6000", "-Mode", "send"], 25000); } catch (e) { /* ignore */ }
    await sleep(1300);
  }
  report.measurements.memoryIslandVisible = visibleSnapshot;
  report.measurements.hotkeyActivation = {
    trials: trials.length,
    mode: "release-hotkey-only (no CDP; marks + Win32 EnumWindows)",
    visibleMs: summarize(trials.map((t) => t.win32VisibleMs).filter((v) => v != null)),
    focusedMs: summarize(trials.map((t) => t.win32FocusedMs).filter((v) => v != null)),
    sendToShownMs: summarize(trials.map((t) => t.sendToShownMs).filter((v) => v != null)),
    shownToPaintMs: summarize(trials.map((t) => t.shownToPaintMs).filter((v) => v != null)),
    sendToPaintMs: summarize(trials.map((t) => t.sendToPaintMs).filter((v) => v != null)),
    samples: trials,
    note: "Island window is prewarmed at startup (island-primary profile) and hidden; each trial is real hotkey show -> hotkey dismiss. Marks island-window-shown/island-first-paint are written by the release build too (PROTONSEARCH_PERF_HOOKS=1).",
  };
  await sleep(trimWaitMs);
  const hiddenTrim = await memorySnapshot(app.pid, "island-hidden-after-hotkey-trim-wait");
  report.measurements.visibleVsHiddenTrim = {
    waitMs: trimWaitMs,
    visible: visibleSnapshot,
    hiddenTrim,
    delta: visibleSnapshot ? compareSnapshots(visibleSnapshot, hiddenTrim) : null,
  };
  report.status = "ok";
}

async function main() {
  const pw = loadPlaywright();
  if (!existsSync(exe)) throw new Error("exe not found: " + exe);
  mkdirSync(dirname(outPath), { recursive: true }); mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
  report.conditions = { exe, exeBytes: statSync(exe).size, exeMtime: statSync(exe).mtime.toISOString(), dataDir, port, takes };
  try { report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim(); report.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length; } catch { /* ignore */ }
  try {
    setPrimaryMode("dynamic_island");
    truncateMarks();
    const logDir = resolve(repoRoot, ".bench"); mkdirSync(logDir, { recursive: true });
    const errFd = openSync(resolve(logDir, "island-hotkey-app-stderr.log"), "w");
    const outFd = openSync(resolve(logDir, "island-hotkey-app-stdout.log"), "w");
    app = spawn(exe, [], { env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", PROTONSEARCH_CDP_PORT: String(port), WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") }, stdio: ["ignore", outFd, errFd] });
    report.conditions.appPid = app.pid;
    if (noCdp) { await releaseHotkeyRun(); return; }
    let browser = null;
    const deadline = Date.now() + 60000;
    while (!browser && Date.now() < deadline) { try { browser = await pw.chromium.connectOverCDP("http://127.0.0.1:" + port); } catch { await sleep(500); } }
    if (!browser) throw new Error("could not connect to CDP");
    const pages = browser.contexts().flatMap((c) => c.pages());
    let island = null;
    for (const page of pages) { try { const kind = await page.evaluate(() => document.documentElement.dataset.window || null); if (kind === "island") island = page; } catch { /* ignore */ } }
    if (!island) throw new Error("island page not found at startup (island-primary profile)");
    report.conditions.initialPages = pages.length;
    await island.waitForLoadState("domcontentloaded", { timeout: 5000 }).catch(() => {});
    await island.evaluate(async () => {
      const deadline = Date.now() + 180000;
      while (Date.now() < deadline) {
        try { const status = await window.__TAURI_INTERNALS__.invoke("get_status"); if (status && (status.state === "ready" || status.state === "paused") && status.filenameEntries > 0) return; } catch { /* starting */ }
        await new Promise((r) => setTimeout(r, 500));
      }
    }).catch(() => {});
    report.measurements.indexReady = await island.evaluate(async () => { try { const s = await window.__TAURI_INTERNALS__.invoke("get_status"); return { state: s.state, entries: s.filenameEntries }; } catch (e) { return { error: String(e) }; } });

    // Memory: prewarmed island hidden -> visible after hotkey -> hidden+trim.
    report.measurements.memoryIslandPrewarmedHidden = await memorySnapshot(app.pid, "island-prewarmed-hidden");
    report.measurements.idleCpu = await runPs(METRICS_PS, ["-ProcessId", String(app.pid), "-Mode", "cpu", "-SampleSeconds", String(idleSeconds)], (idleSeconds + 40) * 1000).catch((e) => ({ error: String(e.message) }));
    await sleep(trimWaitMs);
    const prewarmedTrim = await memorySnapshot(app.pid, "island-prewarmed-hidden-after-trim-wait");
    report.measurements.islandPrewarmedTrim = { waitMs: trimWaitMs, start: report.measurements.memoryIslandPrewarmedHidden, afterWait: prewarmedTrim, delta: compareSnapshots(report.measurements.memoryIslandPrewarmedHidden, prewarmedTrim) };

    const trials = [];
    for (let i = 0; i < takes; i++) {
      try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ }
      await sleep(450);
      const since = readMarksSince();
      let win = null;
      try { win = await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-TimeoutMs", "8000", "-Mode", "show"], 30000); } catch (e) { trials.push({ take: i, error: String(e.message) }); continue; }
      await sleep(300);
      const marks = readMarksSince().marks.concat(since.marks);
      const shown = markEpochs(marks, "island-window-shown").filter((v) => v >= (win.sendAtUtc || 0) - 400).slice(-1)[0] ?? null;
      const paint = markEpochs(marks, "island-first-paint").filter((v) => (shown == null ? true : v >= shown - 60)).slice(-1)[0] ?? null;
      trials.push({
        take: i,
        win32VisibleMs: win && win.visibleMs != null ? win.visibleMs : null,
        win32FocusedMs: win && win.focusedMs != null ? win.focusedMs : null,
        sendToShownMs: shown != null ? +(shown - win.sendAtUtc).toFixed(2) : null,
        shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
        sendToPaintMs: paint != null ? +(paint - win.sendAtUtc).toFixed(2) : null,
        timedOut: win ? win.timedOut : null,
      });
      if (i === 0) {
        report.measurements.memoryIslandVisible = await memorySnapshot(app.pid, "island-visible-after-hotkey");
        report.measurements.islandVisibleVsPrewarmedTrim = { visible: report.measurements.memoryIslandVisible, prewarmedAfterTrim: prewarmedTrim, delta: compareSnapshots(prewarmedTrim, report.measurements.memoryIslandVisible) };
        await sleep(1500);
      }
      readMarksSince();
      await sleep(200);
    }
    report.measurements.hotkeyActivation = {
      trials: trials.length,
      visibleMs: summarize(trials.map((t) => t.win32VisibleMs).filter((v) => v != null)),
      focusedMs: summarize(trials.map((t) => t.win32FocusedMs).filter((v) => v != null)),
      sendToShownMs: summarize(trials.map((t) => t.sendToShownMs).filter((v) => v != null)),
      shownToPaintMs: summarize(trials.map((t) => t.shownToPaintMs).filter((v) => v != null)),
      sendToPaintMs: summarize(trials.map((t) => t.sendToPaintMs).filter((v) => v != null)),
      samples: trials,
      note: "Island window is prewarmed (created at startup) but hidden; the hotkey (primary mode island, island enabled) calls toggle_island -> show_island_window. Lazy WebView creation is NOT included here; island-perf.mjs first show_island covers that.",
    };
    // Hide and measure trim after a hotkey show -> hide cycle.
    try { await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")); } catch { /* ignore */ }
    await sleep(1200);
    const hiddenImmediate = await memorySnapshot(app.pid, "island-hidden-immediate");
    await sleep(trimWaitMs);
    const hiddenTrim = await memorySnapshot(app.pid, "island-hidden-after-hotkey-trim-wait");
    report.measurements.visibleVsHiddenTrim = { waitMs: trimWaitMs, visible: report.measurements.memoryIslandVisible, hiddenImmediate, hiddenTrim, delta: compareSnapshots(report.measurements.memoryIslandVisible, hiddenTrim) };
    report.status = "ok";
  } catch (e) {
    report.status = "blocked"; report.blocked.push(String(e && e.stack ? e.stack : e));
  } finally {
    killTree(app ? app.pid : report.conditions.appPid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  console.log("island-hotkey: status=" + report.status);
  const a = report.measurements.hotkeyActivation;
  if (a) console.log("  hotkey->island: visible p50=" + a.visibleMs.p50 + " p95=" + a.visibleMs.p95 + "ms | send->shown p50=" + a.sendToShownMs.p50 + " p95=" + a.sendToShownMs.p95 + "ms | shown->paint p50=" + a.shownToPaintMs.p50 + " | send->paint p50=" + a.sendToPaintMs.p50 + "ms n=" + a.trials);
  const mp = report.measurements.memoryIslandPrewarmedHidden;
  if (mp && mp.totalWorkingSetMiB != null) console.log("  island-prewarmed hidden tree: " + mp.totalWorkingSetMiB + " MiB WS / " + mp.totalPrivateMiB + " MiB private across " + mp.processCount + " processes");
  const vt = report.measurements.visibleVsHiddenTrim;
  if (vt && vt.delta) console.log("  hotkey visible->hidden+trim: " + vt.visible.totalWorkingSetMiB + " -> " + vt.hiddenTrim.totalWorkingSetMiB + " MiB (" + vt.delta.totalDeltaMiB + "), trimmed=" + vt.delta.trimmedCount);
  if (report.blocked.length) console.log("  blocked: " + report.blocked.join(" | "));
  console.log("  results: " + outPath);
}
main().catch((e) => { console.error("island-hotkey: " + (e && e.stack ? e.stack : e)); process.exit(1); });
