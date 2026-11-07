import type { CSSProperties } from 'react';

export interface TileSize { width: number; height: number }
/** The tile size the pixel values in dock-tiles.css were authored for. */
export const BASE_TILE = 46;
export const DEFAULT_TILE: TileSize = { width: BASE_TILE, height: BASE_TILE };
/** Inclusive integer ranges copied from preview-tahoe-glass.html. */
export const ICON_SIZE = { min: 32, max: 64 } as const;   // global tile height, --icon-size
export const ICON_WIDTH = { min: 32, max: 96 } as const;  // global tile width, --icon-width
export const TILE_WIDTH = { min: 32, max: 200 } as const; // per-widget width override
export const TILE_HEIGHT = { min: 32, max: 96 } as const; // per-widget height override
/** The value when it is an integer inside the range, else undefined. Invalid values are rejected, never clamped. */
export function validInt(value: unknown, range: { min: number; max: number }): number | undefined {
  return typeof value === 'number' && Number.isInteger(value) && value >= range.min && value <= range.max ? value : undefined;
}
/** A valid per-widget width or height replaces the matching shared (global) dimension. */
export function tileSize(shared: TileSize, item?: { tileWidth?: unknown; tileHeight?: unknown }): TileSize {
  return { width: validInt(item?.tileWidth, TILE_WIDTH) ?? shared.width, height: validInt(item?.tileHeight, TILE_HEIGHT) ?? shared.height };
}
export function largestTile(sizes: TileSize[]): TileSize {
  return sizes.reduce((max, s) => ({ width: Math.max(max.width, s.width), height: Math.max(max.height, s.height) }), { width: 0, height: 0 });
}
/** Inline custom properties for one slot; the glyph scale follows the smaller side (1.000 at 46). */
export function slotStyle(size: TileSize): CSSProperties {
  return { '--icon-width': `${size.width}px`, '--icon-size': `${size.height}px`, '--dock-glyph-scale': (Math.min(size.width, size.height) / BASE_TILE).toFixed(3) } as CSSProperties;
}
