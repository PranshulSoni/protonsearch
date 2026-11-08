import { useEffect, useState } from 'react';
import type { WidgetViewProps } from './sdk/types';
import { Icon } from './sdk/primitives';

export interface FocusSettings extends Record<string, unknown> {
  durationMinutes: number;
  remainingMs: number;
  endsAt: number | null;
}
export function focusSettings(value: Record<string, unknown>): FocusSettings {
  const durationMinutes = typeof value.durationMinutes === 'number' && Number.isInteger(value.durationMinutes) && value.durationMinutes >= 1 && value.durationMinutes <= 180 ? value.durationMinutes : 25;
  const maximum = durationMinutes * 60000;
  return {
    ...value, durationMinutes,
    remainingMs: typeof value.remainingMs === 'number' && Number.isFinite(value.remainingMs) && value.remainingMs >= 0 && value.remainingMs <= maximum ? value.remainingMs : maximum,
    endsAt: typeof value.endsAt === 'number' && Number.isFinite(value.endsAt) && value.endsAt > 0 ? value.endsAt : null,
  };
}
export function remainingFocus(state: FocusSettings, now: number) {
  return state.endsAt === null ? state.remainingMs : Math.max(0, Math.min(state.durationMinutes * 60000, state.endsAt - now));
}
export function FocusTimer({ settings, size, onSettingsChange }: WidgetViewProps) {
  const state = focusSettings(settings);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (state.endsAt === null) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [state.endsAt]);
  const remaining = remainingFocus(state, now);
  useEffect(() => {
    if (state.endsAt !== null && remaining === 0) onSettingsChange?.({ ...state, endsAt: null, remainingMs: 0 });
  }, [state.endsAt, remaining, onSettingsChange]);
  const seconds = Math.ceil(remaining / 1000);
  const running = state.endsAt !== null && remaining > 0;
  return <div className="focus-widget">
    <div className="focus-time"><span className="fresh-kicker">FOCUS</span><b>{String(Math.floor(seconds / 60)).padStart(2, '0')}:{String(seconds % 60).padStart(2, '0')}</b>
      {remaining === 0 && <small role="status">Session complete</small>}
      {size === 'wide' && <progress aria-label="Focus session progress" max={state.durationMinutes * 60000} value={state.durationMinutes * 60000 - remaining} />}
    </div>
    <button className="fresh-control" disabled={!onSettingsChange} aria-label={running ? 'Pause focus timer' : 'Start focus timer'} onClick={() => {
      const left = remainingFocus(state, Date.now()) || state.durationMinutes * 60000;
      onSettingsChange?.({ ...state, remainingMs: left, endsAt: running ? null : Date.now() + left });
    }}><Icon name={running ? 'pause' : 'play'} /></button>
    {size !== 'compact' && <button className="fresh-control" disabled={!onSettingsChange} aria-label="Reset focus timer" onClick={() => onSettingsChange?.({ ...state, endsAt: null, remainingMs: state.durationMinutes * 60000 })}>↺</button>}
    {size === 'wide' && <select aria-label="Focus duration" disabled={running || !onSettingsChange} value={state.durationMinutes} onChange={(e) => onSettingsChange?.({ ...state, durationMinutes: Number(e.target.value), endsAt: null, remainingMs: Number(e.target.value) * 60000 })}>
      {[...new Set([15, 25, 45, 60, state.durationMinutes])].sort((a, b) => a - b).map((minutes) => <option key={minutes} value={minutes}>{minutes} min</option>)}
    </select>}
  </div>;
}
