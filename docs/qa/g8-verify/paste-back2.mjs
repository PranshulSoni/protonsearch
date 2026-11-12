const { chromium } = await import('playwright');
const b = await chromium.connectOverCDP('http://127.0.0.1:9223');
const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf('main')]; const island = pages[kinds.indexOf('island')];
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('show_island'));
await new Promise(r=>setTimeout(r,900));
const dots = island.locator("[data-page], .pageDot, [aria-label^='Go to']");
await dots.nth(2).click({timeout:2000}).catch(()=>{});
await new Promise(r=>setTimeout(r,700));
await island.locator('button', { hasText: 'Images' }).first().click({timeout:2000}).catch(()=>{});
await new Promise(r=>setTimeout(r,700));
const rows = island.locator('.island-clip-row');
const n = await rows.count();
let texts = [];
for (let i=0;i<n;i++) texts.push((await rows.nth(i).innerText()).replace(/\n/g,' | '));
let clicked = -1;
for (let i=0;i<n;i++) { if ((texts[i]||'').includes('320 x 200')) { await rows.nth(i).click({timeout:2000}); clicked = i; break; } }
await new Promise(r=>setTimeout(r,2000));
console.log(JSON.stringify({n, texts, clicked}));
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('hide_island'));
await b.close();
