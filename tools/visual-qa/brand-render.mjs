// Brand asset pipeline: EVERY ProtonSearch logo/icon in the build is rendered
// from the single source of truth assets/brand/protonsearch-mark.svg.
// Outputs: app/window/tray icons (consumed by `npx tauri icon`), favicons, and
// the NSIS installer header/sidebar artwork. Run scripts/build-brand-assets.ps1.
import { chromium } from "playwright";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const markSvg = readFileSync(resolve(root, "assets/brand/protonsearch-mark.svg"), "utf8");
const iconsDir = resolve(root, "apps/desktop/src-tauri/icons");
const publicDir = resolve(root, "apps/desktop/public");
mkdirSync(iconsDir, { recursive: true });
mkdirSync(publicDir, { recursive: true });
const inner = markSvg.replace(/^<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");

function tileSvg(size, radiusRatio = 0.22) {
  const r = Math.round(size * radiusRatio);
  const inset = size * 0.16;
  const scale = (size - inset * 2) / 64;
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="' + size + '" height="' + size + '" viewBox="0 0 ' + size + ' ' + size + '">',
    '<rect width="' + size + '" height="' + size + '" rx="' + r + '" fill="#fbfcfa"/>',
    '<rect x="0.5" y="0.5" width="' + (size - 1) + '" height="' + (size - 1) + '" rx="' + r + '" fill="none" stroke="#102448" stroke-opacity="0.08"/>',
    '<g transform="translate(' + inset + ',' + inset + ') scale(' + scale + ')">' + inner + '</g>',
    "</svg>",
  ].join("");
}

function headerSvg() {
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="150" height="57" viewBox="0 0 150 57">',
    '<rect width="150" height="57" fill="#fbfcfa"/>',
    '<rect x="0.5" y="0.5" width="149" height="56" fill="none" stroke="#e0e5df"/>',
    '<g transform="translate(12,12) scale(0.515625)">' + inner + "</g>",
    '<text x="50" y="32" font-family="Segoe UI Variable, Segoe UI, sans-serif" font-size="12.5" font-weight="700" fill="#202b31" letter-spacing="-0.3">ProtonSearch</text>',
    '<text x="50" y="43" font-family="Segoe UI Variable, Segoe UI, sans-serif" font-size="6.5" font-weight="600" fill="#828e8b" letter-spacing="0.6">SEARCH &amp; COMMAND</text>',
    "</svg>",
  ].join("");
}

function sidebarSvg() {
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="164" height="314" viewBox="0 0 164 314">',
    '<rect width="164" height="314" fill="#f3f6f2"/>',
    '<rect x="163" y="0" width="1" height="314" fill="#e0e5df"/>',
    '<g transform="translate(34,34) scale(1.5)">' + inner + "</g>",
    '<text x="34" y="152" font-family="Segoe UI Variable, Segoe UI, sans-serif" font-size="17" font-weight="700" fill="#202b31" letter-spacing="-0.5">ProtonSearch</text>',
    '<text x="34" y="172" font-family="Segoe UI Variable, Segoe UI, sans-serif" font-size="10.5" font-weight="600" fill="#5c6862">Find anything, fast.</text>',
    '<text x="34" y="272" font-family="Segoe UI Variable, Segoe UI, sans-serif" font-size="9" font-weight="600" fill="#828e8b">LOCAL-FIRST</text>',
    '<text x="34" y="286" font-family="Segoe UI Variable, Segoe UI, sans-serif" font-size="9" font-weight="600" fill="#828e8b">NO TELEMETRY</text>',
    "</svg>",
  ].join("");
}

const browser = await chromium.launch();
async function shot(svg, width, height, outPath) {
  const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
  await page.setContent('<html><body style="margin:0;background:transparent">' + svg + "</body></html>", { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(80);
  await page.screenshot({ path: outPath, omitBackground: true });
  await page.close();
}

await shot(tileSvg(1024), 1024, 1024, resolve(iconsDir, "source-icon.png"));
for (const [size, name] of [[16, "favicon-16x16.png"], [32, "favicon-32x32.png"], [48, "favicon-48x48.png"], [180, "favicon-180x180.png"]]) {
  await shot(tileSvg(size), size, size, resolve(publicDir, name));
}
await shot(headerSvg(), 150, 57, resolve(iconsDir, "nsis-header.png"));
await shot(sidebarSvg(), 164, 314, resolve(iconsDir, "nsis-sidebar.png"));
await browser.close();
writeFileSync(resolve(iconsDir, "BRAND_SOURCE.txt"), "Rendered from assets/brand/protonsearch-mark.svg by tools/brand/render-brand-assets.mjs\n");
console.log("rendered brand assets (app icon source, favicons, NSIS artwork)");