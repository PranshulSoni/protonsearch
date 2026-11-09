import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
const OUT = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify";
mkdirSync(OUT, { recursive: true });
const b = await chromium.connectOverCDP("http://127.0.0.1:9223");
const pages = []; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds = []; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf("main")]; const island = pages[kinds.indexOf("island")];
if (!main || !island) throw new Error("missing pages " + JSON.stringify(kinds));
const status = await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("get_status"));
const clipSource = status.sources.find(s => s.id === "clipboard");
const rows = await main.evaluate(async () => {
  const r = await window.__TAURI_INTERNALS__.invoke("search", { req: { requestId: 99001, query: "", scope: "clipboard", filters: {}, limit: 20 } });
  return r.results.map(x => ({ id: x.id, kind: x.kind, title: x.title, subtitle: x.subtitle, hasThumb: !!x.meta?.thumbnailDataUrl, thumbLen: (x.meta?.thumbnailDataUrl || "").length, imageWidth: x.meta?.imageWidth, imageHeight: x.meta?.imageHeight, mime: x.meta?.mime }));
});
await main.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_island"));
await new Promise(r => setTimeout(r, 900));
const dots = island.locator("[data-page], .pageDot, [aria-label^='Go to']");
const dotCount = await dots.count();
if (dotCount >= 3) { await dots.nth(2).click({ timeout: 2500 }).catch(() => {}); }
await new Promise(r => setTimeout(r, 900));
try { const tab = island.locator("button", { hasText: "Images" }).first(); await tab.click({ timeout: 2000 }); await new Promise(r => setTimeout(r, 700)); } catch {}
const dom = await island.evaluate(() => ({ body: document.body.innerText.slice(0, 900), imgs: [...document.querySelectorAll("img")].map(i => ({ src: (i.getAttribute("src") || "").slice(0, 40), w: i.width, h: i.height })).slice(0, 8), buttons: [...document.querySelectorAll("button")].map(b => (b.innerText || "").trim()).filter(Boolean).slice(0, 20) }));
await island.screenshot({ path: OUT + "/p0-2-island-images.png" });
const evidence = { capturedAt: new Date().toISOString(), clipboardSource: clipSource, rows, dotCount, dom };
writeFileSync(OUT + "/p0-2-clipboard-evidence.json", JSON.stringify(evidence, null, 2));
console.log(JSON.stringify(evidence, null, 1).slice(0, 7000));
await b.close();
