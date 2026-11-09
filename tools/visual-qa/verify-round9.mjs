import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const main=pages[kinds.indexOf("main")];const launcher=pages[kinds.indexOf("launcher")];
const st=await main.evaluate(async()=>window.__TAURI_INTERNALS__.invoke("get_status"));
console.log("STATUS", JSON.stringify({clipboard: st.sources.find(s=>s.id==="clipboard")}));
const agents=await main.evaluate(async()=>window.__TAURI_INTERNALS__.invoke("detect_agents"));
console.log("AGENTS", JSON.stringify(agents.map(a=>[a.name,a.version,a.available])));
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_launcher"));await new Promise(r=>setTimeout(r,700));
const dom = await launcher.evaluate(() => ({ chips: [...document.querySelectorAll(".filter-chip, .scope-chip, [role=tab]")].map(c=>c.textContent.trim()), categories: [...document.querySelectorAll(".category-chip, [data-category]")].map(c=>c.textContent.trim()).slice(0,10) }));
console.log("DOM", JSON.stringify(dom));
const clickChip = (label) => launcher.evaluate((l)=>{const el=[...document.querySelectorAll("button,[role=radio],[role=tab]")].find(e=>(e.textContent||"").trim().toUpperCase()===l); if(el){el.click(); return true;} return false;}, label);
const query = async (q) => launcher.evaluate(async (qq)=>{const input=document.querySelector("input.search-input")??document.querySelector("input");const setter=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,"value").set;input.focus();setter.call(input,qq);input.dispatchEvent(new Event("input",{bubbles:true}));await new Promise(r=>setTimeout(r,900));const groups=[...document.querySelectorAll(".result-group")].map(g=>({label:g.querySelector(".group-label")?.textContent,rows:[...g.querySelectorAll('[role="option"], .result')].slice(0,5).map(r=>({t:r.querySelector(".filename")?.textContent,tag:r.querySelector(".result-tag")?.textContent??null,meta:r.querySelector(".result-meta")?.textContent??null}))}));return groups;},q);
console.log("CLIP", JSON.stringify(await (async()=>{await clickChip("CLIPBOARD");await new Promise(r=>setTimeout(r,400));return query("");})()));
const cats = await launcher.evaluate(()=>({labels:[...document.querySelectorAll(".category-chip, [data-category]")].map(c=>c.textContent.trim()), body: document.body.innerText.slice(0,600)}));
console.log("CATS", JSON.stringify(cats).slice(0,900));
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_launcher"));
await b.close().catch(()=>{});
