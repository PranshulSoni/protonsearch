import type {
  AppsData,
  ChatGPTData,
  ClaudeData,
  AgentData,
  ClockData,
  FilesData,
  FinalWidgetDataFor,
  FinalWidgetId,
  FoldersData,
  MediaData,
  SearchData,
  SystemData,
  TrashData,
  VolumeData,
  WeatherData,
  WindowsData,
} from './types';

const SAMPLE_NOW = new Date(2026, 10, 7, 4, 27, 0).getTime();

const windows: WindowsData = {
  pinnedApps: ['Files', 'Settings', 'Terminal'],
  recentApps: ['VS Code', 'ProtonSearch', 'Spotify'],
};

const search: SearchData = {
  indexReady: true,
  indexedFiles: 12840,
  lastIndexed: '2 min ago',
};

const apps: AppsData = {
  items: [
    { id: 'vscode', title: 'VS Code', icon: null, active: true },
    { id: 'terminal', title: 'Terminal', icon: null, active: false },
    { id: 'browser', title: 'Browser', icon: null, active: false },
    { id: 'slack', title: 'Slack', icon: null, active: false },
    { id: 'figma', title: 'Figma', icon: null, active: false },
    { id: 'notion', title: 'Notion', icon: null, active: false },
  ],
  activeTitle: 'Sidebar-Bridge.tsx',
  windowCount: 4,
  memoryBytes: 2576980378,
};

const spotify: MediaData = {
  title: 'Tunnel Vision',
  artist: 'Aurora B.Poreal',
  source: 'Spotify',
  playing: true,
  canToggle: true,
  canNext: true,
  canPrevious: true,
  album: 'Afterglow',
  albumArtist: 'Aurora B.Poreal',
  playbackStatus: 'Playing',
  playbackRate: 1,
  positionMs: 134000,
  durationMs: 228000,
  device: 'Windows · Speakers',
  volume: 68,
};

const hours: WeatherData['hours'] = Array.from({ length: 8 }, (_, index) => ({
  time: SAMPLE_NOW + index * 60 * 60 * 1000,
  temperature: [24, 25, 26, 26, 25, 23, 22, 21][index] ?? null,
  weatherCode: [2, 2, 1, 1, 2, 3, 3, 2][index] ?? null,
}));

const weather: WeatherData = {
  temperature: 24,
  weatherCode: 2,
  time: SAMPLE_NOW,
  utcOffsetSeconds: 19800,
  hours,
  highC: 27,
  lowC: 18,
  feelsLikeC: 25,
  windKph: 12,
  humidityPercent: 58,
  uvIndex: 3,
  visibilityKm: 10,
};

const system: SystemData = {
  cpu: 24,
  ram: 58,
  battery: 84,
  batteryCharging: false,
  temperatureC: null,
  uptimeSeconds: 273600,
  connectivity: {
    connectionType: 'wifi',
    wifi: { connected: true, ssid: 'Proton-5G', signalPercent: 86, state: 'connected' },
    ethernet: { connected: false, adapter: 'Realtek PCIe' },
    vpn: { connected: false, adapter: null },
    bluetooth: { available: true, enabled: true, name: 'MX Master 3S' },
    downloadBytesPerSecond: 0,
    uploadBytesPerSecond: 0,
  },
};

const claude: ClaudeData = {
  status: 'live',
  plan: 'Max 20x',
  model: 'Opus 4.5',
  sessionPercent: 62,
  weeklyPercent: 38,
  resetIn: '2h 14m',
  weeklyResetIn: '4d 06h',
  tokensUsed: 184000,
  tokensLimit: 300000,
};

const chatgpt: ChatGPTData = {
  status: 'live',
  plan: 'Pro',
  model: 'GPT-5.2',
  sessionPercent: 41,
  weeklyPercent: 74,
  resetIn: '3h 52m',
  weeklyResetIn: '2d 09h',
  tokensUsed: 320,
  tokensLimit: 500,
  creditsPercent: 64,
  creditsUsed: 320,
  creditsLimit: 500,
};

const agent: AgentData = {
  status: 'live',
  workspace: 'protonsearch',
  branch: 'feature/dock-widgets',
  task: 'Port dock widgets to native providers',
  duration: '1h 24m',
  tokens: '96.4K',
  filesChanged: 7,
  toolCalls: 34,
};

const files: FilesData = {
  recent: [
    { name: 'dock-renderer.tsx', path: 'src/widgets/dock-renderer.tsx', kind: 'tsx', modifiedAt: '2 min ago', sizeBytes: 14200 },
    { name: 'providers.rs', path: 'src-tauri/src/providers.rs', kind: 'rs', modifiedAt: '18 min ago', sizeBytes: 9800 },
    { name: 'CONTRACT.md', path: 'docs/widgets/CONTRACT.md', kind: 'md', modifiedAt: '1h ago', sizeBytes: 5400 },
    { name: 'proton.config.js', path: 'proton.config.js', kind: 'js', modifiedAt: 'yesterday', sizeBytes: 21800 },
  ],
};

const folders: FoldersData = {
  folders: [
    { id: 'desktop', name: 'Desktop', path: 'C:\\Users\\Pranshul\\Desktop', icon: 'desktop', itemCount: 12 },
    { id: 'documents', name: 'Documents', path: 'C:\\Users\\Pranshul\\Documents', icon: 'documents', itemCount: 84 },
    { id: 'downloads', name: 'Downloads', path: 'C:\\Users\\Pranshul\\Downloads', icon: 'downloads', itemCount: 7 },
    { id: 'pictures', name: 'Pictures', path: 'C:\\Users\\Pranshul\\Pictures', icon: 'pictures', itemCount: 340 },
    { id: 'projects', name: 'Projects', path: 'C:\\Users\\Pranshul\\Documents\\Projects', icon: 'projects', itemCount: 18 },
    { id: 'screenshots', name: 'Screenshots', path: 'C:\\Users\\Pranshul\\Pictures\\Screenshots', icon: 'screenshots', itemCount: 52 },
    { id: 'videos', name: 'Videos', path: 'C:\\Users\\Pranshul\\Videos', icon: 'videos', itemCount: 23 },
    { id: 'music', name: 'Music', path: 'C:\\Users\\Pranshul\\Music', icon: 'music', itemCount: 145 },
  ],
};

const trash: TrashData = {
  itemCount: 18,
  sizeBytes: 2576980378,
  lastEmptiedAt: '3 days ago',
};

const volume: VolumeData = {
  level: 68,
  muted: false,
  device: 'Speakers (Realtek)',
  mediaApp: 'Spotify · Tunnel Vision',
};

const clock: ClockData = {
  now: SAMPLE_NOW,
  timezone: 'GMT+5:30',
  nextEvent: { title: 'Design review · Dock widgets', startsAt: '16:30' },
};

export const FINAL_WIDGET_SAMPLES = {
  windows,
  search,
  apps,
  spotify,
  weather,
  system,
  claude,
  chatgpt,
  agent,
  files,
  folders,
  trash,
  volume,
  clock,
} satisfies { [Id in FinalWidgetId]: FinalWidgetDataFor<Id> };

export function sampleFor<Id extends FinalWidgetId>(id: Id): FinalWidgetDataFor<Id> {
  return FINAL_WIDGET_SAMPLES[id];
}

export function previewDataFor<Id extends FinalWidgetId>(
  id: Id,
  preview: boolean,
  data: FinalWidgetDataFor<Id> | undefined,
): FinalWidgetDataFor<Id> | undefined {
  return preview ? FINAL_WIDGET_SAMPLES[id] : data;
}

export const SAMPLE_TIMESTAMP = SAMPLE_NOW;
