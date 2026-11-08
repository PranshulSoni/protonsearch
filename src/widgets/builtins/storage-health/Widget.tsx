import type { WidgetViewProps } from '../../sdk/types';
import { WidgetHeader, WidgetState, Icon } from '../../sdk/primitives';
import type { Data } from './types';
export function Widget({ snapshot, size }: WidgetViewProps) {
 const data = snapshot.data as Partial<Data> | undefined;
 if (data && (typeof data.totalBytes !== 'number' || typeof data.freeBytes !== 'number' || typeof data.mountPoint !== 'string')) return <WidgetState status="error" message="Invalid storage snapshot" />;
 return <div className="fresh-action"><Icon name="folder" className="fresh-icon" /><span><WidgetHeader>STORAGE HEALTH</WidgetHeader>{data ? <><b>{Math.round(data.freeBytes! / 1073741824)} GB free</b>{size !== 'compact' && <small>{data.mountPoint} · {Math.round(data.totalBytes! / 1073741824)} GB total</small>}{size === 'wide' && <small>{data.totalBytes! > 0 ? Math.round((1 - data.freeBytes! / data.totalBytes!) * 100) : 0}% used</small>}{snapshot.status === 'stale' && <WidgetState status="stale" />}</> : <WidgetState status={snapshot.status} />}</span></div>;
}
