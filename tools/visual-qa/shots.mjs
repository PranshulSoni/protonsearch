#!/usr/bin/env node
// Visual QA screenshot harness. Renders the REAL React UI in Chromium against
// the browser mock bridge (clearly labelled as such in the report), at several
// viewport sizes, DPI scales and themes, then writes PNGs for human review.
//
// Usage: node shots.mjs [--base URL] [--out DIR] [--no-server]
import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const getArg = (name, fallback) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : fallback;
};
const baseUrl = getArg("--base", "http://127.0.0.1:4173");
const outDir = resolve(here, getArg("--out", "../../docs/qa/visual/screenshots"));
const startServer = !args.includes("--no-server");
mkdirSync(outDir, { recursive: true });

let server = null;
async function waitForServer(url, timeoutMs = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url + "/index.html");
      if (res.ok) return;
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("preview server did not start: " + url);
}

if (startServer) {
  const npmCmd = process.platform === "win32" ? "npx.cmd" : "npx";
  server = spawn(npmCmd, ["vite", "preview", "--port", "4173", "--strictPort"], {
    cwd: resolve(here, "../../apps/desktop"),
    stdio: "ignore",
    shell: process.platform === "win32",
  });
  await waitForServer(baseUrl);
}

const browser = await chromium.launch();
const results = [];

async function shot(name, { viewport, url, scale = 1, actions = [], fullPage = false }) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: scale,
    colorScheme: url.includes("theme=dark") ? "dark" : "light",
    reducedMotion: "no-preference",
  });
  const page = await context.newPage();
  await page.goto(baseUrl + url, { waitUntil: "networkidle" });
  await page.waitForTimeout(350);
  for (const action of actions) {
    if (action.type === "click") await page.getByText(action.text, { exact: false }).first().click();
    if (action.type === "press") await page.keyboard.press(action.key);
    if (action.type === "fill") await page.locator(action.selector).fill(action.value);
    if (action.type === "wait") await page.waitForTimeout(action.ms);
  }
  await page.waitForTimeout(250);
  const path = resolve(outDir, name + ".png");
  await page.screenshot({ path, fullPage });
  const size = page.viewportSize();
  results.push({ name, path, width: size?.width, height: size?.height, scale });
  await context.close();
}

const desktop = { width: 1180, height: 800 };
const laptopNarrow = { width: 900, height: 660 };
const launcherViewport = { width: 860, height: 640 };

// ---- Quick launcher states -------------------------------------------------
await shot("launcher-results-light", {
  viewport: launcherViewport,
  url: "/?view=launcher",
  actions: [{ type: "fill", selector: "input.search-input", value: "architecture" }, { type: "wait", ms: 200 }],
});
await shot("launcher-selected", {
  viewport: launcherViewport,
  url: "/?view=launcher",
  actions: [
    { type: "fill", selector: "input.search-input", value: "architecture" },
    { type: "wait", ms: 200 },
    { type: "press", key: "ArrowDown" },
    { type: "press", key: "ArrowDown" },
  ],
});
await shot("launcher-empty", { viewport: launcherViewport, url: "/?view=launcher" });
await shot("launcher-zero", {
  viewport: launcherViewport,
  url: "/?view=launcher",
  actions: [{ type: "fill", selector: "input.search-input", value: "zzzzzz not a real file" }, { type: "wait", ms: 250 }],
});
await shot("launcher-loading", {
  viewport: launcherViewport,
  url: "/?view=launcher&slow=1",
  actions: [{ type: "fill", selector: "input.search-input", value: "architecture" }, { type: "wait", ms: 120 }],
});
await shot("launcher-results-dark", {
  viewport: launcherViewport,
  url: "/?view=launcher&theme=dark",
  actions: [{ type: "fill", selector: "input.search-input", value: "architecture" }, { type: "wait", ms: 200 }],
});

// ---- Desktop app -----------------------------------------------------------
await shot("app-home-light", { viewport: desktop, url: "/?view=main" });
await shot("app-home-dark", { viewport: desktop, url: "/?view=main&theme=dark" });
await shot("app-home-narrow", { viewport: laptopNarrow, url: "/?view=main" });
await shot("app-home-dpi125", { viewport: desktop, url: "/?view=main", scale: 1.25 });
await shot("app-home-dpi200", { viewport: { width: 760, height: 520 }, url: "/?view=main", scale: 2 });
for (const [name, label] of [
  ["indexes", "Search & Indexes"],
  ["activity", "Activity"],
  ["workflows", "Workflows"],
  ["preferences", "Preferences"],
]) {
  await shot("app-" + name + "-light", {
    viewport: desktop,
    url: "/?view=main",
    actions: [{ type: "click", text: label }],
  });
  await shot("app-" + name + "-dark", {
    viewport: desktop,
    url: "/?view=main&theme=dark",
    actions: [{ type: "click", text: label }],
  });
}
await shot("app-indexes-error", {
  viewport: desktop,
  url: "/?view=main&state=error",
  actions: [{ type: "click", text: "Search & Indexes" }],
});

// ---- Onboarding ------------------------------------------------------------
const onboardingSteps = ["Welcome", "Choose locations", "Search depth", "Launcher", "Ready"];
for (let i = 0; i < onboardingSteps.length; i++) {
  const actions = [];
  for (let s = 0; s < i; s++) actions.push({ type: "click", text: "Continue" });
  await shot("onboarding-step-" + (i + 1) + "-" + onboardingSteps[i].toLowerCase().replace(/\\s+/g, "-"), {
    viewport: desktop,
    url: "/?view=main&onboard=1",
    actions,
  });
  await shot("onboarding-step-" + (i + 1) + "-dark", {
    viewport: desktop,
    url: "/?view=main&onboard=1&theme=dark",
    actions,
  });
}

await browser.close();
if (server) server.kill();

console.log(JSON.stringify({ outDir, count: results.length, results }, null, 2));
