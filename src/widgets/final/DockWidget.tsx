import type { CSSProperties, ReactNode } from 'react';
import type { WidgetSize } from '../sdk/types';
import { getWidgetMeta, settingsFor } from './meta';
import { sampleFor } from './sample';
import { bytes, clockParts, durationMs, highLow, pct, temperature, uptime, usageTone, compactNumber } from './format';
import { Icon, WeatherIcon, weatherKind } from './icons';
import { AppGlyph, AppIconStack, MetricBar, StatusDot } from './primitives';
import type {
  AgentData,
  AppsData,
  ChatGPTData,
  ClaudeData,
  ClockData,
  FilesData,
  FinalWidgetId,
  FoldersData,
  MediaData,
  SearchData,
  SystemData,
  TrashData,
  VolumeData,
  WeatherData,
  WindowsData,
  WidgetActionHandler,
  DockWidgetProps,
} from './types';
import type { FinalWidgetData } from './types';

type SlotProps = DockWidgetProps;

const TINT_COLORS: Record<string, string> = {
  blue: '#74b7ff',
  green: '#5fd99a',
  amber: '#f0bd6a',
  violet: '#b49cff',
  teal: '#6fd6cf',
  red: '#f08b8b',
  silver: '#c3c9d1',
};

function emit(onAction: WidgetActionHandler | undefined, action: Parameters<WidgetActionHandler>[0]) {
  onAction?.(action);
}

function tintStyle(id: FinalWidgetId): CSSProperties {
  const meta = getWidgetMeta(id);
  return { '--fw-tint': TINT_COLORS[meta.tint] ?? '#c3c9d1', '--fw-tint-rgb': meta.tintRgb } as CSSProperties;
}

function baseClass(id: FinalWidgetId, size: WidgetSize, extra = ''): string {
  return `fw-dock fw-tint-${getWidgetMeta(id).tint} fw-dock--${size}${extra ? ` ${extra}` : ''}`;
}

function mediaProgress(value: number | null | undefined): number {
  if (value == null || !Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

function appsSlot(id: 'apps', size: WidgetSize, data: AppsData | undefined, settings: { maxApps: number; showTitles: boolean; activeOnly: boolean }) {
  const active = data?.activeTitle || data?.items?.find((item) => item.active)?.title || null;
  const items = data?.items ?? [];
  const scoped = settings.activeOnly ? items.filter((item) => item.active).slice(0, 1) : items;
  const titles = scoped.slice(0, settings.maxApps).map((item) => item.title);
  const count = data ? data.windowCount ?? items.length : null;
  if (size === 'compact') {
    return (
      <>
        <span className="fw-dock-ic fw-dock-ic--apps">
          <AppIconStack titles={titles} limit={2} />
        </span>
        <span className="fw-dock-value">{count == null ? '--' : count}</span>
      </>
    );
  }
  return (
    <>
      <span className="fw-dock-ic fw-dock-ic--apps fw-dock-ic--lg">
        <AppIconStack titles={titles} limit={size === 'wide' ? 3 : 2} />
      </span>
      <span className="fw-dock-stack fw-dock-stack--tight">
        <strong className="fw-dock-title">{active ?? 'Running apps'}</strong>
        {settings.showTitles ? (
          <small className="fw-dock-sub">
            {data?.items?.[0]?.title ?? 'Windows'} · {count == null ? '--' : count} {count === 1 ? 'win' : 'win'} · {bytes(data?.memoryBytes)}
          </small>
        ) : null}
      </span>
    </>
  );
}

function usageSlot(
  size: WidgetSize,
  data: ClaudeData | ChatGPTData | undefined,
  settings: { plan: string; showSession: boolean; showWeekly: boolean; showReset: boolean; warnAt: number },
  kind: 'claude' | 'chatgpt',
) {
  const session = data?.sessionPercent ?? null;
  const weekly = data?.weeklyPercent ?? null;
  const reset = kind === 'claude' ? data?.resetIn : (data as ChatGPTData | undefined)?.resetIn;
  const connected = data?.status !== 'disconnected' && data?.status !== 'empty';
  return (
    <>
      <span className="fw-dock-ic fw-usage-plan-ic">
        <Icon name={kind} size={16} />
      </span>
      <span className="fw-dock-stack fw-dock-stack--tight">
        <strong className="fw-dock-title">
          {settings.plan}
          {size !== 'compact' ? <span className="fw-dock-title-accent"> · {connected ? pct(session) : '--'}</span> : null}
        </strong>
        {size === 'compact' ? (
          <span className="fw-dock-value fw-dock-value--tiny">{connected ? pct(session) : '--'}</span>
        ) : (
          <small className="fw-dock-sub">
            {settings.showWeekly ? `Week ${connected ? pct(weekly) : '--'}` : 'Local session'}
            {settings.showReset && reset ? ` · ${reset}` : ''}
          </small>
        )}
      </span>
      {size !== 'compact' && settings.showSession ? (
        <span className="fw-dock-bar-wrap">
          <MetricBar value={session} warnAt={settings.warnAt} label={`${kind} session usage`} tone={kind === 'claude' ? 'violet' : 'green'} />
        </span>
      ) : null}
    </>
  );
}

export function DockWidget({ id, size, settings, data, preview = false, status, onAction }: DockWidgetProps) {
  const meta = getWidgetMeta(id);
  const source = (preview ? sampleFor(id) : data) as FinalWidgetData | undefined;
  const style = { ...tintStyle(id), width: meta.widths[size] } as CSSProperties;
  const interactive = id === 'windows' || id === 'search';
  const action = id === 'windows' ? 'start' : id === 'search' ? 'search' : null;
  const common = {
    className: baseClass(id, size, interactive ? 'is-interactive' : ''),
    style,
    'data-widget': id,
    'data-size': size,
    'data-preview': preview ? 'true' : undefined,
    'aria-label': `${meta.title}, ${size === 'compact' ? 'Pill' : size === 'standard' ? 'Tile' : 'Card'}${preview ? ', sample preview' : ''}`,
    title: `${meta.title} · ${size === 'compact' ? 'Pill' : size === 'standard' ? 'Tile' : 'Card'}`,
  };

  let content: ReactNode;

  switch (id) {
    case 'windows': {
      const s = settingsFor('windows', settings);
      const win = source as WindowsData | undefined;
      content = (
        <>
          <span className="fw-win-logo">
            <Icon name="windows" size={size === 'compact' ? 19 : 21} />
          </span>
          {s.showLabel ? <span className="fw-win-label">Start</span> : null}
          {size === 'wide' && s.showRecentHint ? (
            <span className="fw-win-recent">
              {win?.recentApps?.length ? `${win.recentApps.length} recent` : 'Pinned apps'}
            </span>
          ) : null}
        </>
      );
      break;
    }
    case 'search': {
      const s = settingsFor('search', settings);
      const search = source as SearchData | undefined;
      content = (
        <>
          <span className="fw-dock-ic">
            <Icon name="search" size={17} />
          </span>
          <span className="fw-dock-stack fw-dock-stack--tight">
            <strong className="fw-dock-title">Search</strong>
            {size !== 'compact' && s.showHint ? <small className="fw-dock-sub">Apps, files, folders</small> : null}
          </span>
          {s.showShortcut ? <kbd className="fw-kbd">{size === 'wide' ? 'Alt + Space' : 'Alt'}</kbd> : null}
          {size === 'wide' && search?.indexReady ? <span className="fw-dock-meta">{compactNumber(search.indexedFiles)} files</span> : null}
        </>
      );
      break;
    }
    case 'apps': {
      const s = settingsFor('apps', settings);
      content = appsSlot('apps', size, source as AppsData | undefined, s);
      break;
    }

    case 'spotify': {
      const s = settingsFor('spotify', settings);
      const media = source as MediaData | undefined;
      const progress = media?.positionMs != null && media.durationMs ? (media.positionMs / media.durationMs) * 100 : 0;
      content = (
        <>
          <span className={`fw-dock-art${media?.title ? '' : ' is-empty'}`} aria-hidden="true">
            <span className="fw-dock-art-inner">
              <Icon name="music" size={size === 'compact' ? 14 : 16} />
            </span>
          </span>
          {size !== 'compact' ? (
            <span className="fw-dock-stack fw-dock-stack--tight fw-dock-stack--grow">
              <strong className="fw-dock-title">{media?.title ?? 'Nothing playing'}</strong>
              <small className="fw-dock-sub">{media?.artist ?? 'Connect a media session'}</small>
              {s.showProgress && media?.title ? (
                <span className="fw-dock-progress" aria-hidden="true">
                  <span className="fw-dock-progress-fill" style={{ width: `${mediaProgress(progress)}%` }} />
                </span>
              ) : null}
            </span>
          ) : null}
          {s.showControls && media?.title ? (
            <button
              type="button"
              className="fw-dock-control"
              onClick={() => emit(onAction, 'media:toggle')}
              disabled={!onAction || media.canToggle === false}
              aria-label={media.playing ? 'Pause Spotify' : 'Play Spotify'}
            >
              <Icon name={media.playing ? 'pause' : 'play'} size={12} />
            </button>
          ) : null}
        </>
      );
      break;
    }
    case 'weather': {
      const s = settingsFor('weather', settings);
      const weather = source as WeatherData | undefined;
      const units = s.units;
      const kind = weatherKind(weather?.weatherCode);
      const label = weather
        ? weatherLabelShort(weather.weatherCode)
        : s.weatherEnabled ? 'No data' : 'Set location';
      content = (
        <>
          <span className="fw-dock-weather-ic">
            <WeatherIcon kind={kind} size={size === 'compact' ? 22 : 25} />
          </span>
          <span className="fw-dock-weather-temp">{weather ? temperature(weather.temperature, units) : '--°'}</span>
          {size !== 'compact' ? (
            <span className="fw-dock-stack fw-dock-stack--tight">
              <small className="fw-dock-sub">{label}</small>
              {weather ? <span className="fw-dock-weather-meta">{highLow(weather.highC, weather.lowC, units)}</span> : null}
              {size === 'wide' && (weather || s.weatherEnabled) ? <span className="fw-dock-location">{s.location}</span> : null}
            </span>
          ) : null}
        </>
      );
      break;
    }
    case 'system': {
      const s = settingsFor('system', settings);
      const system = source as SystemData | undefined;
      const compact = size === 'compact';
      const wide = size === 'wide';
      const extras = [
        s.showTemp ? (system?.temperatureC == null ? '-- temp' : `${Math.round(system.temperatureC)}°C`) : null,
        s.showUptime ? uptime(system?.uptimeSeconds) : null,
      ].filter(Boolean).join(' · ');
      content = (
        <span className="fw-sys-dock">
          {s.showCpu ? (
            <span className="fw-sys-mini">
              <span className="fw-sys-mini-top"><Icon name="cpu" size={10} /> CPU</span>
              <strong>{pct(system?.cpu)}</strong>
              {wide ? <MetricBar value={system?.cpu} label="CPU load" tone="blue" /> : null}
            </span>
          ) : null}
          {s.showRam ? (
            <span className="fw-sys-mini">
              <span className="fw-sys-mini-top"><Icon name="ram" size={10} /> RAM</span>
              <strong>{pct(system?.ram)}</strong>
              {wide ? <MetricBar value={system?.ram} label="RAM load" tone="blue" /> : null}
            </span>
          ) : null}
          {!compact && s.showBattery ? (
            <span className="fw-sys-mini">
              <span className="fw-sys-mini-top"><Icon name="battery" size={10} /> BAT</span>
              <strong>{pct(system?.battery)}</strong>
              {wide ? <MetricBar value={system?.battery} label="Battery level" tone="green" /> : null}
            </span>
          ) : null}
          {wide && extras ? <span className="fw-sys-extra">{extras}</span> : null}
          {!s.showCpu && !s.showRam && (!s.showBattery || compact) ? <span className="fw-dock-value">--</span> : null}
        </span>
      );
      break;
    }
    case 'claude':
    case 'chatgpt': {
      const s = settingsFor(id, settings);
      content = usageSlot(size, source as ClaudeData | ChatGPTData | undefined, s, id);
      break;
    }
    case 'agent': {
      const s = settingsFor('agent', settings);
      const agent = source as AgentData | undefined;
      const state = agent?.status === 'live' ? 'running' : agent?.status === 'empty' ? 'idle' : 'offline';
      content = (
        <>
          <span className="fw-agent-dock-head">
            <StatusDot state={state} />
            <strong className="fw-dock-title">{state === 'running' ? 'running' : state === 'idle' ? 'idle' : 'offline'}</strong>
          </span>
          {size !== 'compact' ? (
            <span className="fw-dock-stack fw-dock-stack--tight">
              <small className="fw-dock-sub fw-dock-sub--strong">{agent?.workspace ?? s.workspace}</small>
              <span className="fw-dock-meta">{s.showSession && agent?.duration ? agent.duration : meta.shortTitle}</span>
            </span>
          ) : null}
          {size === 'wide' ? <span className="fw-agent-dock-task">{agent?.task ?? 'No task'}</span> : null}
        </>
      );
      break;
    }
    case 'files': {
      const s = settingsFor('files', settings);
      const fileData = source as FilesData | undefined;
      const latest = fileData?.recent?.[0];
      content = (
        <>
          <span className="fw-dock-ic">
            <Icon name="files" size={17} />
          </span>
          <span className="fw-dock-stack fw-dock-stack--tight">
            <strong className="fw-dock-title">{latest?.name ?? 'Recent files'}</strong>
            <small className="fw-dock-sub">{fileData?.recent?.length ?? '--'} files in {s.source}</small>
          </span>
          {size === 'wide' && s.showType && latest?.kind ? <span className="fw-file-badge">{latest.kind.toUpperCase()}</span> : null}
        </>
      );
      break;
    }
    case 'folders': {
      const s = settingsFor('folders', settings);
      const folderData = source as FoldersData | undefined;
      const folders = folderData?.folders ?? [];
      content = (
        <>
          <span className="fw-dock-ic">
            <Icon name="folders" size={18} />
          </span>
          <span className="fw-dock-stack fw-dock-stack--tight">
            <strong className="fw-dock-title">{size === 'compact' ? folders.length || '--' : 'Quick folders'}</strong>
            <small className="fw-dock-sub">{size === 'compact' ? 'locations' : `${folders.length} known locations`}</small>
          </span>
          {size === 'wide' ? <span className="fw-folder-dock-names">{folders.slice(0, s.maxItems).map((f) => f.name).join(' · ')}</span> : null}
        </>
      );
      break;
    }
    case 'trash': {
      const s = settingsFor('trash', settings);
      const trash = source as TrashData | undefined;
      content = (
        <>
          <span className="fw-dock-ic">
            <Icon name="trash" size={16} />
          </span>
          <span className="fw-dock-value">{trash?.itemCount == null ? '--' : trash.itemCount}</span>
          {size !== 'compact' ? (
            <span className="fw-dock-stack fw-dock-stack--tight">
              <small className="fw-dock-sub">items</small>
              {s.showSize ? <span className="fw-dock-meta">{bytes(trash?.sizeBytes)}</span> : null}
            </span>
          ) : null}
        </>
      );
      break;
    }
    case 'volume': {
      const s = settingsFor('volume', settings);
      const sound = source as VolumeData | undefined;
      const level = sound?.level;
      content = (
        <>
          <span className="fw-dock-ic">
            <Icon name={sound?.muted ? 'volumeMute' : 'volume'} size={17} />
          </span>
          <span className="fw-dock-value">{level == null ? '--' : sound?.muted ? 'Mute' : `${Math.round(level)}%`}</span>
          {size !== 'compact' ? (
            <span className="fw-dock-volume">
              {s.showSlider ? <MetricBar value={sound?.muted ? 0 : level} label="Volume level" tone="green" /> : null}
              {s.showDevice ? <small className="fw-dock-sub">{sound?.device ?? 'No output device'}</small> : null}
            </span>
          ) : null}
        </>
      );
      break;
    }
    case 'clock': {
      const s = settingsFor('clock', settings);
      const clock = source as ClockData | undefined;
      const parts = clockParts(clock?.now, s.hour12, s.showSeconds, clock?.timezone);
      content = (
        <>
          <span className="fw-dock-stack fw-dock-stack--tight">
            <strong className="fw-dock-clock">
              {parts.time}
              {parts.suffix ? <span className="fw-dock-clock-suffix"> {parts.suffix}</span> : null}
            </strong>
            {s.showDate ? <small className="fw-dock-sub">{parts.date}</small> : null}
            {size === 'wide' && s.showTimezone ? <span className="fw-dock-meta">{parts.timezone}</span> : null}
          </span>
        </>
      );
      break;
    }
    default:
      content = null;
  }

  if (interactive && action && onAction) {
    return (
      <button type="button" {...common} onClick={() => emit(onAction, action)}>
        {content}
      </button>
    );
  }

  return (
    <div {...common} role="group" aria-roledescription="dock widget">
      {content}
    </div>
  );
}

function weatherLabelShort(code: number | null | undefined): string {
  return weatherKind(code) === 'sun' ? 'Clear' : weatherKind(code) === 'partly' ? 'Partly cloudy' : weatherKind(code) === 'rain' ? 'Rain' : 'Cloudy';
}

export function dockActionFor(id: FinalWidgetId): 'start' | 'search' | null {
  if (id === 'windows') return 'start';
  if (id === 'search') return 'search';
  return null;
}

export function mediaDuration(data: MediaData | undefined): string {
  return durationMs(data?.durationMs);
}

export function systemTempLabel(data: SystemData | undefined): string {
  return data?.temperatureC == null ? '--' : `${Math.round(data.temperatureC)}°C`;
}

export function usageToneFor(value: number | null | undefined, warnAt: number) {
  return usageTone(value, warnAt);
}

export type { SlotProps };
