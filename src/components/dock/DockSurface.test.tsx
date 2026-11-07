// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import DockSurface, { DOCK_DENSE_WIDTHS, DOCK_WIDTHS } from './DockSurface';
import { defaultLayout, effectiveSize } from './layout';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

describe('DockSurface web preview', () => {
  beforeEach(() => {
    window.history.replaceState(null, '', '/?window=dock&preview=1');
  });

  afterEach(() => {
    cleanup();
    window.history.replaceState(null, '', '/');
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  it('opens a sample widget card in an overlay instead of a 132px dock', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    const slot = container.querySelector<HTMLElement>('[data-widget-id="clock"]');
    expect(slot).toBeTruthy();
    fireEvent.click(slot!);

    const dialog = screen.getByRole('dialog', { name: 'Clock card' });
    expect(within(dialog).getByText('SAMPLE')).toBeTruthy();
    expect(within(dialog).getByText('4:27')).toBeTruthy();

    fireEvent.click(screen.getByRole('button', { name: 'Close Clock card' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('fits the rail to the available screen width in the native dock', () => {
    const innerWidth = window.innerWidth;
    const availWidth = window.screen.availWidth;
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      invoke: vi.fn().mockResolvedValue({ status: 'disconnected', updatedAt: 0 }),
      transformCallback: vi.fn().mockReturnValue(1),
    };
    Object.defineProperty(window, 'innerWidth', { configurable: true, writable: true, value: 1200 });
    Object.defineProperty(window.screen, 'availWidth', { configurable: true, writable: true, value: 1920 });
    try {
      const layout = defaultLayout();
      const daily = layout.profiles.daily;
      const expected = daily.reduce(
        (total, item) => total + DOCK_WIDTHS[item.id][effectiveSize(item)],
        0,
      ) + (daily.length - 1) * 5 + 16;
      const { container } = render(
        <DockSurface layout={layout} onChange={() => {}} onCustomize={() => {}} />,
      );
      const rail = container.querySelector<HTMLElement>('.dock-final-rail');
      expect(rail?.style.width).toBe(`${expected}px`);
      expect(expected).toBeGreaterThan(1200 - 24);
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, writable: true, value: innerWidth });
      Object.defineProperty(window.screen, 'availWidth', { configurable: true, writable: true, value: availWidth });
    }
  });

  it('compacts the rail so it fits a narrow screen', () => {
    const innerWidth = window.innerWidth;
    Object.defineProperty(window, 'innerWidth', { configurable: true, writable: true, value: 1000 });
    try {
      const layout = defaultLayout();
      const daily = layout.profiles.daily;
      const expected = daily.reduce(
        (total, item) => total + DOCK_DENSE_WIDTHS[item.id],
        0,
      ) + (daily.length - 1) * 5 + 16;
      const { container } = render(
        <DockSurface layout={layout} onChange={() => {}} onCustomize={() => {}} />,
      );
      const rail = container.querySelector<HTMLElement>('.dock-final-rail');
      expect(rail?.classList.contains('is-dense')).toBe(true);
      expect(rail?.style.width).toBe(`${expected}px`);
      expect(expected).toBeLessThanOrEqual(1000 - 8);
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, writable: true, value: innerWidth });
    }
  });
});
