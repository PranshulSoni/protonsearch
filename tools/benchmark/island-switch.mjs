#!/usr/bin/env node
// Dynamic Island card-switch smoothness harness (release build, black box, no CDP).
// Companion to island-hotkey.mjs. Starts the app in the island-primary profile, waits for
// settle, then island-probe.ps1 summons the island (Alt+Space) and sends Ctrl+Down switches
// while sampling from outside: UI-thread WM_NULL round trips, window rect changes and a
// pixel-change timeline. In-app frame marks (island-switch/open/close/typing) and ui-stall
// marks are read from perf-marks.jsonl when the build writes them.
//
// Usage:
//   node tools/benchmark/island-switch.mjs --exe <ProtonSearch.exe> --data-dir <dir> --out <result.json>
//        [--label baseline] [--switches 16] [--gap-ms 900] [--settle-ms 12000] [--kill-stale]
// Leaves NO app running (killTree in finally).

import { fileURLToPath } from "node:url";
import { dirname, resolve, join } from "node:path";
import { existsSync, mkdirSync, writeFileSync, readFileSync, statSync, rmSync } from "node:fs";
import { spawn, execFile, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { hostname, tmpdir } from "node:os";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const PROBE_PS = resolve(__dirname, "island-probe.ps1");
const EXE_NAME = "ProtonSearch.exe";
const FRAME_PREFIXES = ["island-switch", "island-open", "island-close", "island-typing"];

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
const isNum = (v) => typeof v === "number" && Number.isFinite(v);
const round = (v, d = 2) => (isNum(v) ? +v.toFixed(d) : null);
const sumOf = (arr) => arr.filter(isNum).reduce((a, b) => a + b, 0);
const stripBom = (s) => (s.charCodeAt(0) === 0xfeff ? s.slice(1) : s);
const fmt = (v) => (v == null ? "      -" : String(v).padStart(7));

function appRunning() {
  try {
    const out = execFileSync("tasklist", ["/FI", `IMAGENAME eq ${EXE_NAME}`, "/FO", "CSV", "/NH"], { encoding: "utf8", windowsHide: true });
    return out.toLowerCase().includes(EXE_NAME.toLowerCase());
  } catch { return false; }
}
// "island-switch:a>b|n=5|p50=1.2|..." -> { prefix: "island-switch", fields: { n: 5, p50: 1.2, ... } }
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
function readProbeFile(path) {
  const j = JSON.parse(stripBom(readFileSync(path, "utf8")));
  if (j.error) throw new Error(j.error);
  return j;
}

// All probe times are ms since probe start; marks are converted with probe.startEpochMs.
function analyze(probe, marks, gapMs) {
  const t0 = probe.startEpochMs;
  const keys = probe.keys || [];
  const pix = probe.pixelChanges || [];
  const stalls = probe.stalls || [];
  const rects = probe.rects || [];
  const perSwitch = keys.map((k, idx) => {
    const start = k.t;
    const end = idx + 1 < keys.length ? keys[idx + 1].t : start + gapMs;
    const inWin = (t) => t >= start && t < end;
    const px = pix.filter(inWin);
    let maxVisualGapMs = px.length === 1 ? 0 : null;
    if (px.length > 1) {
      let g = 0;
      for (let j = 1; j < px.length; j++) g = Math.max(g, px[j] - px[j - 1]);
      maxVisualGapMs = round(g);
    }
    const st = stalls.filter((s) => inWin(s.t) && s.lat >= 50);
    const rc = rects.filter((r) => inWin(r.t));
    return {
      i: k.i,
      sent: k.sent === true,
      keyMs: round(start),
      firstPixelMs: px.length ? round(px[0] - start) : null,
      settleMs: px.length ? round(px[px.length - 1] - start) : null,
      visualFrames: px.length,
      maxVisualGapMs,
      uiStalls: st.length,
      maxUiStallMs: st.length ? round(Math.max(...st.map((s) => s.lat))) : null,
      rectChanges: rc.length,
      rects: rc.map((r) => ({ dtMs: round(r.t - start), w: r.w, h: r.h, l: r.l, top: r.top })),
    };
  });

  const sentRows = perSwitch.filter((p) => p.sent);
  const pick = (key) => sentRows.map((p) => p[key]).filter(isNum);
  const ge = (n) => stalls.filter((s) => s.lat >= n).length;
  const hk = probe.hotkeyAt;
  const summonStalls = hk == null ? [] : stalls.filter((s) => s.t >= hk && s.t < hk + 3000 && s.lat >= 50);

  const groups = {};
  const uiStallMarks = [];
  for (const m of marks) {
    const ui = /^ui-stall-(\d+(?:\.\d+)?)ms$/.exec(m.label);
    if (ui) { uiStallMarks.push({ ms: Number(ui[1]), tMs: round(m.epochMs - t0) }); continue; }
    const p = parseMark(m.label);
    if (!FRAME_PREFIXES.includes(p.prefix)) continue;
    if (!groups[p.prefix]) groups[p.prefix] = [];
    groups[p.prefix].push(p.fields);
  }
  const frameMarks = {};
  for (const [prefix, rows] of Object.entries(groups)) {
    if (prefix === "island-typing") { frameMarks[prefix] = { count: rows.length, ms: summarize(rows.map((f) => f.ms).filter(isNum)) }; continue; }
    frameMarks[prefix] = {
      count: rows.length,
      p95OfMarks: summarize(rows.map((f) => f.p95).filter(isNum)),
      maxOfMarks: summarize(rows.map((f) => f.max).filter(isNum)),
      drop: sumOf(rows.map((f) => f.drop)),
      long: sumOf(rows.map((f) => f.long)),
    };
  }

  const keysSent = keys.filter((k) => k.sent === true).length;
  const keysSkipped = keys.filter((k) => k.sent === false).length;
  const respondedSwitches = sentRows.filter((p) => p.visualFrames >= 1 || p.rectChanges >= 1).length;
  const invalid = keysSent === 0 || respondedSwitches < keysSent / 2;
  const summary = {
    switches: sentRows.length,
    validity: { keysSent, keysSkipped, respondedSwitches, invalid },
    firstPixelMs: summarize(pick("firstPixelMs")),
    settleMs: summarize(pick("settleMs")),
    visualFrames: summarize(pick("visualFrames")),
    maxVisualGapMs: summarize(pick("maxVisualGapMs")),
    uiLatency: probe.uiLatency,
    stalls: { ge50: ge(50), ge100: ge(100), ge250: ge(250), totalBlockedMs: round(sumOf(stalls.filter((s) => s.lat >= 50).map((s) => s.lat))) },
    summon: {
      hotkeyAtMs: hk ?? null,
      visibleMs: probe.summonVisibleMs ?? null,
      focusedMs: probe.summonFocusedMs ?? null,
      stalls50: summonStalls.length,
      maxStallMs: summonStalls.length ? round(Math.max(...summonStalls.map((s) => s.lat))) : null,
      totalStallMs: round(sumOf(summonStalls.map((s) => s.lat))),
    },
    pixel: { samples: probe.pixelSamples, rateHz: probe.pixelRateHz, changes: pix.length },
    frameMarks,
    appUiStalls: { count: uiStallMarks.length, maxMs: uiStallMarks.length ? Math.max(...uiStallMarks.map((m) => m.ms)) : null, list: uiStallMarks },
  };
  return { perSwitch, summary, markRows: marks.map((m) => ({ label: m.label, relMs: round(m.epochMs - t0) })) };
}

const args = parseArgs(process.argv.slice(2));
if (typeof args.exe !== "string" || typeof args["data-dir"] !== "string" || typeof args.out !== "string") {
  console.error("usage: node tools/benchmark/island-switch.mjs --exe <ProtonSearch.exe> --data-dir <dir> --out <result.json> [--label x] [--switches 16] [--gap-ms 900] [--settle-ms 12000] [--kill-stale]");
  process.exit(2);
}
const exe = resolve(args.exe);
const dataDir = resolve(args["data-dir"]);
const outPath = resolve(args.out);
const label = typeof args.label === "string" ? args.label : "run";
const switches = Number(args.switches || 16);
const gapMs = Number(args["gap-ms"] || 900);
const settleMs = Number(args["settle-ms"] || 12000);
const killStale = Boolean(args["kill-stale"]);
const SETTINGS_FILE = resolve(dataDir, "v2", "settings.json");
const MARKS_FILE = resolve(dataDir, "v2", "perf-marks.jsonl");

const report = { schema: "ps-island-switch/1", label, exe, exeSha256: null, capturedAt: new Date().toISOString(), conditions: { switches, gapMs, settleMs, dpiAware: true, host: hostname() }, status: "running", summary: null, perSwitch: [], raw: null, limitations: [], blocked: [] };
let app = null;

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
  app = spawn(exe, [], { env: { ...process.env, PROTONSEARCH_DATA_DIR: dataDir, PROTONSEARCH_PERF_HOOKS: "1", WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2") }, detached: false, stdio: "ignore", windowsHide: true });
  app.on("error", () => {});
  report.conditions.appPid = app.pid;
  await sleep(settleMs);
  if (app.exitCode !== null) throw new Error("app exited during settle, exitCode=" + app.exitCode);

  const probeOut = join(tmpdir(), `island-probe-${process.pid}-${Date.now()}.json`);
  let probe;
  try {
    await runPs(PROBE_PS, ["-ProcessId", String(app.pid), "-Switches", String(switches), "-GapMs", String(gapMs), "-OutFile", probeOut], switches * gapMs + 60000);
    probe = readProbeFile(probeOut);
  } catch (e) {
    let detail = String(e.message);
    if (existsSync(probeOut)) { try { readProbeFile(probeOut); } catch (fe) { detail = fe.message; } }
    throw new Error("island-probe failed: " + detail);
  } finally {
    rmSync(probeOut, { force: true });
  }
  await sleep(500); // let the app flush its last perf marks

  const { perSwitch, summary, markRows } = analyze(probe, readMarks(), gapMs);
  report.conditions.display = { primaryPx: `${probe.screenW}x${probe.screenH}` };
  report.conditions.initialRect = probe.initialRect;
  report.perSwitch = perSwitch;
  report.summary = summary;
  report.raw = { stalls: probe.stalls, rects: probe.rects.slice(0, 200), marks: markRows };
  report.limitations = [
    `external black-box timeline: pixel sampling ~${probe.pixelRateHz} Hz cannot resolve individual 120 Hz frames; use island-frame marks for pacing`,
    "first switch may include cold paths",
    "UI latency is a WM_NULL round trip from outside (3000 ms cap): a proxy for UI-thread blocking, not per-event cost",
  ];
  report.status = summary.validity.invalid ? "invalid" : "ok";

  console.log(`island-switch: label=${label} switches=${perSwitch.length} pixelRateHz=${probe.pixelRateHz} display=${report.conditions.display.primaryPx}`);
  console.log("   #  firstPx  settle  frames  gapMax  stalls50  maxStall  rects");
  for (const p of perSwitch) {
    console.log(`  ${String(p.i).padStart(2)} ${fmt(p.firstPixelMs)} ${fmt(p.settleMs)} ${String(p.visualFrames).padStart(7)} ${fmt(p.maxVisualGapMs)} ${String(p.uiStalls).padStart(9)} ${fmt(p.maxUiStallMs)} ${String(p.rectChanges).padStart(6)}`);
  }
  if (summary.validity.invalid) console.log("!!! WARNING: no visual response - app UI not loaded (a plain `cargo build --release` loads devUrl and shows a WebView2 error page; build with `npx tauri build --no-bundle` or `--features tauri/custom-protocol`) or keys not delivered (respondedSwitches=" + summary.validity.respondedSwitches + " keysSent=" + summary.validity.keysSent + ")");
  const s = summary;
  console.log(`  p50/p95 firstPixel=${s.firstPixelMs.p50}/${s.firstPixelMs.p95} settle=${s.settleMs.p50}/${s.settleMs.p95} | ui p95=${s.uiLatency.p95} max=${s.uiLatency.max} | stalls >=50=${s.stalls.ge50} >=100=${s.stalls.ge100} >=250=${s.stalls.ge250} blockedMs=${s.stalls.totalBlockedMs} | summon visible=${s.summon.visibleMs} focused=${s.summon.focusedMs} stalls50=${s.summon.stalls50}`);
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
console.log(`island-switch: status=${report.status} out=${outPath}`);
if (report.status !== "ok") {
  console.log(String(report.blocked[0] || "").split("\n")[0]);
  process.exitCode = 1;
}
