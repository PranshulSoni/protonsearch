import { useState } from 'react';
import type { WidgetViewProps } from './sdk/types';
import { Icon, WidgetState } from './sdk/primitives';
import { native } from '../services/native';
import { refreshProvider } from './sdk/provider';

export function formatRate(value: number | null) {
  if (value === null) return '—';
  if (value >= 1048576) return `${(value / 1048576).toFixed(1)} MB/s`;
  return `${Math.round(value / 1024)} KB/s`;
}
export function NetworkWidget({ snapshot, size }: WidgetViewProps) {
  const data = snapshot.data as { downloadBytesPerSecond?: number | null; uploadBytesPerSecond?: number | null; interfaces?: string[] } | undefined;
  const valid = data && [data.downloadBytesPerSecond, data.uploadBytesPerSecond].every((v) => v === null || (typeof v === 'number' && Number.isFinite(v) && v >= 0)) && Array.isArray(data.interfaces) && data.interfaces.every((name) => typeof name === 'string');
  if (!valid) return <WidgetState status={snapshot.status === 'live' ? 'error' : snapshot.status} message={snapshot.message} />;
  return <div className="network-widget"><Icon name="wifi" className="fresh-icon" /><span className="network-values"><span className="fresh-kicker">NETWORK</span><b>↓ {formatRate(data.downloadBytesPerSecond!)}{size !== 'compact' && <> · ↑ {formatRate(data.uploadBytesPerSecond!)}</>}</b>
    {size === 'wide' && <small>{data.interfaces!.join(', ') || 'No active adapter'}</small>}
    {snapshot.status !== 'live' && <WidgetState status={snapshot.status} message={snapshot.message} />}
  </span></div>;
}
type MediaData = { title: string; artist: string; source: string; playing: boolean; canToggle: boolean; canNext: boolean; canPrevious: boolean };
function mediaData(value: unknown): value is MediaData {
  if (!value || typeof value !== 'object') return false;
  const data = value as Record<string, unknown>;
  return ['title', 'artist', 'source'].every((key) => typeof data[key] === 'string') && ['playing', 'canToggle', 'canNext', 'canPrevious'].every((key) => typeof data[key] === 'boolean');
}
export function MediaWidget({ snapshot, size }: WidgetViewProps) {
  const [pending, setPending] = useState(false), [error, setError] = useState('');
  const data = mediaData(snapshot.data) ? snapshot.data : null;
  async function act(action: 'media:toggle' | 'media:next' | 'media:previous') {
    setPending(true); setError('');
    try { await native.widgetAction(action); refreshProvider('media'); }
    catch (reason) { setError(String(reason)); }
    finally { setPending(false); }
  }
  return <div className="fresh-media">
    {size === 'compact' && <small className="compact-provider-state" role="status">{snapshot.status === 'live' && data ? data.playing ? 'Playing' : 'Paused' : snapshot.status === 'empty' ? 'No session' : snapshot.status === 'stale' ? 'Stale' : 'Offline'}</small>}
    <div className="fresh-media-art"><Icon name="launcher-play" /></div>
    <div className="fresh-media-info"><span className="fresh-kicker">{data?.source || 'MEDIA SESSION'}</span><b>{data?.title || (data ? 'Untitled media' : 'No media session')}</b><small>{data?.artist || snapshot.message || 'Start playback in a media app'}</small>
      {(snapshot.status !== 'live' || !data) && <WidgetState status={!data && snapshot.status === 'live' ? 'error' : snapshot.status} />}
    </div>
    {size === 'wide' && <button className="fresh-control" aria-label="Previous track" disabled={pending || snapshot.status !== 'live' || !data?.canPrevious} onClick={() => void act('media:previous')}>‹</button>}
    <button className="fresh-control" aria-label={data?.playing ? 'Pause media' : 'Play media'} disabled={pending || snapshot.status !== 'live' || !data?.canToggle} onClick={() => void act('media:toggle')}><Icon name={data?.playing ? 'pause' : 'play'} /></button>
    {size === 'wide' && <button className="fresh-control" aria-label="Next track" disabled={pending || snapshot.status !== 'live' || !data?.canNext} onClick={() => void act('media:next')}>›</button>}
    {error && <small className="widget-action-error" role="alert">{error}</small>}
  </div>;
}
