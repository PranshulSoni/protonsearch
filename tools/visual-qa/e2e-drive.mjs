#!/usr/bin/env node
// Drive the REAL Tauri/WebView2 app over CDP: capture true app windows with live
// engine data and measure input-to-visible latency. This is not a mock harness.
//
// Usage:
//   node e2e-drive.mjs --exe ../../target/debug/ProtonSearch.exe --out ../docs/qa/visual/app-captures
//   node e2e-drive.mjs --attach-only          (connect to an app already running with Tauri debug port)
//
// Implementation notes:
// - Tauri/WebView2 honours WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, so we launch the
//   app with --remote-debugging-port=9222 and attach with Playwright CDP.
// - The launcher is a pre-created hidden window; if it is hidden, CDP still lists
//   its page. We call the app's Tauri command surface through the page when needed.
// - Latency measurement is end-to-end: performance.now() immediately before typing
//   the query, until the result list has rendered a matching row (double rAF).
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdirSync, existsSync } from "node:fs";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const getArg = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const exe = getArg("--exe", resolve(process.cwd(), "../../target/debug/ProtonSearch.exe"));
const outDir = resolve(getArg("--out", resolve(process.cwd(), "../../docs/qa/visual/app-captures")));
const attachOnly = args.includes("--attach-only");
// Debug builds load tauri.conf.json devUrl (http://localhost:5173); serve the
// production frontend there so the real app renders without a dev server.
const serveFrontend = args.includes("--serve");
const dataDir = getArg("--data-dir", resolve(process.cwd(), "../../.devdata"));
const port = Number(getArg("--port", "9222"));
mkdirSync(outDir, { recursive: true });

let appProcess = null;
async function connect() {
  const deadline = Date.now() + 45000;
  while (Date.now() < deadline) {
    try {
      const res = await fetch("http://127.0.0.1:" + port + "/json/version");
      if (res.ok) return await chromium.connectOverCDP("http://127.0.0.1:" + port);
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error("Could not connect to WebView2 CDP on port " + port);
}

let server = null;
if (serveFrontend) {
  mkdirSync(dataDir, { recursive: true });
  const npmCmd = process.platform === "win32" ? "npx.cmd" : "npx";
  server = spawn(npmCmd, ["vite", "preview", "--port", "5173", "--strictPort"], {
    cwd: resolve(process.cwd(), "../../apps/desktop"),
    stdio: "ignore",
    shell: process.platform === "win32",
  });
  const deadline = Date.now() + 30000;
  let up = false;
  while (Date.now() < deadline) {
    try {
      const res = await fetch("http://127.0.0.1:5173/index.html");
      if (res.ok) { up = true; break; }
    } catch { /* retry */ }
    await new Promise((r) => setTimeout(r, 300));
  }
  if (!up) throw new Error("frontend preview server did not start on 5173");
}

if (!attachOnly) {
  if (!existsSync(exe)) {
    console.error("exe not found: " + exe);
    process.exit(2);
  }
  appProcess = spawn(exe, [], {
    env: {
      ...process.env,
      PROTONSEARCH_DATA_DIR: dataDir,
      PROTONSEARCH_CDP_PORT: String(port),
      // The sandbox cannot write %LOCALAPPDATA%; keep the WebView2 profile local.
      WEBVIEW2_USER_DATA_FOLDER: resolve(dataDir, "webview2"),
    },
    stdio: "ignore",
  });
}

const browser = await connect();
// Tauri creates both webviews at startup; wait until both have loaded.
async function collectPages() {
  const found = [];
  for (const context of browser.contexts()) {
    for (const page of context.pages()) found.push(page);
  }
  return found;
}
let pages = [];
const pageDeadline = Date.now() + 40000;
while (Date.now() < pageDeadline) {
  pages = await collectPages();
  const ready = [];
  for (const page of pages) {
    try {
      const windowKind = await page.evaluate(() => document.documentElement.dataset.window ?? null);
      if (windowKind) ready.push(page);
    } catch {
      /* page still navigating */
    }
  }
  if (ready.length >= 2) break;
  await new Promise((r) => setTimeout(r, 500));
}
console.log(JSON.stringify({ connected: true, pages: pages.length, urls: pages.map((p) => p.url()) }));

async function windowKind(page) {
  try {
    return await page.evaluate(() => document.documentElement.dataset.window ?? null);
  } catch {
    return null;
  }
}
const kinds = await Promise.all(pages.map((page) => windowKind(page)));
const launcher = pages[kinds.indexOf("launcher")] ?? pages[0];
const main = pages[kinds.indexOf("main")] ?? pages.find((page) => page !== launcher) ?? pages[0];
if (!launcher || !main) throw new Error("could not identify launcher/main webviews");
// The launcher is pre-warmed hidden; make it visible through the shell so the
// screenshots reflect the real summoned window.
try {
  await launcher.evaluate(() => window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke("show_launcher"));
} catch { /* attach-only or browser build */ }
await new Promise((r) => setTimeout(r, 700));

async function shot(page, name) {
  try {
    const path = resolve(outDir, name + ".png");
    await page.screenshot({ path, animations: "disabled" });
    return path;
  } catch (error) {
    console.error("screenshot failed", name, String(error));
    return null;
  }
}

// --- Real latency measurement -------------------------------------------------
const measurements = [];
async function measureQuery(page, query, label) {
  await page.evaluate(() => {
    const input = document.querySelector("input");
    if (input) {
      input.focus();
      input.value = "";
      input.dispatchEvent(new Event("input", { bubbles: true }));
    }
  });
  await page.waitForTimeout(400);
  const result = await page.evaluate(async (q) => {
    const input = document.querySelector("input.search-input") ?? document.querySelector("input");
    if (!input) return { error: "no input" };
    const listSelector = '[role="option"], .result, [role="listbox"] [aria-selected]';
    input.focus();
    const start = performance.now();
    // Native setter + input event: exactly what a keystroke produces.
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
    setter.call(input, q);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    const deadline = performance.now() + 3000;
    let visibleAt = null;
    while (performance.now() < deadline) {
      const rows = document.querySelectorAll(listSelector);
      if (rows.length > 0) {
        await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        visibleAt = performance.now();
        break;
      }
      await new Promise((r) => setTimeout(r, 4));
    }
    return { start, visibleAt, rows: document.querySelectorAll(listSelector).length, tookMs: visibleAt ? visibleAt - start : null };
  }, query);
  measurements.push({ label, query, ...result });
  return result;
}

// Launcher states (real data).
try {
  await launcher.evaluate(() => document.querySelector("input")?.focus());
} catch {
  /* hidden window may not accept focus; commands still work */
}
await shot(launcher, "real-launcher-empty");
for (const [label, query] of [
  ["architecture", "arch"],
  ["recent-exact", "readme"],
  ["typo", "protonserch"],
  ["zero", "zzzzzz-no-such-file"],
]) {
  await measureQuery(launcher, query, label);
  await shot(launcher, "real-launcher-" + label);
}

// Desktop app routes.
await shot(main, "real-app-home");
for (const label of ["Search & Indexes", "Activity", "Workflows", "Preferences"]) {
  try {
    await main.getByText(label, { exact: false }).first().click({ timeout: 3000 });
    await main.waitForTimeout(300);
    await shot(main, "real-app-" + label.toLowerCase().replace(/[^a-z0-9]+/g, "-"));
  } catch (error) {
    console.error("route capture failed", label, String(error));
  }
}

const report = { outDir, pages: pages.length, measurements, capturedAt: new Date().toISOString() };
console.log(JSON.stringify(report, null, 2));
try {
  const { writeFileSync } = await import("node:fs");
  writeFileSync(resolve(outDir, "latency-real-app.json"), JSON.stringify(report, null, 2));
} catch (error) {
  console.error("could not persist latency report", String(error));
}
try {
  await browser.close();
} catch {
  /* the app process may already be gone */
}
if (appProcess) appProcess.kill();
if (server) server.kill();
process.exit(0);