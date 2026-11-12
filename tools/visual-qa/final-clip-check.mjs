import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const launcher=pages[kinds.indexOf("launcher")];const main=pages[kinds.indexOf("main")];
const agents=await main.evaluate(async()=>window.__TAURI_INTERNALS__.invoke("detect_agents"));
console.log("AGENTS", JSON.stringify(agents.map(a=>[a.name,a.version,a.available])));
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_launcher"));await new Promise(r=>setTimeout(r,500));
await launcher.evaluate(()=>{const el=[...document.querySelectorAll("button,[role=radio],[role=tab]")].find(e=>(e.textContent||"").trim().toUpperCase()==="CLIPBOARD");if(el)el.click();});
await new Promise(r=>setTimeout(r,1100));
const info = await launcher.evaluate(() => ({
  chips:[...document.querySelectorAll(".filter-chip, .scope-chip, [role=tab]")].map(c=>c.textContent.trim()).slice(0,10),
  sections:[...document.querySelectorAll(".result-subgroup")].map(g=>g.querySelector(".result-subgroup-label")?.textContent),
  rows:[...document.querySelectorAll('[role="option"], .result')].map(r=>r.innerText.replace(/\s+/g," ").slice(0,80)),
  body: document.body.innerText.slice(0,400),
}));
console.log("CLIP", JSON.stringify(info, null, 2).slice(0,3500));
await launcher.screenshot({ path: "../../docs/qa/visual/live-shots/clipboard-grouped-final.png" });
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_launcher"));
await b.close().catch(()=>{});
