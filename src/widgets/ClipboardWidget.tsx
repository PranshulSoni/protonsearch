import type { WidgetViewProps } from './sdk/types';
import { Icon, WidgetState } from './sdk/primitives';

type ClipboardData = { availableCount: number; countIsLowerBound: boolean; items: { preview: string; source: string; pinned: boolean }[] };
export function clipboardData(value: unknown): value is ClipboardData {
  if (!value || typeof value !== 'object') return false;
  const data = value as ClipboardData;
  return Number.isInteger(data.availableCount) && data.availableCount >= 0 && data.availableCount <= 50 &&
    data.countIsLowerBound === (data.availableCount === 50) && Array.isArray(data.items) && data.items.length <= 3 &&
    data.items.length <= data.availableCount && data.items.every((item) => item &&
      typeof item.preview === 'string' && [...item.preview].length <= 120 &&
      typeof item.source === 'string' && [...item.source].length <= 64 && typeof item.pinned === 'boolean');
}
export function ClipboardWidget({ snapshot, settings, size, onAction, onSettingsChange }: WidgetViewProps) {
  const data = clipboardData(snapshot.data) ? snapshot.data : null;
  const showPreviews = settings.showPreviews === true;
  const status = snapshot.status === 'live' && !data ? 'error' : snapshot.status;
  const count = data ? `${data.availableCount}${data.countIsLowerBound ? '+' : ''}` : '—';
  const summary = size === 'compact' ? `${count} clips` : `${count} saved ${data?.availableCount === 1 ? 'clip' : 'clips'}`;
  return <div className="clipboard-widget">
    <button className="clipboard-open fresh-action" aria-label="Browse saved clipboard history" onClick={() => onAction('search', 'clip:')}>
      <Icon name="copy" className="fresh-icon" /><span><span className="fresh-kicker">CLIPBOARD</span><b>{summary}</b>
        {size !== 'compact' && !showPreviews && <small>Previews hidden</small>}
      </span>
    </button>
    {size !== 'compact' && <button className="clipboard-privacy fresh-control" aria-label={showPreviews ? 'Hide clipboard previews' : 'Show clipboard previews'}
      title={showPreviews ? 'Hide clipboard previews' : 'Show clipboard previews'} disabled={!onSettingsChange}
      onClick={() => onSettingsChange?.({ ...settings, showPreviews: !showPreviews })}><Icon name={showPreviews ? 'user' : 'more'} /></button>}
    {showPreviews && size !== 'compact' && data?.items.length ? <div className="clipboard-previews">
      {data.items.slice(0, size === 'wide' ? 2 : 1).map((item, index) => <div key={index} className="clipboard-preview-row">
        <span>{item.pinned ? 'Pinned · ' : ''}{item.preview || 'Empty text'}</span>{size === 'wide' && <small>{item.source}</small>}
      </div>)}
    </div> : null}
    {status !== 'live' && <div className="clipboard-provider-state"><WidgetState status={status} message={snapshot.message} /></div>}
  </div>;
}
