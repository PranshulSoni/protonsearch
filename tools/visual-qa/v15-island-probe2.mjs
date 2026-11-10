import { chromium } from "playwright";
const d=Date.now()+20000; let b;
while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok){b=await chromium.connectOverCDP("http://127.0.0.1:9223");break;}}catch{}await new Promise(r=>setTimeout(r,400));}
if(!b) throw new Error("no cdp");
const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const info=[];for(const p of pages){let k=null;try{k=await p.evaluate(()=>document.documentElement.dataset.window);}catch{} info.push({k,url:p.url()});}
console.log("PAGES",JSON.stringify(info));
let island=null;for(const p of pages){const k=await p.evaluate(()=>document.documentElement.dataset.window).catch(()=>null); if(k==="island") island=p;}
console.log("ISLAND_PAGE",!!island);
if(island){
 await island.evaluate(()=>window.__TAURI_INTERNALS__.invoke("show_island")).catch(()=>{});
 await island.waitForTimeout(700);
 const st=await island.evaluate(()=>{
   const s=document.querySelector(".island-surface"); const inner=document.querySelector(".island-inner"); const card=document.querySelector(".island-card");
   const chain=[]; let el=s; while(el){const cs=getComputedStyle(el); chain.push({tag:el.tagName,cls:String(el.className).slice(0,60),bg:cs.backgroundColor,op:cs.opacity,rect:el.getBoundingClientRect().toJSON()}); el=el.parentElement;}
   return {html:getComputedStyle(document.documentElement).backgroundColor, body:getComputedStyle(document.body).backgroundColor, surface:s?{bg:getComputedStyle(s).backgroundColor,op:getComputedStyle(s).opacity}:null, inner:inner?{bg:getComputedStyle(inner).backgroundColor,op:getComputedStyle(inner).opacity}:null, card:card?{bg:getComputedStyle(card).backgroundColor,op:getComputedStyle(card).opacity}:null, chain:chain.slice(0,5), text:document.body.innerText.slice(0,200)};
 });
 console.log("STYLES",JSON.stringify(st).slice(0,4000));
 await island.screenshot({path:"../../docs/qa/visual/live-shots/v15-island-page.png"});
}
await b.close().catch(()=>{});
