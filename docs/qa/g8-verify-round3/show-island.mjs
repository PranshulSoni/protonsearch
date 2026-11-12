const { chromium } = await import('playwright');
const b = await chromium.connectOverCDP('http://127.0.0.1:9223');
const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
console.log("PAGES=" + JSON.stringify(kinds));
const main = pages[kinds.indexOf('main')];
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('show_island'));
await new Promise(r=>setTimeout(r,1200));
const pages2=[]; for (const c of b.contexts()) for (const p of c.pages()) pages2.push(p);
const kinds2=[]; for (const p of pages2) { try { kinds2.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds2.push(null); } }
console.log("AFTER_SHOW=" + JSON.stringify(kinds2));
await b.close();
