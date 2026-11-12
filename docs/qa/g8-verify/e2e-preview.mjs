import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
const OUT = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify";
mkdirSync(OUT, { recursive: true });
const b = await chromium.connectOverCDP("http://127.0.0.1:9223");
const pages = []; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds = []; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf("main")]; const launcher = pages[kinds.indexOf("launcher")];
if (!main || !launcher) throw new Error("missing main/launcher pages " + JSON.stringify(kinds));
const status = await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("get_status"));
const api = [];
for (const q of ["report", "diagram", "blob", "large"]) {
  const resp = await main.evaluate(async (query) => {
    const r = await window.__TAURI_INTERNALS__.invoke("search", { req: { requestId: Date.now(), query, scope: "all", filters: {}, limit: 10 } });
    const first = r.results[0] || null;
    let preview = null;
    if (first) preview = await window.__TAURI_INTERNALS__.invoke("get_preview", { req: { resultId: first.id, maxChars: 1200 } });
    return { first: first ? { id: first.id, kind: first.kind, title: first.title, path: first.path || null, hasSnippet: first.meta?.snippet != null } : null, preview: preview ? { status: preview.status, format: preview.format, textLen: (preview.text||"").length, snippetLen: (preview.snippet||"").length, fields: (preview.fields||[]).length, highlights: (preview.highlightRanges||[]).length, truncated: preview.truncated, imageMime: preview.imageMime || null, imageDataUrlLen: (preview.imageDataUrl||"").length, unsupportedReason: preview.unsupportedReason || null, error: preview.error || null } : null };
  }, q);
  api.push({ query: q, ...resp });
}
await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher"));
await new Promise(r => setTimeout(r, 900));
const ui = [];
for (const q of ["report", "diagram", "blob"]) {
  await launcher.evaluate(() => { const i = document.querySelector("input.search-input") || document.querySelector("input"); if (i) { const s = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; s.call(i, ""); i.dispatchEvent(new Event("input", { bubbles: true })); i.focus(); } });
  await new Promise(r => setTimeout(r, 250));
  await launcher.evaluate((query) => { const i = document.querySelector("input.search-input") || document.querySelector("input"); const s = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; s.call(i, query); i.dispatchEvent(new Event("input", { bubbles: true })); i.focus(); }, q);
  await new Promise(r => setTimeout(r, 800));
  const rows = await launcher.evaluate(() => document.querySelectorAll('[role="option"]').length);
  await launcher.keyboard.press("Control+Space");
  await new Promise(r => setTimeout(r, 700));
  const panel = await launcher.evaluate(() => { const el = document.querySelector('[data-testid="preview-panel"]'); return el ? { text: el.innerText.slice(0, 500), html: el.innerHTML.slice(0, 400), imgs: [...el.querySelectorAll("img")].map(i => ({ w: i.width, h: i.height, src: (i.getAttribute("src")||"").slice(0, 30) })), testid: el.getAttribute("data-testid") } : null; });
  const file = OUT + "/p0-1-preview-" + q + ".png";
  await launcher.screenshot({ path: file });
  ui.push({ query: q, rows, panel, file });
}
const evidence = { capturedAt: new Date().toISOString(), engine: { state: status.state, entries: status.filenameEntries, clipboardCount: status.sources?.find(s => s.id === "clipboard")?.itemCount }, api, ui };
writeFileSync(OUT + "/p0-1-preview-evidence.json", JSON.stringify(evidence, null, 2));
console.log(JSON.stringify(evidence, null, 1).slice(0, 7000));
await b.close();
