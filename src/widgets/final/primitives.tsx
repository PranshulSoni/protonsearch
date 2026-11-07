import type { CSSProperties, ReactNode } from 'react';
import type { WidgetStatus } from '../sdk/types';
import { Icon, AppIcon, appGlyphFor, type AppGlyphName } from './icons';
import { getWidgetMeta } from './meta';
import type { FinalWidgetId } from './types';

export function statusLabel(status: WidgetStatus | undefined, preview: boolean): string {
  if (preview) return 'SAMPLE';
  if (!status) return 'LOCAL';
  const labels: Record<WidgetStatus, string> = {
    live: 'LIVE',
    loading: 'SYNCING',
    empty: 'EMPTY',
    stale: 'STALE',
    disconnected: 'OFFLINE',
    'permission-denied': 'LOCKED',
    error: 'ERROR',
  };
  return labels[status];
}

export function WidgetHeader({
  id,
  title,
  status,
  preview,
  aside,
}: {
  id: FinalWidgetId;
  title?: string;
  status?: WidgetStatus;
  preview: boolean;
  aside?: ReactNode;
}) {
  const meta = getWidgetMeta(id);
  return (
    <header className="fw-card-head">
      <span className="fw-card-icon" aria-hidden="true">
        <Icon name={meta.icon} size={16} />
      </span>
      <div className="fw-card-heading">
        <h3 className="fw-card-title">{title ?? meta.title}</h3>
      </div>
      {aside ? <div className="fw-card-head-aside">{aside}</div> : null}
      <span className={`fw-card-status${preview ? ' is-sample' : ''}`}>{statusLabel(status, preview)}</span>
    </header>
  );
}

export function CardFooter({ left, right }: { left: ReactNode; right?: ReactNode }) {
  return (
    <footer className="fw-card-foot">
      <span className="fw-card-foot-left">{left}</span>
      {right ? <span className="fw-card-foot-right">{right}</span> : null}
    </footer>
  );
}

export function MetricBar({
  value,
  warnAt = 100,
  label,
  tone = 'tint',
}: {
  value: number | null | undefined;
  warnAt?: number;
  label: string;
  tone?: 'tint' | 'blue' | 'green' | 'amber' | 'violet' | 'red';
}) {
  const safe = value == null || !Number.isFinite(value) ? null : Math.min(100, Math.max(0, value));
  const state = safe == null ? 'unknown' : safe >= 95 ? 'critical' : safe >= warnAt ? 'warn' : 'normal';
  return (
    <span
      className={`fw-bar fw-bar--${tone} is-${state}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      {...(safe == null ? {} : { 'aria-valuenow': Math.round(safe) })}
    >
      <span className="fw-bar-fill" style={{ width: `${safe ?? 0}%` }} />
    </span>
  );
}

export function EmptyState({
  title,
  detail,
  action,
}: {
  title: string;
  detail: string;
  action?: ReactNode;
}) {
  return (
    <div className="fw-empty" role="status">
      <span className="fw-empty-mark" aria-hidden="true">
        <Icon name="plus" size={13} />
      </span>
      <div>
        <strong>{title}</strong>
        <p>{detail}</p>
      </div>
      {action ? <div className="fw-empty-action">{action}</div> : null}
    </div>
  );
}

export function StatusDot({ state = 'idle' }: { state?: 'running' | 'idle' | 'offline' }) {
  return <span className={`fw-dot fw-dot--${state}`} aria-hidden="true" />;
}

export function AppGlyph({ title, size = 18 }: { title: string; size?: number }) {
  return <AppIcon kind={appGlyphFor(title)} size={size} />;
}

export function AlbumArt({ title, size = 96 }: { title?: string | null; size?: number }) {
  const style: CSSProperties = { width: size, height: size };
  return (
    <span className="fw-album" style={style} aria-label={title ? `Album art placeholder for ${title}` : 'No album art'}>
      <span className="fw-album-rings" aria-hidden="true" />
      <Icon name="music" size={Math.round(size * 0.3)} />
    </span>
  );
}

export function AppIconStack({ titles, limit = 4 }: { titles: string[]; limit?: number }) {
  return (
    <span className="fw-app-stack" aria-label={`${titles.length} running apps`}>
      {titles.slice(0, limit).map((title) => (
        <span className="fw-app-stack-item" key={title} title={title}>
          <AppGlyph title={title} size={15} />
        </span>
      ))}
    </span>
  );
}

export function Chip({ children, tone }: { children: ReactNode; tone?: 'accent' | 'quiet' }) {
  return <span className={`fw-chip${tone ? ` fw-chip--${tone}` : ''}`}>{children}</span>;
}

export function NoValue({ label = 'Not available' }: { label?: string }) {
  return <span className="fw-no-value">{label}</span>;
}

export type { AppGlyphName };
