// @vitest-environment jsdom
import { afterEach, expect, test } from 'vitest';
import { cleanup, render } from '@testing-library/react';
import { ClockFace, Ring } from './glyphs';

afterEach(cleanup);

test('ring omits the fill for unknown values', () => {
  for (const value of [null, undefined, Number.NaN]) {
    const view = render(<Ring value={value} warnAt={0} />);
    expect(view.container.querySelector('.dock-ring-track')).not.toBeNull();
    expect(view.container.querySelector('.dock-ring-fill')).toBeNull();
    expect(view.container.querySelector('.dock-ring--warn')).toBeNull();
    view.unmount();
  }
});

test('ring fills by percent and warns at the threshold', () => {
  const half = render(<Ring value={50} />);
  const fill = half.container.querySelector('.dock-ring-fill');
  expect(parseFloat(fill?.getAttribute('stroke-dasharray') ?? '')).toBeCloseTo(47.12, 1);
  expect(half.container.querySelector('.dock-ring--warn')).toBeNull();
  half.unmount();

  const over = render(<Ring value={150} />);
  const full = over.container.querySelector('.dock-ring-fill');
  expect(parseFloat(full?.getAttribute('stroke-dasharray') ?? '')).toBeCloseTo(2 * Math.PI * 15, 6);
  over.unmount();

  const hot = render(<Ring value={85} warnAt={80} />);
  expect(hot.container.querySelector('.dock-ring--warn')).not.toBeNull();
  hot.unmount();
});

test('clock face hands follow the time', () => {
  const three = new Date(2026, 10, 7, 3, 0, 0);
  const plain = render(<ClockFace now={three} />);
  expect(plain.container.querySelector('.dock-clock-hour')?.getAttribute('transform')).toBe('rotate(90 18 18)');
  expect(plain.container.querySelector('.dock-clock-minute')?.getAttribute('transform')).toBe('rotate(0 18 18)');
  expect(plain.container.querySelector('.dock-clock-second')).toBeNull();
  plain.unmount();

  const ticking = render(<ClockFace now={three} seconds />);
  expect(ticking.container.querySelector('.dock-clock-second')).not.toBeNull();
  ticking.unmount();
});
