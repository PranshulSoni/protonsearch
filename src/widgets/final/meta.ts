import type { ProviderId, WidgetSize } from '../sdk/types';
import type {
  FinalWidgetId,
  FinalWidgetMeta,
  FinalWidgetSettings,
  FinalWidgetSettingsFor,
  FinalWidgetSettingsMap,
} from './types';

export const FINAL_WIDGET_IDS: readonly FinalWidgetId[] = [
  'windows',
  'search',
  'apps',
  'spotify',
  'weather',
  'system',
  'claude',
  'chatgpt',
  'agent',
  'files',
  'folders',
  'trash',
  'volume',
  'clock',
];

export const DEFAULT_DOCK_ORDER: readonly FinalWidgetId[] = [
  'windows',
  'search',
  'apps',
  'spotify',
  'weather',
  'system',
  'claude',
  'agent',
  'clock',
];

export const STORE_WIDGET_IDS: readonly FinalWidgetId[] = ['chatgpt', 'files', 'folders', 'trash', 'volume'];

export const SIZE_LABELS: Record<WidgetSize, 'Pill' | 'Tile' | 'Card'> = {
  compact: 'Pill',
  standard: 'Tile',
  wide: 'Card',
};

const SIZES: readonly WidgetSize[] = ['compact', 'standard', 'wide'];

export type FinalSettingField =
  | { key: string; label: string; description: string; type: 'toggle' }
  | {
      key: string;
      label: string;
      description: string;
      type: 'range';
      min: number;
      max: number;
      step: number;
      suffix: string;
    }
  | {
      key: string;
      label: string;
      description: string;
      type: 'select';
      options: readonly { value: string; label: string }[];
    }
  | { key: string; label: string; description: string; type: 'text'; placeholder: string; maxLength: number };

export const FINAL_SETTING_FIELDS: Record<FinalWidgetId, readonly FinalSettingField[]> = {
  windows: [
    { key: 'showLabel', label: 'Show Start label', description: 'Shows the word Start beside the Windows logo.', type: 'toggle' },
    { key: 'showRecentHint', label: 'Recent app hints', description: 'Shows pinned and recently used apps.', type: 'toggle' },
  ],
  search: [
    { key: 'showHint', label: 'Search scope hint', description: 'Explains what ProtonSearch searches locally.', type: 'toggle' },
    { key: 'showShortcut', label: 'Show Alt + Space', description: 'Shows the launcher shortcut in the widget.', type: 'toggle' },
  ],
  apps: [
    { key: 'maxApps', label: 'App icons', description: 'How many app icons the card can show.', type: 'range', min: 3, max: 8, step: 1, suffix: '' },
    { key: 'showTitles', label: 'App names', description: 'Shows app names under each icon.', type: 'toggle' },
    { key: 'activeOnly', label: 'Active app only', description: 'Collapses the group to the current window.', type: 'toggle' },
  ],
  spotify: [
    { key: 'showProgress', label: 'Progress bar', description: 'Shows elapsed time and track progress.', type: 'toggle' },
    { key: 'showControls', label: 'Transport controls', description: 'Shows previous, play or pause and next buttons.', type: 'toggle' },
    { key: 'showDevice', label: 'Output device', description: 'Shows the Spotify Connect device row.', type: 'toggle' },
    { key: 'showVolume', label: 'Volume slider', description: 'Shows the media volume control.', type: 'toggle' },
    { key: 'compactProgress', label: 'Compact progress', description: 'Uses a slimmer progress line in dense sizes.', type: 'toggle' },
  ],
  weather: [
    { key: 'location', label: 'Location label', description: 'Display name shown in the widget.', type: 'text', placeholder: 'Ahmedabad', maxLength: 40 },
    { key: 'units', label: 'Temperature units', description: 'Celsius or Fahrenheit.', type: 'select', options: [{ value: 'C', label: 'Celsius (C)' }, { value: 'F', label: 'Fahrenheit (F)' }] },
    { key: 'showHourly', label: 'Hourly forecast', description: 'Shows the forecast strip.', type: 'toggle' },
    { key: 'showDetails', label: 'Weather details', description: 'Shows wind, humidity, UV and visibility.', type: 'toggle' },
    { key: 'forecastHours', label: 'Forecast hours', description: 'Number of hourly slots to show.', type: 'range', min: 3, max: 8, step: 1, suffix: 'h' },
  ],
  system: [
    { key: 'showCpu', label: 'CPU load', description: 'Shows processor utilisation.', type: 'toggle' },
    { key: 'showRam', label: 'Memory load', description: 'Shows RAM utilisation.', type: 'toggle' },
    { key: 'showTemp', label: 'Temperature slot', description: 'Shows -- and Not available when unknown.', type: 'toggle' },
    { key: 'showBattery', label: 'Battery', description: 'Shows charge level and charging state.', type: 'toggle' },
    { key: 'showWifi', label: 'Wi-Fi', description: 'Shows SSID and signal strength.', type: 'toggle' },
    { key: 'showBluetooth', label: 'Bluetooth', description: 'Shows radio state and connected device.', type: 'toggle' },
    { key: 'showNetwork', label: 'Ethernet and VPN', description: 'Shows wired and tunnel connection rows.', type: 'toggle' },
    { key: 'showUptime', label: 'Uptime', description: 'Shows system uptime in the footer.', type: 'toggle' },
  ],
  claude: [
    { key: 'plan', label: 'Plan label', description: 'Shown in the card and dock.', type: 'text', placeholder: 'Max 20x', maxLength: 24 },
    { key: 'showSession', label: 'Session window', description: 'Shows the current session usage block.', type: 'toggle' },
    { key: 'showWeekly', label: 'Weekly window', description: 'Shows the seven day usage block.', type: 'toggle' },
    { key: 'showReset', label: 'Reset countdown', description: 'Shows time until the window resets.', type: 'toggle' },
    { key: 'warnAt', label: 'Warn threshold', description: 'Bars turn amber at this percentage.', type: 'range', min: 50, max: 100, step: 5, suffix: '%' },
  ],
  chatgpt: [
    { key: 'plan', label: 'Plan label', description: 'Shown in the card and dock.', type: 'text', placeholder: 'Pro', maxLength: 24 },
    { key: 'showSession', label: '5-hour window', description: 'Shows the current short usage block.', type: 'toggle' },
    { key: 'showWeekly', label: 'Weekly window', description: 'Shows the seven day usage block.', type: 'toggle' },
    { key: 'showReset', label: 'Reset countdown', description: 'Shows time until the window resets.', type: 'toggle' },
    { key: 'warnAt', label: 'Warn threshold', description: 'Bars turn amber at this percentage.', type: 'range', min: 50, max: 100, step: 5, suffix: '%' },
  ],
  agent: [
    { key: 'workspace', label: 'Workspace name', description: 'Folder or project name shown in the card.', type: 'text', placeholder: 'protonsearch', maxLength: 48 },
    { key: 'showTokens', label: 'Token count', description: 'Shows tokens used in this session.', type: 'toggle' },
    { key: 'showFiles', label: 'Files changed', description: 'Shows the number of files touched.', type: 'toggle' },
    { key: 'showSession', label: 'Session duration', description: 'Shows elapsed agent session time.', type: 'toggle' },
    { key: 'showToolCalls', label: 'Tool calls', description: 'Shows the number of tools invoked.', type: 'toggle' },
  ],
  files: [
    { key: 'source', label: 'Source folder', description: 'Folder label used for the recent list.', type: 'text', placeholder: 'Projects', maxLength: 40 },
    { key: 'maxItems', label: 'Files shown', description: 'How many recent files to display.', type: 'range', min: 3, max: 8, step: 1, suffix: '' },
    { key: 'showType', label: 'File type badges', description: 'Shows a language or format badge on each row.', type: 'toggle' },
  ],
  folders: [
    { key: 'maxItems', label: 'Folders shown', description: 'How many known folders to display.', type: 'range', min: 3, max: 8, step: 1, suffix: '' },
    { key: 'showCounts', label: 'Item counts', description: 'Shows the number of items beside each folder.', type: 'toggle' },
  ],
  trash: [
    { key: 'showSize', label: 'Reclaimable size', description: 'Shows the space the Trash is holding.', type: 'toggle' },
    { key: 'showLastEmptied', label: 'Last emptied', description: 'Shows the relative time of the last cleanup.', type: 'toggle' },
  ],
  volume: [
    { key: 'showDevice', label: 'Output device', description: 'Shows the active playback device.', type: 'toggle' },
    { key: 'showSlider', label: 'Level slider', description: 'Shows an interactive volume track.', type: 'toggle' },
    { key: 'showMute', label: 'Mute button', description: 'Shows a quick mute control.', type: 'toggle' },
  ],
  clock: [
    { key: 'hour12', label: '12-hour time', description: 'Uses AM and PM instead of 24-hour time.', type: 'toggle' },
    { key: 'showSeconds', label: 'Seconds', description: 'Adds a live seconds readout and hand.', type: 'toggle' },
    { key: 'showDate', label: 'Date', description: 'Shows weekday, month and day.', type: 'toggle' },
    { key: 'showTimezone', label: 'Timezone', description: 'Shows the local timezone and UTC offset.', type: 'toggle' },
  ],
};

const provider = (ids: ProviderId[]) => ids;

export const FINAL_WIDGET_META: readonly FinalWidgetMeta[] = [
  {
    id: 'windows',
    title: 'Windows Start',
    shortTitle: 'Start',
    description: 'Start menu target with pinned and recent app hints.',
    category: 'System',
    tint: 'blue',
    tintRgb: '116, 183, 255',
    icon: 'windows',
    defaultSize: 'compact',
    sizes: SIZES,
    defaultWidth: 76,
    widths: { compact: 76, standard: 148, wide: 168 },
    cardWidth: 448,
    providers: provider([]),
    permissions: [],
    optional: false,
    settingsDefaults: { showLabel: true, showRecentHint: true },
  },
  {
    id: 'search',
    title: 'ProtonSearch',
    shortTitle: 'Search',
    description: 'One search across apps, files, folders and OCR text.',
    category: 'System',
    tint: 'blue',
    tintRgb: '116, 183, 255',
    icon: 'search',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 168,
    widths: { compact: 136, standard: 168, wide: 212 },
    cardWidth: 448,
    providers: provider([]),
    permissions: ['search:local'],
    optional: false,
    settingsDefaults: { showHint: true, showShortcut: true },
  },
  {
    id: 'apps',
    title: 'Running Apps',
    shortTitle: 'Apps',
    description: 'Active window, grouped app icons and window count.',
    category: 'System',
    tint: 'blue',
    tintRgb: '116, 183, 255',
    icon: 'apps',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 174,
    widths: { compact: 64, standard: 174, wide: 208 },
    cardWidth: 452,
    providers: provider(['running-apps']),
    permissions: ['running-apps:read'],
    optional: false,
    settingsDefaults: { maxApps: 4, showTitles: true, activeOnly: false },
  },
  {
    id: 'spotify',
    title: 'Spotify',
    shortTitle: 'Spotify',
    description: 'Now playing, transport controls, device and volume.',
    category: 'Media',
    tint: 'green',
    tintRgb: '95, 217, 154',
    icon: 'spotify',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 192,
    widths: { compact: 78, standard: 192, wide: 236 },
    cardWidth: 460,
    providers: provider(['media']),
    permissions: ['media:read', 'media:control'],
    optional: false,
    settingsDefaults: { showProgress: true, showControls: true, showDevice: true, showVolume: true, compactProgress: false },
  },
  {
    id: 'weather',
    title: 'Weather',
    shortTitle: 'Weather',
    description: 'Current conditions, details and hourly forecast.',
    category: 'Weather',
    tint: 'amber',
    tintRgb: '240, 189, 106',
    icon: 'weather',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 164,
    widths: { compact: 84, standard: 164, wide: 222 },
    cardWidth: 452,
    providers: provider([]),
    permissions: ['weather:read'],
    optional: false,
    settingsDefaults: { location: 'Ahmedabad', units: 'C', showHourly: true, showDetails: true, forecastHours: 6 },
  },
  {
    id: 'system',
    title: 'System Status',
    shortTitle: 'System',
    description: 'CPU, memory, battery and connectivity in one card.',
    category: 'System',
    tint: 'blue',
    tintRgb: '116, 183, 255',
    icon: 'system',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 192,
    widths: { compact: 126, standard: 192, wide: 224 },
    cardWidth: 452,
    providers: provider(['system', 'network']),
    permissions: ['system:read', 'network:read'],
    optional: false,
    settingsDefaults: {
      showCpu: true,
      showRam: true,
      showTemp: true,
      showBattery: true,
      showWifi: true,
      showBluetooth: true,
      showNetwork: true,
      showUptime: true,
    },
  },
  {
    id: 'claude',
    title: 'Claude Usage',
    shortTitle: 'Claude',
    description: 'Session and weekly limits for Claude Code.',
    category: 'AI',
    tint: 'violet',
    tintRgb: '180, 156, 255',
    icon: 'claude',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 166,
    widths: { compact: 80, standard: 166, wide: 232 },
    cardWidth: 452,
    providers: provider([]),
    permissions: [],
    optional: false,
    settingsDefaults: { plan: 'Max 20x', showSession: true, showWeekly: true, showReset: true, warnAt: 80 },
  },
  {
    id: 'chatgpt',
    title: 'ChatGPT Usage',
    shortTitle: 'ChatGPT',
    description: '5-hour and weekly usage for ChatGPT.',
    category: 'AI',
    tint: 'green',
    tintRgb: '95, 217, 154',
    icon: 'chatgpt',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 166,
    widths: { compact: 80, standard: 166, wide: 232 },
    cardWidth: 452,
    providers: provider([]),
    permissions: [],
    optional: true,
    settingsDefaults: { plan: 'Pro', showSession: true, showWeekly: true, showReset: true, warnAt: 80 },
  },
  {
    id: 'agent',
    title: 'AI Workspace Agent',
    shortTitle: 'Agent',
    description: 'Workspace agent status, task and session metrics.',
    category: 'AI',
    tint: 'violet',
    tintRgb: '180, 156, 255',
    icon: 'agent',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 156,
    widths: { compact: 112, standard: 156, wide: 244 },
    cardWidth: 452,
    providers: provider([]),
    permissions: [],
    optional: false,
    settingsDefaults: {
      workspace: 'protonsearch',
      showTokens: true,
      showFiles: true,
      showSession: true,
      showToolCalls: true,
    },
  },
  {
    id: 'files',
    title: 'Recent Files',
    shortTitle: 'Recent',
    description: 'Recently modified files with type, time and size.',
    category: 'Files',
    tint: 'teal',
    tintRgb: '111, 214, 207',
    icon: 'files',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 190,
    widths: { compact: 80, standard: 190, wide: 240 },
    cardWidth: 452,
    providers: provider(['storage']),
    permissions: ['storage:read'],
    optional: true,
    settingsDefaults: { source: 'Projects', maxItems: 4, showType: true },
  },
  {
    id: 'folders',
    title: 'Quick Folders',
    shortTitle: 'Folders',
    description: 'Desktop, Documents, Downloads and more.',
    category: 'Files',
    tint: 'amber',
    tintRgb: '240, 189, 106',
    icon: 'folders',
    defaultSize: 'standard',
    sizes: SIZES,
    defaultWidth: 180,
    widths: { compact: 80, standard: 180, wide: 230 },
    cardWidth: 452,
    providers: provider(['storage']),
    permissions: ['storage:read'],
    optional: true,
    settingsDefaults: { maxItems: 6, showCounts: true },
  },
  {
    id: 'trash',
    title: 'Trash',
    shortTitle: 'Trash',
    description: 'Recycle Bin count, reclaimable size and empty action.',
    category: 'Files',
    tint: 'red',
    tintRgb: '240, 139, 139',
    icon: 'trash',
    defaultSize: 'compact',
    sizes: SIZES,
    defaultWidth: 78,
    widths: { compact: 78, standard: 160, wide: 210 },
    cardWidth: 440,
    providers: provider(['storage']),
    permissions: ['storage:read', 'storage:write'],
    optional: true,
    settingsDefaults: { showSize: true, showLastEmptied: true },
  },
  {
    id: 'volume',
    title: 'Volume',
    shortTitle: 'Volume',
    description: 'Output device, level, mute and current media app.',
    category: 'Media',
    tint: 'silver',
    tintRgb: '195, 201, 209',
    icon: 'volume',
    defaultSize: 'compact',
    sizes: SIZES,
    defaultWidth: 96,
    widths: { compact: 96, standard: 176, wide: 214 },
    cardWidth: 440,
    providers: provider([]),
    permissions: ['audio:read', 'audio:write'],
    optional: true,
    settingsDefaults: { showDevice: true, showSlider: true, showMute: true },
  },
  {
    id: 'clock',
    title: 'Clock',
    shortTitle: 'Clock',
    description: 'Live time, date, timezone and next calendar event.',
    category: 'Clock',
    tint: 'silver',
    tintRgb: '195, 201, 209',
    icon: 'clock',
    defaultSize: 'compact',
    sizes: SIZES,
    defaultWidth: 84,
    widths: { compact: 84, standard: 160, wide: 200 },
    cardWidth: 440,
    providers: provider([]),
    permissions: ['calendar:read'],
    optional: false,
    settingsDefaults: { hour12: false, showSeconds: false, showDate: true, showTimezone: true },
  },
];

const META_BY_ID = new Map(FINAL_WIDGET_META.map((item) => [item.id, item]));

export function getWidgetMeta(id: FinalWidgetId): FinalWidgetMeta {
  const meta = META_BY_ID.get(id);
  if (!meta) throw new Error(`Unknown final widget: ${id}`);
  return meta;
}

export function isFinalWidgetId(value: string): value is FinalWidgetId {
  return META_BY_ID.has(value as FinalWidgetId);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function settingsFor<Id extends FinalWidgetId>(
  id: Id,
  value?: unknown,
): FinalWidgetSettingsFor<Id> {
  const defaults = getWidgetMeta(id).settingsDefaults as FinalWidgetSettingsMap[Id];
  const next: Record<string, unknown> = { ...defaults };
  if (!isRecord(value)) return next as unknown as FinalWidgetSettingsMap[Id];
  const fields = FINAL_SETTING_FIELDS[id];
  for (const field of fields) {
    if (!(field.key in value)) continue;
    const incoming = value[field.key];
    if (field.type === 'toggle') {
      if (typeof incoming === 'boolean') next[field.key] = incoming;
      continue;
    }
    if (field.type === 'range') {
      if (typeof incoming === 'number' && Number.isFinite(incoming)) {
        next[field.key] = Math.min(field.max, Math.max(field.min, Math.round(incoming)));
      }
      continue;
    }
    if (field.type === 'select') {
      if (typeof incoming === 'string' && field.options.some((option) => option.value === incoming)) {
        next[field.key] = incoming;
      }
      continue;
    }
    if (typeof incoming === 'string') {
      next[field.key] = incoming.trim().slice(0, field.maxLength);
    }
  }
  return next as unknown as FinalWidgetSettingsMap[Id];
}

export function validateWidgetSettings(id: FinalWidgetId, value: unknown): boolean {
  if (!isRecord(value)) return false;
  const fields = FINAL_SETTING_FIELDS[id];
  const known = new Set(fields.map((field) => field.key));
  for (const key of Object.keys(value)) {
    if (!known.has(key)) return false;
  }
  for (const field of fields) {
    if (!(field.key in value)) continue;
    const incoming = value[field.key];
    if (field.type === 'toggle' && typeof incoming !== 'boolean') return false;
    if (field.type === 'range') {
      if (typeof incoming !== 'number' || !Number.isFinite(incoming)) return false;
      if (incoming < field.min || incoming > field.max) return false;
    }
    if (field.type === 'select' && (typeof incoming !== 'string' || !field.options.some((option) => option.value === incoming))) {
      return false;
    }
    if (field.type === 'text' && (typeof incoming !== 'string' || incoming.length > field.maxLength)) return false;
  }
  return true;
}

export function settingsAs<Id extends FinalWidgetId>(
  id: Id,
  value?: FinalWidgetSettings | Record<string, unknown> | null,
): FinalWidgetSettingsFor<Id> {
  return settingsFor(id, value);
}
