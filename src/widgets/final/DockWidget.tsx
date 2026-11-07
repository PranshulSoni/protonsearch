import type { ReactNode } from 'react';
import { getWidgetMeta, settingsFor } from './meta';
import { sampleFor } from './sample';
import { clockParts, pct, temperature } from './format';
import { Icon, WeatherIcon, weatherKind } from './icons';
import { ClockFace, Ring } from './glyphs';
import { StatusDot } from './primitives';
import type {
  AgentData,
  ChatGPTData,
  ClaudeData,
  ClockData,
  DockWidgetProps,
  FilesData,
  FinalWidgetData,
  FinalWidgetId,
  MediaData,
  SystemData,
  TrashData,
  VolumeData,
  WeatherData,
} from './types';

export function DockWidget({ id, settings, data, preview = false }: DockWidgetProps) {
  const meta = getWidgetMeta(id);
  const source = (preview ? sampleFor(id) : data) as FinalWidgetData | undefined;
  let glyph: ReactNode = <Icon name={meta.icon} size={22} />;
  let label = meta.title;
  let off = false;

  switch (id) {
    case 'spotify': {
      const media = source as MediaData | undefined;
      const title = media?.title;
      glyph = (
        <>
          <Icon name="music" size={20} />
          {title && media?.playing ? (
            <span className="dock-eq" aria-hidden="true">
              <i />
              <i />
              <i />
            </span>
          ) : null}
        </>
      );
      label = title ? `${media?.playing ? 'Playing' : 'Paused'} ${title}` : 'Nothing playing';
      off = !title;
      break;
    }
    case 'weather': {
      const s = settingsFor('weather', settings);
      const weather = source as WeatherData | undefined;
      const kind = weatherKind(weather?.weatherCode);
      const temp = weather ? temperature(weather.temperature, s.units) : '--°';
      glyph = (
        <span className="dock-weather">
          {weather ? <WeatherIcon kind={kind} size={16} /> : <Icon name="weather" size={16} />}
          <b className="dock-glyph-value">{temp}</b>
        </span>
      );
      label = weather
        ? `${temp}, ${weatherLabelShort(weather.weatherCode)}`
        : s.weatherEnabled ? 'No weather data' : 'Weather location not set';
      off = !weather;
      break;
    }
    case 'system': {
      const cpu = (source as SystemData | undefined)?.cpu;
      glyph = (
        <Ring value={cpu}>
          <b className="dock-glyph-value">{cpu == null ? '--' : Math.round(cpu)}</b>
        </Ring>
      );
      label = cpu == null ? 'CPU unavailable' : `CPU ${Math.round(cpu)}%`;
      off = cpu == null;
      break;
    }
    case 'claude':
    case 'chatgpt': {
      const s = settingsFor(id, settings);
      const usage = source as ClaudeData | ChatGPTData | undefined;
      const live = usage?.status === 'live';
      glyph = live ? (
        <Ring value={usage?.sessionPercent} warnAt={s.warnAt}>
          <Icon name={id} size={14} />
        </Ring>
      ) : (
        <Icon name={id} size={22} />
      );
      label = live ? `Session ${pct(usage?.sessionPercent)}` : 'Not connected';
      off = !live;
      break;
    }
    case 'agent': {
      const agent = source as AgentData | undefined;
      const state = agent?.status === 'live' ? 'running' : agent?.status === 'empty' ? 'idle' : 'offline';
      glyph = (
        <>
          <Icon name="agent" size={22} />
          <StatusDot state={state} />
        </>
      );
      label = `Agent ${state}`;
      off = state === 'offline';
      break;
    }
    case 'files': {
      const recent = (source as FilesData | undefined)?.recent;
      label = recent ? `${recent.length} recent files` : 'Recent files unavailable';
      break;
    }
    case 'trash': {
      const count = (source as TrashData | undefined)?.itemCount;
      glyph = (
        <>
          <Icon name="trash" size={22} />
          {count != null && count > 0 ? <span className="dock-badge">{count > 99 ? '99+' : count}</span> : null}
        </>
      );
      label = count == null ? 'Trash unavailable' : `${count} items in Trash`;
      break;
    }
    case 'volume': {
      const sound = source as VolumeData | undefined;
      const level = sound?.level;
      glyph = (
        <>
          <Icon name={sound?.muted ? 'volumeMute' : 'volume'} size={20} />
          {level != null && !sound?.muted ? (
            <span className="dock-level" aria-hidden="true">
              <span style={{ width: `${Math.round(level)}%` }} />
            </span>
          ) : null}
        </>
      );
      label = level == null ? 'Volume unavailable' : sound?.muted ? 'Muted' : `Volume ${Math.round(level)}%`;
      off = level == null;
      break;
    }
    case 'clock': {
      const s = settingsFor('clock', settings);
      const clock = source as ClockData | undefined;
      const parts = clockParts(clock?.now, s.hour12, s.showSeconds, clock?.timezone);
      glyph = <ClockFace now={clock?.now ?? Date.now()} seconds={s.showSeconds} />;
      label = parts.suffix ? `${parts.time} ${parts.suffix}` : parts.time;
      break;
    }
  }

  return (
    <span
      className={`dock-tile fw-tint-${meta.tint}${off ? ' is-off' : ''}`}
      role="img"
      aria-label={label}
      data-widget={id}
      data-preview={preview ? 'true' : undefined}
    >
      {glyph}
    </span>
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
