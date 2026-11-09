const { chromium } = await import('playwright');
const b = await chromium.connectOverCDP('http://127.0.0.1:9223');
const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf('main')];
for (let i=0;i<12;i++) {
  const st = await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('get_status'));
  const act = await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('get_activity', { limit: 5 }).catch(()=>null));
  const r = await main.evaluate(async ()=>{ const r=await window.__TAURI_INTERNALS__.invoke('search',{req:{requestId:88001,query:'KOALA',scope:'content',filters:{},limit:10}}); return r.results.map(x=>({id:x.id,kind:x.kind,title:x.title,snippet:x.meta?.snippet})); });
  console.log(JSON.stringify({i,state:st.state,entries:st.filenameEntries,sources:st.sources?.map(s=>s.id+':'+s.state+':'+s.itemCount),contentHits:r.length,first:r[0]||null}));
  if (r.length) break;
  await new Promise(r=>setTimeout(r,2000));
}
await b.close();
