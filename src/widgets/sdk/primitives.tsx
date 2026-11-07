import type { ButtonHTMLAttributes } from 'react';
import type { Children, WidgetStatus } from './types';
import { Icon } from '../../design/Icon';
export { Icon };
export function WidgetFrame({ children }: Children) { return <div className="fresh-widget">{children}</div>; }
export function WidgetHeader({ children }: Children) { return <span className="fresh-kicker">{children}</span>; }
export function Metric({ label, value }: { label: string; value: string }) { return <div><b>{value}</b><span>{label}</span></div>; }
export function ActionButton(props: ButtonHTMLAttributes<HTMLButtonElement>) { return <button type="button" className="fresh-control" {...props} />; }
const labels: Record<WidgetStatus, string> = { loading: 'Loading', live: 'Available', empty: 'Nothing here yet', stale: 'Stale data', disconnected: 'Disconnected', 'permission-denied': 'Permission required', error: 'Unavailable' };
export function WidgetState({ status, message }: { status: WidgetStatus; message?: string }) { return <small role="status">{message || labels[status]}</small>; }
export function MiniChart({ values }: { values: number[] }) {
  const points = values.map((value, index) => `${index * 100 / Math.max(1, values.length - 1)},${30 - Math.min(100, Math.max(0, value)) * .3}`).join(' ');
  return <svg viewBox="0 0 100 30" role="img" aria-label="Recent activity"><polyline points={points} fill="none" stroke="currentColor" /></svg>;
}
