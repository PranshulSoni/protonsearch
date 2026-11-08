import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { isNative } from '../services/native';
import type { Snapshot, WidgetViewProps } from './sdk/types';

type Event = { title: string; location: string; startMs: number; endMs: number; allDay: boolean };
type CalendarData = { sourceName: string; events: Event[] };
export function calendarSource(settings: Record<string, unknown>) {
  return typeof settings.calendarToken === 'string' && /^[0-9a-f]{32}$/i.test(settings.calendarToken)
    && (settings.calendarProfile === 'daily' || settings.calendarProfile === 'founder')
    ? { token: settings.calendarToken, profile: settings.calendarProfile } : null;
}
function useCalendar(settings: Record<string, unknown>): Snapshot<CalendarData> {
  const source = calendarSource(settings);
  const [snapshot, setSnapshot] = useState<Snapshot<CalendarData>>({ status: 'disconnected', updatedAt: 0 });
  const identity = `${source?.profile}:${source?.token}`;
  const [receivedFor, setReceivedFor] = useState('');
  useEffect(() => {
    if (!source || !isNative()) return;
    let current = true, pending = false, queued = false;
    const refresh = () => {
      if (!current || document.hidden) return;
      if (pending) { queued = true; return; }
      pending = true;
      void invoke<Snapshot<CalendarData>>('calendar_snapshot', source)
        .then((value) => { if (current) { setSnapshot(value); setReceivedFor(identity); } })
        .catch((error) => { if (current) { setSnapshot({ status: 'error', updatedAt: 0, message: String(error) }); setReceivedFor(identity); } })
        .finally(() => { pending = false; if (queued && current) { queued = false; refresh(); } });
    };
    refresh();
    const timer = setInterval(refresh, 30000);
    const listener = listen('settings-updated', refresh).catch(() => () => {});
    document.addEventListener('visibilitychange', refresh);
    return () => { current = false; clearInterval(timer); document.removeEventListener('visibilitychange', refresh); void listener.then((remove) => remove()); };
  }, [identity]);
  return !source || !isNative() ? { status: 'disconnected', updatedAt: 0 } : receivedFor === identity ? snapshot : { status: 'loading', updatedAt: 0 };
}
export function CalendarWidget({ settings, size }: WidgetViewProps) {
  const snapshot = useCalendar(settings);
  const event = snapshot.data?.events[0];
  const now = new Date();
  const state = snapshot.status === 'loading' ? 'Loading calendar' : snapshot.status === 'empty' ? 'No upcoming events'
    : snapshot.status === 'permission-denied' ? 'Select calendar again' : snapshot.status === 'error' ? 'Calendar unavailable' : 'No calendar linked';
  const date = event ? new Date(event.startMs) : now;
  const time = event?.allDay ? 'All day' : event ? date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : 'Next events appear here';
  return <div className="fresh-calendar" title={snapshot.message || snapshot.data?.sourceName}>
    {size === 'compact' && <small className="compact-provider-state" role="status">{event ? time : calendarSource(settings) ? state : 'Unlinked'}</small>}
    <div className="fresh-calendar-date"><span>{date.toLocaleDateString('en-US', { month: 'short' }).toUpperCase()}</span><b>{date.getDate()}</b></div>
    <div className="fresh-calendar-copy"><span className="fresh-kicker">YOUR DAY</span><strong>{event?.title || state}</strong><small role={!event ? 'status' : undefined}>{event ? `${time}${event.location ? ` · ${event.location}` : ''}` : state === 'Calendar unavailable' ? snapshot.message : time}</small>
      {size === 'wide' && snapshot.data?.events.slice(1).map((next, index) => <small className="calendar-secondary" key={index}>{new Date(next.startMs).toLocaleDateString([], { month: 'short', day: 'numeric' })} · {next.title}</small>)}
    </div>
  </div>;
}
export function CalendarSettings({ settings, profile, onChange }: { settings: Record<string, unknown>; profile: 'daily' | 'founder'; onChange: (settings: Record<string, unknown>) => void }) {
  const [pending, setPending] = useState(false), [error, setError] = useState('');
  const active = useRef(true);
  useEffect(() => { active.current = true; return () => { active.current = false; }; }, []);
  const source = calendarSource(settings);
  const change = async (remove: boolean) => {
    if (!isNative() || pending) return;
    setPending(true); setError('');
    try {
      if (remove && source) {
        await invoke('calendar_forget_source', source);
        if (active.current) { const next = { ...settings }; delete next.calendarToken; delete next.calendarProfile; delete next.calendarName; onChange(next); }
      } else {
        const selected = await invoke<{ token: string; name: string; profile: string } | null>('calendar_select_source', { profile });
        if (selected && selected.profile === profile && active.current) onChange({ ...settings, calendarToken: selected.token, calendarProfile: profile, calendarName: selected.name });
      }
    } catch (value) { if (active.current) setError(String(value)); }
    finally { if (active.current) setPending(false); }
  };
  return <div className="weather-settings" aria-label="Local calendar settings"><small>Choose a local UTF-8 .ics file (up to 1 MiB). One-off events only; UTC, local times or all-day dates. Recurrence and other timezones are unsupported. No account or network access.</small>
    {source && <small>{typeof settings.calendarName === 'string' ? settings.calendarName : 'Local calendar selected'}</small>}
    <button disabled={pending || !isNative()} onClick={() => void change(false)}>{pending ? 'Please wait…' : 'Choose local calendar'}</button>
    {source && <button disabled={pending} onClick={() => void change(true)}>Unlink calendar</button>}
    {error && <small role="alert">{error}</small>}
  </div>;
}

