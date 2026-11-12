const { chromium } = await import('playwright');
const b = await chromium.connectOverCDP('http://127.0.0.1:9223');
const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf('main')];
const st = await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('get_status'));
const r = await main.evaluate(async ()=>{ const r=await window.__TAURI_INTERNALS__.invoke('search',{req:{requestId:99003,query:'',scope:'clipboard',filters:{},limit:20}}); return r.results.map(x=>({id:x.id,kind:x.kind,title:x.title,hasThumb:!!x.meta?.thumbnailDataUrl,thumbLen:(x.meta?.thumbnailDataUrl||'').length,w:x.meta?.imageWidth,h:x.meta?.imageHeight,mime:x.meta?.mime})); });
console.log(JSON.stringify({state:st.state,clipboardCount:st.sources?.find(s=>s.id==='clipboard')?.itemCount,rows:r},null,1).slice(0,6000));
await b.close();
