import { chromium } from "playwright";
async function connect(){const d=Date.now()+30000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const main=pages[kinds.indexOf("main")];
const ev = (fn) => main.evaluate(fn);
const status0 = await ev(async()=>window.__TAURI_INTERNALS__.invoke("agent_status"));
console.log("STATUS_BEFORE", JSON.stringify(status0));
// enable the feature and run one real lifecycle check (no credentials configured -> provider error is expected)
await ev(async()=>window.__TAURI_INTERNALS__.invoke("update_settings",{patch:{agent:{enabled:true}}}));
await new Promise(r=>setTimeout(r,400));
const status1 = await ev(async()=>window.__TAURI_INTERNALS__.invoke("agent_status"));
console.log("STATUS_ENABLED", JSON.stringify(status1));
const started = await ev(async()=>{ try { await window.__TAURI_INTERNALS__.invoke("agent_start",{request:{runId:"ux-real-1",prompt:"Reply with the single word OK.",contextPaths:[],maxSteps:null}}); return "started"; } catch(e){ return "start-error: "+String(e); } });
console.log("START", JSON.stringify(started));
await new Promise(r=>setTimeout(r,8000));
const status2 = await ev(async()=>window.__TAURI_INTERNALS__.invoke("agent_status"));
console.log("STATUS_AFTER_RUN", JSON.stringify(status2));
const cancel = await ev(async()=>{ try { await window.__TAURI_INTERNALS__.invoke("agent_cancel",{runId:"ux-real-1"}); return "cancelled"; } catch(e){ return String(e); } });
const stop = await ev(async()=>{ try { await window.__TAURI_INTERNALS__.invoke("agent_stop",{}); return "stopped"; } catch(e){ return String(e); } });
await new Promise(r=>setTimeout(r,1200));
const status3 = await ev(async()=>window.__TAURI_INTERNALS__.invoke("agent_status"));
console.log("FINAL", JSON.stringify({cancel, stop, status3}));
// capture the Workflows panel state
await ev(async()=>{ const nav=[...document.querySelectorAll("a,button")].find(e=>(e.textContent||"").trim()==="Workflows"); if(nav) nav.click(); });
await new Promise(r=>setTimeout(r,1200));
await main.screenshot({ path: "../../docs/qa/visual/live-shots/pi-panel-real.png" });
await b.close().catch(()=>{});
