import reference from '../design/reference-widgets.json';
import { catalog, runWidgetAction } from './catalog';
import { useProvider } from './sdk/provider';
import { WidgetState } from './sdk/primitives';
import type { WidgetDefinition, WidgetInstance } from './sdk/types';
import { widgetSize } from './sdk/sizing';
export type { WidgetDefinition, WidgetInstance, WidgetSize } from './sdk/types';
const modules = import.meta.glob<{ default: WidgetDefinition }>('./builtins/*/manifest.ts', { eager: true });
export const widgetDefinitions = [...catalog, ...Object.values(modules).map(module => module.default)];
if (new Set(widgetDefinitions.map(widget => widget.id)).size !== widgetDefinitions.length) throw new Error('Duplicate widget IDs');
export function getWidget(id: string) { return widgetDefinitions.find(widget => widget.id === id); }
export function WidgetRenderer({ instance, onSearch, onSettingsChange }: { instance: WidgetInstance; onSearch: (query: string) => void; onSettingsChange?: (settings: Record<string, unknown>) => void }) {
  const definition = getWidget(instance.id);
  const snapshot = useProvider(definition?.providerId ?? 'none', definition?.refresh.intervalMs);
  if (!definition) return <WidgetState status="error" message="Widget unavailable" />;
  const Component = definition.Component;
  const className = (reference as Record<string, { attributes: Record<string, string> }>)[instance.id]?.attributes.class ?? 'tile widget fresh-widget';
  const size = widgetSize(definition, instance.size);
  return <div className={className} data-widget-id={instance.id} data-widget-size={size} style={{ width: '100%', height: 66 }}><Component settings={definition.validateSettings(instance.settings) ? instance.settings! : definition.defaults} snapshot={snapshot} size={size} onSettingsChange={onSettingsChange} onAction={(action, payload) => { void runWidgetAction(action, payload, onSearch).catch(console.error); }} /></div>;
}
