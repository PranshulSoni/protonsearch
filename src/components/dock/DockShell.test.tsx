// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DockShell } from './DockShell';
import { defaultLayout } from './layout';

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke, isTauri: () => true }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

function snapshot(data: unknown = {}) {
  return { status: 'live', data, updatedAt: Date.now() };
}

function mockCommands(saved: unknown) {
  invoke.mockImplementation((command: string) => {
    if (command === 'load_dock_settings') return Promise.resolve(saved);
    if (command === 'provider_snapshot') return Promise.resolve(snapshot());
    if (command === 'known_folders_snapshot') return Promise.resolve(snapshot({ folders: [], recent: [] }));
    if (command === 'running_apps_snapshot') return Promise.resolve(snapshot({ items: [] }));
    return Promise.resolve();
  });
}

function nativeWindow() {
  Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {}, configurable: true });
}

beforeEach(() => {
  nativeWindow();
});

afterEach(() => {
  cleanup();
  invoke.mockReset();
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  vi.unstubAllGlobals();
});

describe('DockShell integration', () => {
  it('switches profiles and persists the selection', async () => {
    mockCommands(defaultLayout());
    render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    fireEvent.click(screen.getByRole('tab', { name: 'Work' }));
    expect(screen.getByRole('tab', { name: 'Work' }).getAttribute('aria-selected')).toBe('true');
    await waitFor(() => {
      const saved = invoke.mock.calls.filter(([command]) => command === 'save_dock_settings').at(-1)?.[1]?.value;
      expect(saved).toMatchObject({ version: 2, profile: 'work' });
    }, { timeout: 1500 });
  });

  it('preserves malformed saved settings and never overwrites them', async () => {
    mockCommands({ version: 1, profile: 'daily', profiles: { daily: {}, founder: [] } });
    render(<DockShell />);
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toMatch(/without overwriting/);
    await new Promise((resolve) => setTimeout(resolve, 450));
    expect(invoke.mock.calls.filter(([command]) => command === 'save_dock_settings')).toHaveLength(0);
  });

  it('opens the widget card for a dock slot with its size, settings and anchor', async () => {
    mockCommands(defaultLayout());
    const { container } = render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    const slot = container.querySelector<HTMLElement>('[data-widget-id="clock"]')!;
    slot.getBoundingClientRect = () => ({ x: 120, y: 40, width: 84, height: 66, left: 120, top: 40, right: 204, bottom: 106, toJSON() {} });
    fireEvent.click(slot);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('open_widget_card', {
      id: 'clock',
      size: 'compact',
      settings: expect.objectContaining({ hour12: false }),
      anchor: { x: 120, y: 40, width: 84, height: 66 },
    }));
  });

  it('starts Windows from the Start tile', async () => {
    mockCommands(defaultLayout());
    const { container } = render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    fireEvent.click(container.querySelector<HTMLElement>('[data-widget-id="windows"]')!);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('open_start'));
    expect(invoke.mock.calls.map(([command]) => command)).not.toContain('open_widget_card');
  });

  it('opens Spotlight from the Search tile', async () => {
    mockCommands(defaultLayout());
    const { container } = render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    fireEvent.click(container.querySelector<HTMLElement>('[data-widget-id="search"]')!);
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('open_search', { query: '' }));
    expect(invoke.mock.calls.map(([command]) => command)).not.toContain('open_widget_card');
  });

  it('reports only the profile shelf and rail as native hit regions', async () => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => setTimeout(() => callback(0), 0));
    vi.stubGlobal('cancelAnimationFrame', clearTimeout);
    mockCommands(defaultLayout());
    const { container } = render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    const rail = container.querySelector<HTMLElement>('.dock-final-rail')!;
    const pills = container.querySelector<HTMLElement>('.dock-final-shelf')!;
    rail.getBoundingClientRect = () => ({ x: 100, y: 50, width: 800, height: 82, left: 100, top: 50, right: 900, bottom: 132, toJSON() {} });
    pills.getBoundingClientRect = () => ({ x: 300, y: 10, width: 360, height: 28, left: 300, top: 10, right: 660, bottom: 38, toJSON() {} });
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('fit_dock', {
      width: 832,
      height: 132,
      regions: [
        { x: 216, y: 10, width: 360, height: 28, radius: 15 },
        { x: 16, y: 50, width: 800, height: 82, radius: 18 },
      ],
    }));
  });

  it('requests enough width when the rail content is clipped', async () => {
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => setTimeout(() => callback(0), 0));
    vi.stubGlobal('cancelAnimationFrame', clearTimeout);
    mockCommands(defaultLayout());
    const { container } = render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    const rail = container.querySelector<HTMLElement>('.dock-final-rail')!;
    const pills = container.querySelector<HTMLElement>('.dock-final-shelf')!;
    rail.getBoundingClientRect = () => ({ x: 12, y: 50, width: 800, height: 82, left: 12, top: 50, right: 812, bottom: 132, toJSON() {} });
    Object.defineProperty(rail, 'scrollWidth', { value: 1100, configurable: true });
    pills.getBoundingClientRect = () => ({ x: 300, y: 10, width: 360, height: 28, left: 300, top: 10, right: 660, bottom: 38, toJSON() {} });
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('fit_dock', {
      width: 1132,
      height: 132,
      regions: [
        { x: 304, y: 10, width: 360, height: 28, radius: 15 },
        { x: 16, y: 50, width: 1100, height: 82, radius: 18 },
      ],
    }));
  });

  it('opens the native settings window from Customize Widgets', async () => {
    mockCommands(defaultLayout());
    render(<DockShell />);
    await screen.findByRole('tab', { name: 'Daily' });
    fireEvent.click(screen.getByRole('button', { name: 'Customize Widgets' }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('open_settings'));
  });

  it('renders the widget manager in settings-only mode and closes natively', async () => {
    mockCommands(defaultLayout());
    render(<DockShell settingsOnly />);
    expect(await screen.findByRole('heading', { name: 'Dock & Widgets' })).toBeTruthy();
    expect(screen.getByRole('tab', { name: /Installed/ })).toBeTruthy();
    expect(screen.getByRole('tab', { name: /Widget Store/ })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Close Dock and Widgets' }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith('hide_settings'));
  });
});
