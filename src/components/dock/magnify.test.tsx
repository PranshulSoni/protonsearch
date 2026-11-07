// @vitest-environment jsdom
import { cleanup, fireEvent, render } from '@testing-library/react';
import { useRef } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { magnifyLayout, magnifyScale, useMagnify } from './magnify';

// jsdom has no PointerEvent, so fireEvent.pointerMove would drop clientX.
window.PointerEvent ??= class extends MouseEvent {
  pointerId: number;
  constructor(t: string, i: PointerEventInit = {}) { super(t, i); this.pointerId = i.pointerId ?? 1; }
} as unknown as typeof PointerEvent;

function Rail({ enabled = true }: { enabled?: boolean }) {
  const rail = useRef<HTMLDivElement>(null);
  useMagnify(rail, 'bottom', 'center', enabled);
  return (
    <div ref={rail} className="dock-rail">
      <div className="dock-cell" />
      <div className="dock-cell" />
      <div className="dock-cell" />
    </div>
  );
}

function renderRail(enabled = true) {
  const { container } = render(<Rail enabled={enabled} />);
  const rail = container.querySelector<HTMLElement>('.dock-rail')!;
  const cells = Array.from(container.querySelectorAll<HTMLElement>('.dock-cell'));
  const rect = (left: number, top: number, width: number, height: number) => ({ x: left, y: top, left, top, width, height, right: left + width, bottom: top + height, toJSON() {} });
  rail.getBoundingClientRect = () => rect(0, 0, 174, 60);
  cells.forEach((cell, i) => { cell.getBoundingClientRect = () => rect(7 + 52 * i, 7, 46, 46); });
  return { rail, cells };
}

beforeEach(() => {
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { callback(0); return 0; });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe('magnification math', () => {
  it('scale peaks under the pointer and fades to 1 at the radius', () => {
    expect(magnifyScale(0)).toBeCloseTo(1.55, 6);
    expect(magnifyScale(75)).toBeCloseTo(1.275, 6);
    expect(magnifyScale(-75)).toBeCloseTo(1.275, 6);
    expect(magnifyScale(150)).toBe(1);
    expect(magnifyScale(400)).toBe(1);
  });

  it('neighbours shift symmetrically for a centered dock', () => {
    const { scales, shifts, total } = magnifyLayout([23, 75, 127], [46, 46, 46], 75, 0.5);
    expect(scales[1]).toBeCloseTo(1.55, 6);
    expect(scales[0]).toBeCloseTo(1.4024, 3);
    expect(scales[2]).toBeCloseTo(1.4024, 3);
    expect(shifts[1]).toBeCloseTo(0, 6);
    expect(shifts[0]).toBeCloseTo(-shifts[2], 6);
    expect(total).toBeCloseTo(62.32, 1);
  });

  it('start-aligned docks grow toward the end only', () => {
    const { shifts } = magnifyLayout([23, 75, 127], [46, 46, 46], 23, 0);
    expect(shifts.every((s) => s >= 0)).toBe(true);
    expect(shifts[0]).toBeCloseTo(12.65, 2);
  });

  it('no pointer leaves every cell at rest', () => {
    const { scales, shifts, total } = magnifyLayout([23, 75, 127], [46, 46, 46], null, 0.5);
    expect(scales).toEqual([1, 1, 1]);
    expect(shifts.every((s) => s === 0)).toBe(true);
    expect(total).toBe(0);
  });

  it('each cell grows by its own size', () => {
    const { scales, shifts, total } = magnifyLayout([23, 98], [46, 96], 23, 0);
    expect(scales[0]).toBeCloseTo(1.55, 6);
    expect(scales[1]).toBeCloseTo(1.275, 3);
    expect(shifts[0]).toBeCloseTo(12.65, 2);
    expect(shifts[1]).toBeCloseTo(38.5, 2);
    expect(total).toBeCloseTo(51.7, 1);
  });
});

describe('useMagnify', () => {
  it('pointer over the middle cell magnifies it most', () => {
    const { rail, cells } = renderRail();
    fireEvent.pointerMove(rail, { clientX: 82 });
    expect(cells[1].style.getPropertyValue('--mag')).toBe('1.550');
    expect(cells[0].style.getPropertyValue('--mag')).toBe('1.402');
    expect(rail.style.getPropertyValue('--mag-before')).toBe('31.16px');
  });

  it('leaving restores rest scale', () => {
    const { rail, cells } = renderRail();
    fireEvent.pointerMove(rail, { clientX: 82 });
    fireEvent.pointerLeave(rail);
    expect(cells[1].style.getPropertyValue('--mag')).toBe('1.000');
  });

  it('disabled or reduced motion sets nothing', () => {
    const off = renderRail(false);
    fireEvent.pointerMove(off.rail, { clientX: 82 });
    expect(off.cells[1].style.getPropertyValue('--mag')).toBe('');
    vi.stubGlobal('matchMedia', () => ({ matches: true }));
    const reduced = renderRail();
    fireEvent.pointerMove(reduced.rail, { clientX: 82 });
    expect(reduced.cells[1].style.getPropertyValue('--mag')).toBe('');
  });
});
