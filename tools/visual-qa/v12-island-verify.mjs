import { chromium } from "playwright";
async function connect(){const d=Date.now()+20000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind)); console.log("KINDS", JSON.stringify(kinds), "URLS", JSON.stringify(pages.map(p=>p.url())));
const island=pages[kinds.indexOf("island")];
if(!island){console.log("NO_ISLAND");process.exit(0);}
await island.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_island")).catch(e=>console.log("show err",String(e)));
await island.waitForTimeout(900);
const info=await island.evaluate(()=>({text:document.body.innerText.slice(0,400),surface:document.querySelector(".island")?.getBoundingClientRect()}));
console.log("ISLAND",JSON.stringify(info).slice(0,900));
await island.screenshot({path:"../../docs/qa/visual/live-shots/v12-island-card-search.png"});
const dots=island.locator("[data-page], .pageDot, [aria-label^='Go to']");
const n=await dots.count(); console.log("dots",n);
for(let i=1;i<Math.min(n,8);i++){try{await dots.nth(i).click({timeout:1500});await island.waitForTimeout(700);console.log("card",i,(await island.evaluate(()=>document.body.innerText.slice(0,120).replace(/\n/g," | "))));await island.screenshot({path:"../../docs/qa/visual/live-shots/v12-island-card-"+i+".png"});}catch(e){console.log("card err",i,String(e).slice(0,80));}}
await island.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_island")).catch(()=>{});
await b.close().catch(()=>{});
