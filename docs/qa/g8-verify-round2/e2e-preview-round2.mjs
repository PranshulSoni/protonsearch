import { chromium } from "playwright";
import { mkdirSync, writeFileSync, renameSync, existsSync } from "node:fs";
const OUT = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round2";
const FIX = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/preview-audit/fixtures";
mkdirSync(OUT, { recursive: true });
const b = await chromium.connectOverCDP("http://127.0.0.1:9223");
const pages = []; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds = []; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf("main")]; const launcher = pages[kinds.indexOf("launcher")];
if (!main || !launcher) throw new Error("missing pages " + JSON.stringify(kinds));
const status = await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("get_status"));
let reqId = 40000;
const api = [];
async function previewFor(query, matcher) {
  return await main.evaluate(async ({ query, matcher, reqId }) => {
    const r = await window.__TAURI_INTERNALS__.invoke("search", { req: { requestId: reqId, query, scope: "all", filters: {}, limit: 20 } });
    const cands = r.results.map(x => ({ id: x.id, kind: x.kind, title: x.title, path: x.path || null, snippet: x.meta?.snippet ?? null }));
    const target = cands.find(x => matcher ? (x.title || "").toLowerCase().includes(matcher) : true);
    if (!target) return { query, matcher, candidates: cands.slice(0, 10), preview: null, target: null };
    const p = await window.__TAURI_INTERNALS__.invoke("get_preview", { req: { resultId: target.id, maxChars: 1500 } });
    return { query, matcher, target, candidates: cands.slice(0, 10), preview: { status: p.status, format: p.format, text: (p.text || "").slice(0, 300), snippet: (p.snippet || "").slice(0, 150), textLen: (p.text || "").length, fields: (p.fields || []).length, highlights: (p.highlightRanges || []), truncated: p.truncated, imageMime: p.imageMime || null, imageDataUrlLen: (p.imageDataUrl || "").length, ocrLen: (p.ocrText || "").length, unsupportedReason: p.unsupportedReason || null, error: p.error || null } };
  }, { query, matcher, reqId: reqId++ });
}
const cases = [
  ["notes", "notes.txt"], ["sample", "sample.rs"], ["readme", "readme.md"], ["sample", "sample.json"],
  ["sample", "sample.csv"], ["report", "report.pdf"], ["scanned", "scanned.pdf"], ["report", "report.docx"],
  ["sheet", "sheet.xlsx"], ["diagram", "diagram.png"], ["blob", "blob.bin"], ["unknown", "unknown.xyz"],
  ["large", "large.txt"], ["unicode", "unicode-note.txt"],
];
for (const [q, m] of cases) api.push(await previewFor(q, m));
const tok = [];
for (const [q, m] of [["KOALA-DOCX-2026","report.docx"],["KOALA-JSON-2026","sample.json"],["KOALA-CSV-2026","sample.csv"],["KOALA-UNICODE-2026","unicode-note.txt"]]) tok.push(await previewFor(q, m));
// stale preview: get id for diagram, hide the file, re-preview, restore
let stale = null;
try {
  const before = await previewFor("diagram", "diagram.png");
  if (before.target) {
    const old = FIX + "/diagram.png"; const bak = old + ".bak";
    if (existsSync(old)) renameSync(old, bak);
    await new Promise(r => setTimeout(r, 300));
    const after = await main.evaluate(async ({ id }) => { const p = await window.__TAURI_INTERNALS__.invoke("get_preview", { req: { resultId: id, maxChars: 800 } }); return { status: p.status, format: p.format, truncated: p.truncated, unsupportedReason: p.unsupportedReason || null, error: p.error || null, textLen: (p.text||"").length, img: (p.imageDataUrl||"").length }; }, { id: before.target.id });
    if (existsSync(bak)) renameSync(bak, old);
    stale = { resultId: before.target.id, after };
  }
} catch (e) { stale = { error: String(e) }; }
// UI screenshots
await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher"));
await new Promise(r => setTimeout(r, 800));
async function uiCase(query, name) {
  await launcher.evaluate(() => { const i = document.querySelector("input.search-input") || document.querySelector("input"); const s = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; s.call(i, ""); i.dispatchEvent(new Event("input", { bubbles: true })); i.focus(); });
  await new Promise(r => setTimeout(r, 250));
  await launcher.evaluate((q) => { const i = document.querySelector("input.search-input") || document.querySelector("input"); const s = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; s.call(i, q); i.dispatchEvent(new Event("input", { bubbles: true })); i.focus(); }, query);
  await new Promise(r => setTimeout(r, 900));
  await launcher.keyboard.press("Control+Space");
  await new Promise(r => setTimeout(r, 900));
  const panel = await launcher.evaluate(() => { const el = document.querySelector('[data-testid="preview-panel"]'); return el ? { text: el.innerText.slice(0, 450), marks: el.querySelectorAll("mark").length, imgs: [...el.querySelectorAll("img")].map(i=>({w:i.width,h:i.height,src:(i.getAttribute("src")||"").slice(0,30)})), unsupported: el.querySelectorAll('[data-testid="preview-unsupported"]').length } : null; });
  await launcher.screenshot({ path: OUT + "/p0-1-" + name + ".png" });
  return { query, panel, file: OUT + "/p0-1-" + name + ".png" };
}
const ui = [];
ui.push(await uiCase("report.docx", "docx"));
ui.push(await uiCase("unicode", "unicode"));
ui.push(await uiCase("diagram.png", "image"));
ui.push(await uiCase("blob.bin", "unsupported"));
ui.push(await uiCase("sample.json", "json"));
const evidence = { capturedAt: new Date().toISOString(), engine: { state: status.state, entries: status.filenameEntries }, api, tokens: tok, stale, ui };
writeFileSync(OUT + "/p0-1-round2-evidence.json", JSON.stringify(evidence, null, 2));
console.log("ENGINE " + JSON.stringify(evidence.engine));
for (const a of api.concat(tok)) console.log("API", a.query, "->", a.target ? a.target.title : "NONE", "|", a.preview ? JSON.stringify({st:a.preview.status,f:a.preview.format,txt:a.preview.textLen,hl:a.preview.highlights.length,img:a.preview.imageDataUrlLen,ocr:a.preview.ocrLen,u:a.preview.unsupportedReason,tr:a.preview.truncated}) : "none");
console.log("STALE " + JSON.stringify(stale));
for (const u of ui) console.log("UI", u.query, "->", u.panel ? JSON.stringify({max: u.panel.marks, imgs: u.panel.imgs.length, unsup: u.panel.unsupported, text: u.panel.text.slice(0,120)}) : "NO_PANEL");
await b.close();
