import { describe, expect, it } from 'vitest';
import { ICON_SIZE, TILE_HEIGHT, TILE_WIDTH, largestTile, slotStyle, tileSize, validInt } from './sizing';

describe('tile sizing', () => {
  it('validInt accepts integers inside the range and rejects everything else', () => {
    expect(validInt(32, ICON_SIZE)).toBe(32);
    expect(validInt(64, ICON_SIZE)).toBe(64);
    expect(validInt(31, ICON_SIZE)).toBeUndefined();
    expect(validInt(65, ICON_SIZE)).toBeUndefined();
    expect(validInt(46.5, ICON_SIZE)).toBeUndefined();
    expect(validInt('46', ICON_SIZE)).toBeUndefined();
    expect(validInt(Number.NaN, ICON_SIZE)).toBeUndefined();
    expect(validInt(null, ICON_SIZE)).toBeUndefined();
    expect(validInt(undefined, ICON_SIZE)).toBeUndefined();
    expect(validInt(200, TILE_WIDTH)).toBe(200);
    expect(validInt(201, TILE_WIDTH)).toBeUndefined();
    expect(validInt(96, TILE_HEIGHT)).toBe(96);
    expect(validInt(97, TILE_HEIGHT)).toBeUndefined();
  });

  it('tileSize lets each valid override replace its own dimension', () => {
    const shared = { width: 80, height: 64 };
    expect(tileSize(shared)).toEqual({ width: 80, height: 64 });
    expect(tileSize(shared, { tileWidth: 120, tileHeight: 56 })).toEqual({ width: 120, height: 56 });
    expect(tileSize(shared, { tileWidth: 999, tileHeight: 56 })).toEqual({ width: 80, height: 56 });
    expect(tileSize(shared, { tileWidth: '120', tileHeight: 31 })).toEqual({ width: 80, height: 64 });
  });

  it('slotStyle exposes the resolved size and the glyph scale', () => {
    expect(slotStyle({ width: 46, height: 46 })).toEqual({
      '--icon-width': '46px',
      '--icon-size': '46px',
      '--dock-glyph-scale': '1.000',
    });
    expect(slotStyle({ width: 80, height: 64 })).toEqual({
      '--icon-width': '80px',
      '--icon-size': '64px',
      '--dock-glyph-scale': '1.391',
    });
    expect(slotStyle({ width: 32, height: 96 })).toEqual({
      '--icon-width': '32px',
      '--icon-size': '96px',
      '--dock-glyph-scale': '0.696',
    });
  });

  it('largestTile returns the widest and tallest sizes', () => {
    expect(
      largestTile([
        { width: 46, height: 46 },
        { width: 80, height: 32 },
        { width: 40, height: 96 },
      ]),
    ).toEqual({ width: 80, height: 96 });
    expect(largestTile([])).toEqual({ width: 0, height: 0 });
  });
});
