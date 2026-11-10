import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
const OUT = "C:/Users/Pranshul/Documents/Projects/protonsearch/docs/qa/g8-verify-round3";
mkdirSync(OUT, { recursive: true });
const b = await chromium.connectOverCDP("http://127.0.0.1:9223");
const allPages = () => { const out = []; for (const c of b.contexts()) for (const p of c.pages()) out.push(p); return out; };
const kindOf = async (p) => { try { return await p.evaluate(() => document.documentElement.dataset.window); } catch { return null; } };
async function pagesByKind() { const ps = allPages(); const kinds = []; for (const p of ps) kinds.push(await kindOf(p)); const map = {}; for (let i = 0; i < ps.length; i++) if (kinds[i]) map[kinds[i]] = ps[i]; return map; }
let kinds = await pagesByKind();
const main = kinds.main;
if (!main) throw new Error("no main page " + JSON.stringify(Object.keys(kinds)));
const invoke = (page, cmd, args = {}) => page.evaluate(({ cmd, args }) => window.__TAURI_INTERNALS__.invoke(cmd, args), { cmd, args });
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
async function ensureIsland() {
  for (let i = 0; i < 20; i++) { const k = await pagesByKind(); if (k.island) return k.island; if (i === 0) await invoke(main, "show_island"); await wait(400); }
  throw new Error("island page never appeared");
}
const island = await ensureIsland();
await wait(1000);
async function shot(page, name) { const path = OUT + "/" + name + ".png"; await page.screenshot({ path, animations: "allow" }); return path; }
async function hashFrame(page, name) {
  const buf = await page.screenshot({ path: OUT + "/anim-" + name + ".png", animations: "allow" });
  return createHash("sha256").update(buf).digest("hex").slice(0, 16);
}
// --- (2) enter/exit animation frames ---
await invoke(main, "hide_island"); await wait(700);
await invoke(main, "show_island"); await wait(900);
await invoke(main, "hide_island"); await wait(600);
const enter = [];
await invoke(main, "show_island");
for (let i = 0; i < 10; i++) { try { enter.push({ i, ms: i * 45, hash: await hashFrame(island, "enter-" + String(i).padStart(2, "0")) }); } catch (e) { enter.push({ i, error: String(e).slice(0, 120) }); } await wait(45); }
await wait(700);
const exitFrames = [];
await invoke(main, "hide_island");
for (let i = 0; i < 10; i++) { try { exitFrames.push({ i, ms: i * 45, hash: await hashFrame(island, "exit-" + String(i).padStart(2, "0")) }); } catch (e) { exitFrames.push({ i, error: String(e).slice(0, 120) }); } await wait(45); }
await wait(600);
const anim = { enter, exit: exitFrames, enterDistinct: new Set(enter.filter(x => x.hash).map(x => x.hash)).size, exitDistinct: new Set(exitFrames.filter(x => x.hash).map(x => x.hash)).size };
// --- (3) notifications 0/1/many/dismiss/clear ---
await invoke(main, "notifications_clear");
async function bellCard() { await invoke(main, "show_island"); await wait(900); const k = await pagesByKind(); const isl = k.island; const dots = isl.locator("[aria-label^='Go to']"); const n = await dots.count(); if (n < 8) throw new Error("dots=" + n); await dots.nth(7).click({ timeout: 2000 }); await wait(900); return isl; }
const notif = {};
notif.zero = { path: await shot(await bellCard(), "notif-0"), items: (await invoke(main, "notifications_list")).length };
await invoke(main, "show_local_toast", { title: "Round3 one", body: "One notification for layout check" });
notif.one = { path: await shot(await bellCard(), "notif-1"), items: (await invoke(main, "notifications_list")).length };
for (const [t, bdy] of [["Round3 two", "Second"], ["Round3 three", "Third"], ["Round3 four", "Fourth"]]) await invoke(main, "show_local_toast", { title: t, body: bdy });
const islMany = await bellCard();
notif.many = { path: await shot(islMany, "notif-many"), items: (await invoke(main, "notifications_list")).length };
notif.many.rows = await islMany.locator(".island-notification-row").count();
const dismiss = islMany.locator(".island-dismiss-button").first();
if (await dismiss.count()) { await dismiss.click({ timeout: 2000 }); await wait(800); }
notif.afterDismiss = { path: await shot(islMany, "notif-after-dismiss"), items: (await invoke(main, "notifications_list")).length };
const clear = islMany.locator(".island-notification-footer button").first();
if (await clear.count()) { const enabled = await clear.isEnabled(); if (enabled) { await clear.click({ timeout: 2000 }); await wait(800); } notif.clearButtonEnabled = enabled; }
notif.afterClear = { path: await shot(islMany, "notif-after-clear"), items: (await invoke(main, "notifications_list")).length };
// --- (4) Quick Settings ---
async function card(idx) { await invoke(main, "show_island"); await wait(900); const k = await pagesByKind(); const isl = k.island; const dots = isl.locator("[aria-label^='Go to']"); await dots.nth(idx).click({ timeout: 2000 }); await wait(900); return isl; }
const islQs = await card(4);
await writeFileSync(OUT + "/qs-screenshot-marker.txt", "qs");
const qsPath = await shot(islQs, "qs-main");
const controls = await invoke(main, "get_controls_status");
const qs = { path: qsPath, controlsBefore: controls, actions: [] };
const beforeVolume = controls.volume_percent;
try { await invoke(main, "control_set_volume", { percent: 40 }); const after = await invoke(main, "get_controls_status"); qs.actions.push({ action: "setVolume(40)", ok: true, after: after.volume_percent }); if (beforeVolume != null) await invoke(main, "control_set_volume", { percent: beforeVolume }); } catch (e) { qs.actions.push({ action: "setVolume(40)", ok: false, error: String(e).slice(0, 200) }); }
try { await invoke(main, "control_set_muted", { muted: true }); await invoke(main, "control_set_muted", { muted: controls.muted === true }); qs.actions.push({ action: "mute toggle", ok: true }); } catch (e) { qs.actions.push({ action: "mute toggle", ok: false, error: String(e).slice(0, 200) }); }
try { await invoke(main, "control_set_brightness", { percent: 70 }); qs.actions.push({ action: "setBrightness(70)", ok: true }); } catch (e) { qs.actions.push({ action: "setBrightness(70)", ok: false, error: String(e).slice(0, 240) }); }
try { await invoke(main, "control_set_wifi", { enabled: true }); qs.actions.push({ action: "setWifi(true/idempotent)", ok: true }); } catch (e) { qs.actions.push({ action: "setWifi(true)", ok: false, error: String(e).slice(0, 300) }); }
const btBefore = controls.bluetooth_enabled;
try { await invoke(main, "control_set_bluetooth", { enabled: true }); const after = await invoke(main, "get_controls_status"); qs.actions.push({ action: "setBluetooth(true)", ok: true, afterEnabled: after.bluetooth_enabled, devices: (after.bluetooth_devices || []).length }); } catch (e) { qs.actions.push({ action: "setBluetooth(true)", ok: false, error: String(e).slice(0, 300) }); }
const wifiChevron = islQs.locator('[aria-label="Show Wi-Fi networks"]');
if (await wifiChevron.count()) { await wifiChevron.click({ timeout: 2000 }); await wait(800); qs.wifiDrilldownPath = await shot(islQs, "qs-wifi-drilldown"); qs.wifiDrilldownText = (await islQs.evaluate(() => document.body.innerText)).slice(0, 500); const back = islQs.locator("text=Back to Quick Settings").first(); if (await back.count()) { await back.click({ timeout: 2000 }); await wait(600); } }
const btChevron = islQs.locator('[aria-label="Show Bluetooth devices"]');
if (await btChevron.count()) { await btChevron.click({ timeout: 2000 }); await wait(800); qs.btDrilldownPath = await shot(islQs, "qs-bt-drilldown"); qs.btDrilldownText = (await islQs.evaluate(() => document.body.innerText)).slice(0, 500); }
if (btBefore != null) { try { await invoke(main, "control_set_bluetooth", { enabled: btBefore === true }); } catch {} }
qs.controlsAfter = await invoke(main, "get_controls_status");
// --- (5) island search parity + app icons + filters ---
await invoke(main, "show_island"); await wait(900);
let k = await pagesByKind(); const islSearch = k.island;
await islSearch.locator("[aria-label^='Go to']").first().click({ timeout: 2000 }); await wait(700);
async function typeIn(page, selector, text) { await page.evaluate(({ selector, text }) => { const el = document.querySelector(selector); const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set; set.call(el, text); el.dispatchEvent(new Event("input", { bubbles: true })); el.focus(); }, { selector, text }); }
await typeIn(islSearch, ".island-search-input", "edge");
await wait(1400);
const islRows = await islSearch.locator("#island-search-results [role=option]").allInnerTexts();
const islIcons = await islSearch.evaluate(() => [...document.querySelectorAll("#island-search-results img")].map(i => (i.getAttribute("src") || "").slice(0, 30)));
const islFilters = await islSearch.evaluate(() => [...document.querySelectorAll(".island-filter")].map(f => ({ label: f.textContent, checked: f.getAttribute("aria-checked") })));
const search = { island: { query: "edge", rows: islRows, iconSrcPrefixes: islIcons, filters: islFilters, path: await shot(islSearch, "island-search-edge") } };
const filterResults = [];
for (const label of ["All", "Files", "Folders", "Apps"]) {
  const f = islSearch.locator(".island-filter", { hasText: label }).first();
  if (await f.count()) { await f.click({ timeout: 2000 }); await wait(800); filterResults.push({ label, checked: await f.getAttribute("aria-checked"), rows: await islSearch.locator("#island-search-results [role=option]").count(), text: (await islSearch.locator("#island-search-results").innerText()).slice(0, 160) }); }
}
search.island.filtersApplied = filterResults;
await invoke(main, "show_launcher"); await wait(900);
const k2 = await pagesByKind(); const launcher = k2.launcher;
await typeIn(launcher, "input.search-input", "edge");
await wait(1200);
search.launcher = { rows: await launcher.locator("[role=option]").allInnerTexts(), icons: await launcher.evaluate(() => [...document.querySelectorAll("img")].map(i => (i.getAttribute("src") || "").slice(0, 30))), path: await shot(launcher, "launcher-search-edge") };
await invoke(main, "hide_launcher");
// --- (6) mode switch safety + lazy hotkey (mode switch tested here; hotkey separately after restart) ---
const settings = await invoke(main, "get_settings");
async function setPrimaryMode(mode) { const launcherCfg = { ...settings.launcher, primaryMode: mode }; return await invoke(main, "update_settings", { patch: { launcher: launcherCfg } }); }
const mode = { start: settings.launcher.primaryMode };
try { const r1 = await setPrimaryMode("dynamic_island"); await wait(1600); const kk = await pagesByKind(); mode.toIsland = { ok: true, updateReturned: r1 != null || true, pages: Object.keys(kk), status: await invoke(main, "get_status") ? "alive" : "dead" }; } catch (e) { mode.toIsland = { ok: false, error: String(e).slice(0, 300) }; }
try { const r2 = await setPrimaryMode("normal"); await wait(1600); const kk = await pagesByKind(); const s2 = await invoke(main, "get_settings"); mode.toNormal = { ok: true, pages: Object.keys(kk), primaryMode: s2.launcher.primaryMode }; } catch (e) { mode.toNormal = { ok: false, error: String(e).slice(0, 300) }; }
mode.finalAlive = !!(await invoke(main, "get_status"));
const evidence = { capturedAt: new Date().toISOString(), head: "5142a25", animation: anim, notifications: notif, quickSettings: qs, search, mode };
writeFileSync(OUT + "/round3-evidence.json", JSON.stringify(evidence, null, 2));
console.log(JSON.stringify({ animation: anim, notif: { zero: notif.zero.items, one: notif.one.items, many: notif.many.items, rows: notif.many.rows, afterDismiss: notif.afterDismiss.items, afterClear: notif.afterClear.items, clearEnabled: notif.clearButtonEnabled }, qs: { actions: qs.actions, wifi: qs.controlsBefore.wifi, bluetooth: qs.controlsBefore.bluetooth, brightness: qs.controlsBefore.brightness_capability, volume: qs.controlsBefore.volume_percent }, search: { islandRows: islRows.length, launcherRows: search.launcher.rows.length, islandIcons: islIcons, filters: filterResults }, mode }, null, 1).slice(0, 12000));
await b.close();
