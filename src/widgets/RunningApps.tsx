import { useState } from 'react';
import { native } from '../services/native';
import { WidgetState } from './sdk/primitives';
import { refreshProvider } from './sdk/provider';
import type { WidgetViewProps } from './sdk/types';

type RunningApp = { id: string; title: string; icon: string | null; active: boolean };
function runningApps(value: unknown): RunningApp[] | null {
  if (!value || typeof value !== 'object') return null;
  const items = (value as { items?: unknown }).items;
  if (!Array.isArray(items) || items.length > 12) return null;
  return new Set(items.map((item) => item?.id)).size === items.length && items.every((item) => item && typeof item.id === 'string' && item.id.length > 0 && item.id.length <= 128 &&
    typeof item.title === 'string' && item.title.length > 0 && item.title.length <= 512 && typeof item.active === 'boolean' &&
    (item.icon === null || (typeof item.icon === 'string' && item.icon.length <= 65536 && /^data:image\/png;base64,[A-Za-z0-9+/]+={0,2}$/.test(item.icon))))
    ? items as RunningApp[] : null;
}

export function RunningApps({ snapshot }: WidgetViewProps) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const items = runningApps(snapshot.data);
  const status = snapshot.status === 'live' && !items ? 'error' : snapshot.status;
  async function activate(id: string) {
    if (pending || status !== 'live') return;
    setPending(true);
    setError('');
    try { await native.activateRunningApp(id); refreshProvider('running-apps'); }
    catch (reason) { setError(String(reason)); refreshProvider('running-apps'); }
    finally { setPending(false); }
  }
  return <div className="running-apps">
    <span className="fresh-kicker">RUNNING APPS</span>
    {items?.length ? <div className="running-apps-list" role="group" aria-label="Running applications"
      onKeyDown={(event) => {
        if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
        const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = buttons[index + (event.key === 'ArrowRight' ? 1 : -1)];
        if (index >= 0 && next) { event.preventDefault(); next.focus(); }
      }}>
      {items.map((app) => <button key={app.id} className="running-app" aria-label={`Switch to ${app.title}`}
        title={app.title} aria-current={app.active ? 'true' : undefined} disabled={pending || status !== 'live'}
        onClick={() => void activate(app.id)}>
        {app.icon ? <img src={app.icon} alt="" /> : <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><rect x="3" y="4" width="18" height="16" rx="3" /><path d="M3 9h18" /></svg>}
        <span className="running-app-indicator" aria-hidden="true" />
      </button>)}
    </div> : <WidgetState status={status === 'live' ? 'empty' : status} message={snapshot.message} />}
    {items?.length && status !== 'live' ? <WidgetState status={status} message={snapshot.message} /> : null}
    {error && <small role="alert" className="widget-action-error">{error}</small>}
  </div>;
}
