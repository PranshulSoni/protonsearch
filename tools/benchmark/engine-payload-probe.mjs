#!/usr/bin/env node
// Engine payload / IPC / React split probe (task-64). Debug build + CDP, island-primary
// profile (prewarmed island renders in debug; lazily-created debug islands can be
// about:blank). Measures per-prefix keystroke typing latency and per-card open latency,
// plus the shell-visible SearchResponse payload size and engine wall time via a direct
// invoke from the page (the app's own invoke is captured at module import, so a
// wrapper cannot intercept it).
//
// Usage:
//   node tools/benchmark/engine-payload-probe.mjs --exe target/debug/ProtonSearch.exe --data-dir .devdata --out docs/performance/results/task64-payload-split.json
// Leaves no app running.
import { createRequire } from "node:module";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { mkdirSync, writeFileSync, readFileSync, openSync, existsSync, statSync } from "node:fs";
import { spawn, execFileSync, execFile } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const req = createRequire(resolve(repoRoot, "tools/visual-qa/package.json"));
const { chromium } = req("playwright");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const args = {};
for (let i = 2; i < process.argv.length; i++) { const a = process.argv[i]; if (!a.startsWith("--")) continue; const k = a.slice(2); const v = process.argv[i + 1]; if (v && !v.startsWith("--")) { args[k] = v; i++; } else args[k] = true; }
const exe = resolve(args.exe || resolve(repoRoot, "target", "debug", "ProtonSearch.exe"));
const outPath = resolve(args.out || resolve(repoRoot, "docs", "performance", "results", "task64-payload-split.json"));
const dataDir = resolve(args["data-dir"] || resolve(repoRoot, ".devdata"));
const PREFIXES = ["r", "re", "rea", "read", "readm", "readme", "s", "sc", "scre", "screenshot", "p", "pro", "project"];
const sleepP = (ms) => new Promise((r) => setTimeout(r, ms));
function killTree(pid) { if (!pid) return; try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch (e) { /* gone */ } }
function runPs(script, a, timeoutMs) { return new Promise((res, rej) => { execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script, ...a], { windowsHide: true, timeout: timeoutMs || 60000 }, (err, so, se) => { if (err) return rej(new Error(String(se || err))); try { res(JSON.parse(String(so).trim().split(/\r?\n/).filter(Boolean).pop())); } catch (e) { rej(new Error("bad json")); } }); }); }
const METRICS = resolve(repoRoot, "tools/benchmark/process-metrics.ps1");
const MARKS = resolve(dataDir, "v2", "perf-marks.jsonl");
let marksOffset = 0;
function truncateMarks() { try { writeFileSync(MARKS, ""); marksOffset = 0; } catch (e) { /* ignore */ } }
function readNewMarks() {
  let text = ""; try { text = readFileSync(MARKS, "utf8"); } catch (e) { return []; }
  const buf = Buffer.from(text); if (buf.length < marksOffset) marksOffset = 0;
  const slice = buf.slice(marksOffset).toString("utf8"); marksOffset = buf.length;
  const out = [];
  for (const line of slice.split(/\r?\n/)) { if (!line.trim()) continue; try { out.push(JSON.parse(line)); } catch (e) { /* partial */ } }
  return out;
}
function memorySnapshot(pid, label) { return runPs(METRICS, ["-ProcessId", String(pid), "-Mode", "memory"], 40000).then((m) => ({ label, totalWorkingSetMiB: +(m.totalWorkingSetBytes / 1048576).toFixed(1), totalPrivateMiB: +(m.totalPrivateBytes / 1048576).toFixed(1), processCount: m.processCount })).catch((e) => ({ label, error: String(e.message) })); }
function setPrimary(mode) { const p = resolve(dataDir, "v2", "settings.json"); const d = JSON.parse(readFileSync(p, "utf8")); d.launcher.primaryMode = mode; writeFileSync(p, JSON.stringify(d, null, 2)); }
function summarize(v) { if (!v.length) return { count: 0, p50: null, p95: null, p99: null, min: null, max: null }; const s = [...v].sort((a, b) => a - b); const at = (p) => +s[Math.max(0, Math.min(s.length - 1, Math.ceil((p / 100) * s.length) - 1))].toFixed(2); return { count: s.length, p50: at(50), p95: at(95), p99: at(99), min: +s[0].toFixed(2), max: +s[s.length - 1].toFixed(2) }; }
const report = { schema: "ps-task64-payload/1", kind: "engine-payload-ipc-react-split", status: "running", capturedAt: new Date().toISOString(), host: { hostname: hostname() }, conditions: { exe, exeBytes: existsSync(exe) ? statSync(exe).size : null, exeMtime: existsSync(exe) ? statSync(exe).mtime.toISOString() : null, dataDir, prefixes: PREFIXES, profile: "island-primary (debug + CDP)" }, measurements: {}, blocked: [], limitations: [
  "Shell does not log raw SearchResponse bytes; payloadChars is JSON.stringify(response).length from a direct page-side invoke (the same object the app receives after IPC parse), so it is a lower bound for wire bytes (no framing) and an upper-bound proxy for parse input size.",
  "The app's own search invoke is captured at module import, so engineWall for the app's call is not interceptable; directInvoke wall/took_ms are measured with an identical request on the same page (same engine/DB/cache state).",
  "Typing rows: each prefix typed into an empty island search input via the native value setter + input event (single input event per prefix, like a paste of the prefix), then wait for the result signature to change + double rAF.",
  "Card rows: Ctrl+ArrowDown-driven card switches from the search card; open->rendered = keydown dispatch -> heading text changes -> double rAF; long tasks observed during the window.",
] };
let app = null; let browser = null;
try {
  setPrimary("dynamic_island");
  truncateMarks();
  mkdirSync(dirname(outPath), { recursive: true }); mkdirSync(resolve(dataDir, "webview2"), { recursive: true }); mkdirSync(resolve(repoRoot, ".bench"), { recursive: true });
  try { report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim(); report.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length; } catch (e) { /* ignore */ }
  const errFd = openSync(resolve(repoRoot, ".bench", "payload-probe-stderr.log"), "w");
  const outFd = openSync(resolve(repoRoot, ".bench", "payload-probe-stdout.log"), "w");
  app = spawn(exe, [], { env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", PROTONSEARCH_CDP_PORT: "9222", WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") }, stdio: ["ignore", outFd, errFd] });
  report.conditions.appPid = app.pid;
  for (let i = 0; i < 80 && !browser; i++) { try { browser = await chromium.connectOverCDP("http://127.0.0.1:9222", { timeout: 1500 }); } catch (e) { await sleep(400); } }
  if (!browser) throw new Error("no CDP");
  let island = null;
  for (let i = 0; i < 100 && !island; i++) {
    for (const p of browser.contexts().flatMap((c) => c.pages())) { try { const k = await p.evaluate(() => document.documentElement.dataset.window || null); if (k === "island") island = p; } catch (e) { /* nav */ } }
    if (!island) await sleep(300);
  }
  if (!island) throw new Error("no island page");
  await island.waitForLoadState("domcontentloaded", { timeout: 5000 }).catch(() => {});
  await island.evaluate(async () => {
    const deadline = Date.now() + 180000;
    while (Date.now() < deadline) { try { const s = await window.__TAURI_INTERNALS__.invoke("get_status"); if (s && s.filenameEntries > 0) return; } catch (e) { /* starting */ } await new Promise((r) => setTimeout(r, 500)); }
  }).catch(() => {});
  report.measurements.indexReady = await island.evaluate(async () => { try { const s = await window.__TAURI_INTERNALS__.invoke("get_status"); return { state: s.state, entries: s.filenameEntries }; } catch (e) { return { error: String(e) }; } });
  report.measurements.coldMemory = await memorySnapshot(app.pid, "cold-idle");

  // Long task + event listener recorder on the page.
  await island.evaluate(() => {
    window.__T64__ = { longtasks: [], frames: [], reset() { this.longtasks = []; this.frames = []; } };
    try {
      const po = new PerformanceObserver((list) => { for (const e of list.getEntries()) window.__T64__.longtasks.push({ start: e.startTime, duration: e.duration }); });
      po.observe({ type: "longtask", buffered: true });
    } catch (e) { /* unsupported */ }
  });
  // Open the island through the UI path (prewarmed hidden island).
  await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island"));
  await sleep(900);

  // ---- per-prefix typing rows ----
  const typingRows = [];
  for (const prefix of PREFIXES) {
    const row = { prefix };
    try {
      // direct invoke measurement for engine wall + took_ms + payload size
      const direct = await island.evaluate(async (q) => {
        try {
          const t0 = performance.now();
          let requestId = 0;
          const response = await window.__TAURI_INTERNALS__.invoke("search", { req: { protocol: 1, requestId: ++requestId, query: q, scope: "all", filters: {}, limit: 30 } });
          const wall = performance.now() - t0;
          const json = JSON.stringify(response);
          const results = Array.isArray(response.results) ? response.results : [];
          let iconChars = 0; let actionChars = 0; let nullCount = 0;
          const walk = (v) => { if (v === null) { nullCount += 1; return; } if (Array.isArray(v)) { for (const x of v) walk(x); return; } if (typeof v === "object") { for (const k of Object.keys(v)) walk(v[k]); return; } };
          for (const r of results) {
            if (r.iconDataUrl) iconChars += String(r.iconDataUrl).length;
            if (r.action) actionChars += JSON.stringify(r.action).length;
          }
          walk(response);
          return { wallMs: +wall.toFixed(2), tookMs: response && typeof response.tookMs === "number" ? response.tookMs : null, payloadChars: json.length, resultCount: results.length, iconChars, actionChars, nullCount };
        } catch (e) { return { error: String(e) }; }
      }, prefix);
      Object.assign(row, { directInvoke: direct });
      // app typing path
      const typed = await island.evaluate(async (q) => {
        const input = document.querySelector(".island-search-input") || document.querySelector("input");
        if (!input) return { error: "no input" };
        input.focus();
        const resultsEl = document.querySelector("#island-search-results, .island-results");
        const signature = () => (resultsEl ? resultsEl.textContent || "" : "").slice(0, 60);
        const setValue = (v) => { const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; setter.call(input, v); input.dispatchEvent(new Event("input", { bubbles: true })); };
        setValue("");
        await new Promise((r) => setTimeout(r, 120));
        window.__T64__.reset();
        const baseline = signature();
        const t0 = performance.now();
        setValue(q);
        const deadline = performance.now() + 4000;
        let changedAt = null;
        while (performance.now() < deadline) {
          if (signature() !== baseline) { changedAt = performance.now(); break; }
          await new Promise((r) => setTimeout(r, 4));
        }
        if (changedAt != null) await new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res)));
        const visibleAt = performance.now();
        return { start: t0, changedAt, visibleAt, inputToChangedMs: changedAt != null ? +(changedAt - t0).toFixed(2) : null, inputToVisibleMs: changedAt != null ? +(visibleAt - t0).toFixed(2) : null, longtasks: window.__T64__.longtasks.slice() };
      }, prefix).catch((e) => ({ error: String(e) }));
      Object.assign(row, typed);
      row.longTaskCount = typed.longtasks ? typed.longtasks.length : null;
      row.longTaskTotalMs = typed.longtasks ? +typed.longtasks.reduce((a, t) => a + t.duration, 0).toFixed(1) : null;
      row.longTaskBlockingMs = typed.longtasks ? +typed.longtasks.reduce((a, t) => a + Math.max(0, t.duration - 50), 0).toFixed(1) : null;
      row.marks = readNewMarks().map((m) => m.label);
      typingRows.push(row);
      await sleep(120);
    } catch (e) { typingRows.push({ prefix, error: String(e) }); }
  }
  report.measurements.typing = { rows: typingRows, inputToVisibleMs: summarize(typingRows.map((r) => r.inputToVisibleMs).filter((v) => v != null)), inputToChangedMs: summarize(typingRows.map((r) => r.inputToChangedMs).filter((v) => v != null)), directWallMs: summarize(typingRows.map((r) => r.directInvoke && r.directInvoke.wallMs).filter((v) => v != null)), payloadChars: summarize(typingRows.map((r) => r.directInvoke && r.directInvoke.payloadChars).filter((v) => v != null)) };

  // ---- steady-state typing sequences (real lag scenario: char-by-char) ----
  const steadyStates = [];
  for (const seq of [{ q: "readme", chars: ["r", "re", "rea", "read", "readm", "readme"] }, { q: "screenshot", chars: ["s", "sc", "scr", "scre", "scree", "screen", "screensh", "screenshot"] }]) {
    try {
      const ss = await island.evaluate(async (chars) => {
        const input = document.querySelector(".island-search-input") || document.querySelector("input");
        if (!input) return { error: "no input" };
        input.focus();
        const resultsEl = document.querySelector("#island-search-results, .island-results");
        const signature = () => (resultsEl ? resultsEl.textContent || "" : "").slice(0, 80);
        const setValue = (v) => { const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; setter.call(input, v); input.dispatchEvent(new Event("input", { bubbles: true })); };
        setValue("");
        await new Promise((r) => setTimeout(r, 300));
        window.__T64__.reset();
        const rows = [];
        let value = "";
        for (const ch of chars) {
          value = ch;
          const baseline = signature();
          const t0 = performance.now();
          setValue(value);
          let changedAt = null;
          const deadline = performance.now() + 500;
          while (performance.now() < deadline) {
            if (signature() !== baseline) { changedAt = performance.now(); break; }
            await new Promise((r) => setTimeout(r, 3));
          }
          await new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res)));
          const visibleAt = performance.now();
          rows.push({ value, inputToChangedMs: changedAt != null ? +(changedAt - t0).toFixed(2) : null, inputToVisibleMs: +(visibleAt - t0).toFixed(2), resultOptions: document.querySelectorAll("#island-search-results [role=option], .island-results [role=option]").length });
          await new Promise((r) => setTimeout(r, 50));
        }
        return { rows, longtasks: window.__T64__.longtasks.slice() };
      }, seq.chars).catch((e) => ({ error: String(e) }));
      if (ss.rows) {
        ss.longTaskCount = ss.longtasks.length;
        ss.longTaskTotalMs = +ss.longtasks.reduce((a, t) => a + t.duration, 0).toFixed(1);
        ss.longTaskBlockingMs = +ss.longtasks.reduce((a, t) => a + Math.max(0, t.duration - 50), 0).toFixed(1);
        ss.marks = readNewMarks().map((m) => m.label);
      }
      ss.query = seq.q;
      steadyStates.push(ss);
      await sleep(200);
    } catch (e) { steadyStates.push({ query: seq.q, error: String(e) }); }
  }
  report.measurements.steadyStateTyping = steadyStates;

  // ---- per-card open rows ----
  const cardRows = [];
  const cards = [{ name: "Search", presses: 0 }, { name: "Media", presses: 1 }, { name: "Clipboard", presses: 2 }, { name: "Agents", presses: 3 }, { name: "Quick Settings", presses: 4 }, { name: "Notifications", presses: 5 }, { name: "Browser", presses: 6 }, { name: "Git", presses: 7 }];
  for (const card of cards) {
    try {
      // fresh open resets the island to compact search, then press Ctrl+Down N times
      await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("hide_island")).catch(() => {});
      await sleep(300);
      await island.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island")).catch(() => {});
      await sleep(800);
      const rowMarksBefore = readNewMarks();
      const row = await island.evaluate(async (n) => {
        window.__T64__.reset();
        const headingOf = () => document.querySelector(".island-card-heading")?.textContent?.trim() || "";
        let lastMeasured = null;
        const perPress = [];
        for (let k = 0; k < n; k++) {
          const before = headingOf();
          const t0 = performance.now();
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", code: "ArrowDown", ctrlKey: true, bubbles: true, cancelable: true }));
          const deadline = performance.now() + 800;
          while (performance.now() < deadline && headingOf() === before) await new Promise((r) => setTimeout(r, 3));
          await new Promise((res) => requestAnimationFrame(() => requestAnimationFrame(res)));
          const ms = performance.now() - t0;
          perPress.push({ switchIndex: k, headingBefore: before, headingAfter: headingOf(), ms: +ms.toFixed(2) });
          lastMeasured = +ms.toFixed(2);
          await new Promise((r) => setTimeout(r, 300));
        }
        const heading = headingOf();
        const renderedAt = performance.now();
        const options = document.querySelectorAll("#island-search-results [role=option], .island-results [role=option]").length;
        return { presses: n, heading, renderedAt, ms: lastMeasured != null ? lastMeasured : 0, perPress, resultOptions: options, longtasks: window.__T64__.longtasks.slice() };
      }, card.presses).catch((e) => ({ error: String(e) }));
      if (row.longtasks) { row.longTaskCount = row.longtasks.length; row.longTaskTotalMs = +row.longtasks.reduce((a, t) => a + t.duration, 0).toFixed(1); }
      row.card = card.name;
      row.marks = readNewMarks().map((m) => m.label);
      cardRows.push(row);
      await sleep(250);
    } catch (e) { cardRows.push({ card: card.name, error: String(e) }); }
  }
  report.measurements.cards = { rows: cardRows, ms: summarize(cardRows.map((r) => r.ms).filter((v) => v != null)) };
  report.measurements.memoryAfterProbe = await memorySnapshot(app.pid, "after-probe");
  report.status = "ok";
} catch (e) { report.status = "blocked"; report.blocked.push(String(e && e.stack ? e.stack : e)); }
finally {
  if (browser) { try { await browser.close(); } catch (e) { /* ignore */ } }
  killTree(app ? app.pid : report.conditions.appPid);
  writeFileSync(outPath, JSON.stringify(report, null, 2));
}
console.log("payload-probe: status=" + report.status);
if (report.measurements.typing) {
  console.log("  typing input->visible: " + JSON.stringify(report.measurements.typing.inputToVisibleMs));
  console.log("  direct engine wall: " + JSON.stringify(report.measurements.typing.directWallMs) + " payloadChars: " + JSON.stringify(report.measurements.typing.payloadChars));
  console.log("  per-prefix rows: " + JSON.stringify(report.measurements.typing.rows.map((r) => ({ p: r.prefix, vis: r.inputToVisibleMs, ch: r.inputToChangedMs, wall: r.directInvoke && r.directInvoke.wallMs, took: r.directInvoke && r.directInvoke.tookMs, bytes: r.directInvoke && r.directInvoke.payloadChars, icons: r.directInvoke && r.directInvoke.iconChars, actions: r.directInvoke && r.directInvoke.actionChars, nulls: r.directInvoke && r.directInvoke.nullCount, long: r.longTaskCount, blocking: r.longTaskBlockingMs, marks: r.marks }))));
  if (report.measurements.steadyStateTyping) console.log("  steady-state: " + JSON.stringify(report.measurements.steadyStateTyping.map((s2) => ({ q: s2.q || s2.query, rows: s2.rows && s2.rows.map((r2) => ({ v: r2.value, ms: r2.inputToVisibleMs, ch: r2.inputToChangedMs })), long: s2.longTaskCount, blocking: s2.longTaskBlockingMs, err: s2.error }))));
}
if (report.measurements.cards) console.log("  card switch latency: " + JSON.stringify(report.measurements.cards.ms) + " rows=" + JSON.stringify(report.measurements.cards.rows.map((r) => ({ c: r.card, ms: r.ms, perPress: r.perPress && r.perPress.map((p) => p.ms), heading: r.heading, long: r.longTaskCount }))));
if (report.blocked.length) console.log("  blocked: " + report.blocked.join(" | "));
console.log("  results: " + outPath);
