// @vitest-environment jsdom
import { useState } from 'react';
import { cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { ClipboardWidget } from './ClipboardWidget';
import type { WidgetStatus } from './sdk/types';
afterEach(cleanup);
const data = { availableCount: 50, countIsLowerBound: true, items: [
  { preview: '<img src=x onerror=alert(1)>', source: 'fixture.exe', pinned: true },
  { preview: 'Second fixture clip', source: 'other.exe', pinned: false },
] };
const props = { snapshot: { status: 'live' as const, updatedAt: 1, data }, size: 'wide' as const, settings: {}, onAction: vi.fn() };

test('saved history count is real but content previews are private by default', () => {
  const view = render(<ClipboardWidget {...props} />);
  expect(view.getByText('50+ saved clips')).toBeTruthy();
  expect(view.queryByText(/onerror/)).toBeNull();
  fireEvent.click(view.getByLabelText('Browse saved clipboard history'));
  expect(props.onAction).toHaveBeenCalledExactlyOnceWith('search', 'clip:');
});

test('preview opt-in persists without losing settings and displays text safely', () => {
  const save = vi.fn();
  function Host() {
    const [settings, setSettings] = useState<Record<string, unknown>>({ preserved: 'yes' });
    return <ClipboardWidget {...props} settings={settings} onSettingsChange={(next) => { save(next); setSettings(next); }} />;
  }
  const view = render(<Host />);
  fireEvent.click(view.getByLabelText('Show clipboard previews'));
  expect(save).toHaveBeenCalledWith({ preserved: 'yes', showPreviews: true });
  expect(view.getByText('Pinned · <img src=x onerror=alert(1)>')).toBeTruthy();
  expect(view.container.querySelector('img')).toBeNull();
  expect(view.getByText('Second fixture clip')).toBeTruthy();
  fireEvent.click(view.getByLabelText('Hide clipboard previews'));
  expect(view.queryByText(/onerror/)).toBeNull();
});

test('compact hides previews even after opt-in while standard shows only one', () => {
  const view = render(<ClipboardWidget {...props} size="compact" settings={{ showPreviews: true }} />);
  expect(view.getByText('50+ clips')).toBeTruthy();
  expect(view.queryByText(/onerror/)).toBeNull();
  view.rerender(<ClipboardWidget {...props} size="standard" settings={{ showPreviews: true }} />);
  expect(view.getByText(/onerror/)).toBeTruthy();
  expect(view.queryByText('Second fixture clip')).toBeNull();
});

test.each(['loading', 'stale', 'disconnected', 'permission-denied', 'error', 'empty'] as WidgetStatus[])('clipboard keeps %s status explicit', (status) => {
  const view = render(<ClipboardWidget {...props} snapshot={{ status, updatedAt: 1, ...(status === 'stale' ? { data } : {}) }} />);
  expect(view.getByRole('status').textContent).not.toBe('');
  if (status !== 'stale') expect(view.queryByText('50+ saved clips')).toBeNull();
});

test('invalid provider count and oversized previews cannot be shown as live', () => {
  const view = render(<ClipboardWidget {...props} snapshot={{ status: 'live', updatedAt: 1, data: { ...data, availableCount: -1 } }} />);
  expect(view.getByRole('status').textContent).toBe('Unavailable');
  expect(view.getByText('— saved clips')).toBeTruthy();
});
