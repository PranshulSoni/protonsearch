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
  await new Promise(r=>setTimeout(r,1200));
  // toggle preview
  const ev = new KeyboardEvent("keydown",{key:" ",code:"Space",ctrlKey:true,bubbles:true});
  document.dispatchEvent(ev);
  await new Promise(r=>setTimeout(r,1200));
  const preview = document.querySelector(".preview, [class*=preview]");
  return { body: document.body.innerText.slice(0,900), previewText: preview?preview.innerText.slice(0,500):null, previewHtml: preview?preview.innerHTML.slice(0,300):null };
});
console.log(JSON.stringify(out).slice(0,3000));
await launcher.screenshot({path:"../../docs/qa/visual/live-shots/launcher-preview-real.png"});
await launcher.evaluate(()=>window.__TAURI_INTERNALS__.invoke("hide_launcher"));
await b.close().catch(()=>{});
