import { chromium } from "playwright";
import { mkdirSync, writeFileSync } from "node:fs";
const out = "../../docs/qa/visual/final-audit";
mkdirSync(out, { recursive: true });

async function connect() {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try {
      const r = await fetch("http://127.0.0.1:9223/json/version");
      if (r.ok) return await chromium.connectOverCDP("http://127.0.0.1:9223");
    } catch {}
    await new Promise((r) => setTimeout(r, 400));
  }
  throw new Error("no CDP endpoint");
}

const browser = await connect();
const pages = [];
for (const c of browser.contexts()) for (const p of c.pages()) pages.push(p);
const kind = async (p) => { try { return await p.evaluate(() => document.documentElement.dataset.window); } catch { return null; } };
const kinds = await Promise.all(pages.map(kind));
const launcher = pages[kinds.indexOf("launcher")];
const main = pages[kinds.indexOf("main")];
if (!launcher || !main) throw new Error("app windows not found: " + JSON.stringify(kinds));

const summary = { launcher: {}, main: {}, errorsVisible: [] };

async function launcherQuery(query) {
  return launcher.evaluate(async (q) => {
    const input = document.querySelector("input.search-input") ?? document.querySelector("input");
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
    input.focus(); setter.call(input, q); input.dispatchEvent(new Event("input", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 750));
    const groups = [...document.querySelectorAll(".result-group")].map((g) => ({
      label: g.querySelector(".group-label")?.textContent ?? "",
      rows: [...g.querySelectorAll('[role="option"], .result')].map((row) => row.querySelector(".filename")?.textContent ?? ""),
    }));
    return {
      groups,
      count: document.querySelector(".count")?.textContent ?? null,
      footer: (document.querySelector(".footer")?.textContent ?? "").replace(/\s+/g, " ").trim(),
      chips: [...document.querySelectorAll(".filter-chip, .scope-chip, [role=tab]")].map((c) => c.textContent.trim()),
      card: (() => { const el = document.querySelector(".launcher"); if (!el) return null; const r = el.getBoundingClientRect(); return { w: Math.round(r.width), h: Math.round(r.height), state: el.dataset.state ?? null }; })(),
    };
  }, query);
}

await launcher.evaluate(() => window.__TAURI_INTERNALS__.invoke("show_launcher"));
await new Promise((r) => setTimeout(r, 800));
summary.launcher.empty = await launcherQuery("");
summary.launcher.empty.card = summary.launcher.empty.card;
await launcher.screenshot({ path: out + "/launcher-empty.png" });

summary.launcher.restart = await launcherQuery("restart my PC");
summary.launcher.shutdown = await launcherQuery("shut down my PC");
summary.launcher.web = await launcherQuery("best windows launcher");
await launcher.screenshot({ path: out + "/launcher-web.png" });

// WEB filter chip
try {
  await launcher.locator('.filter-chip:has-text("WEB"), [role="tab"]:has-text("WEB"), button:has-text("WEB")').first().click({ timeout: 2500 });
  summary.launcher.webFilter = await launcherQuery("protonsearch windows launcher");
  await launcher.screenshot({ path: out + "/launcher-web-filter.png" });
} catch (e) { summary.launcher.webFilter = String(e).slice(0, 160); }

// back to ALL and take a normal results shot
try { await launcher.locator('.filter-chip:has-text("ALL"), [role="tab"]:has-text("ALL"), button:has-text("ALL")').first().click({ timeout: 2000 }); } catch {}
summary.launcher.normal = await launcherQuery("architecture");
await launcher.screenshot({ path: out + "/launcher-results.png" });

// --- main app routes ---
async function shotMain(name) {
  await main.waitForTimeout(350);
  await main.screenshot({ path: out + "/app-" + name + ".png" });
}
const status = await main.evaluate(async () => window.__TAURI_INTERNALS__.invoke("get_status"));
summary.main.status = { state: status.state, entries: status.filenameEntries, sources: status.sources?.map((s) => [s.id, s.itemCount, s.state]) };
summary.main.routeTexts = [];
for (const [name, label] of [["home", null], ["indexes", "Search & indexes"], ["activity", "Activity"], ["workflows", "Workflows"], ["preferences", "Preferences"]]) {
  if (label) {
    try { await main.getByText(label, { exact: false }).first().click({ timeout: 3000 }); } catch (e) { summary.main.routeTexts.push([name, "click-failed"]); }
  }
  await main.waitForTimeout(400);
  const info = await main.evaluate(() => {
    const text = document.body.innerText;
    const suspicious = /(os error|panic|undefined|null is not|TypeError|Error:|failed to|exception|\{[^}]*\})/i.exec(text.slice(0, 20000));
    return { heading: document.querySelector("h1")?.textContent ?? null, suspicious: suspicious ? suspicious[0] : null, textLen: text.length };
  });
  summary.main.routeTexts.push([name, info]);
  await shotMain(name);
  if (info.suspicious) summary.errorsVisible.push([name, info.suspicious]);
}

await browser.close().catch(() => {});
writeFileSync(out + "/audit.json", JSON.stringify(summary, null, 2));
console.log(JSON.stringify(summary, null, 2).slice(0, 8000));
