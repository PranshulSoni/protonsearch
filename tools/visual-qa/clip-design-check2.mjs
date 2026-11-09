import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const launcher=pages[kinds.indexOf("launcher")];
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_launcher"));await new Promise(r=>setTimeout(r,500));
const clicked = await launcher.evaluate(()=>{const el=[...document.querySelectorAll("button,[role=radio],[role=tab]")].find(e=>(e.textContent||"").trim().toUpperCase()==="CLIPBOARD"); if(el){el.click(); return true;} return false;});
await new Promise(r=>setTimeout(r,1200));
const info = await launcher.evaluate(() => {
  const rows = [...document.querySelectorAll('[role="option"], .result')];
  const html = document.querySelector(".results, .result-list, .results-viewport")?.innerHTML?.slice(0,1500) ?? document.body.innerHTML.slice(0,1500);
  return { clicked: true, body: document.body.innerText.slice(0,1200), rowCount: rows.length, groups: [...document.querySelectorAll(".result-group")].map(g=>({label:g.querySelector(".group-label")?.textContent, rows:[...g.querySelectorAll('[role="option"], .result')].length})), html };
});
console.log(JSON.stringify(info, null, 2).slice(0, 5000));
await launcher.screenshot({ path: "../../docs/qa/visual/live-shots/clipboard-grouped-new.png" });
await b.close().catch(()=>{});
