import type { ProviderId, WidgetSize, WidgetStatus } from '../sdk/types';

export type FinalWidgetId =
  | 'windows'
  | 'search'
  | 'apps'
  | 'spotify'
  | 'weather'
  | 'system'
  | 'claude'
  | 'chatgpt'
  | 'agent'
  | 'files'
  | 'folders'
  | 'trash'
  | 'volume'
  | 'clock';

export type FinalWidgetCategory = 'System' | 'Media' | 'Weather' | 'AI' | 'Files' | 'Productivity' | 'Clock';
export type FinalWidgetTint = 'blue' | 'green' | 'amber' | 'violet' | 'teal' | 'red' | 'silver';
export type FinalAIStatus = 'disconnected' | 'empty' | 'live';

export interface WindowsSettings {
  showLabel: boolean;
  showRecentHint: boolean;
}
export interface SearchSettings {
  showHint: boolean;
  showShortcut: boolean;
}
export interface AppsSettings {
  maxApps: number;
  showTitles: boolean;
  activeOnly: boolean;
}
export interface SpotifySettings {
  showProgress: boolean;
  showControls: boolean;
  showDevice: boolean;
  showVolume: boolean;
  compactProgress: boolean;
}
export interface WeatherSettings {
  location: string;
  units: 'C' | 'F';
  showHourly: boolean;
  showDetails: boolean;
  forecastHours: number;
}
export interface SystemSettings {
  showCpu: boolean;
  showRam: boolean;
  showTemp: boolean;
  showBattery: boolean;
  showWifi: boolean;
  showBluetooth: boolean;
  showNetwork: boolean;
  showUptime: boolean;
}
export interface ClaudeSettings {
  plan: string;
  showSession: boolean;
  showWeekly: boolean;
  showReset: boolean;
  warnAt: number;
}
export interface ChatGPTSettings {
  plan: string;
  showSession: boolean;
  showWeekly: boolean;
  showReset: boolean;
  warnAt: number;
}
export interface AgentSettings {
  workspace: string;
  showTokens: boolean;
  showFiles: boolean;
  showSession: boolean;
  showToolCalls: boolean;
}
export interface FilesSettings {
  source: string;
  maxItems: number;
  showType: boolean;
}
export interface FoldersSettings {
  maxItems: number;
  showCounts: boolean;
}
export interface TrashSettings {
  showSize: boolean;
  showLastEmptied: boolean;
}
export interface VolumeSettings {
  showDevice: boolean;
  showSlider: boolean;
  showMute: boolean;
}
export interface ClockSettings {
  hour12: boolean;
  showSeconds: boolean;
  showDate: boolean;
  showTimezone: boolean;
}

export interface FinalWidgetSettingsMap {
  windows: WindowsSettings;
  search: SearchSettings;
  apps: AppsSettings;
  spotify: SpotifySettings;
  weather: WeatherSettings;
  system: SystemSettings;
  claude: ClaudeSettings;
  chatgpt: ChatGPTSettings;
  agent: AgentSettings;
  files: FilesSettings;
  folders: FoldersSettings;
  trash: TrashSettings;
  volume: VolumeSettings;
  clock: ClockSettings;
}

export type FinalWidgetSettings = FinalWidgetSettingsMap[FinalWidgetId];
export type FinalWidgetSettingsFor<Id extends FinalWidgetId> = FinalWidgetSettingsMap[Id];

export interface RunningAppSnapshotItem {
  id: string;
  title: string;
  icon: string | null;
  active: boolean;
}

export interface WindowsData {
  pinnedApps?: string[] | null;
  recentApps?: string[] | null;
}
export interface SearchData {
  indexReady?: boolean | null;
  indexedFiles?: number | null;
  lastIndexed?: string | null;
}
export interface AppsData {
  items: RunningAppSnapshotItem[] | null;
  activeTitle?: string | null;
  windowCount?: number | null;
  memoryBytes?: number | null;
}
export interface MediaData {
  title: string | null;
  artist: string | null;
  source: string | null;
  playing: boolean | null;
  canToggle: boolean | null;
  canNext: boolean | null;
  canPrevious: boolean | null;
  album: string | null;
  albumArtist: string | null;
  playbackStatus: string | null;
  playbackRate: number | null;
  positionMs: number | null;
  durationMs: number | null;
  device?: string | null;
  volume?: number | null;
}
export interface WeatherHour {
  time: number;
  temperature: number | null;
  weatherCode: number | null;
}
export interface WeatherData {
  temperature: number | null;
  weatherCode: number | null;
  time: number | null;
  utcOffsetSeconds: number | null;
  hours: WeatherHour[] | null;
  highC?: number | null;
  lowC?: number | null;
  feelsLikeC?: number | null;
  windKph?: number | null;
  humidityPercent?: number | null;
  uvIndex?: number | null;
  visibilityKm?: number | null;
}
export interface SystemConnectivity {
  connectionType?: string | null;
  wifi?: {
    connected: boolean | null;
    ssid: string | null;
    signalPercent: number | null;
    state: string | null;
  } | null;
  ethernet?: { connected: boolean | null; adapter: string | null } | null;
  vpn?: { connected: boolean | null; adapter: string | null } | null;
  bluetooth?: { available: boolean | null; enabled: boolean | null; name: string | null } | null;
  downloadBytesPerSecond?: number | null;
  uploadBytesPerSecond?: number | null;
}
export interface SystemData {
  cpu: number | null;
  ram: number | null;
  battery: number | null;
  batteryCharging: boolean | null;
  temperatureC: number | null;
  uptimeSeconds: number | null;
  connectivity: SystemConnectivity | null;
}
export interface ClaudeData {
  status?: FinalAIStatus;
  model?: string | null;
  sessionPercent?: number | null;
  weeklyPercent?: number | null;
  resetIn?: string | null;
  weeklyResetIn?: string | null;
  tokensUsed?: number | null;
  tokensLimit?: number | null;
  plan?: string | null;
}
export interface ChatGPTData extends ClaudeData {
  creditsPercent?: number | null;
  creditsUsed?: number | null;
  creditsLimit?: number | null;
}
export interface AgentData {
  status?: FinalAIStatus;
  workspace?: string | null;
  branch?: string | null;
  task?: string | null;
  duration?: string | null;
  tokens?: string | null;
  filesChanged?: number | null;
  toolCalls?: number | null;
}
export interface FileItem {
  name: string;
  path: string;
  kind: string | null;
  modifiedAt: string | null;
  sizeBytes?: number | null;
}
export interface FilesData {
  recent: FileItem[] | null;
}
export interface FolderItem {
  id: string;
  name: string;
  path: string;
  icon?: string | null;
  itemCount?: number | null;
}
export interface FoldersData {
  folders: FolderItem[] | null;
}
export interface TrashData {
  itemCount: number | null;
  sizeBytes: number | null;
  lastEmptiedAt: string | null;
}
export interface VolumeData {
  level: number | null;
  muted: boolean | null;
  device: string | null;
  mediaApp?: string | null;
}
export interface ClockEvent {
  title: string;
  startsAt: string | number;
}
export interface ClockData {
  now?: number | null;
  timezone?: string | null;
  nextEvent?: ClockEvent | null;
}

export interface FinalWidgetDataMap {
  windows: WindowsData;
  search: SearchData;
  apps: AppsData;
  spotify: MediaData;
  weather: WeatherData;
  system: SystemData;
  claude: ClaudeData;
  chatgpt: ChatGPTData;
  agent: AgentData;
  files: FilesData;
  folders: FoldersData;
  trash: TrashData;
  volume: VolumeData;
  clock: ClockData;
}

export type FinalWidgetData = FinalWidgetDataMap[FinalWidgetId];
export type FinalWidgetDataFor<Id extends FinalWidgetId> = FinalWidgetDataMap[Id];

export interface FinalWidgetMeta {
  id: FinalWidgetId;
  title: string;
  shortTitle: string;
  description: string;
  category: FinalWidgetCategory;
  tint: FinalWidgetTint;
  tintRgb: string;
  icon: FinalIconName;
  defaultSize: WidgetSize;
  sizes: readonly WidgetSize[];
  defaultWidth: number;
  widths: Record<WidgetSize, number>;
  cardWidth: number;
  providers: readonly ProviderId[];
  permissions: readonly string[];
  optional: boolean;
  settingsDefaults: FinalWidgetSettings;
}

export type FinalIconName =
  | FinalWidgetId
  | 'volumeMute'
  | 'cpu'
  | 'ram'
  | 'temp'
  | 'battery'
  | 'wifi'
  | 'bluetooth'
  | 'ethernet'
  | 'vpn'
  | 'uptime'
  | 'play'
  | 'pause'
  | 'prev'
  | 'next'
  | 'device'
  | 'close'
  | 'back'
  | 'check'
  | 'chevron'
  | 'drag'
  | 'sliders'
  | 'music'
  | 'calendar'
  | 'bolt'
  | 'branch'
  | 'spark'
  | 'terminal'
  | 'wind'
  | 'droplet'
  | 'uv'
  | 'eye'
  | 'folder'
  | 'doc'
  | 'plus'
  | 'store'
  | 'settings';

export type FolderAction = `folder:open:${string}`;
export type VolumeSetAction = `volume:set:${number}`;
export type WidgetAction =
  | 'start'
  | 'search'
  | 'trash:empty'
  | 'media:toggle'
  | 'media:next'
  | 'media:previous'
  | 'volume:up'
  | 'volume:down'
  | 'volume:mute'
  | 'settings:sound'
  | 'settings:network'
  | 'settings:bluetooth'
  | FolderAction
  | VolumeSetAction;

export type WidgetActionHandler = (action: WidgetAction, payload?: unknown) => void;

export interface DockWidgetProps {
  id: FinalWidgetId;
  size: WidgetSize;
  settings?: FinalWidgetSettings;
  data?: FinalWidgetData;
  preview?: boolean;
  status?: WidgetStatus;
  onAction?: WidgetActionHandler;
}

export interface WidgetCardProps {
  id: FinalWidgetId;
  settings?: FinalWidgetSettings;
  data?: FinalWidgetData;
  preview?: boolean;
  status?: WidgetStatus;
  onAction?: WidgetActionHandler;
}

export interface WidgetSettingsProps {
  id: FinalWidgetId;
  value?: FinalWidgetSettings;
  onChange: (settings: FinalWidgetSettings) => void;
  disabled?: boolean;
}
