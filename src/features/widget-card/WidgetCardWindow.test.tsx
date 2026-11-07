// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WidgetCardWindow } from './WidgetCardWindow';

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
let emit: ((event: { payload: unknown }) => void) | undefined;

vi.mock('@tauri-apps/api/core', () => ({ invoke, isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen }));

function snapshot(data: unknown) {
  return { status: 'live', data, updatedAt: Date.now() };
}

beforeEach(() => {
  emit = undefined;
  Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {}, configurable: true });
  listen.mockImplementation((_name: string, callback: (event: { payload: unknown }) => void) => {
    emit = callback;
    return Promise.resolve(() => {});
  });
  invoke.mockImplementation((command: string) => {
    if (command === 'widget_card_state') return Promise.resolve({ id: 'spotify', size: 'standard' });
    if (command === 'provider_snapshot') return Promise.resolve(snapshot({ title: 'Track', artist: 'Artist', playing: true }));
    if (command === 'known_folders_snapshot') return Promise.resolve(snapshot({ folders: [], recent: [] }));
    if (command === 'running_apps_snapshot') return Promise.resolve(snapshot({ items: [] }));
    return Promise.resolve();
  });
});

afterEach(() => {
  cleanup();
  invoke.mockReset();
  listen.mockReset();
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe('WidgetCardWindow', () => {
  it('loads the current card state on mount and subscribes to show events', async () => {
    const { container } = render(<WidgetCardWindow />);
    await waitFor(() => expect(container.querySelector('[data-widget-card="spotify"]')).toBeTruthy());
    expect(document.body.dataset.surface).toBe('widget');
    expect(listen).toHaveBeenCalledWith('widget-card:show', expect.any(Function));

    act(() => emit?.({ payload: { id: 'clock', size: 'wide' } }));
    await waitFor(() => expect(container.querySelector('[data-widget-card="clock"]')).toBeTruthy());
  });

  it('hides the native window on Escape and on the close button', async () => {
    render(<WidgetCardWindow />);
    await screen.findByRole('button', { name: /Close Spotify card/ });
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('hide_widget_card'));
    fireEvent.click(screen.getByRole('button', { name: /Close Spotify card/ }));
    await waitFor(() => expect(invoke.mock.calls.filter(([command]) => command === 'hide_widget_card')).toHaveLength(2));
  });

  it('does not render an invalid event payload', async () => {
    const { container } = render(<WidgetCardWindow />);
    await waitFor(() => expect(container.querySelector('[data-widget-card="spotify"]')).toBeTruthy());
    act(() => emit?.({ payload: { id: 'not-a-widget', size: 'wide' } }));
    expect(container.querySelector('[data-widget-card="not-a-widget"]')).toBeNull();
    expect(container.querySelector('[data-widget-card="spotify"]')).toBeTruthy();
  });
});
