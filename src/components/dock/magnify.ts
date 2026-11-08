import { useEffect, type RefObject } from 'react';
import { ANCHOR, MAG_MAX, type DockAlign, type DockEdge } from './fit'; // MAG_MAX (1.55) lives in fit.ts so the allowances share it
export const MAG_RADIUS = 150;
export function magnifyScale(distance: number): number {
  const d = Math.abs(distance);
  return d >= MAG_RADIUS ? 1 : 1 + ((MAG_MAX - 1) * (1 + Math.cos((Math.PI * d) / MAG_RADIUS))) / 2;
}
/** centers/sizes: un-magnified values along the dock axis. anchor 0 grows toward the end, 1 toward the start. */
export function magnifyLayout(centers: number[], sizes: number[], pointer: number | null, anchor: number) {
  const scales = centers.map((c) => (pointer == null ? 1 : magnifyScale(pointer - c)));
  const extras = scales.map((s, i) => sizes[i] * (s - 1));
  const total = extras.reduce((sum, e) => sum + e, 0);
  let before = 0;
  const shifts = extras.map((e) => { const shift = before + e / 2 - total * anchor; before += e; return shift; });
  return { scales, shifts, total };
}
export function useMagnify(rail: RefObject<HTMLElement | null>, edge: DockEdge, align: DockAlign, enabled: boolean) {
  useEffect(() => {
    const node = rail.current;
    if (!node || !enabled || window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches) return;
    let pointer: number | null = null;
    let frame = 0;
    const vertical = edge !== 'bottom';
    const anchor = ANCHOR[align];
    const apply = () => {
      frame = 0;
      const box = node.getBoundingClientRect();
      const cells = Array.from(node.querySelectorAll<HTMLElement>('.dock-cell'));
      const rects = cells.map((cell) => cell.getBoundingClientRect());
      const centers = rects.map((r) => (vertical ? r.top - box.top + r.height / 2 : r.left - box.left + r.width / 2));
      const sizes = rects.map((r) => (vertical ? r.height : r.width));
      const { scales, shifts, total } = magnifyLayout(centers, sizes, node.dataset.dragging === 'true' ? null : pointer, anchor);
      cells.forEach((cell, i) => {
        cell.style.setProperty('--mag', scales[i].toFixed(3));
        cell.style.setProperty('--shift', `${shifts[i].toFixed(2)}px`);
      });
      node.style.setProperty('--mag-before', `${(total * anchor).toFixed(2)}px`);
      node.style.setProperty('--mag-after', `${(total * (1 - anchor)).toFixed(2)}px`);
    };
    const schedule = () => { if (!frame) frame = requestAnimationFrame(apply); };
    const move = (event: PointerEvent) => {
      const box = node.getBoundingClientRect();
      pointer = vertical ? event.clientY - box.top : event.clientX - box.left;
      schedule();
    };
    const leave = () => { pointer = null; schedule(); };
    node.addEventListener('pointerenter', move);
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerleave', leave);
    window.addEventListener('blur', leave);
    return () => {
      node.removeEventListener('pointerenter', move);
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerleave', leave);
      window.removeEventListener('blur', leave);
      cancelAnimationFrame(frame);
      pointer = null;
      apply();
    };
  }, [rail, edge, align, enabled]);
}
