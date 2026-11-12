#!/usr/bin/env node
// Dynamic Island typing + open/close frame-pacing harness (release build, black box, no CDP).
// Companion to island-switch.mjs. Starts the app in the island-primary profile, waits for settle, then per
// trial summons the island (native-driver.ps1 hotkey), types a query into the search card, clears it and
// dismisses. Text is sent only while rects reports the island window as foreground; otherwise the trial is
// recorded as skipped. In-app marks (island-typing, island-open, island-close, island-window-shown,
// island-first-paint) are read from <dataDir>/v2/perf-marks.jsonl, written only with PROTONSEARCH_PERF_HOOKS=1.
//
// Usage:
//   node tools/benchmark/island-typing.mjs --exe <ProtonSearch.exe> --data-dir <dir> --out <result.json>
//        [--label x] [--trials 12] [--query readme] [--settle-ms 12000] [--kill-stale]
// Leaves NO app running (killTree in finally).

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync, mkdirSync, writeFileSync, readFileSync, statSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const DRIVER_PS = resolve(repoRoot, "docs", "qa", "g8-verify-round4", "native-driver.ps1");
const EXE_NAME = "ProtonSearch.exe";
const ISLAND_TITLE = "ProtonSearch Island";
const NO_MARKS_WARNING = "WARNING: no marks - app UI not loaded (a plain `cargo build --release` loads devUrl and shows a WebView2 error page; build with `npx tauri build --no-bundle` or `--features tauri/custom-protocol`), perf hooks missing from this build, or input not delivered";

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
// One driver call. native-driver.ps1 prints one JSON line; failures print {"error":...} and exit 1.
function runDriver(scriptArgs) {
  return new Promise((res, rej) => {
    execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", DRIVER_PS, ...scriptArgs], { windowsHide: true, timeout: 30000 }, (err, stdout, stderr) => {
      if (err) return rej(new Error("native-driver failed: " + (stderr || String(stdout).trim() || err.message)));
      const line = String(stdout).trim().split(/\r?\n/).filter(Boolean).pop();
      try { res(JSON.parse(line)); } catch { rej(new Error("bad driver JSON: " + line)); }
    });
  });
}
const drive = (...scriptArgs) => runDriver(["-Cmd", ...scriptArgs]);
function killTree(pid) { if (!pid) return; try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* gone */ } }
const isNum = (v) => typeof v === "number" && Number.isFinite(v);
const round = (v, d = 2) => (isNum(v) ? +v.toFixed(d) : null);
const sumOf = (arr) => arr.filter(isNum).reduce((a, b) => a + b, 0);
const stripBom = (s) => (s.charCodeAt(0) === 0xfeff ? s.slice(1) : s);

function appRunning() {
  try {
    const out = execFileSync("tasklist", ["/FI", `IMAGENAME eq ${EXE_NAME}`, "/FO", "CSV", "/NH"], { encoding: "utf8", windowsHide: true });
    return out.toLowerCase().includes(EXE_NAME.toLowerCase());
  } catch { return false; }
}
// "island-typing|ms=12.5" -> { prefix: "island-typing", fields: { ms: 12.5 } }
function parseMark(label) {
  const [head, ...rest] = String(label).split("|");
  const fields = {};
  for (const part of rest) {
    const eq = part.indexOf("=");
    if (eq <= 0) continue;
    const raw = part.slice(eq + 1);
    const num = raw === "" ? NaN : Number(raw);
    fields[part.slice(0, eq)] = Number.isFinite(num) ? num : raw;
  }
  return { prefix: head.split(":")[0], fields };
}
// Foreground check of the island window. Never throws: a failed probe reads as "not foreground", so nothing is typed.
async function islandWindow(pid) {
  try {
    const j = await drive("rects", "-ProcessId", String(pid));
    const list = Array.isArray(j) ? j : Object.values(j).find(Array.isArray) || [];
    const isl = list.filter((w) => String(w.title ?? "").includes(ISLAND_TITLE));
    return { present: isl.length > 0, foreground: isl.some((w) => w.foreground === true) };
  } catch (e) {
    return { present: false, foreground: false, error: String(e.message).split("\n")[0] };
  }
}
function frameSummary(rows) {
  return {
    marks: rows.length,
    p95OfMarks: summarize(rows.map((f) => f.p95).filter(isNum)),
    maxOfMarks: summarize(rows.map((f) => f.max).filter(isNum)),
    drop: sumOf(rows.map((f) => f.drop)),
    long: sumOf(rows.map((f) => f.long)),
  };
}
function setPrimaryMode(mode) {
  const data = JSON.parse(stripBom(readFileSync(SETTINGS_FILE, "utf8")));
  data.launcher = data.launcher || {};
  data.launcher.primaryMode = mode;
  writeFileSync(SETTINGS_FILE, JSON.stringify(data, null, 2));
}
function readMarks() {
  let text = "";
  try { text = readFileSync(MARKS_FILE, "utf8"); } catch { return []; }
  const marks = [];
  for (const line of stripBom(text).split(/\r?\n/)) { if (!line.trim()) continue; try { marks.push(JSON.parse(line)); } catch { /* partial line */ } }
  return marks.filter((m) => typeof m.label === "string" && isNum(m.epochMs));
}

const args = parseArgs(process.argv.slice(2));
if (typeof args.exe !== "string" || typeof args["data-dir"] !== "string" || typeof args.out !== "string") {
  console.error("usage: node tools/benchmark/island-typing.mjs --exe <ProtonSearch.exe> --data-dir <dir> --out <result.json> [--label x] [--trials 12] [--query readme] [--settle-ms 12000] [--kill-stale]");
  process.exit(2);
}
const exe = resolve(args.exe);
const dataDir = resolve(args["data-dir"]);
const outPath = resolve(args.out);
const label = typeof args.label === "string" ? args.label : "run";
const trialCount = Number(args.trials || 12);
const query = typeof args.query === "string" ? args.query : "readme";
const settleMs = Number(args["settle-ms"] || 12000);
const killStale = Boolean(args["kill-stale"]);
const SETTINGS_FILE = resolve(dataDir, "v2", "settings.json");
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");

const report = { schema: "ps-island-typing/1", label, exe, exeSha256: null, capturedAt: new Date().toISOString(), conditions: { trials: trialCount, query, settleMs, gitHead: null, host: hostname(), appPid: null }, status: "running", summary: null, trials: [], raw: null, limitations: [], blocked: [] };
let app = null;
let runStart = 0;

async function main() {
  if (!existsSync(exe)) throw new Error("exe not found: " + exe);
  if (!existsSync(SETTINGS_FILE)) throw new Error("settings.json not found at " + SETTINGS_FILE + " (run the app once to create the data dir)");
  if (appRunning()) {
    if (!killStale) throw new Error(EXE_NAME + " is already running; stop it or pass --kill-stale");
    try { execFileSync("taskkill", ["/IM", EXE_NAME, "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* gone */ }
    await sleep(1500);
    if (appRunning()) throw new Error(EXE_NAME + " still running after --kill-stale");
  }
  report.exeSha256 = createHash("sha256").update(readFileSync(exe)).digest("hex").slice(0, 16);
  report.conditions.exeBytes = statSync(exe).size;
  report.conditions.exeMtime = statSync(exe).mtime.toISOString();
  try { report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim(); } catch { /* not a checkout */ }

  setPrimaryMode("dynamic_island");
  writeFileSync(MARKS_FILE, "");
  mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
  runStart = Date.now();
  app = spawn(exe, [], { env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") }, detached: false, stdio: "ignore", windowsHide: true });
  app.on("error", () => {});
  report.conditions.appPid = app.pid;
  await sleep(settleMs);
  if (app.exitCode !== null) throw new Error("app exited during settle, exitCode=" + app.exitCode);

  let skipRun = 0;
  for (let i = 1; i <= trialCount; i++) {
    if (app.exitCode !== null) throw new Error("app exited mid-run, exitCode=" + app.exitCode);
    const rec = { i, skipped: false, startMs: Date.now() - runStart };
    await drive("hotkey"); // summon
    await sleep(800);
    const win = await islandWindow(app.pid);
    if (!win.foreground) {
      // Never type unless the island is foreground. Dismiss a visible but unfocused island so the next summon starts hidden.
      rec.skipped = true;
      rec.reason = win.error ? "rects failed: " + win.error : win.present ? "island visible but not foreground" : "island window not present";
      if (win.present) await drive("hotkey");
      await sleep(1000);
      rec.endMs = Date.now() - runStart;
      report.trials.push(rec);
      if (++skipRun === 3) { report.limitations.push("stopped early after 3 consecutive skipped trials"); break; }
      continue;
    }
    skipRun = 0;
    await drive("type", "-Text", query);
    await sleep(1200);
    const after = await islandWindow(app.pid);
    if (!after.foreground) {
      // Focus left the island mid-trial: withhold ctrl+a/backspace so they cannot edit another window.
      rec.reason = "island lost foreground after typing; cleanup keys withheld";
      rec.endMs = Date.now() - runStart;
      report.trials.push(rec);
      report.limitations.push("stopped: island lost foreground after typing");
      break;
    }
    await drive("keys", "-Keys", "ctrl+a");
    await drive("keys", "-Keys", "backspace");
    await sleep(500);
    await drive("hotkey"); // dismiss
    await sleep(1000);
    rec.endMs = Date.now() - runStart;
    report.trials.push(rec);
  }
  await sleep(500); // let the app flush its last perf marks

  const marks = readMarks();
  const parsed = marks.map((m) => ({ ...parseMark(m.label), epochMs: m.epochMs }));
  const of = (prefix) => parsed.filter((p) => p.prefix === prefix);
  const typing = of("island-typing");
  const opens = of("island-open");
  const closes = of("island-close");
  const shown = of("island-window-shown").map((p) => p.epochMs).sort((a, b) => a - b);
  const shownToPaint = of("island-first-paint").map((p) => { const start = shown.filter((t) => t <= p.epochMs).pop(); return start === undefined ? null : p.epochMs - start; }).filter(isNum);
  for (const r of report.trials) r.typingMarks = typing.filter((p) => p.epochMs >= runStart + r.startMs && p.epochMs <= runStart + r.endMs).length;
  report.summary = {
    trialsRun: report.trials.length,
    trialsSkipped: report.trials.filter((r) => r.skipped).length,
    typing: summarize(typing.map((p) => p.fields.ms).filter(isNum)),
    open: frameSummary(opens.map((p) => p.fields)),
    close: frameSummary(closes.map((p) => p.fields)),
    shownToPaint: summarize(shownToPaint),
  };
  report.raw = { marks: marks.map((m) => ({ label: m.label, relMs: round(m.epochMs - runStart) })) };
  report.limitations.push(
    "typing ms = in-app last keystroke -> results painted; the app may mark once per settled paint, so a trial can carry several (trials[].typingMarks)",
    "open/close pacing = in-app rAF frame marks (450 ms after summon, 250 ms before hide), not an external capture",
    "foreground check and keystrokes are separate PowerShell calls: the guard is not atomic",
    "first trial may include cold paths",
  );
  if (typing.length === 0 && opens.length === 0) {
    report.limitations.unshift(NO_MARKS_WARNING);
    console.log(NO_MARKS_WARNING);
    report.status = "invalid";
  } else {
    if (typing.length === 0) report.limitations.push("no island-typing marks: typing latency not measured");
    report.status = "ok";
  }

  const s = report.summary;
  const frameLine = (name, f) => `  ${name.padEnd(6)} marks=${f.marks} p95 p50/p95/max=${f.p95OfMarks.p50}/${f.p95OfMarks.p95}/${f.p95OfMarks.max} max p50/p95/max=${f.maxOfMarks.p50}/${f.maxOfMarks.p95}/${f.maxOfMarks.max} drop=${f.drop} long=${f.long}`;
  console.log(`island-typing: label=${label} trials=${s.trialsRun} skipped=${s.trialsSkipped} query=${query}`);
  console.log(`  typing ms  n=${s.typing.count} p50=${s.typing.p50} p95=${s.typing.p95} p99=${s.typing.p99} max=${s.typing.max} mean=${s.typing.mean}`);
  console.log(frameLine("open", s.open));
  console.log(frameLine("close", s.close));
  console.log(`  shown->first-paint ms n=${s.shownToPaint.count} p50=${s.shownToPaint.p50} p95=${s.shownToPaint.p95} max=${s.shownToPaint.max}`);
}

try {
  await main();
} catch (e) {
  report.status = "blocked";
  report.blocked.push(String(e && e.stack ? e.stack : e));
} finally {
  if (app) killTree(app.pid);
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(report, null, 2));
}
console.log(`island-typing: status=${report.status} out=${outPath}`);
if (report.status === "blocked") console.log(String(report.blocked[0]).split("\n")[0]);
if (report.status !== "ok") process.exitCode = 1;
