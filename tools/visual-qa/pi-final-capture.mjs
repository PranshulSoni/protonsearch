import { chromium } from "playwright";
async function connect(){const d=Date.now()+30000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const main=pages[kinds.indexOf("main")];
const ev=(fn)=>main.evaluate(fn);
// one real lifecycle run: enable feature, start, observe, stop, disable again
await ev(async()=>window.__TAURI_INTERNALS__.invoke("update_settings",{patch:{agent:{enabled:true}}}));
const status0=await ev(async()=>window.__TAURI_INTERNALS__.invoke("agent_status"));
console.log("STATUS", JSON.stringify(status0));
const start=await ev(async()=>{try{await window.__TAURI_INTERNALS__.invoke("agent_start",{request:{runId:"final-check-1",prompt:"Reply with OK.",contextPaths:[],maxSteps:null}});return "started";}catch(e){return "err:"+String(e);}});
await new Promise(r=>setTimeout(r,9000));
const after=await ev(async()=>window.__TAURI_INTERNALS__.invoke("agent_status"));
const stop=await ev(async()=>{try{await window.__TAURI_INTERNALS__.invoke("agent_stop",{});return "stopped";}catch(e){return String(e);}});
await ev(async()=>window.__TAURI_INTERNALS__.invoke("update_settings",{patch:{agent:{enabled:false}}}));
console.log("LIFECYCLE", JSON.stringify({start, after, stop}));
// navigate to Workflows and scroll to the Pi panel, then capture
await ev(async()=>{const nav=[...document.querySelectorAll("a,button")].find(e=>(e.textContent||"").trim()==="Workflows");if(nav)nav.click();});
await new Promise(r=>setTimeout(r,1200));
const panelInfo = await ev(async()=>{
  const panel=[...document.querySelectorAll(".agent-panel, section, div")].find(el=>/Pi agent|optional Pi/i.test(el.textContent||"") && el.querySelector("textarea"));
  if(panel){panel.scrollIntoView({block:"start"});await new Promise(r=>setTimeout(r,400));}
  const body=document.body.innerText;
  return { hasPi: /Pi 1.5.0|Pi not detected|Not running/.test(body), hasHonestCopy: /not a sandbox/.test(body), hasDisabledWrites: /disabled in this alpha/.test(body), scrolled: !!panel, snippet: body.slice(body.indexOf("Pi agent")>0?body.indexOf("Pi agent"):0, (body.indexOf("Pi agent")>0?body.indexOf("Pi agent"):0)+700) };
});
console.log("PANEL", JSON.stringify(panelInfo).slice(0,1500));
await main.screenshot({ path: "../../docs/qa/visual/live-shots/pi-panel-real-final.png" });
await b.close().catch(()=>{});
