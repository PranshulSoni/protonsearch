import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
const out = "../../docs/qa/visual/live-shots";
mkdirSync(out, { recursive: true });
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));
console.log("kinds", JSON.stringify(kinds));
const main=pages[kinds.indexOf("main")];const island=pages[kinds.indexOf("island")];
if (!island) { console.log("NO_ISLAND_PAGE"); process.exit(0); }
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_island"));
await new Promise(r=>setTimeout(r,900));
const info = await island.evaluate(()=>({w:window.innerWidth,h:window.innerHeight,text:document.body.innerText.slice(0,600),surface:document.querySelector(".island")?.getBoundingClientRect()}));
console.log("ISLAND", JSON.stringify(info).slice(0,1200));
await island.screenshot({ path: out + "/island-live-compact.png" });
// switch cards via dots
const dots = island.locator("[data-page], .pageDot, [aria-label^='Go to']");
const count = await dots.count();
console.log("dots", count);
for (let i=1;i<Math.min(count,8);i++) {
  try { await dots.nth(i).click({timeout:1500}); await island.waitForTimeout(650); const name = await island.evaluate(()=>document.body.innerText.slice(0,120).replace(/\n/g," | ")); console.log("card",i,JSON.stringify(name)); await island.screenshot({path: out + "/island-card-" + i + ".png"}); } catch(e) { console.log("card",i,"err",String(e).slice(0,120)); }
}
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_island"));
await b.close().catch(()=>{});
