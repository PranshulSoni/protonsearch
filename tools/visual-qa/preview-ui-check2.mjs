import { chromium } from "playwright";
async function connect(){const d=Date.now()+20000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const launcher=pages[kinds.indexOf("launcher")];
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_launcher"));
await launcher.waitForTimeout(700);
const out = await launcher.evaluate(async () => {
  const input=document.querySelector("input.search-input")??document.querySelector("input");
  const setter=Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype,"value").set;
  input.focus(); setter.call(input,"architecture"); input.dispatchEvent(new Event("input",{bubbles:true}));
  await new Promise(r=>setTimeout(r,1300));
  // try several dispatch targets for Ctrl+Space
  for (const target of [input, document, window]) {
    target.dispatchEvent(new KeyboardEvent("keydown",{key:" ",code:"Space",ctrlKey:true,bubbles:true,cancelable:true}));
    await new Promise(r=>setTimeout(r,500));
  }
  await new Promise(r=>setTimeout(r,1200));
  const candidates = [...document.querySelectorAll('[class*="preview"], aside, .launcher-preview')].map(el=>({cls:el.className,text:(el.innerText||"").slice(0,200)}));
  return { body: document.body.innerText.slice(0,300), candidates: candidates.slice(0,6), htmlSnippet: document.querySelector(".launcher")?.outerHTML.slice(0,300) };
});
console.log(JSON.stringify(out).slice(0,3000));
await launcher.screenshot({path:"../../docs/qa/visual/live-shots/launcher-preview-real2.png"});
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_launcher"));
await b.close().catch(()=>{});
