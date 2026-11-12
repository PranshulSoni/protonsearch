const { chromium } = await import('playwright');
const b = await chromium.connectOverCDP('http://127.0.0.1:9223');
const pages=[]; for (const c of b.contexts()) for (const p of c.pages()) pages.push(p);
const kinds=[]; for (const p of pages) { try { kinds.push(await p.evaluate(()=>document.documentElement.dataset.window)); } catch { kinds.push(null); } }
const main = pages[kinds.indexOf('main')]; const island = pages[kinds.indexOf('island')];
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('show_island'));
await new Promise(r=>setTimeout(r,900));
const dots = island.locator("[data-page], .pageDot, [aria-label^='Go to']");
await dots.nth(2).click({timeout:2000}).catch(()=>{});
await new Promise(r=>setTimeout(r,800));
const tab = island.locator('button', { hasText: 'Images' }).first();
await tab.click({timeout:2000}).catch(()=>{});
await new Promise(r=>setTimeout(r,600));
const opts = island.locator('[role="option"], .clip-row, li');
const n = await opts.count();
console.log('OPTIONS=' + n);
let clicked = false;
try { const target = island.locator('text=320 x 200').first(); if (await target.count()) { await target.click({timeout:2000}); clicked = true; } } catch {}
await new Promise(r=>setTimeout(r,300));
await island.keyboard.press('Enter');
await new Promise(r=>setTimeout(r,600));
const body = await island.evaluate(()=>document.body.innerText.slice(0,400));
console.log(JSON.stringify({n, clicked, body}));
await main.evaluate(()=>window.__TAURI_INTERNALS__.invoke('hide_island'));
await b.close();
