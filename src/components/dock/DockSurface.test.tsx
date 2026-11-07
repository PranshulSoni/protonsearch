// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import DockSurface from './DockSurface';
import { defaultLayout } from './layout';

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
});
