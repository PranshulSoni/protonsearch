#!/usr/bin/env node
// Show/hide leak probe for the release app (black box, no CDP). After startup settle it
// takes snapshot S0, runs --cycles real Alt+Space show -> hide cycles (hotkey-activate.ps1),
// and snapshots the app process tree every --snapshot-every cycles (private bytes, HandleCount
// sum, process count). Leak signals are private bytes and handles; working set is context only.
//
// Usage:
//   node tools/benchmark/island-leak.mjs --exe target/release/ProtonSearch.exe --data-dir .devdata --out docs/performance/results/island-leak.json [--profile island|normal] [--cycles 20] [--snapshot-every 5] [--settle-ms 15000] [--label x] [--kill-stale] [--no-perf-hooks]
// Leaves NO app running.

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, openSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const HOTKEY_PS = resolve(__dirname, "hotkey-activate.ps1");
const METRICS_PS = resolve(__dirname, "process-metrics.ps1");
const LOG_DIR = resolve(repoRoot, ".bench");

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
function gitHead() { try { return execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim(); } catch { return null; } }
function staleAppRunning() {
  let out = "";
  try { out = execFileSync("tasklist", ["/FI", "IMAGENAME eq ProtonSearch.exe", "/FO", "CSV", "/NH"], { windowsHide: true }).toString(); } catch (e) { out = String(e.stdout || ""); }
  return out.toLowerCase().includes('"protonsearch.exe"');
}

const args = parseArgs(process.argv.slice(2));
const need = (k) => { if (!args[k] || args[k] === true) throw new Error("missing --" + k); return args[k]; };
const exe = resolve(need("exe"));
const outPath = resolve(need("out"));
const dataDir = resolve(need("data-dir"));
const profile = args.profile || "island";
if (profile !== "island" && profile !== "normal") throw new Error("--profile must be island or normal");
const primaryMode = profile === "island" ? "dynamic_island" : "normal";
const cycles = Number(args.cycles || 20);
const snapshotEvery = Number(args["snapshot-every"] || 5);
const settleMs = Number(args["settle-ms"] || 15000);
const reportLabel = args.label || "island-leak";
const killStale = Boolean(args["kill-stale"]);
// --no-perf-hooks omits PROTONSEARCH_PERF_HOOKS: the perf-hook UI watchdog wakes the main thread ~100x/s, so use it for production-representative idle numbers.
const perfHooks = !args["no-perf-hooks"];
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");

function setPrimaryMode(mode) {
  const p = resolve(dataDir, "v2", "settings.json");
  if (!existsSync(p)) throw new Error("missing " + p + " (--data-dir must already hold v2/settings.json)");
  const data = JSON.parse(readFileSync(p, "utf8"));
  data.launcher.primaryMode = mode;
  writeFileSync(p, JSON.stringify(data, null, 2));
}
function truncateMarks() { writeFileSync(MARKS_FILE, ""); }
async function memorySnapshot(pid, tag) {
  if (!pid) return { label: tag, error: "no pid" };
  try {
    const m = await runPs(METRICS_PS, ["-ProcessId", String(pid), "-Mode", "memory"], 40000);
    return { label: tag, ...m, totalWorkingSetMiB: +(m.totalWorkingSetBytes / 1048576).toFixed(1), totalPrivateMiB: +(m.totalPrivateBytes / 1048576).toFixed(1) };
  } catch (e) { return { label: tag, error: String(e.message) }; }
}
function handleCount(pids) {
  const cmd = `(Get-Process -Id ${pids.join(",")} -ErrorAction SilentlyContinue | Measure-Object HandleCount -Sum).Sum`;
  return new Promise((res) => execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", cmd], { windowsHide: true, timeout: 30000 }, (err, stdout) => {
    const text = String(stdout || "").trim();
    const n = Number(text);
    res(err || text === "" || !Number.isFinite(n) ? null : n);
  }));
}
async function snapshot(pid, name, cycle) {
  const m = await memorySnapshot(pid, name);
  if (m.error) return { name, cycle, error: m.error };
  const handles = await handleCount(m.processes.map((p) => p.pid));
  return {
    name, cycle, processCount: m.processCount, wsMiB: m.totalWorkingSetMiB, privateMiB: m.totalPrivateMiB, handles,
    processes: m.processes.map((p) => ({ pid: p.pid, name: p.name, wsMiB: +(p.wsBytes / 1048576).toFixed(1), privateMiB: +(p.privateBytes / 1048576).toFixed(1) })),
  };
}
const delta = (a, b) => (a == null || b == null ? null : { abs: +(a - b).toFixed(1), pct: b ? +(((a - b) / b) * 100).toFixed(1) : null });

function analyze(snaps, rows) {
  const s0 = snaps[0];
  const snapshots = snaps.map((s, i) => (i === 0 || s.error ? s : {
    ...s,
    vsS0: { privateMiB: delta(s.privateMiB, s0.privateMiB), handles: delta(s.handles, s0.handles) },
    vsPrev: { privateMiB: delta(s.privateMiB, snaps[i - 1].privateMiB), handles: delta(s.handles, snaps[i - 1].handles) },
  }));
  const fin = snapshots[snapshots.length - 1];
  const vis = rows.filter((r) => r.visibleMs != null).map((r) => r.visibleMs);
  let drift = null;
  if (vis.length >= 10) {
    const firstP95 = summarize(vis.slice(0, 5)).p95;
    const lastP95 = summarize(vis.slice(-5)).p95;
    drift = { firstP95, lastP95, ratio: +(lastP95 / firstP95).toFixed(2), exceeds: lastP95 > firstP95 * 1.5 };
  }
  const privPct = fin.vsS0?.privateMiB?.pct ?? null;
  const handlePct = fin.vsS0?.handles?.pct ?? null;
  const counts = [...new Set(snaps.map((s) => s.processCount))];
  const reasons = [];
  if (privPct == null || privPct > 15) reasons.push(`private bytes ${privPct ?? "unavailable"}% vs S0 (limit 15%)`);
  if (handlePct == null || handlePct > 10) reasons.push(`handles ${handlePct ?? "unavailable"}% vs S0 (limit 10%)`);
  if (counts.length !== 1 || counts[0] !== s0.processCount) reasons.push(`process count changed: ${counts.join(", ")} (S0 ${s0.processCount})`);
  if (drift?.exceeds) reasons.push(`visibleMs p95 drift ${drift.ratio}x (last 5 vs first 5, limit 1.5x)`);
  return {
    verdict: reasons.length ? "possible-leak" : "no-leak-signal",
    verdictReasons: reasons,
    snapshots,
    timings: { visibleMs: summarize(vis), focusedMs: summarize(rows.filter((r) => r.focusedMs != null).map((r) => r.focusedMs)) },
    drift,
    cyclesTimedOut: rows.filter((r) => r.timedOut).length,
    cyclesVisibleBefore: rows.filter((r) => r.visibleBefore).length,
    cycles: rows,
  };
}

async function main() {
  mkdirSync(LOG_DIR, { recursive: true });
  mkdirSync(dirname(outPath), { recursive: true });
  const report = {
    schema: "ps-island-leak/1", kind: "island-hotkey-show-hide-leak", label: reportLabel, status: "running",
    capturedAt: new Date().toISOString(), exeSha256: null, conditions: {}, measurements: {}, blocked: [],
    limitations: [
      "Black box: no CDP. Cycles are real Alt+Space via SendInput (hotkey-activate.ps1): show waits for a new visible window of the app PID, send hides. Hide is not verified directly; cyclesVisibleBefore and cyclesTimedOut are reported but are not in the verdict.",
      "WS is volatile because of WebView2 trimming, so private bytes and handle counts are the leak signals; WS is context only.",
      "Tree = app PID plus descendants (msedgewebview2 children) via process-metrics.ps1; handles = sum of HandleCount over the same PIDs at snapshot time.",
      "Verdict uses the final snapshot vs S0: private <= 15 %, handles <= 10 %, process count equal in every snapshot, visibleMs p95 of last 5 <= 1.5x first 5 (drift needs >= 10 cycles). Heuristic; a slow leak beyond --cycles is not seen, so repeat at higher --cycles before concluding.",
      "Only the show/hide hotkey path is exercised (no search, clipboard or drill-down input).",
      "Writes <data-dir>/v2/settings.json launcher.primaryMode and truncates v2/perf-marks.jsonl; primaryMode is not restored (same as island-hotkey.mjs).",
    ],
  };
  const snaps = [], rows = [];
  let app = null;
  try {
    if (!existsSync(exe)) throw new Error("exe not found: " + exe);
    report.exeSha256 = createHash("sha256").update(readFileSync(exe)).digest("hex").slice(0, 16);
    report.conditions = { exe, exeBytes: statSync(exe).size, exeMtime: statSync(exe).mtime.toISOString(), dataDir, profile, primaryMode, perfHooks, cycles, snapshotEvery, settleMs, host: hostname(), gitHead: gitHead() };
    if (staleAppRunning()) {
      if (!killStale) throw new Error("ProtonSearch.exe already running; close it or pass --kill-stale");
      try { execFileSync("taskkill", ["/IM", "ProtonSearch.exe", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* already gone */ }
      await sleep(1500);
      if (staleAppRunning()) throw new Error("ProtonSearch.exe still running after taskkill");
    }
    setPrimaryMode(primaryMode);
    truncateMarks();
    mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
    const outFd = openSync(resolve(LOG_DIR, "island-leak-app-stdout.log"), "w");
    const errFd = openSync(resolve(LOG_DIR, "island-leak-app-stderr.log"), "w");
    const env = { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") };
    for (const k of Object.keys(env)) if (k.toUpperCase() === "PROTONSEARCH_PERF_HOOKS") delete env[k]; // drop an inherited value too (Windows env names are case-insensitive)
    if (perfHooks) env.PROTONSEARCH_PERF_HOOKS = "1";
    app = spawn(exe, [], { env, stdio: ["ignore", outFd, errFd] });
    report.conditions.appPid = app.pid;
    const alive = () => { if (app.exitCode !== null || app.signalCode !== null) throw new Error(`app exited early (code ${app.exitCode}, signal ${app.signalCode})`); };
    await sleep(settleMs);
    alive();
    snaps.push(await snapshot(app.pid, "S0", 0));
    for (let c = 1; c <= cycles; c++) {
      alive();
      const show = await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-Mode", "show", "-TimeoutMs", "8000"], 30000);
      rows.push({ cycle: c, visibleMs: show.visibleMs, focusedMs: show.focusedMs, timedOut: show.timedOut, visibleBefore: show.visibleBefore });
      await sleep(600);
      await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-Mode", "send"], 30000);
      await sleep(800);
      if (c % snapshotEvery === 0 || c === cycles) { await sleep(4000); alive(); snaps.push(await snapshot(app.pid, "cycle-" + c, c)); }
    }
    report.measurements = analyze(snaps, rows);
    report.status = "ok";
  } catch (e) {
    report.status = "blocked";
    report.blocked.push(String(e && e.stack ? e.stack : e));
    report.partial = { snapshots: snaps, cycles: rows };
  } finally {
    if (app) killTree(app.pid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  if (report.status === "ok") {
    const m = report.measurements, s0 = m.snapshots[0], f = m.snapshots[m.snapshots.length - 1];
    console.log(`island-leak: status=ok verdict=${m.verdict}`);
    console.log(`  private ${s0.privateMiB} -> ${f.privateMiB} MiB (${f.vsS0?.privateMiB?.pct ?? "n/a"}%), handles ${s0.handles} -> ${f.handles} (${f.vsS0?.handles?.pct ?? "n/a"}%), processes ${s0.processCount} -> ${f.processCount}`);
    console.log(`  visibleMs p50=${m.timings.visibleMs.p50} p95=${m.timings.visibleMs.p95} max=${m.timings.visibleMs.max}; cyclesTimedOut=${m.cyclesTimedOut} cyclesVisibleBefore=${m.cyclesVisibleBefore}`);
    for (const r of m.verdictReasons) console.log("  reason: " + r);
  } else {
    console.log("island-leak: status=blocked " + report.blocked[0].split("\n")[0]);
    process.exitCode = 1;
  }
  console.log("  wrote " + outPath);
}

main().catch((e) => { console.error("island-leak: " + (e && e.stack ? e.stack : e)); process.exit(1); });
