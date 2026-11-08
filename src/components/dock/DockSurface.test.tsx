// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import DockSurface from './DockSurface';
import { defaultLayout } from './layout';

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

  it('keeps Customize Widgets as a separate pill from the profile tabs', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );

    const shelf = container.querySelector<HTMLElement>('.dock-final-shelf');
    const tabs = container.querySelector<HTMLElement>('.dock-final-pills');
    const customize = container.querySelector<HTMLElement>('.dock-final-customize');
    expect(shelf).toBeTruthy();
    expect(tabs).toBeTruthy();
    expect(customize).toBeTruthy();
    expect(shelf?.contains(tabs!)).toBe(true);
    expect(shelf?.contains(customize!)).toBe(true);
    expect(tabs?.contains(customize!)).toBe(false);
  });

  it('renders single widgets as tiles and running apps as a group', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    expect(container.querySelectorAll('.dock-final-slot.dock-cell')).toHaveLength(8);
    expect(container.querySelectorAll('.dock-app')).toHaveLength(4);
    expect(container.querySelectorAll('.dock-dot')).toHaveLength(4);
    expect(container.querySelector('[data-widget-id="clock"] .dock-tip')?.textContent).toBe('Clock');
  });

  it('right-click opens a card even for Start', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    fireEvent.contextMenu(container.querySelector<HTMLElement>('[data-widget-id="windows"]')!);
    expect(screen.getByRole('dialog', { name: 'Windows Start card' })).toBeTruthy();
  });

  it('puts the resolved tile size on every slot', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    const slots = container.querySelectorAll<HTMLElement>('.dock-final-slot');
    expect(slots).toHaveLength(9);
    slots.forEach((slot) => {
      expect(slot.style.getPropertyValue('--icon-width')).toBe('46px');
      expect(slot.style.getPropertyValue('--icon-size')).toBe('46px');
      expect(slot.style.getPropertyValue('--dock-glyph-scale')).toBe('1.000');
    });
  });

  it('leaves the hot state when the window loses focus', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    const dock = container.querySelector<HTMLElement>('.dock-final')!;
    fireEvent.pointerEnter(container.querySelector<HTMLElement>('.dock-final-rail')!);
    expect(dock.getAttribute('data-hot')).toBe('true');

    act(() => window.dispatchEvent(new Event('blur')));
    expect(dock.getAttribute('data-hot')).toBeNull();
  });

  it('heats again on pointer move after blur', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    const dock = container.querySelector<HTMLElement>('.dock-final')!;
    const rail = container.querySelector<HTMLElement>('.dock-final-rail')!;
    fireEvent.pointerEnter(rail);
    act(() => window.dispatchEvent(new Event('blur')));
    fireEvent.pointerMove(rail);
    expect(dock.getAttribute('data-hot')).toBe('true');
  });

  it('leaves the hot state when the document is hidden', () => {
    const { container } = render(
      <DockSurface layout={defaultLayout()} onChange={() => {}} onCustomize={() => {}} />,
    );
    const dock = container.querySelector<HTMLElement>('.dock-final')!;
    fireEvent.pointerEnter(container.querySelector<HTMLElement>('.dock-final-rail')!);
    expect(dock.getAttribute('data-hot')).toBe('true');

    Object.defineProperty(document, 'hidden', { value: true, configurable: true });
    act(() => document.dispatchEvent(new Event('visibilitychange')));
    expect(dock.getAttribute('data-hot')).toBeNull();
    delete (document as unknown as Record<string, unknown>).hidden;
  });
});
