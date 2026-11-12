// Brand raster pipeline: the app/exe/tray icon is the dark tile mark
// (assets/brand/protonsearch-tile.webp); every in-app logo is the bare P
// (assets/brand/protonsearch-p.webp). Run from the repo root:
//   node tools/visual-qa/brand-raster.mjs [preview.png]
// It also runs `npx tauri icon` and then writes the sharpened .ico and tray sizes.
import { chromium } from "playwright";
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const src = (file) => "data:image/webp;base64," + readFileSync(resolve(root, "assets/brand", file)).toString("base64");
const SIZES = [16, 20, 24, 32, 48, 64, 128, 256, 1024];
// Every size Explorer, the taskbar and the tray ask for at 100-200% scaling.
const ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 128, 256];

const browser = await chromium.launch();
const page = await browser.newPage();
const out = await page.evaluate(async ({ tileSrc, pSrc, sizes, icoSizes }) => {
  const load = (s) => new Promise((ok, fail) => { const i = new Image(); i.onload = () => ok(i); i.onerror = fail; i.src = s; });
  const canvas = (w, h) => Object.assign(document.createElement("canvas"), { width: w, height: h });
  const ctx = (c) => { const x = c.getContext("2d"); x.imageSmoothingQuality = "high"; return x; };
  const pixels = (img) => { const c = canvas(img.width, img.height); ctx(c).drawImage(img, 0, 0); return ctx(c).getImageData(0, 0, c.width, c.height); };
  const scaled = (master, s) => {
    let cur = master;
    while (cur.width >= s * 4) { const h = canvas(cur.width >> 1, cur.height >> 1); ctx(h).drawImage(cur, 0, 0, h.width, h.height); cur = h; }
    const c = canvas(s, s); ctx(c).drawImage(cur, 0, 0, s, s); return c;
  };
  const bbox = (W, H, hit) => {
    let x0 = W, y0 = H, x1 = -1, y1 = -1;
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (hit(x, y)) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
    return { x0, y0, x1, y1 };
  };

  // Tile: find the dark body, then the mark (silver or red glow) inside it, and zoom so the
  // mark fills ~74% of the icon while the crop stays inside the body.
  const tile = await load(tileSrc);
  const t = pixels(tile), td = t.data, TW = t.width;
  const sum = (o) => td[o] + td[o + 1] + td[o + 2];
  const body = bbox(TW, t.height, (x, y) => td[(y * TW + x) * 4 + 3] > 200 && sum((y * TW + x) * 4) < 240);
  const pad = Math.round((body.x1 - body.x0) * 0.06), edge = Math.round((body.x1 - body.x0) * 0.03);
  const mark = bbox(TW, t.height, (x, y) => {
    if (x < body.x0 + pad || x > body.x1 - pad || y < body.y0 + pad || y > body.y1 - pad) return false;
    const o = (y * TW + x) * 4;
    return sum(o) > 300 || (td[o] > 110 && td[o] > 2 * td[o + 1]);
  });
  const zoomed = (fill) => {
    const side = Math.min(Math.max(mark.x1 - mark.x0, mark.y1 - mark.y0) / fill, body.x1 - body.x0 - 2 * edge, body.y1 - body.y0 - 2 * edge);
    const at = (c, lo, hi) => Math.min(Math.max(c - side / 2, lo + edge), hi - edge - side);
    const c = canvas(1024, 1024), x = ctx(c);
    x.beginPath(); x.roundRect(0, 0, 1024, 1024, 205); x.clip();
    x.drawImage(tile, at((mark.x0 + mark.x1) / 2, body.x0, body.x1), at((mark.y0 + mark.y1) / 2, body.y0, body.y1), side, side, 0, 0, 1024, 1024);
    return c;
  };
  const tileMaster = zoomed(0.84);
  // Explorer lists and the tray draw 16-24 px: a tighter crop keeps the P legible there,
  // and a light unsharp mask restores the edges the downscale softens.
  const smallMaster = zoomed(0.93);
  const sharpen = (c, amount) => {
    const x = ctx(c), img = x.getImageData(0, 0, c.width, c.height), d = img.data, src = d.slice(), w = c.width;
    for (let y = 1; y < c.height - 1; y++) for (let i = 1; i < w - 1; i++) {
      const o = (y * w + i) * 4;
      if (src[o + 3] < 255) continue;
      for (let k = 0; k < 3; k++) {
        let blur = 0;
        for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) blur += src[o + (dy * w + dx) * 4 + k] * (dx && dy ? 1 : dx || dy ? 2 : 4);
        d[o + k] = src[o + k] + amount * (src[o + k] - blur / 16);
      }
    }
    x.putImageData(img, 0, 0);
    return c;
  };

  // Bare P: knock an opaque white background out (border-connected white, plus large flat-white
  // counters such as the bowl), unblending the 1 px fringe from white so edges stay clean on dark UI.
  const p = await load(pSrc);
  const P = pixels(p), pd = P.data, W = P.width, H = P.height, N = W * H;
  const holes = [];
  if (pd[3] > 250) {
    const minc = (i) => Math.min(pd[i * 4], pd[i * 4 + 1], pd[i * 4 + 2]);
    const bg = new Uint8Array(N), seen = new Uint8Array(N), stack = new Int32Array(N);
    for (let s = 0; s < N; s++) {
      if (seen[s] || minc(s) < 236) continue;
      const members = []; let top = 0, border = false, total = 0;
      const push = (j) => { if (!seen[j] && minc(j) >= 236) { seen[j] = 1; stack[top++] = j; } };
      seen[s] = 1; stack[top++] = s;
      while (top) {
        const i = stack[--top], x = i % W, y = (i / W) | 0;
        members.push(i); total += minc(i);
        if (x === 0 || y === 0 || x === W - 1 || y === H - 1) border = true;
        if (x > 0) push(i - 1); if (x < W - 1) push(i + 1); if (y > 0) push(i - W); if (y < H - 1) push(i + W);
      }
      const flat = members.length > N * 0.003 && total / members.length >= 250;
      if (!border && members.length > N * 0.001) holes.push({ size: members.length, mean: Math.round(total / members.length), at: [s % W, (s / W) | 0], bg: flat });
      if (border || flat) for (const i of members) bg[i] = 1;
    }
    for (let i = 0; i < N; i++) {
      const o = i * 4;
      if (bg[i]) { pd[o + 3] = 0; continue; }
      const x = i % W, y = (i / W) | 0;
      if (!((x > 0 && bg[i - 1]) || (x < W - 1 && bg[i + 1]) || (y > 0 && bg[i - W]) || (y < H - 1 && bg[i + W]))) continue;
      const a = Math.max(255 - pd[o], 255 - pd[o + 1], 255 - pd[o + 2]) / 255;
      if (a <= 0) { pd[o + 3] = 0; continue; }
      for (let c = 0; c < 3; c++) pd[o + c] = Math.round((pd[o + c] - 255 * (1 - a)) / a);
      pd[o + 3] = Math.round(a * 255);
    }
  }
  const pc = canvas(W, H); ctx(pc).putImageData(P, 0, 0);
  const pb = bbox(W, H, (x, y) => pd[(y * W + x) * 4 + 3] > 16);
  const pw = pb.x1 - pb.x0 + 1, ph = pb.y1 - pb.y0 + 1, k = 1024 / (Math.max(pw, ph) * 1.04);
  const pMaster = canvas(1024, 1024);
  ctx(pMaster).drawImage(pc, pb.x0, pb.y0, pw, ph, (1024 - pw * k) / 2, (1024 - ph * k) / 2, pw * k, ph * k);

  const png = (c) => c.toDataURL("image/png");
  const res = { tile: {}, p: {}, ico: {}, holes };
  for (const s of sizes) {
    res.tile[s] = png(s === 1024 ? tileMaster : scaled(tileMaster, s));
    res.p[s] = png(s === 1024 ? pMaster : scaled(pMaster, s));
  }
  const small = {};
  for (const s of icoSizes) {
    small[s] = s <= 48 ? sharpen(scaled(smallMaster, s), 0.6) : scaled(tileMaster, s);
    res.ico[s] = png(small[s]);
  }
  // Preview on a taskbar-grey strip: the 256 tile, the small sizes at 1x, and 32 px blown up 4x.
  const pv = canvas(560, 280), v = ctx(pv);
  v.fillStyle = "#202020"; v.fillRect(0, 0, 560, 280);
  v.drawImage(small[256], 12, 12);
  let px = 290;
  for (const s of [16, 20, 24, 32]) { v.drawImage(small[s], px, 20); px += s + 12; }
  v.imageSmoothingEnabled = false; v.drawImage(small[32], 290, 70, 128, 128);
  res.preview = png(pv);
  return res;
}, { tileSrc: src("protonsearch-tile.webp"), pSrc: src("protonsearch-p.webp"), sizes: SIZES, icoSizes: ICO_SIZES });
await browser.close();

const write = (path, url) => writeFileSync(resolve(root, path), Buffer.from(url.split(",")[1], "base64"));
for (const s of SIZES) {
  write(`apps/desktop/public/brand/app-tile-${s}.png`, out.tile[s]);
  write(`apps/desktop/public/brand/transparent-p-${s}.png`, out.p[s]);
}
write("apps/desktop/src-tauri/icons/source-icon.png", out.tile[1024]);
// tauri icon makes every platform icon from the source; the Windows .ico, the 32 px window
// icon and the tray sizes are then replaced with the per-size sharpened renders.
execSync("npx tauri icon src-tauri/icons/source-icon.png", { cwd: resolve(root, "apps/desktop"), stdio: "ignore" });
const frames = ICO_SIZES.map((s) => Buffer.from(out.ico[s].split(",")[1], "base64"));
const head = Buffer.alloc(6 + 16 * frames.length);
head.writeUInt16LE(1, 2);
head.writeUInt16LE(frames.length, 4);
let offset = head.length;
frames.forEach((frame, i) => {
  const s = ICO_SIZES[i] % 256; // 0 means 256 in an ICO directory entry
  head.writeUInt8(s, 6 + 16 * i);
  head.writeUInt8(s, 7 + 16 * i);
  head.writeUInt16LE(1, 10 + 16 * i);
  head.writeUInt16LE(32, 12 + 16 * i);
  head.writeUInt32LE(frame.length, 14 + 16 * i);
  head.writeUInt32LE(offset, 18 + 16 * i);
  offset += frame.length;
});
const ico = Buffer.concat([head, ...frames]);
writeFileSync(resolve(root, "apps/desktop/src-tauri/icons/icon.ico"), ico);
writeFileSync(resolve(root, "apps/desktop/public/favicon.ico"), ico);
write("apps/desktop/src-tauri/icons/32x32.png", out.ico[32]);
for (const s of [16, 20, 24]) write(`apps/desktop/src-tauri/icons/tray-${s}.png`, out.ico[s]);
if (process.argv[2]) write(process.argv[2], out.preview);
console.log(JSON.stringify(out.holes));
