import type { DockRegion } from '../../services/native';
import type { TileSize } from './sizing';
export type DockEdge = 'bottom' | 'left' | 'right';
export type DockAlign = 'start' | 'center' | 'end';
export interface Box { left: number; top: number; width: number; height: number }
export const MAG_RISE_PCT = 55, MAG_MAX = 1 + MAG_RISE_PCT / 100; // peak scale 1.55; magnify.ts imports MAG_MAX
export const MAG_ALONG_MIN = 80, TIP_ALLOWANCE = 38, BOTTOM_INSET = 14, TOP_MARGIN = 8;
/** Room a magnified tile and its tooltip need beyond the rail, inward: 64 at a 46px tile. Integer math keeps 55% of 40 exact. */
export const magHeadroom = (cross: number) => Math.ceil((MAG_RISE_PCT * cross) / 100) + TIP_ALLOWANCE;
/** Room the spread neighbours need along the axis: 80 until the largest along-size passes 101px. */
export const magAlong = (along: number) => Math.max(MAG_ALONG_MIN, Math.ceil((MAG_RISE_PCT * along) / 100) + 24);
export const ANCHOR: Record<DockAlign, number> = { start: 0, center: 0.5, end: 1 };
/** Window size and hit regions after the native resize; mirrors the edge/align layout in dock-tiles.css. `tile` is the largest configured tile in px. */
export function dockFit(edge: DockEdge, align: DockAlign, shelf: Box, rail: Box, viewport: { width: number; height: number }, hot: boolean, tile: TileSize) {
  const a = ANCHOR[align], vertical = edge !== 'bottom';
  const along = magAlong(vertical ? tile.height : tile.width), headroom = magHeadroom(vertical ? tile.width : tile.height);
  const left = Math.min(shelf.left, rail.left), top = Math.min(shelf.top, rail.top);
  const right = Math.max(shelf.left + shelf.width, rail.left + rail.width), bottom = Math.max(shelf.top + shelf.height, rail.top + rail.height);
  const width = vertical ? Math.ceil(Math.max(right - left, rail.width + headroom)) + 22 : Math.ceil(right - left + along) + 32;
  const height = vertical ? Math.ceil(bottom - top + along) + 32 : BOTTOM_INSET + Math.ceil(rail.height) + headroom + TOP_MARGIN;
  const dx = (width - viewport.width) * (edge === 'bottom' ? a : edge === 'left' ? 0 : 1);
  const dy = (height - viewport.height) * (edge === 'bottom' ? 1 : a);
  const place = (box: Box, radius: number): Required<DockRegion> => ({ x: Math.round(box.left + dx), y: Math.round(box.top + dy), width: Math.ceil(box.width), height: Math.ceil(box.height), radius });
  const r = place(rail, 18);
  if (hot && vertical) { r.y -= Math.round(along * a); r.height += along; r.width += headroom; if (edge === 'right') r.x -= headroom; }
  else if (hot) { r.x -= Math.round(along * a); r.width += along; r.y -= headroom; r.height += headroom; }
  return { width, height, regions: [place(shelf, 15), r] };
}
