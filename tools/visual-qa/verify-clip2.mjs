import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const launcher=pages[kinds.indexOf("launcher")];
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_launcher"));await new Promise(r=>setTimeout(r,500));
const doQuery = async (chip, q) => launcher.evaluate(async (args)=>{const {l,q}=args;const chips=[...document.querySelectorAll("button,[role=radio],[role=tab]")];const target=chips.find(e=>(e.textContent||"").trim().toUpperCase()===l); if(target) target.click(); await new Promise(r=>setTimeout(r,300));const input=document.querySelector("input.search-input")??document.querySelector("input");const setter=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,"value").set;input.focus();setter.call(input,q);input.dispatchEvent(new Event("input",{bubbles:true}));await new Promise(r=>setTimeout(r,900));return {body: document.body.innerText.slice(0,700), groups:[...document.querySelectorAll(".result-group")].map(g=>({label:g.querySelector(".group-label")?.textContent,rows:[...g.querySelectorAll('[role="option"], .result')].slice(0,4).map(r=>r.innerText.replace(/\s+/g," ").slice(0,90))}))};}, {l:chip,q});
for (const [chip,q] of [["CLIPBOARD","example"],["CLIPBOARD","milk"],["CLIPBOARD",""],["FILES","arch"],["FOLDERS","proton"]]) {
  const res = await doQuery(chip,q);
  console.log(chip, JSON.stringify(q), JSON.stringify(res.groups).slice(0,700));
}
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_launcher"));
await b.close().catch(()=>{});
