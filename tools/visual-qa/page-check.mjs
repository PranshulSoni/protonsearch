import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
for (const p of pages) { const info = await p.evaluate(()=>({kind:document.documentElement.dataset.window,title:document.title,body:document.body?.innerText?.slice(0,160).replace(/\s+/g," ")})); console.log(JSON.stringify(info)); }
await b.close().catch(()=>{});
