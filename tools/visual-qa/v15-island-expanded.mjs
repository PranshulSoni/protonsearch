import { chromium } from "playwright";
const d=Date.now()+20000; let b;
while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok){b=await chromium.connectOverCDP("http://127.0.0.1:9223");break;}}catch{}await new Promise(r=>setTimeout(r,400));}
if(!b) throw new Error("no cdp");
const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
let island=null;for(const p of pages){const k=await p.evaluate(()=>document.documentElement.dataset.window).catch(()=>null); if(k==="island") island=p;}
if(!island) throw new Error("no island");
await island.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_island")).catch(()=>{});
await island.waitForTimeout(300);
const input=island.locator("input").first();
if(await input.count()){await input.click({timeout:2000}).catch(()=>{}); await input.type("architecture",{delay:40}).catch(()=>{});}
await island.waitForTimeout(1200);
const st=await island.evaluate(()=>({w:innerWidth,h:innerHeight,surface:document.querySelector(".island-surface")?.getBoundingClientRect().toJSON(),text:document.body.innerText.slice(0,300), cards:document.querySelectorAll(".island-card").length}));
console.log("STATE",JSON.stringify(st).slice(0,1200));
await island.screenshot({path:"../../docs/qa/visual/live-shots/v15-island-expanded-page.png"});
await b.close().catch(()=>{});
