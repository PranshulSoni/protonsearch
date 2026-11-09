#!/usr/bin/env node
// Gate-0 preview P0 reproduction: drive the REAL Tauri/WebView2 app over CDP
// against a real indexed data directory, dump the IPC search payloads and
// screenshot the launcher preview pane for each supported format.
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { resolve, join } from "node:path";

const require = createRequire("C:/Users/Pranshul/Documents/Projects/protonsearch/tools/visual-qa/");
const { chromium } = require("playwright");

const repo = resolve(process.cwd());
const exe = join(repo, "target", "debug", "ProtonSearch.exe");
const dataDir = join(repo, "docs", "qa", "preview-audit", "appdata");
const outDir = join(repo, "docs", "qa", "preview-audit", "screenshots");
const payloadDir = join(repo, "docs", "qa", "preview-audit", "payloads");
const port = 9333;
mkdirSync(outDir, { recursive: true });
mkdirSync(payloadDir, { recursive: true });
if (!existsSync(exe)) { console.error("missing exe", exe); process.exit(2); }

// Frontend: debug builds load the devUrl; serve the production dist on 5173.
const npx = process.platform === "win32" ? "npx.cmd" : "npx";
const server = spawn(npx, ["vite", "preview", "--port", "5173", "--strictPort"], {
  cwd: join(repo, "apps", "desktop"),
  stdio: "ignore",
  shell: process.platform === "win32",
});
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
let up = false;
for (let i = 0; i < 100; i++) {
  try { const res = await fetch("http://127.0.0.1:5173/index.html"); if (res.ok) { up = true; break; } } catch {}
  await wait(300);
}
if (!up) { console.error("vite preview did not start"); server.kill(); process.exit(2); }

const app = spawn(exe, [], {
  env: {
    ...process.env,
    PROTONSEARCH_DATA_DIR: dataDir,
    PROTONSEARCH_CDP_PORT: String(port),
    WEBVIEW2_USER_DATA_FOLDER: join(dataDir, "webview2"),
  },
  stdio: "ignore",
});

async function connect() {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) {
    try {
      const res = await fetch("http://127.0.0.1:" + port + "/json/version");
      if (res.ok) return await chromium.connectOverCDP("http://127.0.0.1:" + port);
    } catch {}
    await wait(500);
  }
  throw new Error("no CDP");
}
const browser = await connect();
const collect = () => browser.contexts().flatMap((c) => c.pages());
let pages = [];
for (let i = 0; i < 80; i++) {
  pages = await collect();
  let ready = 0;
  for (const p of pages) { try { if (await p.evaluate(() => document.documentElement.dataset.window)) ready++; } catch {} }
  if (ready >= 2) break;
  await wait(500);
}
const kinds = await Promise.all(pages.map((p) => p.evaluate(() => document.documentElement.dataset.window ?? null).catch(() => null)));
console.log(JSON.stringify({ pages: pages.length, kinds }));
const launcher = pages[kinds.indexOf("launcher")];
if (!launcher) throw new Error("launcher webview not found");
await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher"));
await wait(800);

// Wait until the engine pipeline is ready.
const status = await launcher.evaluate(async () => {
  const deadline = Date.now() + 120000;
  let last = null;
  while (Date.now() < deadline) {
    try {
      last = await window.__TAURI_INTERNALS__.invoke("get_status");
      if (last && last.state === "ready" && last.filenameEntries > 0) return last;
    } catch {}
    await new Promise((r) => setTimeout(r, 400));
  }
  return last;
});
console.log(JSON.stringify({ engine: { state: status?.state, files: status?.filenameEntries, docs: status?.contentDocuments, ocr: status?.ocrDocuments } }));
// Guard: this capture must run against the isolated fixture data dir, never the
// live dev data. Abort loudly instead of producing misleading screenshots.
// EngineStatus.roots is the authoritative list of roots the engine is scanning.
const guardRoots = (status?.roots ?? []).map((r) => r.path);
console.log(JSON.stringify({ guardRoots }));
if (!guardRoots.some((p) => String(p).toLowerCase().includes("preview-audit") && String(p).toLowerCase().includes("fixtures"))) {
  console.error("ABORT: app is not using the fixture data dir; roots=" + JSON.stringify(guardRoots));
  try { await browser.close(); } catch {}
  app.kill();
  server.kill();
  process.exit(3);
}

async function shot(name) {
  const path = join(outDir, name + ".png");
  await launcher.screenshot({ path, animations: "disabled" });
  return path;
}
async function typeQuery(q) {
  await launcher.evaluate((query) => {
    const input = document.querySelector("input.search-input") ?? document.querySelector("input");
    if (!input) return;
    input.focus();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
    setter.call(input, query);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, q);
  await wait(900);
}
async function press(key, ctrl = false) {
  await launcher.evaluate(({ key, ctrl }) => {
    const target = document.activeElement ?? document.body;
    const init = { key, bubbles: true, ctrlKey: ctrl };
    target.dispatchEvent(new KeyboardEvent("keydown", init));
    target.dispatchEvent(new KeyboardEvent("keyup", init));
  }, { key, ctrl });
  await wait(500);
}
const previewText = () => launcher.evaluate(() => {
  const panel = document.querySelector('[data-testid="preview-panel"]');
  if (!panel) return null;
  const note = panel.querySelector(".preview-note");
  const snippet = panel.querySelector(".preview-snippet");
  return {
    present: true,
    note: note ? note.textContent : null,
    snippet: snippet ? snippet.textContent : null,
    text: panel.textContent ? panel.textContent.slice(0, 1200) : null,
  };
});
const dumpPayload = (q) => launcher.evaluate(async (query) => {
  const req = { protocol: 1, requestId: 1, query, scope: "all", filters: {}, limit: 20 };
  return await window.__TAURI_INTERNALS__.invoke("search", { req });
}, q);

const scenarios = [
  { name: "file-txt", query: "notes.txt", down: 1 },
  { name: "file-pdf", query: "report.pdf", down: 1 },
  { name: "file-docx", query: "report.docx", down: 1 },
  { name: "file-xlsx", query: "sheet.xlsx", down: 1 },
  { name: "file-image", query: "diagram.png", down: 1 },
  { name: "file-binary", query: "blob.bin", down: 1 },
  { name: "content-pdf", query: "WALRUS-PDF-2026", down: 1 },
  { name: "content-docx", query: "KOALA-DOCX-2026", down: 1 },
  { name: "content-xlsx", query: "TIGER-XLSX-2026", down: 1 },
  { name: "ocr-image", query: "IMAGE-OCR-PROBE-2026", down: 1 },
  { name: "scanned-pdf", query: "scanned", down: 2 },
];
const appSettings = await launcher.evaluate(async () => {
  try { return await window.__TAURI_INTERNALS__.invoke("get_settings"); } catch (e) { return { error: String(e) }; }
});
const settingsProbe = { contentEnabled: appSettings?.indexing?.contentEnabled, ocrEnabled: appSettings?.indexing?.ocrEnabled, roots: appSettings?.indexing?.roots, resultLimit: appSettings?.resultLimit };
const statusProbe = await launcher.evaluate(async () => { try { return await window.__TAURI_INTERNALS__.invoke("get_status"); } catch (e) { return { error: String(e) }; } });
console.log(JSON.stringify({ settingsProbe, warnings: statusProbe?.warnings?.filter((w) => /content|extract|ocr/i.test(w)).slice(0, 8) }));
const report = { capturedAt: new Date().toISOString(), settingsProbe, engine: { state: status?.state, files: status?.filenameEntries, docs: status?.contentDocuments, ocr: status?.ocrDocuments }, scenarios: [] };

await launcher.evaluate(() => document.querySelector("input")?.focus());
await shot("00-launcher-empty");
for (const sc of scenarios) {
  await typeQuery(sc.query);
  for (let i = 0; i < sc.down; i++) await press("ArrowDown");
  await wait(450);
  const before = await launcher.evaluate(() => document.querySelector('[role="option"][aria-selected="true"], [role="option"]')?.textContent ?? null);
  await press(" ", true); // Ctrl+Space opens preview
  const preview = await previewText();
  const screenshot = await shot("preview-" + sc.name);
  const payload = await dumpPayload(sc.query);
  const timings = (payload.timings || []).map((t) => ({ tier: t.tier, label: t.label, hits: t.hits, tookMs: t.tookMs }));
  await press(" ", true); // close preview
  const rows = payload.results.map((r) => ({ kind: r.kind, title: r.title, path: r.path, snippet: r.meta?.snippet ?? null, label: r.meta?.label ?? null, line: r.meta?.line ?? null }));
  report.scenarios.push({ scenario: sc.name, query: sc.query, selectedRow: before, preview, screenshot, timings, ipc: rows });
  console.log(JSON.stringify({ scenario: sc.name, previewNote: preview?.note, previewSnippet: preview?.snippet ? preview.snippet.slice(0, 120) : null, rows: rows.length }));
}
writeFileSync(join(payloadDir, "app-ipc-preview-payloads.json"), JSON.stringify(report, null, 2));
writeFileSync(join(outDir, "preview-capture-report.json"), JSON.stringify(report, null, 2));
console.log("wrote", join(payloadDir, "app-ipc-preview-payloads.json"));
try { await browser.close(); } catch {}
console.log(JSON.stringify({ appPid: app.pid }));
try { app.kill(); } catch {}
server.kill();
process.exit(0);
