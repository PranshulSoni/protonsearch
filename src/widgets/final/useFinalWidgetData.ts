import { useEffect, useMemo, useState } from 'react';
import { useWeather } from '../../services/weather';
import { isNative } from '../../services/native';
import type { Snapshot, WidgetStatus } from '../sdk/types';
import { useProvider } from '../sdk/provider';
import type {
  AppsData,
  ClockData,
  FinalWidgetData,
  FinalWidgetId,
  FinalWidgetSettings,
  FilesData,
  FoldersData,
  MediaData,
  SystemConnectivity,
  SystemData,
  TrashData,
  VolumeData,
  WeatherData,
  WeatherSettings,
  WindowsData,
} from './types';

type RecordValue = Record<string, unknown>;

export interface FinalWidgetView {
  data?: FinalWidgetData;
  status: WidgetStatus;
  preview: boolean;
}

export type FinalWidgetViews = Record<FinalWidgetId, FinalWidgetView>;
export type FinalWidgetSettingsById = Partial<Record<FinalWidgetId, FinalWidgetSettings>>;

function record(value: unknown): RecordValue | undefined {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as RecordValue : undefined;
}

function text(value: unknown): string | null {
  return typeof value === 'string' && value.length ? value : null;
}

function number(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function flag(value: unknown): boolean | null {
  return typeof value === 'boolean' ? value : null;
}

function connection(value: unknown): SystemConnectivity | null {
  const data = record(value);
  if (!data) return null;
  const wifi = record(data.wifi);
  const ethernet = record(data.ethernet);
  const vpn = record(data.vpn);
  const bluetooth = record(data.bluetooth);
  return {
    connectionType: text(data.connectionType),
    wifi: wifi ? {
      connected: flag(wifi.connected),
      ssid: text(wifi.ssid),
      signalPercent: number(wifi.signalPercent),
      state: text(wifi.state),
    } : null,
    ethernet: ethernet ? { connected: flag(ethernet.connected), adapter: text(ethernet.adapter) } : null,
    vpn: vpn ? { connected: flag(vpn.connected), adapter: text(vpn.adapter) } : null,
    bluetooth: bluetooth ? {
      available: flag(bluetooth.available),
      enabled: flag(bluetooth.enabled),
      name: text(bluetooth.name),
    } : null,
    downloadBytesPerSecond: number(data.downloadBytesPerSecond),
    uploadBytesPerSecond: number(data.uploadBytesPerSecond),
  };
}

export function mapRunningApps(snapshot: Snapshot): AppsData | undefined {
  const data = record(snapshot.data);
  if (!data || !Array.isArray(data.items)) return undefined;
  const items = data.items.flatMap((entry) => {
    const item = record(entry);
    if (!item || typeof item.id !== 'string' || typeof item.title !== 'string') return [];
    return [{ id: item.id, title: item.title, icon: text(item.icon), active: item.active === true }];
  });
  const active = items.find((item) => item.active)?.title ?? null;
  return {
    items,
    activeTitle: active,
    windowCount: null,
    memoryBytes: null,
  };
}

export function mapWindows(snapshot: Snapshot): WindowsData | undefined {
  const apps = mapRunningApps(snapshot);
  if (!apps) return undefined;
  return {
    pinnedApps: null,
    recentApps: (apps.items ?? []).map((item) => item.title),
  };
}

export function mapSystem(snapshot: Snapshot): SystemData | undefined {
  const data = record(snapshot.data);
  if (!data) return undefined;
  return {
    cpu: number(data.cpu),
    ram: number(data.ram),
    battery: number(data.battery),
    batteryCharging: flag(data.batteryCharging),
    temperatureC: number(data.temperatureC),
    uptimeSeconds: number(data.uptimeSeconds),
    connectivity: connection(data.connectivity),
  };
}

export function mapMedia(snapshot: Snapshot, volumeSnapshot?: Snapshot): MediaData | undefined {
  const data = record(snapshot.data);
  if (!data) return undefined;
  const sound = record(volumeSnapshot?.data);
  return {
    title: text(data.title),
    artist: text(data.artist),
    source: text(data.source),
    playing: flag(data.playing),
    canToggle: flag(data.canToggle),
    canNext: flag(data.canNext),
    canPrevious: flag(data.canPrevious),
    album: text(data.album),
    albumArtist: text(data.albumArtist),
    playbackStatus: text(data.playbackStatus),
    playbackRate: number(data.playbackRate),
    positionMs: number(data.positionMs),
    durationMs: number(data.durationMs),
    device: text(data.device) ?? text(sound?.device),
    volume: number(data.volume) ?? number(sound?.level),
  };
}

export function mapKnownFolders(snapshot: Snapshot): FilesData & FoldersData | undefined {
  const data = record(snapshot.data);
  if (!data) return undefined;
  const recent = Array.isArray(data.recent) ? data.recent.flatMap((entry) => {
    const item = record(entry);
    if (!item || typeof item.name !== 'string') return [];
    const modified = typeof item.modifiedAt === 'number' && Number.isFinite(item.modifiedAt)
      ? new Date(item.modifiedAt).toISOString()
      : text(item.modifiedAt);
    return [{
      name: item.name,
      path: typeof item.path === 'string' ? item.path : '',
      kind: text(item.kind),
      modifiedAt: modified,
      sizeBytes: number(item.sizeBytes),
    }];
  }) : null;
  const folders = Array.isArray(data.folders) ? data.folders.flatMap((entry) => {
    const item = record(entry);
    if (!item || typeof item.id !== 'string' || typeof item.name !== 'string') return [];
    return [{
      id: item.id,
      name: item.name,
      path: typeof item.path === 'string' ? item.path : '',
      icon: text(item.icon),
      itemCount: number(item.itemCount),
    }];
  }) : null;
  return { recent, folders };
}

export function mapTrash(snapshot: Snapshot): TrashData | undefined {
  const data = record(snapshot.data);
  if (!data) return undefined;
  return {
    itemCount: number(data.itemCount),
    sizeBytes: number(data.sizeBytes),
    lastEmptiedAt: text(data.lastEmptiedAt),
  };
}

export function mapVolume(snapshot: Snapshot): VolumeData | undefined {
  const data = record(snapshot.data);
  if (!data) return undefined;
  return {
    level: number(data.level),
    muted: flag(data.muted),
    device: text(data.device),
    mediaApp: text(data.mediaApp),
  };
}

export function mapClock(now: number): ClockData {
  return { now, timezone: null, nextEvent: null };
}

export function mapWeather(snapshot: Snapshot): WeatherData | undefined {
  const data = record(snapshot.data);
  if (!data) return undefined;
  return {
    temperature: number(data.temperature),
    weatherCode: number(data.weatherCode),
    time: number(data.time),
    utcOffsetSeconds: number(data.utcOffsetSeconds),
    hours: Array.isArray(data.hours) ? data.hours.flatMap((entry) => {
      const hour = record(entry);
      if (!hour || typeof hour.time !== 'number') return [];
      return [{ time: hour.time, temperature: number(hour.temperature), weatherCode: number(hour.weatherCode) }];
    }) : null,
    highC: number(data.highC),
    lowC: number(data.lowC),
    feelsLikeC: number(data.feelsLikeC),
    windKph: number(data.windKph),
    humidityPercent: number(data.humidityPercent),
    uvIndex: number(data.uvIndex),
    visibilityKm: number(data.visibilityKm),
  };
}

export function isPreviewMode(search = window.location.search): boolean {
  return !isNative() && new URLSearchParams(search).get('preview') === '1';
}

function statusOf(snapshot: Snapshot): WidgetStatus {
  return snapshot.status;
}

export function useFinalWidgetData(
  settings: FinalWidgetSettingsById = {},
  options: { preview?: boolean } = {},
): FinalWidgetViews {
  const preview = options.preview ?? isPreviewMode();
  const system = useProvider('system', 5000);
  const media = useProvider('media', 2000);
  const volume = useProvider('volume', 5000);
  const trash = useProvider('trash', 60000);
  const folders = useProvider('known-folders', 60000);
  const runningApps = useProvider('running-apps', 5000);
  const weather = useWeather((settings.weather as unknown as Record<string, unknown> | undefined) ?? {});
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  return useMemo(() => {
    const known = mapKnownFolders(folders);
    const sound = mapVolume(volume);
    const views: FinalWidgetViews = {
      windows: { data: mapWindows(runningApps), status: statusOf(runningApps), preview },
      search: { status: 'live', preview },
      apps: { data: mapRunningApps(runningApps), status: statusOf(runningApps), preview },
      spotify: { data: mapMedia(media, volume), status: statusOf(media), preview },
      weather: { data: mapWeather(weather), status: statusOf(weather), preview },
      system: { data: mapSystem(system), status: statusOf(system), preview },
      claude: { status: 'disconnected', preview },
      chatgpt: { status: 'disconnected', preview },
      agent: { status: 'disconnected', preview },
      files: { data: known ? { recent: known.recent } : undefined, status: statusOf(folders), preview },
      folders: { data: known ? { folders: known.folders } : undefined, status: statusOf(folders), preview },
      trash: { data: mapTrash(trash), status: statusOf(trash), preview },
      volume: { data: sound, status: statusOf(volume), preview },
      clock: { data: mapClock(now), status: 'live', preview },
    };
    return views;
  }, [folders, media, now, preview, runningApps, system, trash, volume, weather]);
}
