#!/usr/bin/env node
// Idle memory curve for the release app (black box, no CDP, no input). Starts the app,
// samples the process tree rooted at the app PID every --interval-s from spawn for
// --duration-s, then takes one 20 s idle CPU sample. Budget metric = process-tree working set
// (ProtonSearch + msedgewebview2 children), per docs/performance/V10_BENCHMARK_PLAN.md T9.
//
// Usage:
//   node tools/benchmark/idle-curve.mjs --exe target/release/ProtonSearch.exe --data-dir .devdata --out docs/performance/results/idle-curve.json [--profile island|normal] [--duration-s 120] [--interval-s 5] [--budget-mib 150] [--label x] [--kill-stale] [--no-perf-hooks]
// Leaves NO app running.

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, openSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const METRICS_PS = resolve(__dirname, "process-metrics.ps1");
const LOG_DIR = resolve(repoRoot, ".bench");

function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) { const a = argv[i]; if (!a.startsWith("--")) continue; const key = a.slice(2); const next = argv[i + 1]; if (next !== undefined && !next.startsWith("--")) { out[key] = next; i++; } else out[key] = true; }
  return out;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
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
const durationS = Number(args["duration-s"] || 120);
const intervalS = Number(args["interval-s"] || 5);
if (!(intervalS > 0)) throw new Error("--interval-s must be > 0");
const budgetMib = Number(args["budget-mib"] || 150);
const reportLabel = args.label || "idle-curve";
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

function analyze(series, idleCpu) {
  const ok = series.filter((s) => s.wsMiB != null);
  if (!ok.length) throw new Error("no successful memory samples");
  const peak = ok.reduce((a, b) => (b.wsMiB > a.wsMiB ? b : a));
  const last = ok[ok.length - 1];
  let lastOver = -1;
  ok.forEach((s, i) => { if (s.wsMiB > budgetMib) lastOver = i; });
  return {
    budgetMiB: budgetMib,
    peakWorkingSetMiB: peak.wsMiB,
    peakAtSeconds: peak.t,
    finalWorkingSetMiB: last.wsMiB,
    finalPrivateMiB: last.privateMiB,
    finalAtSeconds: last.t,
    settleSeconds: lastOver === ok.length - 1 ? null : ok[lastOver + 1].t,
    withinBudgetAtEnd: last.wsMiB <= budgetMib,
    idleCpu,
    series,
  };
}

async function main() {
  mkdirSync(LOG_DIR, { recursive: true });
  mkdirSync(dirname(outPath), { recursive: true });
  const report = {
    schema: "ps-idle-curve/1", kind: "idle-memory-curve", label: reportLabel, status: "running",
    capturedAt: new Date().toISOString(), exeSha256: null, conditions: {}, measurements: {}, blocked: [],
    limitations: [
      "Budget metric is process-tree working set (ProtonSearch + msedgewebview2 children, rooted at the app PID) per docs/performance/V10_BENCHMARK_PLAN.md T9; the default 150 MiB is that plan's investigation budget. Private bytes are context only.",
      "Black box: no CDP and no input. The app is only started and sampled; no window is toggled, so this is the idle state.",
      "WebView2 trims working sets while hidden, so the curve steps down; settleSeconds is resolved to one --interval-s sample.",
      "Default 120 s covers startup and the first trim window only; the T9 idle state after longer idle is not sampled.",
      "Idle CPU is one 20 s sample taken after the curve ends (process-metrics.ps1 -Mode cpu).",
      "Writes <data-dir>/v2/settings.json launcher.primaryMode and truncates v2/perf-marks.jsonl; primaryMode is not restored (same as island-hotkey.mjs).",
    ],
  };
  const series = [];
  let app = null;
  try {
    if (!existsSync(exe)) throw new Error("exe not found: " + exe);
    report.exeSha256 = createHash("sha256").update(readFileSync(exe)).digest("hex").slice(0, 16);
    report.conditions = { exe, exeBytes: statSync(exe).size, exeMtime: statSync(exe).mtime.toISOString(), dataDir, profile, primaryMode, perfHooks, durationS, intervalS, budgetMib, host: hostname(), gitHead: gitHead() };
    if (staleAppRunning()) {
      if (!killStale) throw new Error("ProtonSearch.exe already running; close it or pass --kill-stale");
      try { execFileSync("taskkill", ["/IM", "ProtonSearch.exe", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* already gone */ }
      await sleep(1500);
      if (staleAppRunning()) throw new Error("ProtonSearch.exe still running after taskkill");
    }
    setPrimaryMode(primaryMode);
    truncateMarks();
    mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
    const outFd = openSync(resolve(LOG_DIR, "idle-curve-app-stdout.log"), "w");
    const errFd = openSync(resolve(LOG_DIR, "idle-curve-app-stderr.log"), "w");
    const env = { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") };
    for (const k of Object.keys(env)) if (k.toUpperCase() === "PROTONSEARCH_PERF_HOOKS") delete env[k]; // drop an inherited value too (Windows env names are case-insensitive)
    if (perfHooks) env.PROTONSEARCH_PERF_HOOKS = "1";
    app = spawn(exe, [], { env, stdio: ["ignore", outFd, errFd] });
    report.conditions.appPid = app.pid;
    const alive = () => { if (app.exitCode !== null || app.signalCode !== null) throw new Error(`app exited early (code ${app.exitCode}, signal ${app.signalCode})`); };
    const t0 = Date.now();
    for (let k = 0; k * intervalS <= durationS; k++) {
      await sleep(Math.max(0, t0 + k * intervalS * 1000 - Date.now()));
      alive();
      const at = +((Date.now() - t0) / 1000).toFixed(1);
      const m = await memorySnapshot(app.pid, "t" + at);
      series.push(m.error ? { t: at, error: m.error } : { t: at, wsMiB: m.totalWorkingSetMiB, privateMiB: m.totalPrivateMiB, processCount: m.processCount });
    }
    const idleCpu = await runPs(METRICS_PS, ["-ProcessId", String(app.pid), "-Mode", "cpu", "-SampleSeconds", "20"], 60000).catch((e) => ({ error: String(e.message) }));
    report.measurements = analyze(series, idleCpu);
    report.status = "ok";
  } catch (e) {
    report.status = "blocked";
    report.blocked.push(String(e && e.stack ? e.stack : e));
    report.partial = { series };
  } finally {
    if (app) killTree(app.pid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  if (report.status === "ok") {
    const m = report.measurements;
    console.log(`idle-curve: status=ok peakWS=${m.peakWorkingSetMiB} MiB at ${m.peakAtSeconds}s; final WS=${m.finalWorkingSetMiB} MiB private=${m.finalPrivateMiB} MiB at ${m.finalAtSeconds}s`);
    console.log(`  settleSeconds=${m.settleSeconds} withinBudgetAtEnd=${m.withinBudgetAtEnd} (budget ${m.budgetMiB} MiB)`);
    console.log(`  idle CPU ${m.idleCpu.cpuPctOfOneCore ?? "n/a"}% of one core, ${m.idleCpu.cpuPctOfMachine ?? "n/a"}% of machine`);
  } else {
    console.log("idle-curve: status=blocked " + report.blocked[0].split("\n")[0]);
    process.exitCode = 1;
  }
  console.log("  wrote " + outPath);
}

main().catch((e) => { console.error("idle-curve: " + (e && e.stack ? e.stack : e)); process.exit(1); });
