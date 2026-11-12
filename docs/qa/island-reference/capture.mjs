// Authoritative V9 reference capture (task-28). Regenerate with:
//   node docs/qa/island-reference/capture.mjs
import { chromium } from "file:///C:/Users/Pranshul/Documents/Projects/protonsearch/tools/visual-qa/node_modules/playwright/index.mjs";
import { mkdir, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import path from "node:path";

const ROOT = "C:\\Users\\Pranshul\\Documents\\Projects\\protonsearch";
const ISLAND_HTML = pathToFileURL(path.join(ROOT, "docs/reference/v10-handoff/protonsearch_dynamic_island_cards_v9_polished.html")).href;
const SETTINGS_HTML = pathToFileURL(path.join(ROOT, "docs/reference/v10-handoff/protonsearch_island_settings_placement_concept.html")).href;
const OUT = path.join(ROOT, "docs/qa/island-reference");
const WIDTHS = [1150, 860, 520, 430, 335];
const manifest = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function metrics(page, label, extra = {}) {
  return await page.evaluate(({ label, extra }) => {
    const box = (el) => { if (!el) return null; const r = el.getBoundingClientRect(); return { x: +r.x.toFixed(1), y: +r.y.toFixed(1), w: +r.width.toFixed(1), h: +r.height.toFixed(1) }; };
    const style = (el, props) => { if (!el) return null; const c = getComputedStyle(el); const o = {}; for (const p of props) o[p] = c.getPropertyValue(p); return o; };
    const q = (s) => document.querySelector(s);
    const dots = [...document.querySelectorAll(".pageDot")];
    return {
      label, extra,
      viewport: { w: innerWidth, h: innerHeight, dpr: devicePixelRatio },
      island: { box: box(q("#island")), classes: q("#island") ? q("#island").className : null, style: style(q("#island"), ["width","height","border-radius","background-color","border-color","border-width","box-shadow","top","left","transform","transition","opacity","visibility"]) },
      inner: { box: box(q("#panel")), style: style(q("#panel"), ["height","overflow-y","transform","opacity"]) },
      dots: { count: dots.length, navBox: box(q(".pageDots")), navStyle: style(q(".pageDots"), ["height","top","right","left","bottom","gap","background-color","border-top","position"]), active: q(".pageDot.active")?.getAttribute("aria-label") ?? null, samples: dots.slice(0,2).map((d) => ({ box: box(d), dot: box(d.querySelector("i")), dotStyle: style(d.querySelector("i"), ["width","height","border-radius","background-color","transition"]) })) },
      fonts: {
        searchInput: style(q(".searchRow input"), ["font-size","font-weight","letter-spacing","color"]),
        cardH2: style(q(".cardTop h2"), ["font-size","font-weight","letter-spacing","line-height","color"]),
        cardP: style(q(".cardTop p"), ["font-size","line-height","color"]),
        itemStrong: style(q(".itemText strong"), ["font-size","font-weight","line-height"]),
        itemSmall: style(q(".itemText small"), ["font-size","margin-top","color"]),
        filter: style(q(".filter"), ["font-size","padding","min-height","border-radius","color"]),
        resultFoot: style(q(".resultFoot"), ["font-size","min-height","color"]),
        inlineNav: style(q(".inlineNav button"), ["font-size","padding","min-height","border-radius"]),
        clipTitle: style(q(".clipRow b"), ["font-size","font-weight"]),
        clipSmall: style(q(".clipRow small"), ["font-size","color"]),
        snippet: style(q(".snippet"), ["font-size","line-height","padding","border-radius","max-height"]),
        chooseTitle: style(q(".choiceText .choiceTitle"), ["font-size","font-weight"]),
        chooseMeta: style(q(".choiceText .choiceMeta"), ["font-size","color"]),
        settingTile: style(q(".settingTile"), ["width","height","min-height","border-radius","background-color","border-color"]),
        settingToggle: style(q(".settingToggle"), ["padding","font-size"]),
        settingExpand: style(q(".settingExpand"), ["width","min-width"]),
        agentStatus: style(q(".agentStatus"), ["font-size","padding","border-radius","color"]),
        agentBtn: style(q(".agentBtn"), ["font-size","min-height","padding","border-radius"]),
        notificationTitle: style(q(".notificationText b"), ["font-size"]),
        notificationBody: style(q(".notificationText small"), ["font-size","line-height","color"]),
      },
      counts: { results: document.querySelectorAll(".itemRow").length, filters: document.querySelectorAll(".filter").length, clipRows: document.querySelectorAll(".clipRow").length, agents: document.querySelectorAll(".agentList .clipRow").length, settings: document.querySelectorAll(".settingTile").length, notifications: document.querySelectorAll(".notificationText").length },
    };
  }, { label, extra });
}

async function shot(page, width, state, opts = {}) {
  const dir = path.join(OUT, "v9", String(width));
  await mkdir(dir, { recursive: true });
  const file = path.join(dir, state + ".png");
  await page.locator(opts.stage ? "#stage" : "#island").screenshot({ path: file });
  manifest.push({ width, state, file: path.relative(OUT, file).replaceAll("\\", "/"), viewport: page.viewportSize() });
  const m = await metrics(page, state);
  await writeFile(path.join(dir, state + ".metrics.json"), JSON.stringify(m, null, 2));
  return m;
}

async function islandAt(page, width) {
  await page.goto(ISLAND_HTML, { waitUntil: "load" });
  await page.setViewportSize({ width, height: 700 });
  await sleep(150);
}

const browser = await chromium.launch({ headless: true });
for (const width of WIDTHS) {
  const context = await browser.newContext({ viewport: { width, height: 700 }, deviceScaleFactor: 1 });
  const page = await context.newPage();

  await islandAt(page, width);
  await shot(page, width, "00-hidden-stage", { stage: true });

  await page.click("#summon"); await sleep(300);
  await shot(page, width, "01-compact-search");

  await page.fill("#searchBox", "arch"); await sleep(750);
  await shot(page, width, "02-expanded-search");

  await page.fill("#searchBox", "zzzzzz"); await sleep(650);
  await shot(page, width, "03-zero-results");

  await islandAt(page, width);
  await page.click("#summon"); await sleep(300);
  const cardNames = ["search","media","clipboard","agent","controls","browser","git","notifications"];
  for (let i = 0; i < 8; i++) {
    if (i !== 0) { await page.click('.pageDot[data-page="' + i + '"]'); await sleep(800); }
    const slug = String(i).padStart(2, "0") + "-" + cardNames[i];
    await shot(page, width, "04-card-" + slug);
    if (i === 1) { await page.click('[data-media="output"]'); await sleep(800); await shot(page, width, "04-card-01b-media-output"); }
    if (i === 2) {
      for (const type of ["All","Text","Images","Links","Code","Files","Pinned"]) {
        await page.click('[data-cliptype="' + type + '"]'); await sleep(250);
        await shot(page, width, "04-card-02-clipboard-" + type.toLowerCase());
      }
    }
    if (i === 3) { await page.click('[data-agentaction="spawn"]'); await page.click('[data-agentaction="spawn"]'); await sleep(350); await shot(page, width, "04-card-03b-agent-hierarchy"); }
    if (i === 4) {
      await page.click('[data-expand="wifi"]'); await sleep(800); await shot(page, width, "04-card-04b-controls-wifi");
      await page.click('[data-detail=""]'); await sleep(800);
      await page.click('[data-expand="bluetooth"]'); await sleep(800); await shot(page, width, "04-card-04c-controls-bluetooth");
      await page.click('[data-detail=""]'); await sleep(800);
    }
    if (i === 7) {
      await page.click('[data-notification]'); await sleep(200);
      await page.click('[data-notification]'); await sleep(250);
      await shot(page, width, "04-card-07b-notifications-1");
      await page.click('[data-notification]'); await sleep(250);
      await shot(page, width, "04-card-07c-notifications-0");
    }
  }

  await page.emulateMedia({ reducedMotion: "reduce" });
  await islandAt(page, width);
  await page.click("#summon"); await sleep(200);
  await shot(page, width, "05-reduced-motion-compact");
  await context.close();
}
const browserSettings = await chromium.launch({ headless: true });
{
  const context = await browserSettings.newContext({ viewport: { width: 1240, height: 980 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  await page.goto(SETTINGS_HTML, { waitUntil: "load" }); await sleep(300);
  const settingsMetrics = async (label) => {
    const m = await page.evaluate((label) => {
      const box = (el) => { if (!el) return null; const r = el.getBoundingClientRect(); return { x:+r.x.toFixed(1), y:+r.y.toFixed(1), w:+r.width.toFixed(1), h:+r.height.toFixed(1) }; };
      const style = (el, props) => { if (!el) return null; const c = getComputedStyle(el); const o = {}; for (const p of props) o[p] = c.getPropertyValue(p); return o; };
      const q = (s) => document.querySelector(s);
      return {
        label, viewport: { w: innerWidth, h: innerHeight },
        frame: { box: box(q(".frame")), style: style(q(".frame"), ["max-width","border-radius","grid-template-columns","min-height","box-shadow","background-color","border-color"]) },
        side: { box: box(q(".side")), style: style(q(".side"), ["padding","gap","background-color","border-right"]) },
        main: { box: box(q(".main")), style: style(q(".main"), ["padding","min-width"]) },
        h1: style(q(".h1"), ["font-size","font-weight","letter-spacing","margin"]),
        lead: style(q(".lead"), ["font-size","line-height","color","margin"]),
        eyebrow: style(q(".eyebrow"), ["font-size","font-weight","letter-spacing","color"]),
        tabs: style(q(".tabs button.active"), ["padding","font-size","font-weight","border-bottom-color","color"]),
        monitorarea: { box: box(q(".monitorarea")), style: style(q(".monitorarea"), ["padding","min-height","border-radius","background-color","border-color"]) },
        screen: { box: box(q(".screen")), style: style(q(".screen"), ["width","aspect-ratio","border","border-radius","box-shadow","background-color"]) },
        ghost: { box: box(q(".ghost")), style: style(q(".ghost"), ["width","height","border-radius","background-color","border-color","padding","gap","box-shadow"]) },
        anchors: [...document.querySelectorAll(".anchor")].map((a) => ({ box: box(a), active: a.classList.contains("active"), style: style(a, ["height","border-radius","background-color","border-color"]) })),
        fields: [...document.querySelectorAll(".formline input, .formline select")].slice(0,3).map((e) => ({ tag: e.tagName, box: box(e), style: style(e, ["width","padding","font-size","border-radius","border-color"]) })),
        sectionrow: style(q(".sectionrow strong"), ["font-size","font-weight"]),
        cardrow: { box: box(q(".cardrow")), style: style(q(".cardrow"), ["min-height","padding","gap","border-bottom"]) },
        ctitle: style(q(".ctitle"), ["font-size","font-weight"]),
        tag: style(q(".tag"), ["font-size","padding","border-radius"]),
        miniisl: { box: box(q(".miniisl")), style: style(q(".miniisl"), ["height","border-radius","background-color","border-color"]) },
        dot: style(q(".dot"), ["width","height","border-radius","background-color"]),
        dotActive: style(q(".dot.active"), ["width","height","border-radius","background-color"]),
        buttons: [...document.querySelectorAll(".bottom .btn")].map((b) => ({ text: b.textContent.trim(), style: style(b, ["padding","font-size","border-radius","background-color","border-color","color"]) })),
        cardlist: box(q(".cardlist")),
      };
    }, label);
    await writeFile(path.join(OUT, "settings", label + ".metrics.json"), JSON.stringify(m, null, 2));
  };
  await page.locator("#placement-app").screenshot({ path: path.join(OUT, "settings/01-placement-default.png") });
  manifest.push({ width: 1240, state: "settings-placement-default", file: "settings/01-placement-default.png" });
  await settingsMetrics("01-placement-default");
  await page.click('[data-tab="cards"]'); await sleep(250);
  await page.locator("#placement-app").screenshot({ path: path.join(OUT, "settings/02-cards-order.png") });
  manifest.push({ width: 1240, state: "settings-cards-order", file: "settings/02-cards-order.png" });
  await settingsMetrics("02-cards-order");
  await page.click('[data-tab="behavior"]'); await sleep(250);
  await page.locator("#placement-app").screenshot({ path: path.join(OUT, "settings/03-behavior.png") });
  manifest.push({ width: 1240, state: "settings-behavior", file: "settings/03-behavior.png" });
  await settingsMetrics("03-behavior");
  await page.click('[data-tab="placement"]'); await sleep(200);
  await page.click('[data-anchor="bottom_right"]'); await sleep(200);
  await page.locator("#placement-app").screenshot({ path: path.join(OUT, "settings/04-placement-bottom-right.png") });
  manifest.push({ width: 1240, state: "settings-placement-bottom-right", file: "settings/04-placement-bottom-right.png" });
  await settingsMetrics("04-placement-bottom-right");
  await page.click('[data-anchor="top_center"]'); await page.click('[data-monitor="1"]'); await sleep(200);
  await page.locator("#placement-app").screenshot({ path: path.join(OUT, "settings/05-placement-external.png") });
  manifest.push({ width: 1240, state: "settings-placement-external", file: "settings/05-placement-external.png" });
  await settingsMetrics("05-placement-external");
  await page.setViewportSize({ width: 860, height: 980 });
  await page.reload({ waitUntil: "load" }); await sleep(300);
  await page.locator("#placement-app").screenshot({ path: path.join(OUT, "settings/06-placement-860.png") });
  manifest.push({ width: 860, state: "settings-placement-860", file: "settings/06-placement-860.png" });
  await settingsMetrics("06-placement-860");
  await context.close();
}
await writeFile(path.join(OUT, "manifest.json"), JSON.stringify({ generatedAt: new Date().toISOString(), source: "docs/reference/v10-handoff/protonsearch_dynamic_island_cards_v9_polished.html", widths: WIDTHS, entries: manifest }, null, 2));
await browser.close();
await browserSettings.close();
console.log("captured", manifest.length, "screenshots");
