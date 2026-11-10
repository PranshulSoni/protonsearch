import { chromium } from "playwright";
async function connect(){const d=Date.now()+20000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
let kinds=await Promise.all(pages.map(kind)); console.log("before",JSON.stringify(kinds));
const launcher=pages[kinds.indexOf("launcher")] ?? pages[0];
const res = await launcher.evaluate(async()=>{
  try { const s = await window.__TAURI_INTERNALS__.invoke("update_settings", { patch: { launcher: { primaryMode: "dynamic_island" } } }); return {ok:true, mode:s.launcher.primaryMode}; } catch(e){ return {ok:false, err:String(e)}; }
});
console.log("mode switch", JSON.stringify(res));
await new Promise(r=>setTimeout(r,1200));
const alive = await launcher.evaluate(()=>document.title).catch(()=>null); console.log("launcher alive", alive);
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_island")).catch(e=>console.log("show err",String(e)));
await new Promise(r=>setTimeout(r,1500));
const b2=await connect();const pages2=[];for(const c of b2.contexts())for(const p of c.pages())pages2.push(p);
const kinds2=await Promise.all(pages2.map(kind)); console.log("after",JSON.stringify(kinds2),JSON.stringify(pages2.map(p=>p.url())));
const island=pages2[kinds2.indexOf("island")];
if(island){
  await island.waitForTimeout(600);
  const info=await island.evaluate(()=>({text:document.body.innerText.slice(0,500),surface:document.querySelector(".island")?.getBoundingClientRect()}));
  console.log("ISLAND",JSON.stringify(info).slice(0,1200));
  await island.screenshot({path:"../../docs/qa/visual/live-shots/v12-island-card-search.png"});
  const dots=island.locator("[data-page], .pageDot, [aria-label^='Go to']");
  const n=await dots.count(); console.log("dots",n);
  for(let i=1;i<Math.min(n,8);i++){try{await dots.nth(i).click({timeout:1500});await island.waitForTimeout(700);console.log("card",i,(await island.evaluate(()=>document.body.innerText.slice(0,140).replace(/\n/g," | "))));await island.screenshot({path:"../../docs/qa/visual/live-shots/v12-island-card-"+i+".png"});}catch(e){console.log("card err",i,String(e).slice(0,80));}}
}
await b2.close().catch(()=>{});
