import { useEffect, useState } from 'react';
import reference from '../design/reference-widgets.json';
import { Icon, WidgetState } from './sdk/primitives';
import { native } from '../services/native';
import { FocusTimer } from './FocusTimer';
import { WeatherWidget } from './WeatherWidget';
import { CalendarWidget } from './CalendarWidget';
import { SystemControls } from './SystemControls';
import { ClipboardWidget } from './ClipboardWidget';
import { RunningApps } from './RunningApps';
import { NetworkWidget, MediaWidget } from './LocalProviders';
import { Widget as StorageWidget } from './builtins/storage-health/Widget';
import './providers.css';
import type { WidgetDefinition, WidgetViewProps, WidgetSize } from './sdk/types';
function Clock({ analog = false, size = 'standard' }: { analog?: boolean; size?: WidgetSize }) {
  const [now, setNow] = useState(new Date());
  useEffect(() => { const timer = setInterval(() => setNow(new Date()), 1000); return () => clearInterval(timer); }, []);
  if (analog) return <div className="analog"><span className="analog-inner-ring" /><span style={{ position: 'absolute', left: '50%', bottom: '50%', width: 2, height: 15, background: 'currentColor', transformOrigin: 'bottom', transform: `rotate(${now.getHours() * 30 + now.getMinutes() / 2}deg)` }} /><span style={{ position: 'absolute', left: '50%', bottom: '50%', width: 1, height: 21, background: 'currentColor', transformOrigin: 'bottom', transform: `rotate(${now.getMinutes() * 6}deg)` }} /></div>;
  return <div><div className="time-main">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', ...(size === 'wide' ? { second: '2-digit' } : {}), hour12: false })}</div><div className="time-date"><Icon name="calendar" className="dock-glyph time-calendar" /><span className="date-text">{now.toLocaleDateString('en-US', size === 'compact' ? { weekday: 'short' } : { weekday: 'short', month: 'short', day: 'numeric' })}</span></div>{size === 'wide' && <small>{Intl.DateTimeFormat().resolvedOptions().timeZone}</small>}</div>;
}
function Content({ id, title, snapshot, size, settings, onSettingsChange, onAction }: WidgetViewProps & { id: string; title: string }) {
  if (id === 'apps') return <RunningApps snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  if (id === 'clipboard') return <ClipboardWidget snapshot={snapshot} settings={settings} size={size} onAction={onAction} onSettingsChange={onSettingsChange} />;
  if (id === 'controls') return <SystemControls snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  if (id === 'focus') return <FocusTimer snapshot={snapshot} settings={settings} size={size} onSettingsChange={onSettingsChange} onAction={onAction} />;
  if (id === 'network') return <NetworkWidget snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  if (id === 'media') return <MediaWidget snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  if (id === 'disk') return <StorageWidget snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  if (id === 'energy') { const data = snapshot.data as { battery?: number | null } | undefined; return <div className="network-widget"><Icon name="settings" className="fresh-icon" /><span><span className="fresh-kicker">BATTERY</span><b>{typeof data?.battery === 'number' ? `${Math.round(data.battery)}%` : 'No battery sensor'}</b>{snapshot.status !== 'live' && <WidgetState status={snapshot.status} />}</span></div>; }
  if (id === 'time' || id === 'analog') return <Clock analog={id === 'analog'} size={size} />;
  if (id === 'windows') return <button className="ps-dock-action ps-windows-action" aria-label="Open installed Windows apps" title="Installed apps · Windows Start fallback" onClick={() => onAction('start')}><span className="ps-windows-logo"><Icon name="windows" /></span><span className="ps-windows-copy">Apps</span></button>;
  if (id === 'protonsearch') return <button className="ps-dock-action ps-search-action" aria-label="Open ProtonSearch" onClick={() => onAction('search', '')}><span className="ps-dock-logo"><Icon name="search" /></span><span className="ps-dock-copy"><b>ProtonSearch</b><small>Alt + Space · All sources</small></span><span className="ps-dock-shortcut"><Icon name="search" /></span></button>;
  const query = ({ git: 'commits:', notes: 'notes:', downloads: 'file:Downloads', launcher: '', apps: '', activity: 'memory:' } as Record<string, string>)[id];
  if (query !== undefined) return <button className="fresh-action" onClick={() => onAction('search', query)} aria-label={`Search ${title}`}><Icon name={id === 'git' ? 'git' : 'folder'} className="fresh-icon" /><span><span className="fresh-kicker">{id === 'git' ? 'DEVELOPER' : title.toUpperCase()}</span><b>{title}</b><small>{id === 'git' ? 'Search commits & TODOs' : 'Search local history'}</small></span><Icon name="arrow-right" className="fresh-icon fresh-inline-arrow" /></button>;
  if (id === 'status') {
    const data = snapshot.data as { cpu?: number; ram?: number; battery?: number } | undefined;
    return <div className="fresh-status"><div className="fresh-status-label"><Icon name="settings" className="fresh-icon" /><span>SYSTEM</span>{snapshot.status !== 'live' && <WidgetState status={snapshot.status} />}</div><div className="fresh-stats">{(size === 'compact' ? ['cpu'] as const : size === 'standard' ? ['cpu', 'ram'] as const : ['cpu', 'ram', 'battery'] as const).map(key => <div key={key}><b>{data?.[key] === undefined || data[key] === null ? '—' : Math.round(data[key]!)}<em>%</em></b><span>{key === 'battery' ? 'BAT' : key.toUpperCase()}</span><i style={{ width: `${data?.[key] ?? 0}%` }} /></div>)}</div></div>;
  }
  if (id === 'weather') return <WeatherWidget snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  if (id === 'calendar') return <CalendarWidget snapshot={snapshot} settings={settings} size={size} onAction={onAction} />;
  return <div className="fresh-action"><Icon name={id === 'ai' || id === 'chatgpt' ? 'sparkles' : id === 'gmail' ? 'mail' : 'settings'} className="fresh-icon" /><span><span className="fresh-kicker">{title.toUpperCase()}</span><b>{title}</b><WidgetState status={snapshot.status === 'loading' ? 'disconnected' : snapshot.status} /></span></div>;
}
export const catalog: WidgetDefinition[] = Object.entries(reference).map(([id, template]) => {
  const attrs = template.attributes;
  const title = attrs['data-widget-name'];
  const Component = (props: WidgetViewProps) => <Content {...props} id={id} title={title} />;
  const standard = id === 'apps' ? 220 : id === 'network' ? 210 : id === 'disk' ? 180 : Number(attrs['data-default-w']);
  const widths: Record<string, [number, number]> = { apps: [130, 500], time: [94, 200], status: [110, 250], weather: [110, 340], media: [96, 300], calendar: [60, 260], protonsearch: [64, 280], focus: [120, 310], network: [140, 300], disk: [140, 260], energy: [100, 220], controls: [100, 300], clipboard: [110, 330] };
  const variants = widths[id];
  return { id, version: 1, title, category: ['revenue', 'sales', 'followers', 'visitors'].includes(id) ? 'Founder' : 'Utilities', description: title, icon: 'settings', sizes: id === 'protonsearch' ? ['compact', 'standard'] : variants ? ['compact', 'standard', 'wide'] : ['standard'], sizeWidths: variants ? { compact: variants[0], standard, wide: variants[1] } : { standard }, defaultSize: 'standard', defaultWidth: standard, minWidth: variants ? Math.min(variants[0], Number(attrs['data-min-w'])) : Number(attrs['data-min-w']), maxWidth: Math.max(variants?.[1] ?? standard, Number(attrs['data-max-w'])), profiles: ['daily', 'founder'], permissions: id === 'apps' ? ['running-apps:read'] : id === 'calendar' ? ['calendar:read'] : id === 'weather' ? ['weather:read'] : id === 'status' || id === 'energy' ? ['system:read'] : id === 'network' ? ['network:read'] : id === 'media' ? ['media:read', 'media:control'] : id === 'disk' ? ['storage:read'] : id === 'controls' ? ['settings:open'] : id === 'clipboard' ? ['clipboard:read'] : [], providerId: id === 'apps' ? 'running-apps' : id === 'status' || id === 'energy' ? 'system' : id === 'network' ? 'network' : id === 'media' ? 'media' : id === 'disk' ? 'storage' : id === 'clipboard' ? 'clipboard' : 'none', refresh: { mode: 'poll', intervalMs: id === 'apps' ? 5000 : id === 'network' ? 2000 : id === 'media' ? 5000 : id === 'clipboard' ? 30000 : 10000 }, defaults: {}, validateSettings: (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value), actions: id === 'controls' ? ['settings:sound', 'settings:network', 'settings:bluetooth', 'settings:notifications'] : id === 'media' ? ['media:toggle', 'media:next', 'media:previous'] : id === 'windows' ? ['start'] : ['clipboard', 'git', 'notes', 'downloads', 'launcher', 'apps', 'activity', 'protonsearch'].includes(id) ? ['search'] : [], Component, Preview: () => <Component snapshot={{ status: 'disconnected', updatedAt: 0 }} settings={{}} size="standard" onAction={() => {}} /> };
});
export async function runWidgetAction(action: string, payload: unknown, onSearch: (query: string) => void) { if (action === 'search') onSearch(typeof payload === 'string' ? payload : ''); else if (action === 'start') await native.openStart(); }


