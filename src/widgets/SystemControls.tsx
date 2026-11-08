import { useState } from 'react';
import { isNative, native, type WidgetActionId } from '../services/native';
import { Icon } from './sdk/primitives';
import type { WidgetViewProps } from './sdk/types';

const controls: { id: WidgetActionId; label: string; icon: string }[] = [
  { id: 'settings:sound', label: 'Sound', icon: 'sound' },
  { id: 'settings:network', label: 'Network', icon: 'wifi' },
  { id: 'settings:bluetooth', label: 'Bluetooth', icon: 'bluetooth' },
  { id: 'settings:notifications', label: 'Notifications', icon: 'launcher-message' },
];
function ControlIcon({ name }: { name: string }) {
  if (name !== 'sound' && name !== 'bluetooth') return <Icon name={name} />;
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d={name === 'sound' ? 'M4 10h4l5-4v12l-5-4H4zM17 9a5 5 0 0 1 0 6m2-9a9 9 0 0 1 0 12' : 'M12 2v20l7-6-14-9m0 10 14-10-7-5'} />
  </svg>;
}
export function SystemControls({ size }: WidgetViewProps) {
  const [pending, setPending] = useState(false), [error, setError] = useState('');
  const available = isNative();
  async function open(action: WidgetActionId) {
    setPending(true); setError('');
    try { await native.widgetAction(action); }
    catch (reason) { setError(String(reason)); }
    finally { setPending(false); }
  }
  return <div className={`system-controls system-controls-${size}`}>
    {size !== 'compact' && <span className="fresh-kicker">WINDOWS SETTINGS</span>}
    <div className="system-controls-row" role="group" aria-label="Open Windows settings" aria-busy={pending}>
      {controls.slice(0, size === 'compact' ? 2 : size === 'standard' ? 3 : 4).map((control) => (
        <button key={control.id} className="fresh-control" aria-label={`Open ${control.label.toLowerCase()} settings`}
          title={available ? `Open ${control.label.toLowerCase()} settings` : 'Available in the Windows app'}
          disabled={!available || pending} onClick={() => void open(control.id)}>
          <ControlIcon name={control.icon} />{size === 'wide' && <span>{control.label}</span>}
        </button>
      ))}
    </div>
    {!available && <small className="system-controls-status" role="status">Windows app required</small>}
    {error && <small className="widget-action-error" role="alert">{error}</small>}
  </div>;
}
