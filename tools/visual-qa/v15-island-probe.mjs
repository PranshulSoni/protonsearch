import { chromium } from "playwright";
const d=Date.now()+20000; let b;
while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok){b=await chromium.connectOverCDP("http://127.0.0.1:9223");break;}}catch{}await new Promise(r=>setTimeout(r,400));}
if(!b) throw new Error("no cdp");
const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const info=[];
for(const p of pages){let kind=null;try{kind=await p.evaluate(()=>document.documentElement.dataset.window);}catch{} info.push({kind,url:p.url(),title:await p.title().catch(()=>null)});}
console.log("PAGES",JSON.stringify(info));
const island=pages.find(async()=>true);
// find island by kind
let ip=null, lp=null;
for(const p of pages){const k=await p.evaluate(()=>document.documentElement.dataset.window).catch(()=>null); if(k==="island") ip=p; if(k==="launcher") lp=p;}
console.log("island?",!!ip,"launcher?",!!lp);
const target=ip??pages[0];
const res=await target.evaluate(async()=>{
  try{await window.__TAURI_INTERNALS__.invoke("show_island");}catch(e){return {err:String(e)};}
  await new Promise(r=>setTimeout(r,600));
  const s=document.querySelector(".island-surface");
  const inner=document.querySelector(".island-inner");
  const card=document.querySelector(".island-card");
  const chain=[];
  let el=s; while(el){const cs=getComputedStyle(el); chain.push({tag:el.tagName,cls:el.className,background:cs.backgroundColor,bgImage:cs.backgroundImage,opacity:cs.opacity,filter:cs.filter,mix:cs.mixBlendMode,isolation:cs.isolation,rect:el.getBoundingClientRect().toJSON()}); el=el.parentElement;}
  return {bodyBg:getComputedStyle(document.body).backgroundColor, htmlBg:getComputedStyle(document.documentElement).backgroundColor, surface:s?{bg:getComputedStyle(s).backgroundColor,opacity:getComputedStyle(s).opacity,rect:s.getBoundingClientRect().toJSON()}:null, inner:inner?{bg:getComputedStyle(inner).backgroundColor,opacity:getComputedStyle(inner).opacity}:null, card:card?{bg:getComputedStyle(card).backgroundColor,opacity:getComputedStyle(card).opacity}:null, chain:chain.slice(0,6)};
});
console.log("STYLES",JSON.stringify(res).slice(0,5000));
await target.screenshot({path:"../../docs/qa/visual/live-shots/v15-island-page.png"});
await b.close().catch(()=>{});
