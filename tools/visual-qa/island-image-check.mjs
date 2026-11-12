import { chromium } from "playwright";
async function connect(){const d=Date.now()+20000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const main=pages[kinds.indexOf("main")];const island=pages[kinds.indexOf("island")];
const st=await main.evaluate(async()=>{const s=await window.__TAURI_INTERNALS__.invoke("get_status");return s.sources.find(x=>x.id==="clipboard")});
console.log("CLIP_SOURCE", JSON.stringify(st));
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_island"));
await island.waitForTimeout(700);
// select clipboard card (dot index 2)
const dots = island.locator("[data-page], .pageDot, [aria-label^='Go to']");
await dots.nth(2).click({timeout:2000});
await island.waitForTimeout(700);
// click Images tab
const tab = island.locator("button", { hasText: "Images" }).first();
try { await tab.click({timeout:2000}); await island.waitForTimeout(700); } catch(e) { console.log("images tab err", String(e).slice(0,100)); }
const out = await island.evaluate(()=>({body:document.body.innerText.slice(0,700), imgs:[...document.querySelectorAll('img')].map(i=>({src:(i.getAttribute('src')||'').slice(0,30),w:i.width,h:i.height})).slice(0,5)}));
console.log("IMAGES_TAB", JSON.stringify(out).slice(0,1800));
await island.screenshot({path:"../../docs/qa/visual/live-shots/island-clipboard-images.png"});
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_island"));
await b.close().catch(()=>{});
