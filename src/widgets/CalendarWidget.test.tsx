// @vitest-environment jsdom
import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { CalendarSettings, CalendarWidget, calendarSource } from './CalendarWidget';
const { invoke, updated } = vi.hoisted(() => ({ invoke: vi.fn(), updated: { callback: null as (() => void) | null } }));
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('../services/native', () => ({ isNative: () => true }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockImplementation((_name, callback) => { updated.callback = callback; return Promise.resolve(() => {}); }) }));
afterEach(() => { cleanup(); vi.useRealTimers(); invoke.mockReset(); });
const token = '0123456789abcdef0123456789abcdef';
const settings = { calendarToken: token, calendarProfile: 'daily', unrelated: true };
const props = { snapshot: { status: 'disconnected' as const, updatedAt: 0 }, size: 'standard' as const, onAction: () => {} };
const live = { status: 'live', updatedAt: 1, data: { sourceName: 'Fixture.ics', events: [{ title: 'Fixture meeting', location: 'Room 2', startMs: 4100000000000, endMs: 4100003600000, allDay: false }, { title: 'Second fixture', location: '', startMs: 4100007200000, endMs: 4100010800000, allDay: true }] } };
test('unlinked calendar never invokes native reads', () => {
 const view = render(<CalendarWidget {...props} settings={{}} />);
 expect(view.getByText('No calendar linked')).toBeTruthy(); expect(invoke).not.toHaveBeenCalled();
 expect(calendarSource({ calendarToken: 'path.ics', calendarProfile: 'daily' })).toBeNull();
});
test.each(['compact', 'standard', 'wide'] as const)('renders actual fixture event at %s width', async size => {
 invoke.mockResolvedValue(live); const view = render(<CalendarWidget {...props} size={size} settings={settings} />);
 await act(async () => {}); expect(view.getByText('Fixture meeting')).toBeTruthy();
 expect(invoke).toHaveBeenCalledWith('calendar_snapshot', { profile: 'daily', token });
 expect(!!view.queryByText(/Second fixture/)).toBe(size === 'wide');
});
test.each(['empty', 'permission-denied', 'error'] as const)('never retains old events on %s', async status => {
 invoke.mockResolvedValue(live); const view = render(<CalendarWidget {...props} settings={settings} />); await act(async () => {});
 invoke.mockResolvedValue({ status, updatedAt: 2, message: 'Unsupported calendar fixture' });
 await act(async () => { updated.callback?.(); }); expect(view.queryByText('Fixture meeting')).toBeNull(); expect(view.getAllByRole('status').length).toBeGreaterThan(0);
});
test('ignores old source completion after profile changes and clears hidden polling', async () => {
 vi.useFakeTimers(); let resolve: (value: unknown) => void = () => {};
 invoke.mockReturnValue(new Promise(done => { resolve = done; }));
 const view = render(<CalendarWidget {...props} settings={settings} />);
 view.rerender(<CalendarWidget {...props} settings={{}} />);
 await act(async () => { resolve(live); }); expect(view.queryByText('Fixture meeting')).toBeNull();
 expect(vi.getTimerCount()).toBe(0);
});
test('picker cancellation preserves settings and selection preserves unrelated settings', async () => {
 const save = vi.fn(); invoke.mockResolvedValue(null);
 const view = render(<CalendarSettings profile="daily" settings={{ unrelated: true }} onChange={save} />);
 await act(async () => { fireEvent.click(view.getByText('Choose local calendar')); }); expect(save).not.toHaveBeenCalled();
 invoke.mockResolvedValue({ profile: 'daily', token, name: 'Fixture.ics' });
 await act(async () => { fireEvent.click(view.getByText('Choose local calendar')); });
 expect(save).toHaveBeenCalledWith({ ...settings, calendarName: 'Fixture.ics' });
});
test('picker completion after editor unmount cannot save into another profile', async () => {
 const save = vi.fn(); let resolve: (value: unknown) => void = () => {};
 invoke.mockReturnValue(new Promise(done => { resolve = done; }));
 const view = render(<CalendarSettings profile="daily" settings={{}} onChange={save} />);
 fireEvent.click(view.getByText('Choose local calendar')); view.unmount();
 await act(async () => { resolve({ profile: 'daily', token, name: 'Fixture.ics' }); }); expect(save).not.toHaveBeenCalled();
});

test('configured profile switches ignore earlier pending events', async () => {
 let daily: (value: unknown) => void = () => {}, founder: (value: unknown) => void = () => {};
 invoke.mockReturnValueOnce(new Promise(done => { daily = done; })).mockReturnValueOnce(new Promise(done => { founder = done; }));
 const view = render(<CalendarWidget {...props} settings={settings} />);
 view.rerender(<CalendarWidget {...props} settings={{ calendarToken: 'ffffffffffffffffffffffffffffffff', calendarProfile: 'founder' }} />);
 await act(async () => { daily(live); }); expect(view.queryByText('Fixture meeting')).toBeNull();
 await act(async () => { founder({ ...live, data: { ...live.data, events: [{ ...live.data.events[0], title: 'Founder fixture' }] } }); });
 expect(view.getByText('Founder fixture')).toBeTruthy(); expect(view.queryByText('Fixture meeting')).toBeNull();
});
test('settings updates during a pending read queue a fresh persisted read', async () => {
 let finish: (value: unknown) => void = () => {};
 invoke.mockReturnValueOnce(new Promise(done => { finish = done; })).mockResolvedValue({ status: 'empty', updatedAt: 2 });
 const view = render(<CalendarWidget {...props} settings={settings} />);
 await act(async () => { updated.callback?.(); }); expect(invoke).toHaveBeenCalledTimes(1);
 await act(async () => { finish(live); }); expect(invoke).toHaveBeenCalledTimes(2);
 expect(view.queryByText('Fixture meeting')).toBeNull(); expect(view.getByText('No upcoming events')).toBeTruthy();
});
