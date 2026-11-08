// @vitest-environment jsdom
import { useState } from 'react';
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { FocusTimer, focusSettings, remainingFocus } from './FocusTimer';
afterEach(() => { cleanup(); vi.useRealTimers(); });

test('focus timer pauses, restores persisted time, resumes and completes after sleep', () => {
  vi.useFakeTimers(); vi.setSystemTime(new Date('2026-11-05T00:00:00Z'));
  let persisted: Record<string, unknown> = { durationMinutes: 1 };
  function Host() {
    const [settings, setSettings] = useState(persisted);
    return <FocusTimer snapshot={{ status: 'loading', updatedAt: 0 }} settings={settings} size="wide" onAction={() => {}}
      onSettingsChange={(next) => { persisted = next; setSettings(next); }} />;
  }
  let view = render(<Host />);
  fireEvent.click(view.getByLabelText('Start focus timer'));
  act(() => vi.advanceTimersByTime(12000));
  expect(view.getByText('00:48')).toBeTruthy();
  fireEvent.click(view.getByLabelText('Pause focus timer'));
  expect(persisted.endsAt).toBeNull();
  view.unmount();
  act(() => vi.advanceTimersByTime(30000));
  view = render(<Host />);
  expect(view.getByText('00:48')).toBeTruthy();
  fireEvent.click(view.getByLabelText('Start focus timer'));
  view.unmount();
  act(() => vi.advanceTimersByTime(60000));
  view = render(<Host />);
  expect(view.getByText('Session complete')).toBeTruthy();
  expect(persisted.endsAt).toBeNull();
  expect(persisted.remainingMs).toBe(0);
  fireEvent.click(view.getByLabelText('Reset focus timer'));
  expect(view.getByText('01:00')).toBeTruthy();
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});

test('invalid persisted timer data falls back without losing unrelated settings', () => {
  const state = focusSettings({ durationMinutes: -2, endsAt: Infinity, remainingMs: NaN, custom: true });
  expect(state).toEqual({ durationMinutes: 25, remainingMs: 1500000, endsAt: null, custom: true });
  expect(remainingFocus({ ...state, endsAt: 100 }, 200)).toBe(0);
});
