import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
const out = "../../docs/qa/visual/live-shots";
mkdirSync(out, { recursive: true });
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));
console.log("kinds", JSON.stringify(kinds));
const main=pages[kinds.indexOf("main")];
const monitors=await main.evaluate(async()=>window.__TAURI_INTERNALS__.invoke("list_monitors"));
console.log("MONITORS", JSON.stringify(monitors));
const island=pages[kinds.indexOf("island")];
if (island) { await island.screenshot({path: out + "/island-placeholder.png"}); }
await main.evaluate(async()=>window.__TAURI_INTERNALS__.invoke("show_island"));
await new Promise(r=>setTimeout(r,900));
if (island) {
  const info=await island.evaluate(()=>({w:window.innerWidth,h:window.innerHeight,body:document.body.innerText.slice(0,120)}));
  console.log("ISLAND", JSON.stringify(info));
  await island.screenshot({path: out + "/island-shown.png"});
}
await main.evaluate(async()=>window.__TAURI_INTERNALS__.invoke("hide_island"));
await b.close().catch(()=>{});
