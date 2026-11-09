import { chromium } from "playwright";
import { mkdirSync } from "node:fs";
const out = "../../docs/qa/visual/live-shots";
mkdirSync(out, { recursive: true });
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const launcher=pages[kinds.indexOf("launcher")];
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_launcher"));await new Promise(r=>setTimeout(r,600));
const info = await launcher.evaluate(() => {
  const chips = [...document.querySelectorAll(".filter-chip, .scope-chip, [role=tab]")].map(c=>c.textContent.trim());
  const sections = [...document.querySelectorAll(".clip-section, .category-section, .result-group")].map(s=>({cls:s.className, label:(s.querySelector(".group-label, .section-label")?.textContent||"").trim(), rows:[...s.querySelectorAll('[role="option"], .result')].length}));
  return { chips: chips.slice(0, 14), chipRows: document.querySelectorAll(".filter-strip, .filter-row").length, sections, body: document.body.innerText.slice(0,700) };
});
console.log(JSON.stringify(info, null, 2).slice(0, 3000));
await launcher.screenshot({ path: out + "/clipboard-grouped-new.png" });
await b.close().catch(()=>{});
