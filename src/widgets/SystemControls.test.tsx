// @vitest-environment jsdom
import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { SystemControls } from './SystemControls';
const { widgetAction, isNative } = vi.hoisted(() => ({ widgetAction: vi.fn(), isNative: vi.fn(() => true) }));
vi.mock('../services/native', () => ({ native: { widgetAction }, isNative }));
const props = { settings: {}, snapshot: { status: 'loading' as const, updatedAt: 0 }, onAction: () => {} };
afterEach(() => { cleanup(); widgetAction.mockReset(); isNative.mockReturnValue(true); });

test.each([['compact', 2], ['standard', 3], ['wide', 4]] as const)('%s exposes designed Settings actions without pretend toggles', (size, count) => {
  const view = render(<SystemControls {...props} size={size} />);
  const buttons = view.getAllByRole('button');
  expect(buttons).toHaveLength(count);
  expect(buttons.every((button) => !button.hasAttribute('aria-pressed'))).toBe(true);
  expect(view.queryByText('Disconnected')).toBeNull();
});

test('browser preview disables native actions and explains availability', () => {
  isNative.mockReturnValue(false);
  const view = render(<SystemControls {...props} size="wide" />);
  for (const button of view.getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  expect(view.getByRole('status').textContent).toBe('Windows app required');
});

test('Settings action disables repeat clicks while pending and surfaces native failure', async () => {
  let fail: (reason: Error) => void = () => {};
  widgetAction.mockImplementation(() => new Promise<void>((_, reject) => { fail = reject; }));
  const view = render(<SystemControls {...props} size="wide" />);
  fireEvent.click(view.getByLabelText('Open sound settings'));
  expect(widgetAction).toHaveBeenCalledExactlyOnceWith('settings:sound');
  for (const button of view.getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
  fail(new Error('Windows could not open Settings'));
  await waitFor(() => expect(view.getByRole('alert').textContent).toContain('Windows could not open Settings'));
  expect((view.getByLabelText('Open sound settings') as HTMLButtonElement).disabled).toBe(false);
});

test.each([['network', 'settings:network'], ['bluetooth', 'settings:bluetooth'], ['notifications', 'settings:notifications']])('opens fixed %s Settings action', async (label, action) => {
  widgetAction.mockResolvedValue(undefined);
  const view = render(<SystemControls {...props} size="wide" />);
  fireEvent.click(view.getByLabelText(`Open ${label} settings`));
  await waitFor(() => expect(widgetAction).toHaveBeenCalledExactlyOnceWith(action));
});
