import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
const OUT = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify";
mkdirSync(OUT, { recursive: true });
const b = await chromium.connectOverCDP("http://127.0.0.1:9223");
const pages = []; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds = []; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf("main")]; const launcher = pages[kinds.indexOf("launcher")];
const status = await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("get_status"));
async function searchAndPreview(query, matcher) {
  return await main.evaluate(async ({ query, matcher }) => {
    const r = await window.__TAURI_INTERNALS__.invoke("search", { req: { requestId: (Date.now() * 1000 + Math.floor(Math.random() * 1000)), query, scope: "all", filters: {}, limit: 15 } });
    const cands = r.results.map(x => ({ id: x.id, kind: x.kind, title: x.title, path: x.path || null, snippet: x.meta?.snippet ?? null }));
    const target = cands.find(x => (matcher ? (x.title && x.title.toLowerCase().includes(matcher)) : true));
    if (!target) return { query, matcher, candidates: cands.slice(0, 8), preview: null, target: null };
    const p = await window.__TAURI_INTERNALS__.invoke("get_preview", { req: { resultId: target.id, maxChars: 1200 } });
    return { query, matcher, target, candidates: cands.slice(0, 8), preview: { status: p.status, format: p.format, text: (p.text||"").slice(0, 400), snippet: (p.snippet||"").slice(0, 200), fields: p.fields||[], highlights: p.highlightRanges||[], truncated: p.truncated, imageMime: p.imageMime||null, imageDataUrlLen: (p.imageDataUrl||"").length, ocrLen: (p.ocrText||"").length, unsupportedReason: p.unsupportedReason||null, error: p.error||null } };
  }, { query, matcher });
}
let apiCounter = 0;
const api = [];
api.push(await searchAndPreview("notes", "notes.txt"));
api.push(await searchAndPreview("KOALA-DOCX-2026", "report.docx"));
api.push(await searchAndPreview("report", "report.pdf"));
api.push(await searchAndPreview("diagram", "diagram.png"));
api.push(await searchAndPreview("blob", "blob.bin"));
api.push(await searchAndPreview("large", "large.txt"));
await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher"));
await new Promise(r => setTimeout(r, 800));
async function uiCase(query, titlePiece, name) {
  await launcher.evaluate(() => { const i = document.querySelector("input.search-input") || document.querySelector("input"); const s = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; s.call(i, ""); i.dispatchEvent(new Event("input", { bubbles: true })); i.focus(); });
  await new Promise(r => setTimeout(r, 250));
  await launcher.evaluate((q) => { const i = document.querySelector("input.search-input") || document.querySelector("input"); const s = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; s.call(i, q); i.dispatchEvent(new Event("input", { bubbles: true })); i.focus(); }, query);
  await new Promise(r => setTimeout(r, 800));
  let clicked = false;
  try { const opt = launcher.locator('[role="option"]', { hasText: titlePiece }).first(); if (await opt.count()) { await opt.click({ timeout: 2500 }); clicked = true; } } catch {}
  await new Promise(r => setTimeout(r, 400));
  await launcher.keyboard.press("Control+Space");
  await new Promise(r => setTimeout(r, 900));
  const panel = await launcher.evaluate(() => { const el = document.querySelector('[data-testid="preview-panel"]'); return el ? { text: el.innerText.slice(0, 500), marks: el.querySelectorAll("mark").length, imgs: [...el.querySelectorAll("img")].map(i => ({ w: i.width, h: i.height, src: (i.getAttribute("src")||"").slice(0, 30) })), unsupported: el.querySelectorAll('[data-testid="preview-unsupported"]').length } : null; });
  const file = OUT + "/p0-1-preview-" + name + ".png";
  await launcher.screenshot({ path: file });
  return { query, titlePiece, clicked, panel, file };
}
const ui = [];
ui.push(await uiCase("KOALA-DOCX-2026", "report.docx", "highlight"));
ui.push(await uiCase("diagram", "diagram.png", "image"));
ui.push(await uiCase("large.txt", "large.txt", "large"));
const evidence = { capturedAt: new Date().toISOString(), engine: { state: status.state, entries: status.filenameEntries }, api, ui };
writeFileSync(OUT + "/p0-1-preview-evidence.json", JSON.stringify(evidence, null, 2));
for (const a of api) console.log("API", a.query, "->", a.target ? a.target.title : "NO_TARGET", "|", a.preview ? JSON.stringify({ status: a.preview.status, format: a.preview.format, textLen: (a.preview.text||"").length, highlights: a.preview.highlights.length, img: a.preview.imageDataUrlLen, ocr: a.preview.ocrLen, unsupported: a.preview.unsupportedReason, truncated: a.preview.truncated }) : "no-preview");
for (const u of ui) console.log("UI", u.query, "->", u.panel ? JSON.stringify({ text: u.panel.text.slice(0, 140), marks: u.panel.marks, imgs: u.panel.imgs.length, unsupported: u.panel.unsupported }) : "NO_PANEL");
await b.close();
