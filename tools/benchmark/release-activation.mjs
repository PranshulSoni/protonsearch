#!/usr/bin/env node
// Release-build black-box launcher activation baseline. OWNER: perf-bench (L1-C).
//
// CDP is debug-only by design, so a RELEASE build cannot be instrumented from the
// page. This harness measures the user-visible hotkey path only:
//   - launch the release exe with a portable PROTONSEARCH_DATA_DIR
//   - send the real global hotkey (Alt+Space) with SendInput (PowerShell helper)
//   - poll top-level Windows (EnumWindows) for the launcher becoming visible and
//     foreground, with ~1ms resolution
//   - toggle it hidden again via a second hotkey send between trials
//   - 30 trials, first summon labelled cold (process cold; window is premade, so
//     cold/warm is expected to be close - recorded honestly)
//   - process-tree memory + 60 s idle CPU via PowerShell/CIM after trials
//
// Writes JSON under docs/performance/results/ with exe mtime/size, git HEAD/dirty,
// display/DPI facts and every per-trial sample. No page paint or IPC numbers are
// claimed from a release build.

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { existsSync, mkdirSync, writeFileSync, statSync, readFileSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");

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
const args = parseArgs(process.argv.slice(2));
const exe = resolve(args.exe || resolve(repoRoot, "target", "release", "ProtonSearch.exe"));
const outPath = resolve(args.out || resolve(repoRoot, "docs", "performance", "results", "release-activation-" + utcTag() + ".json"));
const dataDir = resolve(args["data-dir"] || resolve(repoRoot, ".devdata"));
const trials = Number(args.trials || 30);
const idleSeconds = Number(args["idle-seconds"] || 60);
const setupWaitMs = Number(args["setup-wait-ms"] || 6000);
const HOTKEY_PS = resolve(__dirname, "hotkey-activate.ps1");
const METRICS_PS = resolve(__dirname, "process-metrics.ps1");
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");
let marksOffset = 0;

function truncateMarks() {
  try { writeFileSync(MARKS_FILE, ""); marksOffset = 0; } catch { /* ignore */ }
}

// Read perf-marks.jsonl entries appended since the previous call. The Rust shell
// writes {label, epochMs, writtenAt} JSONL lines when PROTONSEARCH_PERF_HOOKS=1;
// labels include launcher-window-shown / island-window-shown and the frontend
// first-paint marks (launcher-first-paint / island-first-paint).
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

function utcTag() {
  return new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "") + "Z";
}
function sleep(ms) { return new Promise((r) => setTimeout(r, ms)); }
function percentile(s, p) {
  if (!s.length) return null;
  const i = Math.ceil((p / 100) * s.length) - 1;
  return +s[Math.max(0, Math.min(s.length - 1, i))].toFixed(3);
}
function summarize(values) {
  if (!values.length) return { count: 0, p50: null, p95: null, p99: null, min: null, max: null, mean: null };
  const s = [...values].sort((a, b) => a - b);
  return {
    count: s.length,
    p50: percentile(s, 50),
    p95: percentile(s, 95),
    p99: percentile(s, 99),
    min: +s[0].toFixed(3),
    max: +s[s.length - 1].toFixed(3),
    mean: +(s.reduce((a, b) => a + b, 0) / s.length).toFixed(3),
  };
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
function killTree(pid) {
  if (!pid) return;
  try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch { /* gone */ }
}

const report = {
  schema: "ps-release-activation/1",
  kind: "release-launcher-activation",
  status: "running",
  capturedAt: new Date().toISOString(),
  host: { hostname: hostname(), platform: process.platform },
  conditions: {},
  measurements: {},
  blocked: [],
  limitations: [
    "Release build: CDP is debug-only by design, so no page paint / IPC timing is claimed. Only the user-visible hotkey->visible+foreground path is measured.",
    "Single 120 Hz / 150% DPI internal display (2880x1800 panel, 1920x1200 logical), no second monitor: multi-monitor placement is unmeasured.",
    "Window is pre-created hidden, so cold (first summon after process start) and warm summons are expected to be close; both are reported honestly.",
    "SendInput injection via PowerShell helper; Win32 EnumWindows polling at ~1ms resolution; other agents may be active on the machine.",
  ],
};

let app = null;
async function main() {
  if (!existsSync(exe)) throw new Error("release exe not found: " + exe);
  mkdirSync(dirname(outPath), { recursive: true });
  mkdirSync(dataDir, { recursive: true });
  mkdirSync(resolve(dataDir, "webview2"), { recursive: true });
  const distIndex = resolve(repoRoot, "apps", "desktop", "dist", "index.html");
  report.conditions = {
    exe, exeBytes: statSync(exe).size, exeMtime: statSync(exe).mtime.toISOString(),
    frontendDistMtime: existsSync(distIndex) ? statSync(distIndex).mtime.toISOString() : null,
    dataDir, trials, idleSeconds,
  };
  try {
    report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim();
    report.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length;
  } catch { /* ignore */ }
  report.conditions.display = await runPs(resolve(__dirname, "display-info.ps1"), [], 30000).catch((e) => ({ error: String(e.message) }));

  try {
    truncateMarks();
    app = spawn(exe, [], {
      env: {
        ...process.env,
        PROTONSEARCH_DATA_DIR: dataDir,
        PROTONSEARCH_PERF_HOOKS: "1",
        WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2"),
      },
      stdio: "ignore",
    });
    report.conditions.appPid = app.pid;
    await sleep(setupWaitMs);

    // Probe: no send, validate helper/window detection (windows are hidden -> expect visibleBefore false)
    const probe = await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-TimeoutMs", "400", "-Mode", "probe"], 20000).catch((e) => ({ error: String(e.message) }));
    report.measurements.probe = probe;

    const samples = [];
    for (let i = 0; i < trials; i++) {
      const t = await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-TimeoutMs", "3000", "-Mode", "show"], 20000).catch((e) => ({ error: String(e.message), take: i }));
      const since = readMarksSince();
      const shown = since.marks.filter((m) => m.label === "launcher-window-shown" && m.epochMs >= (t.sendAtUtc ?? 0) - 300).map((m) => m.epochMs).sort((a, b) => a - b).slice(-1)[0] ?? null;
      const paint = since.marks.filter((m) => m.label === "launcher-first-paint" && (shown == null || m.epochMs >= shown - 50)).map((m) => m.epochMs).sort((a, b) => a - b).slice(-1)[0] ?? null;
      const markDeltas = {
        sendToShownMs: shown != null ? +(shown - t.sendAtUtc).toFixed(2) : null,
        shownToPaintMs: shown != null && paint != null ? +(paint - shown).toFixed(2) : null,
        sendToPaintMs: paint != null ? +(paint - t.sendAtUtc).toFixed(2) : null,
        labels: since.marks.map((m) => m.label),
      };
      samples.push({ take: i, cold: i === 0, visibleMs: t.visibleMs ?? null, focusedMs: t.focusedMs ?? null, timedOut: t.timedOut ?? null, title: t.title ?? null, markDeltas, error: t.error || null });
      // toggle hidden again through the same hotkey (send-only), then settle
      await runPs(HOTKEY_PS, ["-ProcessId", String(app.pid), "-Mode", "send"], 15000).catch(() => {});
      await sleep(450);
    }
    const cold = samples.filter((s) => s.cold);
    const warm = samples.filter((s) => !s.cold);
    const visibleAll = samples.map((s) => s.visibleMs).filter((v) => v != null);
    const focusedAll = samples.map((s) => s.focusedMs).filter((v) => v != null);
    report.measurements.activation = {
      trials,
      measuredVisible: visibleAll.length,
      measuredFocused: focusedAll.length,
      visibleMs: summarize(visibleAll),
      focusedMs: summarize(focusedAll),
      cold: { visibleMs: summarize(cold.map((s) => s.visibleMs).filter((v) => v != null)), focusedMs: summarize(cold.map((s) => s.focusedMs).filter((v) => v != null)) },
      warm: { visibleMs: summarize(warm.map((s) => s.visibleMs).filter((v) => v != null)), focusedMs: summarize(warm.map((s) => s.focusedMs).filter((v) => v != null)) },
      samples,
    };

    const sendToShown = samples.map((s) => s.markDeltas?.sendToShownMs).filter((v) => v != null);
    const shownToPaint = samples.map((s) => s.markDeltas?.shownToPaintMs).filter((v) => v != null);
    const sendToPaint = samples.map((s) => s.markDeltas?.sendToPaintMs).filter((v) => v != null);
    report.measurements.activationMarks = {
      measuredSendToShown: sendToShown.length,
      measuredShownToPaint: shownToPaint.length,
      measuredSendToPaint: sendToPaint.length,
      sendToShownMs: summarize(sendToShown),
      shownToPaintMs: summarize(shownToPaint),
      sendToPaintMs: summarize(sendToPaint),
      note: "Rust perf hooks: launcher-window-shown is SystemTime epoch at show; launcher-first-paint is the frontend double-rAF mark after the perf:show event. Harness SendInput time is the t0 reference on the same wall clock (ms resolution). Hotkey invoke epoch is carried in the perf:show event payload but not persisted to JSONL.",
    };

    // idle: launcher toggled hidden; sample memory and 60 s CPU
    report.measurements.memoryIdle = await runPs(METRICS_PS, ["-ProcessId", String(app.pid), "-Mode", "memory"], 30000).catch((e) => ({ error: String(e.message) }));
    report.measurements.idleCpu = await runPs(METRICS_PS, ["-ProcessId", String(app.pid), "-Mode", "cpu", "-SampleSeconds", String(idleSeconds)], (idleSeconds + 40) * 1000).catch((e) => ({ error: String(e.message) }));
    if (report.measurements.memoryIdle && report.measurements.memoryIdle.totalWorkingSetBytes != null) {
      report.measurements.memoryIdle.totalWorkingSetMiB = +(report.measurements.memoryIdle.totalWorkingSetBytes / (1024 * 1024)).toFixed(1);
      report.measurements.memoryIdle.totalPrivateMiB = +(report.measurements.memoryIdle.totalPrivateBytes / (1024 * 1024)).toFixed(1);
    }
    report.status = "ok";
  } catch (e) {
    report.status = "blocked";
    report.blocked.push(String(e && e.stack ? e.stack : e));
  } finally {
    killTree(app ? app.pid : report.conditions.appPid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  const a = report.measurements.activation;
  console.log("release activation: status=" + report.status);
  if (a) {
    console.log("  visible: p50=" + a.visibleMs.p50 + "ms p95=" + a.visibleMs.p95 + "ms p99=" + a.visibleMs.p99 + "ms n=" + a.measuredVisible + "/" + a.trials);
    console.log("  focused: p50=" + a.focusedMs.p50 + "ms p95=" + a.focusedMs.p95 + "ms n=" + a.measuredFocused);
    console.log("  cold visible p50=" + a.cold.visibleMs.p50 + "ms / warm p50=" + a.warm.visibleMs.p50 + "ms");
  }
  const am = report.measurements.activationMarks;
  if (am && am.measuredSendToShown > 0) {
    console.log("  marks send->shown: p50=" + am.sendToShownMs.p50 + "ms p95=" + am.sendToShownMs.p95 + "ms; shown->first-paint: p50=" + am.shownToPaintMs.p50 + "ms p95=" + am.shownToPaintMs.p95 + "ms; send->paint p95=" + am.sendToPaintMs.p95 + "ms (n=" + am.measuredSendToPaint + ")");
  }
  const m = report.measurements.memoryIdle;
  if (m && m.totalWorkingSetMiB != null) console.log("  idle memory: " + m.totalWorkingSetMiB + " MiB WS (" + m.processCount + " processes)");
  const c = report.measurements.idleCpu;
  if (c && c.cpuPctOfMachine != null) console.log("  idle CPU: " + c.cpuPctOfMachine + "% machine / " + c.cpuPctOfOneCore + "% one core over " + c.sampleSeconds + "s");
  if (report.blocked.length) console.log("  blocked: " + report.blocked.join(" | "));
  console.log("  results: " + outPath);
}
main().catch((e) => { console.error("release-activation: " + (e && e.stack ? e.stack : e)); process.exit(1); });
