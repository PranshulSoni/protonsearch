import type { WidgetDefinition } from '../../sdk/types';
import { Widget } from './Widget';
import { Preview } from './Preview';
const definition: WidgetDefinition = { id: 'storage-health', version: 1, title: 'Storage Health', description: 'Free space on the Windows system drive', category: 'Utilities', icon: 'folder', sizes: ['compact','standard','wide'], sizeWidths: { compact: 140, standard: 180, wide: 260 }, defaultSize: 'standard', defaultWidth: 180, minWidth: 88, maxWidth: 390, profiles: ['daily','founder'], permissions: ['storage:read'], providerId: 'storage', refresh: { mode: 'poll', intervalMs: 30000 }, defaults: {}, validateSettings: (value): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value), actions: [], Component: Widget, Preview };
export default definition;
