import { describe, expect, it } from 'vitest';
import { dockFit, magAlong, magHeadroom } from './fit';
import { DEFAULT_TILE } from './sizing';

const box = (left: number, top: number, width: number, height: number) => ({ left, top, width, height });
const bottomView = { width: 1024, height: 146 };
const sideView = { width: 400, height: 900 };

describe('native fit geometry', () => {
  it('bottom/center cold: 912x146 window with the shelf above the rail', () => {
    expect(dockFit('bottom', 'center', box(332, 38, 360, 26), box(112, 72, 800, 60), bottomView, false, DEFAULT_TILE)).toEqual({
      width: 912,
      height: 146,
      regions: [
        { x: 276, y: 38, width: 360, height: 26, radius: 15 },
        { x: 56, y: 72, width: 800, height: 60, radius: 18 },
      ],
    });
  });

  it('bottom/center hot: the rail grows by the along allowance and headroom', () => {
    const fit = dockFit('bottom', 'center', box(332, 38, 360, 26), box(112, 72, 800, 60), bottomView, true, DEFAULT_TILE);
    expect(fit.regions[1]).toEqual({ x: 16, y: 8, width: 880, height: 124, radius: 18 });
  });

  it('bottom/start hot: the rail stays anchored to the left edge', () => {
    const fit = dockFit('bottom', 'start', box(16, 38, 360, 26), box(16, 72, 800, 60), bottomView, true, DEFAULT_TILE);
    expect(fit.width).toBe(912);
    expect(fit.regions[0]).toEqual({ x: 16, y: 38, width: 360, height: 26, radius: 15 });
    expect(fit.regions[1]).toEqual({ x: 16, y: 8, width: 880, height: 124, radius: 18 });
  });

  it('left/center: cold window and hot rail', () => {
    const shelf = box(14, 233, 360, 26);
    const rail = box(14, 267, 60, 400);
    expect(dockFit('left', 'center', shelf, rail, sideView, false, DEFAULT_TILE)).toEqual({
      width: 382,
      height: 546,
      regions: [
        { x: 14, y: 56, width: 360, height: 26, radius: 15 },
        { x: 14, y: 90, width: 60, height: 400, radius: 18 },
      ],
    });
    expect(dockFit('left', 'center', shelf, rail, sideView, true, DEFAULT_TILE).regions[1]).toEqual({
      x: 14,
      y: 50,
      width: 124,
      height: 480,
      radius: 18,
    });
  });

  it('right/end hot: the window and rail follow the right edge', () => {
    const fit = dockFit('right', 'end', box(26, 450, 360, 26), box(326, 484, 60, 400), sideView, true, DEFAULT_TILE);
    expect(fit.width).toBe(382);
    expect(fit.height).toBe(546);
    expect(fit.regions[0]).toEqual({ x: 8, y: 96, width: 360, height: 26, radius: 15 });
    expect(fit.regions[1]).toEqual({ x: 244, y: 50, width: 124, height: 480, radius: 18 });
  });

  it('preview sizing: bottom/center with an 80x64 tile', () => {
    const tile = { width: 80, height: 64 };
    const shelf = box(332, 20, 360, 26);
    const rail = box(112, 54, 800, 78);
    expect(dockFit('bottom', 'center', shelf, rail, bottomView, false, tile)).toEqual({
      width: 912,
      height: 174,
      regions: [
        { x: 276, y: 48, width: 360, height: 26, radius: 15 },
        { x: 56, y: 82, width: 800, height: 78, radius: 18 },
      ],
    });
    expect(dockFit('bottom', 'center', shelf, rail, bottomView, true, tile).regions[1]).toEqual({
      x: 16,
      y: 8,
      width: 880,
      height: 152,
      radius: 18,
    });
  });

  it('preview sizing: left/center with an 80x46 tile (width across, height along)', () => {
    const tile = { width: 80, height: 46 };
    const shelf = box(14, 233, 360, 26);
    const rail = box(14, 267, 94, 400);
    expect(dockFit('left', 'center', shelf, rail, sideView, false, tile)).toEqual({
      width: 382,
      height: 546,
      regions: [
        { x: 14, y: 56, width: 360, height: 26, radius: 15 },
        { x: 14, y: 90, width: 94, height: 400, radius: 18 },
      ],
    });
    expect(dockFit('left', 'center', shelf, rail, sideView, true, tile).regions[1]).toEqual({
      x: 14,
      y: 50,
      width: 176,
      height: 480,
      radius: 18,
    });
  });

  it('preview sizing: bottom/start hot with a 200x46 tile', () => {
    const fit = dockFit('bottom', 'start', box(16, 38, 360, 26), box(16, 72, 800, 60), bottomView, true, { width: 200, height: 46 });
    expect(fit.width).toBe(966);
    expect(fit.height).toBe(146);
    expect(fit.regions[1]).toEqual({ x: 16, y: 8, width: 934, height: 124, radius: 18 });
  });

  it('allowances follow the configured tile', () => {
    expect([32, 40, 46, 64, 96].map(magHeadroom)).toEqual([56, 60, 64, 74, 91]);
    expect([46, 101, 102, 200].map(magAlong)).toEqual([80, 80, 81, 134]);
  });
});
