// @vitest-environment jsdom
import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { RunningApps } from './RunningApps';
import { catalog } from './catalog';
import { widgetWidth } from './sdk/sizing';

const { activate } = vi.hoisted(() => ({ activate: vi.fn() }));
vi.mock('../services/native', () => ({ native: { activateRunningApp: activate } }));
vi.mock('./sdk/provider', () => ({ refreshProvider: vi.fn() }));
afterEach(() => { cleanup(); activate.mockReset(); });
const props = { settings: {}, size: 'standard' as const, onAction: () => {} };
const app = { id: 'opaque-1', title: 'Editor', icon: 'data:image/png;base64,YQ==', active: true };

test('renders a supplied native icon and activates its opaque ID', async () => {
  activate.mockResolvedValue(undefined);
  const view = render(<RunningApps {...props} snapshot={{ status: 'live', updatedAt: 1, data: { items: [app] } }} />);
  expect(view.container.querySelector('img')?.getAttribute('src')).toBe(app.icon);
  const button = view.getByRole('button', { name: 'Switch to Editor' });
  expect(button.getAttribute('aria-current')).toBe('true');
  fireEvent.click(button);
  await waitFor(() => expect(activate).toHaveBeenCalledWith('opaque-1'));
});

test('stale windows cannot be activated and unavailable icons use a fallback', () => {
  const view = render(<RunningApps {...props} snapshot={{ status: 'stale', updatedAt: 1, data: { items: [{ ...app, icon: null }] } }} />);
  expect((view.getByRole('button', { name: 'Switch to Editor' }) as HTMLButtonElement).disabled).toBe(true);
  expect(view.queryByRole('img')).toBeNull();
  expect(view.getByText('Stale data')).toBeTruthy();
});

test('rejects unsafe icon sources and reports activation errors', async () => {
  activate.mockRejectedValue(new Error('Window closed'));
  const view = render(<RunningApps {...props} snapshot={{ status: 'live', updatedAt: 1, data: { items: [{ ...app, icon: 'https://example.com/icon.png' }] } }} />);
  expect(view.getByText('Unavailable')).toBeTruthy();
  view.rerender(<RunningApps {...props} snapshot={{ status: 'live', updatedAt: 1, data: { items: [app] } }} />);
  fireEvent.click(view.getByRole('button', { name: 'Switch to Editor' }));
  await waitFor(() => expect(view.getByRole('alert').textContent).toContain('Window closed'));
});

test('rejects duplicate IDs and unbounded titles', () => {
  const view = render(<RunningApps {...props} snapshot={{ status: 'live', updatedAt: 1, data: { items: [app, app] } }} />);
  expect(view.getByText('Unavailable')).toBeTruthy();
  view.rerender(<RunningApps {...props} snapshot={{ status: 'live', updatedAt: 1, data: { items: [{ ...app, title: 'x'.repeat(513) }] } }} />);
  expect(view.getByText('Unavailable')).toBeTruthy();
});

test('offers designed sizes, keeps saved widths, and lets keyboard focus reach every app', () => {
  const definition = catalog.find((item) => item.id === 'apps')!;
  expect(definition.sizes).toEqual(['compact', 'standard', 'wide']);
  expect(['compact', 'standard', 'wide'].map((size) => widgetWidth(definition, { id: 'apps', size: size as 'compact' | 'standard' | 'wide' }))).toEqual([130, 220, 500]);
  expect(widgetWidth(definition, { id: 'apps', width: 300 })).toBe(300);
  const items = Array.from({ length: 12 }, (_, i) => ({ ...app, id: `id-${i}`, title: `App ${i}` }));
  const view = render(<RunningApps {...props} snapshot={{ status: 'live', updatedAt: 1, data: { items } }} />);
  const first = view.getByRole('button', { name: 'Switch to App 0' });
  first.focus();
  fireEvent.keyDown(first, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(view.getByRole('button', { name: 'Switch to App 1' }));
  const last = view.getByRole('button', { name: 'Switch to App 11' });
  last.focus();
  fireEvent.keyDown(last, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(view.getByRole('button', { name: 'Switch to App 10' }));
});
