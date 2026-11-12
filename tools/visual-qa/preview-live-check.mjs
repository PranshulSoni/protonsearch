import { chromium } from "playwright";
async function connect(){const d=Date.now()+25000;while(Date.now()<d){try{const r=await fetch("http://127.0.0.1:9223/json/version");if(r.ok)return await chromium.connectOverCDP("http://127.0.0.1:9223");}catch{}await new Promise(r=>setTimeout(r,400));}throw new Error("no cdp");}
const b=await connect();const pages=[];for(const c of b.contexts())for(const p of c.pages())pages.push(p);
const kind=async(p)=>{try{return await p.evaluate(()=>document.documentElement.dataset.window);}catch{return null;}};
const kinds=await Promise.all(pages.map(kind));const main=pages[kinds.indexOf("main")];
const out = await main.evaluate(async () => {
  const bridge = window.__TAURI_INTERNALS__;
  const status = await bridge.invoke("get_status");
  const resp = await bridge.invoke("search", { req: { requestId: 42, query: "architecture", scope: "all", filters: null, limit: 5, preview: false } });
  const first = resp.results.find(r => r.kind === "file" || r.kind === "content_match");
  let preview = null;
  if (first) preview = await bridge.invoke("get_preview", { req: { resultId: first.id, maxChars: 1200 } });
  return {
    clipboard: status.sources.find(s => s.id === "clipboard"),
    first: first ? { id: first.id, kind: first.kind, title: first.title, snippet: first.meta?.snippet ?? null } : null,
    preview: preview ? { status: preview.status, format: preview.format, textStart: (preview.text||"").slice(0, 160), fields: preview.fields?.slice(0,4), truncated: preview.truncated, unsupportedReason: preview.unsupportedReason } : null,
  };
});
console.log(JSON.stringify(out, null, 2).slice(0, 3000));
await b.close().catch(()=>{});
