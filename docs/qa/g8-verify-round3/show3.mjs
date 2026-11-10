import { chromium } from "playwright";
const b = await chromium.connectOverCDP("http://127.0.0.1:9223");
const dump = async (label) => {
  const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
  const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push("ERR"); } }
  console.log(label + "=" + JSON.stringify(kinds));
  return { pages, kinds };
};
const first = await dump("BEFORE");
const host = first.pages[first.kinds.indexOf("launcher")] || first.pages[0];
if (!host) throw new Error("no page at all");
const res = await host.evaluate(async () => { try { await window.__TAURI_INTERNALS__.invoke("show_island"); return "ok"; } catch (e) { return "ERR:" + String(e); } });
console.log("SHOW=" + res);
await new Promise(r => setTimeout(r, 1500));
const after = await dump("AFTER");
const island = after.pages[after.kinds.indexOf("island")];
if (island) { const box = await island.evaluate(() => ({ w: innerWidth, h: innerHeight, surface: document.querySelector(".island")?.getBoundingClientRect() })); console.log("ISLAND=" + JSON.stringify(box)); }
await b.close();
