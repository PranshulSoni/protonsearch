// @vitest-environment jsdom
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { WeatherWidget, WeatherSettings } from './WeatherWidget';
import { weatherLocation } from '../services/weather';
const { invoke, updated } = vi.hoisted(() => ({ invoke: vi.fn(), updated: { callback: null as (() => void) | null } }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('../services/native', () => ({ isNative: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockImplementation((_name, callback) => { updated.callback = callback; return Promise.resolve(() => {}); }) }));
afterEach(() => { cleanup(); vi.useRealTimers(); invoke.mockReset(); updated.callback = null; });
const configured = { weatherEnabled: true, weatherLatitude: 0, weatherLongitude: 0, weatherLabel: 'Test fixture' };
const live = { status: 'live', updatedAt: 1000, location: { latitude: 0, longitude: 0 }, data: { temperature: 21.5, weatherCode: 2, time: 1000, utcOffsetSeconds: 0, hours: [{ time: 4600, temperature: 23, weatherCode: 0 }] } };
const props = { snapshot: { status: 'disconnected' as const, updatedAt: 0 }, size: 'standard' as const, onAction: () => {} };

test('weather never calls a provider before a saved explicit location opt-in', () => {
  const view = render(<WeatherWidget {...props} settings={{}} />);
  expect(view.getByText('Weather not configured')).toBeTruthy();
  expect(invoke).not.toHaveBeenCalled();
  expect(weatherLocation({ ...configured, weatherLatitude: 91 })).toBeNull();
  expect(weatherLocation(configured)).toEqual({ latitude: 0, longitude: 0 });
});

test('weather renders real current values and adds hourly content only when wide', async () => {
  invoke.mockResolvedValue(live);
  const view = render(<WeatherWidget {...props} settings={configured} />);
  await act(async () => {});
  expect(view.getByText('22°')).toBeTruthy();
  expect(view.getByText('Partly cloudy')).toBeTruthy();
  expect(view.queryByText('23°')).toBeNull();
  view.rerender(<WeatherWidget {...props} size="wide" settings={configured} />);
  expect(view.getByText('23°')).toBeTruthy();
});

test('settings editor saves only explicit valid configuration and preserves other fields', () => {
  const save = vi.fn();
  const view = render(<WeatherSettings settings={{ unrelated: true }} onChange={save} />);
  fireEvent.click(view.getByLabelText(/Enable weather/));
  fireEvent.change(view.getByLabelText('Latitude'), { target: { value: '91' } });
  fireEvent.submit(view.getByRole('form', { name: 'Weather location settings' }));
  expect(save).not.toHaveBeenCalled();
  expect(view.getByRole('alert')).toBeTruthy();
  fireEvent.change(view.getByLabelText('Latitude'), { target: { value: '0' } });
  fireEvent.change(view.getByLabelText('Longitude'), { target: { value: '0' } });
  fireEvent.submit(view.getByRole('form', { name: 'Weather location settings' }));
  expect(save).toHaveBeenCalledWith({ ...configured, weatherLabel: '', unrelated: true });
});

test('settings-updated refreshes native persisted configuration and stale values remain labeled', async () => {
  invoke.mockResolvedValue(live);
  const view = render(<WeatherWidget {...props} settings={configured} />);
  await act(async () => {});
  invoke.mockResolvedValue({ ...live, status: 'stale', message: 'Weather service unavailable' });
  await act(async () => { updated.callback?.(); });
  expect(view.getByText('Stale data')).toBeTruthy();
  expect(view.getByText('22°')).toBeTruthy();
});

test('weather refresh pauses while hidden and cleans timers on unmount', async () => {
  vi.useFakeTimers();
  invoke.mockResolvedValue(live);
  const view = render(<WeatherWidget {...props} settings={configured} />);
  await act(async () => {});
  Object.defineProperty(document, 'hidden', { configurable: true, value: true });
  await act(async () => { vi.advanceTimersByTime(600000); });
  expect(invoke).toHaveBeenCalledTimes(1);
  Object.defineProperty(document, 'hidden', { configurable: true, value: false });
  await act(async () => { document.dispatchEvent(new Event('visibilitychange')); });
  expect(invoke).toHaveBeenCalledTimes(2);
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});

test('unacknowledged persisted coordinates cannot appear under the newly chosen location', async () => {
  invoke.mockResolvedValue({ ...live, location: { latitude: 50, longitude: 20 } });
  const view = render(<WeatherWidget {...props} settings={configured} />);
  await act(async () => {});
  expect(view.queryByText('22°')).toBeNull();
  expect(view.getByText('Loading')).toBeTruthy();
  invoke.mockResolvedValue(live);
  await act(async () => { updated.callback?.(); });
  expect(view.getByText('22°')).toBeTruthy();
});
