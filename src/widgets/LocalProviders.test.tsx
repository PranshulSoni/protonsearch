// @vitest-environment jsdom
import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { MediaWidget, NetworkWidget } from './LocalProviders';
const { widgetAction } = vi.hoisted(() => ({ widgetAction: vi.fn() }));
vi.mock('../services/native', () => ({ native: { widgetAction } }));
vi.mock('./sdk/provider', () => ({ refreshProvider: vi.fn() }));
afterEach(() => { cleanup(); widgetAction.mockReset(); });
const props = { settings: {}, size: 'wide' as const, onAction: () => {} };
const media = { title: 'Real session title', artist: 'Session artist', source: 'player.exe', playing: true, canToggle: true, canNext: false, canPrevious: true };

test('media displays native metadata and sends only the selected enabled action', async () => {
  widgetAction.mockRejectedValue(new Error('Session rejected action'));
  const view = render(<MediaWidget {...props} snapshot={{ status: 'live', data: media, updatedAt: 1 }} />);
  expect(view.getByText('Real session title')).toBeTruthy();
  expect((view.getByLabelText('Next track') as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(view.getByLabelText('Pause media'));
  expect(widgetAction).toHaveBeenCalledWith('media:toggle');
  await waitFor(() => expect(view.getByRole('alert').textContent).toContain('Session rejected action'));
});

test.each(['empty', 'disconnected', 'stale'] as const)('media disables controls for %s snapshots', (status) => {
  const view = render(<MediaWidget {...props} snapshot={{ status, data: status === 'stale' ? media : undefined, updatedAt: 1 }} />);
  for (const button of view.getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
});

test('network waits for the second sample and renders actual byte rates with adapter identity', () => {
  const view = render(<NetworkWidget {...props} snapshot={{ status: 'loading', updatedAt: 1, data: { downloadBytesPerSecond: null, uploadBytesPerSecond: null, interfaces: ['Wi-Fi'] } }} />);
  expect(view.container.textContent).toContain('↓ —');
  expect(view.container.textContent).toContain('Loading');
  view.rerender(<NetworkWidget {...props} snapshot={{ status: 'live', updatedAt: 2, data: { downloadBytesPerSecond: 1048576, uploadBytesPerSecond: 2048, interfaces: ['Wi-Fi'] } }} />);
  expect(view.container.textContent).toContain('1.0 MB/s');
  expect(view.container.textContent).toContain('2 KB/s');
  expect(view.getByText('Wi-Fi')).toBeTruthy();
});

test('network rejects malformed native rates instead of presenting live values', () => {
  const view = render(<NetworkWidget {...props} snapshot={{ status: 'live', updatedAt: 1, data: { downloadBytesPerSecond: -1, uploadBytesPerSecond: 0, interfaces: [] } }} />);
  expect(view.getByRole('status').textContent).toBe('Unavailable');
});
