import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const main=pages[kinds.indexOf("main")];
await main.evaluate(()=>{const nav=[...document.querySelectorAll("a,button")].find(e=>(e.textContent||"").trim()==="Workflows");if(nav)nav.click();});
await main.waitForTimeout(800);
const panel = main.locator(".agent-panel").first();
const found = await panel.count().catch(()=>0);
if (found) { await panel.scrollIntoViewIfNeeded(); await main.waitForTimeout(400); await panel.screenshot({ path: "../../docs/qa/visual/live-shots/pi-panel-element.png" }); console.log("captured element"); }
else {
  await main.evaluate(()=>window.scrollTo(0, document.body.scrollHeight));
  await main.waitForTimeout(400);
  await main.screenshot({ path: "../../docs/qa/visual/live-shots/pi-panel-scrolled.png" });
  console.log("captured scrolled page");
}
await b.close().catch(()=>{});
