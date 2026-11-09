const { chromium } = await import('playwright');
import { writeFileSync } from "node:fs";
const OUT = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round2";
const b = await chromium.connectOverCDP('http://127.0.0.1:9223');
const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf('main')]; const launcher = pages[kinds.indexOf('launcher')];
const tokens = ["KOALA-DOCX-2026","KOALA-JSON-2026","KOALA-CSV-2026","KOALA-UNICODE-2026"];
const api = [];
for (const token of tokens) {
  const res = await main.evaluate(async (token)=>{ const r=await window.__TAURI_INTERNALS__.invoke('search',{req:{requestId:91000,query:token,scope:'content',filters:{},limit:10}}); const rows=r.results.map(x=>({id:x.id,title:x.title,kind:x.kind})); const p=rows[0]?await window.__TAURI_INTERNALS__.invoke('get_preview',{req:{resultId:rows[0].id,maxChars:1500}}):null; return {rows,p:p?{status:p.status,format:p.format,text:(p.text||"").slice(0,300),highlights:(p.highlightRanges||[]),snippet:(p.snippet||"").slice(0,200)}:null}; }, token);
  api.push({token, ...res});
}
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('show_launcher'));
await new Promise(r=>setTimeout(r,800));
async function uiCase(query,name){
  await launcher.evaluate(()=>{const i=document.querySelector("input.search-input")||document.querySelector("input");const s=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,"value").set;s.call(i,"");i.dispatchEvent(new Event("input",{bubbles:true}));i.focus();});
  await new Promise(r=>setTimeout(r,250));
  await launcher.evaluate((q)=>{const i=document.querySelector("input.search-input")||document.querySelector("input");const s=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,"value").set;s.call(i,q);i.dispatchEvent(new Event("input",{bubbles:true}));i.focus();},query);
  await new Promise(r=>setTimeout(r,1200));
  await launcher.keyboard.press("Control+Space");
  await new Promise(r=>setTimeout(r,1000));
  const panel=await launcher.evaluate(()=>{const el=document.querySelector('[data-testid="preview-panel"]');return el?{text:el.innerText.slice(0,450),marks:el.querySelectorAll("mark").length}:null;});
  await launcher.screenshot({path:OUT+"/p0-1-"+name+".png"});
  return {query,panel,file:OUT+"/p0-1-"+name+".png"};
}
const ui=[];
ui.push(await uiCase("content:KOALA-DOCX-2026","highlight"));
ui.push(await uiCase("content:KOALA-UNICODE-2026","unicode-highlight"));
const ev={capturedAt:new Date().toISOString(),api,ui};
writeFileSync(OUT+"/p0-1-token-evidence.json",JSON.stringify(ev,null,2));
console.log(JSON.stringify(ev,null,1).slice(0,9000));
await b.close();
