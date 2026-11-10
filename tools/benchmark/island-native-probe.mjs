#!/usr/bin/env node
// Native release island probe (task-61): no CDP. Drives the real hotkey and
// SendKeys, reads perf marks written by the AFTER-build frontend instrumentation
// (island-open / island-switch:* / island-close / island-typing / ui-stall),
// and snapshots the process tree memory (cold, hidden-after-use, after switches,
// 20-cycle growth). Built for ONE session per measurement set.
//
// Usage:
//   node tools/benchmark/island-native-probe.mjs --exe target/release/ProtonSearch.exe --out docs/performance/results/island-native-after-<hash>.json
// Leaves NO app running.

import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { existsSync, mkdirSync, writeFileSync, readFileSync, openSync, statSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { hostname } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
function parseArgs(argv) {
  const out = {};
  for (let i = 0; i < argv.length; i++) { const a = argv[i]; if (!a.startsWith("--")) continue; const key = a.slice(2); const next = argv[i + 1]; if (next !== undefined && !next.startsWith("--")) { out[key] = next; i++; } else out[key] = true; }
  return out;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const args = parseArgs(process.argv.slice(2));
const exe = resolve(args.exe || resolve(repoRoot, "target", "release", "ProtonSearch.exe"));
const outPath = resolve(args.out || resolve(repoRoot, "docs", "performance", "results", "island-native-" + new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "") + "Z.json"));
const dataDir = resolve(args["data-dir"] || resolve(repoRoot, ".devdata"));
const cycles = Number(args.cycles || 20);
const trimWaitMs = Number(args["trim-wait-ms"] || 6000);
const coldWaitMs = Number(args["cold-wait-ms"] || 12000);
const HOTKEY = resolve(__dirname, "hotkey-activate.ps1");
const METRICS = resolve(__dirname, "process-metrics.ps1");
const SENDKEYS = resolve(repoRoot, ".bench", "sendkeys.ps1");
const CLICK = resolve(repoRoot, ".bench", "click.ps1");
const MARKS = resolve(dataDir, "v2", "perf-marks.jsonl");
let marksOffset = 0;

function truncateMarks() { try { writeFileSync(MARKS, ""); marksOffset = 0; } catch (e) { /* ignore */ } }
function readMarksSince() {
  let text = ""; try { text = readFileSync(MARKS, "utf8"); } catch (e) { return []; }
  const buf = Buffer.from(text); if (buf.length < marksOffset) marksOffset = 0;
  const slice = buf.slice(marksOffset).toString("utf8"); marksOffset = buf.length;
  const marks = [];
  for (const line of slice.split(/\r?\n/)) { if (!line.trim()) continue; try { marks.push(JSON.parse(line)); } catch (e) { /* partial */ } }
  return marks;
}
function parseFrameMark(label) {
  const i = label.indexOf("|");
  const name = i < 0 ? label : label.slice(0, i);
  const kv = {};
  if (i >= 0) for (const part of label.slice(i + 1).split("|")) { const [k, v] = part.split("="); if (k) kv[k] = v != null && v !== "" && !isNaN(Number(v)) ? Number(v) : v; }
  return { name, kv };
}
function runPs(script, scriptArgs, timeoutMs) {
  return new Promise((res, rej) => {
    execFile("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script, ...scriptArgs], { windowsHide: true, timeout: timeoutMs || 60000 }, (err, stdout, stderr) => {
      if (err) return rej(new Error("PS failed: " + (stderr || err.message)));
      const line = String(stdout).trim().split(/\r?\n/).filter(Boolean).pop();
      try { res(JSON.parse(line)); } catch (e) { rej(new Error("bad PS JSON: " + line)); }
    });
  });
}
function killTree(pid) { if (!pid) return; try { execFileSync("taskkill", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true }); } catch (e) { /* gone */ } }
function memorySnapshot(pid, label) {
  return runPs(METRICS, ["-ProcessId", String(pid), "-Mode", "memory"], 40000).then((m) => ({ label, ...m, totalWorkingSetMiB: +(m.totalWorkingSetBytes / 1048576).toFixed(1), totalPrivateMiB: +(m.totalPrivateBytes / 1048576).toFixed(1) })).catch((e) => ({ label, error: String(e.message) }));
}
function hotkey(pid, mode, timeoutMs) {
  return runPs(HOTKEY, ["-ProcessId", String(pid), "-TimeoutMs", String(timeoutMs || 6000), "-Mode", mode || "show"], 30000);
}
function sendKeys(keys) { return runPs(SENDKEYS, ["-Keys", keys], 15000).catch(() => ({ error: "sendkeys" })); }
function clickAt(x, y) { return runPs(CLICK, ["-X", String(Math.round(x)), "-Y", String(Math.round(y))], 15000).catch(() => ({ error: "click" })); }
function summarize(values) {
  if (!values.length) return { count: 0, p50: null, p95: null, p99: null, min: null, max: null };
  const s = [...values].sort((a, b) => a - b);
  const at = (p) => +s[Math.max(0, Math.min(s.length - 1, Math.ceil((p / 100) * s.length) - 1))].toFixed(2);
  return { count: s.length, p50: at(50), p95: at(95), p99: at(99), min: +s[0].toFixed(2), max: +s[s.length - 1].toFixed(2) };
}
function setPrimary(mode) {
  const p = resolve(dataDir, "v2", "settings.json");
  const d = JSON.parse(readFileSync(p, "utf8"));
  d.launcher.primaryMode = mode;
  writeFileSync(p, JSON.stringify(d, null, 2));
}
function pidTreeRoot() { return report.conditions.appPid; }

const report = {
  schema: "ps-island-native/1",
  kind: "island-native-release-before-after",
  status: "running",
  capturedAt: new Date().toISOString(),
  host: { hostname: hostname(), platform: process.platform },
  conditions: { exe, exeBytes: existsSync(exe) ? statSync(exe).size : null, exeMtime: existsSync(exe) ? statSync(exe).mtime.toISOString() : null, dataDir, cycles, trimWaitMs, coldWaitMs },
  measurements: {},
  blocked: [],
  limitations: [
    "Native RELEASE run: no CDP. Frame stats come from the app's own perf marks (perfProbe.recordFrames: island-open/island-switch:*/island-close) written when PROTONSEARCH_PERF_HOOKS=1; labels carry n/p50/p95/p99/max/drop(>1.5x median)/long(>50ms).",
    "Card switching is driven by real SendKeys Ctrl+ArrowDown on the focused island (same handler as user input); typing by SendKeys into the focused search input.",
    "Layout/Recalc/Script attribution is NOT available natively; the BEFORE build has no frame marks at all, so BEFORE frame pacing is measured in the debug build via CDP and labelled debug. AFTER native attribution is inferred from drop/long counts + ui-stall marks.",
    "Memory is CIM process-tree Working Set + private bytes with command lines; WebView2 roles from command lines.",
  ],
};
let app = null;

async function main() {
  if (!existsSync(exe)) throw new Error("exe not found: " + exe);
  mkdirSync(dirname(outPath), { recursive: true }); mkdirSync(resolve(dataDir, "webview2"), { recursive: true }); mkdirSync(resolve(repoRoot, ".bench"), { recursive: true });
  try { report.conditions.gitHead = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot }).toString().trim(); report.conditions.gitDirty = execFileSync("git", ["status", "--porcelain"], { cwd: repoRoot }).toString().trim().split(/\r?\n/).filter(Boolean).length; } catch (e) { /* ignore */ }
  try {
    setPrimary("dynamic_island");
    truncateMarks();
    const errFd = openSync(resolve(repoRoot, ".bench", "native-probe-stderr.log"), "w");
    const outFd = openSync(resolve(repoRoot, ".bench", "native-probe-stdout.log"), "w");
    app = spawn(exe, [], { env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") }, stdio: ["ignore", outFd, errFd] });
    report.conditions.appPid = app.pid;
    await sleep(coldWaitMs);
    report.measurements.memoryColdIdle = await memorySnapshot(app.pid, "cold-idle-island-prewarmed-hidden");
    const marksCold = readMarksSince();
    report.measurements.coldMarks = marksCold;

    // Open via real hotkey.
    const openStart = Date.now();
    const openWin = await hotkey(app.pid, "show", 8000).catch((e) => ({ error: String(e.message) }));
    await sleep(1500);
    const openMarks = readMarksSince();
    report.measurements.open = {
      win32VisibleMs: openWin && openWin.visibleMs != null ? openWin.visibleMs : null,
      marks: openMarks.map((m) => m.label),
      frame: openMarks.map((m) => parseFrameMark(m.label)).find((p) => p.name === "island-open") || null,
      windowShown: openMarks.filter((m) => m.label === "island-window-shown").length,
    };
    report.measurements.memoryVisible = await memorySnapshot(app.pid, "visible-after-open");

    // Typing latency inside the island search card. Click the compact search
    // area first: SendKeys goes to the focused control, and after a hotkey show
    // focus is on the window, not guaranteed to be the input.
    const winRect = openWin && openWin.rect ? openWin.rect : null;
    if (winRect) await clickAt(winRect.left + winRect.width / 2, winRect.top + 145);
    await sleep(300);
    const beforeTyping = readMarksSince();
    await sendKeys("arch");
    await sleep(700);
    const typingMarks = readMarksSince().concat(beforeTyping).filter((m) => m.label.startsWith("island-typing"));
    report.measurements.typing = { marks: typingMarks.map((m) => m.label), ms: typingMarks.map((m) => { const mm = m.label.match(/ms=([0-9.]+)/); return mm ? +mm[1] : null; }).filter((v) => v != null) };

    // Card switch sweep: Ctrl+ArrowDown through the ordered cards.
    const sweep = [];
    const beforeSweep = readMarksSince();
    for (let i = 0; i < 4; i++) {
      await sendKeys("^{DOWN}");
      await sleep(1000);
    }
    const sweepMarks = readMarksSince().concat(beforeSweep).filter((m) => m.label.startsWith("island-switch:"));
    for (const m of sweepMarks) {
      const parsed = parseFrameMark(m.label);
      sweep.push({ switch: parsed.name, stats: parsed.kv, epochMs: m.epochMs });
    }
    report.measurements.cardSwitchSweep = sweep;

    // Esc chain: expanded card -> compact search -> hide (two presses when a
    // non-search card is active; harmless extra press if already compact).
    const beforeClose = readMarksSince();
    await sendKeys("{ESC}");
    await sleep(300);
    await sendKeys("{ESC}");
    await sleep(1000);
    const closeMarks = readMarksSince().concat(beforeClose);
    report.measurements.close = {
      frame: closeMarks.map((m) => parseFrameMark(m.label)).find((p) => p.name === "island-close") || null,
      marks: closeMarks.map((m) => m.label).filter((l) => l.startsWith("island-") || l.startsWith("ui-stall")),
    };
    await sleep(trimWaitMs);
    report.measurements.memoryHiddenAfterUseTrim = await memorySnapshot(app.pid, "hidden-after-use-trim-wait");

    // 20x open/close/switch growth.
    const growth = [];
    growth.push({ cycle: 0, ...(await memorySnapshot(app.pid, "growth-base")) });
    for (let c = 1; c <= cycles; c++) {
      await hotkey(app.pid, "show", 8000).catch(() => {});
      await sleep(700);
      for (let k = 0; k < 2; k++) { await sendKeys("^{DOWN}"); await sleep(450); }
      await sendKeys("{ESC}"); await sleep(300);
      await sendKeys("{ESC}"); await sleep(800);
      if (c % 5 === 0 || c === cycles) growth.push({ cycle: c, ...(await memorySnapshot(app.pid, "growth-" + c)) });
    }
    const first = growth[0] && growth[0].totalWorkingSetBytes;
    const last = growth[growth.length - 1] && growth[growth.length - 1].totalWorkingSetBytes;
    report.measurements.openCloseSwitchGrowth = { cycles, snapshots: growth, growthMiB: first != null && last != null ? +((last - first) / 1048576).toFixed(1) : null };
    report.measurements.uiStalls = readMarksSince().filter((m) => m.label.startsWith("ui-stall")).map((m) => ({ label: m.label, epochMs: m.epochMs }));
    report.measurements.memoryFinal = await memorySnapshot(app.pid, "final-after-growth");
    report.status = "ok";
  } catch (e) {
    report.status = "blocked"; report.blocked.push(String(e && e.stack ? e.stack : e));
  } finally {
    killTree(app ? app.pid : report.conditions.appPid);
    writeFileSync(outPath, JSON.stringify(report, null, 2));
  }
  console.log("island-native: status=" + report.status);
  const cold = report.measurements.memoryColdIdle;
  if (cold) console.log("  cold idle (island prewarmed hidden): " + cold.totalWorkingSetMiB + " MiB WS / " + cold.totalPrivateMiB + " MiB private, " + cold.processCount + " procs");
  const open = report.measurements.open;
  if (open) console.log("  open: win32 visible=" + open.win32VisibleMs + "ms; island-open frame=" + JSON.stringify(open.frame && open.frame.kv));
  if (report.measurements.typing) console.log("  typing marks: " + JSON.stringify(report.measurements.typing.marks));
  if (report.measurements.cardSwitchSweep) console.log("  card switches: " + report.measurements.cardSwitchSweep.length + " marks: " + JSON.stringify(report.measurements.cardSwitchSweep.map((s) => ({ s: s.switch, p50: s.stats.p50, p95: s.stats.p95, drop: s.stats.drop, long: s.stats.long }))));
  if (report.measurements.close) console.log("  close frame: " + JSON.stringify(report.measurements.close.frame && report.measurements.close.frame.kv));
  if (report.measurements.memoryHiddenAfterUseTrim) console.log("  hidden after use + trim: " + report.measurements.memoryHiddenAfterUseTrim.totalWorkingSetMiB + " MiB WS / " + report.measurements.memoryHiddenAfterUseTrim.totalPrivateMiB + " MiB private");
  if (report.measurements.openCloseSwitchGrowth) console.log("  20x open/close/switch growth: " + report.measurements.openCloseSwitchGrowth.growthMiB + " MiB");
  if (report.blocked.length) console.log("  blocked: " + report.blocked.join(" | "));
  console.log("  results: " + outPath);
}
main().catch((e) => { console.error("island-native: " + (e && e.stack ? e.stack : e)); process.exit(1); });
