import type { CSSProperties, ReactNode } from 'react';
import './final-widgets.css';
import type { WidgetStatus } from '../sdk/types';
import { getWidgetMeta, settingsFor } from './meta';
import { sampleFor } from './sample';
import {
  bytes,
  clockParts,
  compactNumber,
  durationMs,
  fileBadge,
  highLow,
  pct,
  relativeTime,
  temperature,
  uptime,
  usageTone,
} from './format';
import { AppIcon, Icon, WeatherIcon, weatherKind, weatherLabel } from './icons';
import {
  AlbumArt,
  AppGlyph,
  CardFooter,
  Chip,
  EmptyState,
  MetricBar,
  NoValue,
  StatusDot,
  WidgetHeader,
} from './primitives';
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
  WidgetCardProps,
} from './types';

type CardProps = WidgetCardProps;
const CARD_TINT: Record<string, string> = {
  blue: '#74b7ff',
  green: '#5fd99a',
  amber: '#f0bd6a',
  violet: '#b49cff',
  teal: '#6fd6cf',
  red: '#f08b8b',
  silver: '#c3c9d1',
};
const SOURCE_LABELS: Record<FinalWidgetId, string> = {
  windows: 'Start menu',
  search: 'Local index',
  apps: 'Running apps',
  spotify: 'Media session',
  weather: 'Weather provider',
  system: 'System provider',
  claude: 'Claude Code',
  chatgpt: 'ChatGPT',
  agent: 'Workspace agent',
  files: 'Known folders',
  folders: 'Known folders',
  trash: 'Recycle Bin',
  volume: 'Audio output',
  clock: 'Local time',
};

function styleFor(id: FinalWidgetId): CSSProperties {
  const meta = getWidgetMeta(id);
  return {
    width: meta.cardWidth,
    maxWidth: '100%',
    '--fw-tint': CARD_TINT[meta.tint] ?? '#c3c9d1',
    '--fw-tint-rgb': meta.tintRgb,
  } as CSSProperties;
}

function sourceFoot(id: FinalWidgetId, preview: boolean, detail?: string): string {
  return `${SOURCE_LABELS[id]}${preview ? ' · sample data' : detail ? ` · ${detail}` : ''}`;
}

function statusFor(preview: boolean, status: WidgetStatus | undefined, aiStatus?: string): WidgetStatus | undefined {
  if (preview) return undefined;
  if (aiStatus === 'disconnected') return 'disconnected';
  if (aiStatus === 'empty') return 'empty';
  return status;
}

function AppTile({ title, count, showTitles }: { title: string; count?: number | null; showTitles: boolean }) {
  return (
    <div className="fw-app-tile">
      <span className="fw-app-tile-icon">
        <AppGlyph title={title} size={24} />
      </span>
      {showTitles ? <span className="fw-app-tile-name">{title}</span> : null}
      {count != null ? <span className="fw-app-tile-count">{count}</span> : null}
    </div>
  );
}

function ConnectionRow({
  icon,
  label,
  value,
  state = 'unknown',
}: {
  icon: 'wifi' | 'bluetooth' | 'ethernet' | 'vpn';
  label: string;
  value: ReactNode;
  state?: 'on' | 'off' | 'unknown';
}) {
  return (
    <div className={`fw-conn fw-conn--${state}`}>
      <span className="fw-conn-icon">
        <Icon name={icon} size={13} />
      </span>
      <span className="fw-conn-label">{label}</span>
      <span className="fw-conn-value">{value}</span>
    </div>
  );
}

function WifiBars({ value }: { value: number | null | undefined }) {
  const level = value == null ? 0 : value >= 75 ? 3 : value >= 45 ? 2 : value > 0 ? 1 : 0;
  return (
    <span className={`fw-wifi-bars fw-wifi-bars--${level}`} aria-hidden="true">
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}

function AiShell({
  id,
  data,
  settings,
  preview,
  status,
  onAction,
}: {
  id: 'claude' | 'chatgpt';
  data: ClaudeData | ChatGPTData | undefined;
  settings: { plan: string; showSession: boolean; showWeekly: boolean; showReset: boolean; warnAt: number };
  preview: boolean;
  status?: WidgetStatus;
  onAction?: WidgetActionHandler;
}) {
  const connected = data != null && data.status !== 'disconnected' && data.status !== 'empty';
  const effective = statusFor(preview, status, data?.status);
  if (!connected && data?.status !== 'empty') {
    return (
      <>
        <WidgetHeader id={id} status={effective} preview={preview} />
        <div className="fw-card-body">
          <EmptyState
            title="Connect local agent"
            detail={`Sign in to ${id === 'claude' ? 'Claude Code' : 'ChatGPT'} locally to read this session’s usage. No quota data is assumed.`}
            action={
              onAction ? (
                <button type="button" className="fw-btn fw-btn--quiet" onClick={() => onAction('settings:network')}>
                  Connection settings
                </button>
              ) : null
            }
          />
        </div>
        <CardFooter left={sourceFoot(id, preview, 'not connected')} right="No quota data" />
      </>
    );
  }
  if (data?.status === 'empty') {
    return (
      <>
        <WidgetHeader id={id} status="empty" preview={preview} />
        <div className="fw-card-body">
          <EmptyState title="No local session" detail="The local agent has no usage window yet. Start a session to populate this card." />
        </div>
        <CardFooter left={sourceFoot(id, preview, 'empty')} right="Waiting for a session" />
      </>
    );
  }
  const session = data?.sessionPercent ?? null;
  const weekly = data?.weeklyPercent ?? null;
  const sessionTone = usageTone(session, settings.warnAt);
  const weeklyTone = usageTone(weekly, settings.warnAt);
  const chat = id === 'chatgpt' ? (data as ChatGPTData | undefined) : undefined;
  return (
    <>
      <WidgetHeader
        id={id}
        status={effective}
        preview={preview}
        aside={<Chip>{settings.plan || 'Local plan'}</Chip>}
      />
      <div className="fw-card-body fw-ai">
        <div className="fw-ai-head">
          <div>
            <span className="fw-eyebrow">{id === 'claude' ? 'Current plan' : 'Plan'}</span>
            <strong className="fw-ai-plan">{settings.plan || 'Local plan'}</strong>
          </div>
          <span className="fw-ai-model">{data?.model ?? 'Model not reported'}</span>
        </div>
        {settings.showSession ? (
          <div className={`fw-use-block is-${sessionTone}`}>
            <div className="fw-use-top">
              <span>{id === 'claude' ? 'Current session' : '5-hour window'}</span>
              <strong>{pct(session)}</strong>
            </div>
            <MetricBar value={session} warnAt={settings.warnAt} label={`${id} session usage`} tone={id === 'claude' ? 'violet' : 'green'} />
            <div className="fw-use-meta">
              <span>{settings.showReset && data?.resetIn ? `Resets in ${data.resetIn}` : 'Session window'}</span>
              <span>{data?.tokensUsed != null && data?.tokensLimit != null ? `${compactNumber(data.tokensUsed)} / ${compactNumber(data.tokensLimit)}` : 'Tokens --'}</span>
            </div>
          </div>
        ) : null}
        {settings.showWeekly ? (
          <div className={`fw-use-block is-${weeklyTone}`}>
            <div className="fw-use-top">
              <span>Weekly window</span>
              <strong>{pct(weekly)}</strong>
            </div>
            <MetricBar value={weekly} warnAt={settings.warnAt} label={`${id} weekly usage`} tone="blue" />
            <div className="fw-use-meta">
              <span>{settings.showReset && data?.weeklyResetIn ? `Resets in ${data.weeklyResetIn}` : '7-day window'}</span>
              <span>{id === 'chatgpt' ? 'Weekly limit' : 'Sample window'}</span>
            </div>
          </div>
        ) : null}
        {id === 'chatgpt' ? (
          <div className="fw-use-block is-credits">
            <div className="fw-use-top">
              <span>Credits</span>
              <strong>{pct(chat?.creditsPercent ?? null)}</strong>
            </div>
            <MetricBar value={chat?.creditsPercent ?? null} warnAt={settings.warnAt} label="ChatGPT credits" tone="green" />
            <div className="fw-use-meta">
              <span>{chat?.creditsUsed != null && chat?.creditsLimit != null ? `${chat.creditsUsed} / ${chat.creditsLimit} used` : 'Usage --'}</span>
              <span>Credit balance</span>
            </div>
          </div>
        ) : null}
      </div>
      <CardFooter
        left={sourceFoot(id, preview, data?.status === 'live' ? 'local session' : undefined)}
        right={data?.status === 'live' ? 'Session data' : 'Waiting for local data'}
      />
    </>
  );
}

export function WidgetCard({ id, settings, data, preview = false, status, onAction }: CardProps) {
  const meta = getWidgetMeta(id);
  const source = (preview ? sampleFor(id) : data) as unknown;

  let body: ReactNode;

  switch (id) {
    case 'windows': {
      const s = settingsFor('windows', settings);
      const win = source as WindowsData | undefined;
      const empty = !win;
      const pinned = win?.pinnedApps ?? [];
      const recent = win?.recentApps ?? [];
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            <div className="fw-windows-hero">
              <span className="fw-windows-logo">
                <Icon name="windows" size={30} />
              </span>
              <div>
                <span className="fw-eyebrow">Windows</span>
                <strong className="fw-windows-title">{s.showLabel ? 'Start' : 'Windows'}</strong>
              </div>
              <Chip tone="accent">Pinned</Chip>
            </div>
            {empty ? (
              <EmptyState title="Start is local" detail="Pinned and recent apps appear here when Windows exposes them." />
            ) : (
              <>
                <div className="fw-split-label">
                  <span>Pinned</span>
                  <span>{pinned.length} apps</span>
                </div>
                <div className="fw-app-row">
                  {pinned.length ? pinned.slice(0, 6).map((title) => <AppTile key={title} title={title} showTitles />) : <NoValue label="No pinned apps" />}
                </div>
                {s.showRecentHint ? (
                  <div className="fw-recent-hint">
                    <Icon name="clock" size={12} />
                    <span>{recent.length ? `Recent: ${recent.slice(0, 3).join(' · ')}` : 'Recent apps appear here'}</span>
                  </div>
                ) : null}
              </>
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={onAction ? <button type="button" className="fw-link" onClick={() => onAction('start')}>Open Start</button> : 'Start menu'} />
        </>
      );
      break;
    }
    case 'search': {
      const s = settingsFor('search', settings);
      const searchData = source as SearchData | undefined;
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            <button type="button" className="fw-search-hero" onClick={() => onAction?.('search')} disabled={!onAction}>
              <Icon name="search" size={17} />
              <span>{s.showHint ? 'Search apps, files, folders' : 'Search'}</span>
              {s.showShortcut ? <span className="fw-search-hero-kbd">Alt + Space</span> : null}
            </button>
            <div className="fw-scope-row">
              {['Apps', 'Files', 'Folders', 'OCR'].map((scope) => <Chip key={scope}>{scope}</Chip>)}
            </div>
            {searchData ? (
              <div className="fw-search-meta">
                <span><Icon name="folder" size={11} /> {searchData.indexReady ? 'Index ready' : 'Index unavailable'}</span>
                <span>{compactNumber(searchData.indexedFiles)} files · {searchData.lastIndexed ?? 'Not indexed'}</span>
              </div>
            ) : (
              <EmptyState title="Local index is empty" detail="Results appear when the native index is ready." />
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={searchData?.indexReady ? 'Local index' : 'Index offline'} />
        </>
      );
      break;
    }
    case 'apps': {
      const s = settingsFor('apps', settings);
      const appsData = source as AppsData | undefined;
      const items = appsData?.items ?? [];
      const active = appsData?.activeTitle ?? items.find((item) => item.active)?.title ?? null;
      const scoped = s.activeOnly ? items.filter((item) => item.active).slice(0, 1) : items;
      const visible = scoped.slice(0, s.maxApps);
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            {appsData && items.length ? (
              <>
                <div className="fw-apps-active">
                  <span className="fw-apps-active-icon"><AppGlyph title={active ?? 'App'} size={24} /></span>
                  <div>
                    <span className="fw-eyebrow">{active ? 'Active window' : 'No active window'}</span>
                    <strong>{active ?? 'Windows is idle'}</strong>
                    <small>{visible[0]?.title ?? 'No app'} · {appsData.windowCount ?? items.length} windows · {bytes(appsData.memoryBytes)}</small>
                  </div>
                  <span className="fw-pulse-dot" aria-label="Running" />
                </div>
                <div className="fw-split-label"><span>{s.activeOnly ? 'Active app' : 'Grouped apps'}</span><span>{scoped.length} open</span></div>
                <div className="fw-app-row">
                  {visible.map((item) => <AppTile key={item.id} title={item.title} showTitles={s.showTitles} count={item.active ? 1 : null} />)}
                </div>
              </>
            ) : (
              <EmptyState title="No running apps" detail="The running apps provider has not reported any windows." />
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={appsData ? `${items.length} apps · ${appsData.windowCount ?? items.length} windows` : 'Not available'} />
        </>
      );
      break;
    }
    case 'spotify': {
      const s = settingsFor('spotify', settings);
      const media = source as MediaData | undefined;
      const hasTrack = !!media?.title;
      const position = media?.positionMs ?? 0;
      const duration = media?.durationMs ?? 0;
      const progress = duration > 0 ? Math.min(100, Math.max(0, (position / duration) * 100)) : 0;
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body fw-spotify">
            <div className="fw-sp-hero">
              <AlbumArt title={media?.album ?? media?.title} size={96} />
              <div className="fw-sp-copy">
                <span className="fw-sp-now"><span className={`fw-dot fw-dot--${media?.playing ? 'running' : 'idle'}`} /> {media?.playing ? 'NOW PLAYING' : 'PAUSED'}</span>
                <strong className="fw-sp-track">{media?.title ?? 'Nothing playing'}</strong>
                <span className="fw-sp-artist">{media?.artist ?? 'Connect a media session'}</span>
                <span className="fw-sp-album">{media?.album ?? 'Spotify'}{media?.albumArtist ? ` · ${media.albumArtist}` : ''}</span>
              </div>
            </div>
            {s.showProgress && hasTrack ? (
              <div className={`fw-sp-progress${s.compactProgress ? ' is-compact' : ''}`}>
                <span className="fw-sp-track-line"><span style={{ width: `${progress}%` }} /></span>
                <div className="fw-sp-times"><span>{durationMs(media?.positionMs)}</span><span>{durationMs(media?.durationMs)}</span></div>
              </div>
            ) : null}
            {s.showControls ? (
              <div className="fw-sp-controls" aria-label="Spotify transport controls">
                <button type="button" className="fw-round-btn" onClick={() => onAction?.('media:previous')} disabled={!onAction || !hasTrack || media?.canPrevious === false} aria-label="Previous track">
                  <Icon name="prev" size={15} />
                </button>
                <button type="button" className="fw-round-btn fw-round-btn--primary" onClick={() => onAction?.('media:toggle')} disabled={!onAction || !hasTrack || media?.canToggle === false} aria-label={media?.playing ? 'Pause' : 'Play'}>
                  <Icon name={media?.playing ? 'pause' : 'play'} size={16} />
                </button>
                <button type="button" className="fw-round-btn" onClick={() => onAction?.('media:next')} disabled={!onAction || !hasTrack || media?.canNext === false} aria-label="Next track">
                  <Icon name="next" size={15} />
                </button>
              </div>
            ) : null}
            {s.showDevice ? (
              <div className="fw-sp-device">
                <span><Icon name="device" size={13} /> {media?.device ?? 'No output device'}</span>
                <span className="fw-sp-device-state">{media?.source ?? 'Spotify'}</span>
              </div>
            ) : null}
            {s.showVolume ? (
              <div className="fw-sp-volume">
                <span className="fw-vol-ic"><Icon name={media?.volume === 0 ? 'volumeMute' : 'volume'} size={14} /></span>
                <input
                  className="fw-range fw-range--spotify"
                  type="range"
                  min={0}
                  max={100}
                  step={1}
                  value={media?.volume ?? 0}
                  disabled={!onAction || media?.volume == null}
                  aria-label="Spotify volume"
                  onChange={(event) => onAction?.(`volume:set:${Number(event.target.value)}`)}
                />
                <span className="fw-sp-volume-value">{media?.volume == null ? '--' : Math.round(media.volume)}</span>
              </div>
            ) : null}
            {!hasTrack ? <EmptyState title="Nothing playing" detail="Open Spotify or another media app to populate controls and artwork." /> : null}
          </div>
          <CardFooter left={sourceFoot(id, preview, media?.source ? media.source : undefined)} right={media ? (preview ? 'Sample track' : 'Media session') : 'No media'} />
        </>
      );
      break;
    }
    case 'weather': {
      const s = settingsFor('weather', settings);
      const weather = source as WeatherData | undefined;
      const units = s.units;
      const hours = (weather?.hours ?? []).slice(0, s.forecastHours);
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            {weather ? (
              <>
                <div className="fw-wx-head">
                  <span className="fw-wx-icon"><WeatherIcon kind={weatherKind(weather.weatherCode)} size={34} /></span>
                  <div className="fw-wx-temp">{temperature(weather.temperature, units)}</div>
                  <div className="fw-wx-copy">
                    <strong>{weatherLabel(weather.weatherCode)}</strong>
                    <span>{highLow(weather.highC, weather.lowC, units)}</span>
                    <span className="fw-wx-location"><Icon name="eye" size={11} /> {s.location}</span>
                  </div>
                </div>
                {s.showDetails ? (
                  <div className="fw-wx-details">
                    <span><small>Feels like</small><strong>{temperature(weather.feelsLikeC, units)}</strong></span>
                    <span><small>Wind</small><strong>{weather.windKph == null ? '--' : `${Math.round(weather.windKph)} km/h`}</strong></span>
                    <span><small>Humidity</small><strong>{pct(weather.humidityPercent)}</strong></span>
                    <span><small>UV</small><strong>{weather.uvIndex == null ? '--' : Math.round(weather.uvIndex)}</strong></span>
                    <span><small>Visibility</small><strong>{weather.visibilityKm == null ? '--' : `${Math.round(weather.visibilityKm)} km`}</strong></span>
                  </div>
                ) : null}
                {s.showHourly && hours.length ? (
                  <>
                    <div className="fw-split-label"><span>Next hours</span><span>{hours.length} slots</span></div>
                    <div className="fw-wx-hourly">
                      {hours.map((hour) => (
                        <span className="fw-wx-hour" key={hour.time}>
                          <small>{new Date(hour.time).toLocaleTimeString('en-US', { hour: 'numeric' })}</small>
                          <WeatherIcon kind={weatherKind(hour.weatherCode)} size={14} />
                          <strong>{temperature(hour.temperature, units)}</strong>
                        </span>
                      ))}
                    </div>
                  </>
                ) : null}
              </>
            ) : (
              <EmptyState title="Weather unavailable" detail="No weather snapshot has been provided. Values are not estimated." />
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={weather ? s.location : 'Not available'} />
        </>
      );
      break;
    }
    case 'system': {
      const s = settingsFor('system', settings);
      const system = source as SystemData | undefined;
      const conn = system?.connectivity;
      const wifi = conn?.wifi;
      const bluetooth = conn?.bluetooth;
      const ethernet = conn?.ethernet;
      const vpn = conn?.vpn;
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            {system ? (
              <>
                <div className="fw-sys-primary">
                  {s.showCpu ? (
                    <div className="fw-sys-metric">
                      <div className="fw-sys-metric-head"><span><Icon name="cpu" size={12} /> CPU</span><strong>{pct(system.cpu)}</strong></div>
                      <MetricBar value={system.cpu} label="CPU load" tone="blue" />
                    </div>
                  ) : null}
                  {s.showRam ? (
                    <div className="fw-sys-metric">
                      <div className="fw-sys-metric-head"><span><Icon name="ram" size={12} /> RAM</span><strong>{pct(system.ram)}</strong></div>
                      <MetricBar value={system.ram} label="RAM load" tone="blue" />
                    </div>
                  ) : null}
                  {s.showBattery ? (
                    <div className="fw-sys-metric">
                      <div className="fw-sys-metric-head"><span><Icon name="battery" size={12} /> Battery</span><strong>{pct(system.battery)}{system.batteryCharging ? <i className="fw-charge">chg</i> : null}</strong></div>
                      <MetricBar value={system.battery} label="Battery level" tone="green" />
                    </div>
                  ) : null}
                </div>
                {s.showTemp ? (
                  <div className="fw-sys-temp">
                    <span><Icon name="temp" size={13} /> Package temperature</span>
                    <strong>{system.temperatureC == null ? '--' : `${Math.round(system.temperatureC)}°C`}</strong>
                    <span className="fw-sys-temp-note">{system.temperatureC == null ? 'Not available' : 'Local sensor'}</span>
                  </div>
                ) : null}
                <div className="fw-conn-grid">
                  {s.showWifi ? (
                    <ConnectionRow
                      icon="wifi"
                      label="Wi-Fi"
                      state={wifi?.connected === true ? 'on' : wifi?.connected === false ? 'off' : 'unknown'}
                      value={wifi?.connected === true ? <><span>{wifi.ssid ?? 'Connected'}</span><span className="fw-conn-detail">{pct(wifi.signalPercent)}</span><WifiBars value={wifi.signalPercent} /></> : wifi?.connected === false ? 'Off' : '--'}
                    />
                  ) : null}
                  {s.showBluetooth ? (
                    <ConnectionRow
                      icon="bluetooth"
                      label="Bluetooth"
                      state={bluetooth?.enabled === true ? 'on' : bluetooth?.enabled === false ? 'off' : 'unknown'}
                      value={bluetooth?.enabled === true ? bluetooth.name ?? 'On' : bluetooth?.enabled === false ? 'Off' : '--'}
                    />
                  ) : null}
                  {s.showNetwork ? (
                    <>
                      <ConnectionRow icon="ethernet" label="Ethernet" state={ethernet?.connected === true ? 'on' : ethernet?.connected === false ? 'off' : 'unknown'} value={ethernet?.connected === true ? ethernet.adapter ?? 'Connected' : ethernet?.connected === false ? 'Offline' : '--'} />
                      <ConnectionRow icon="vpn" label="VPN" state={vpn?.connected === true ? 'on' : vpn?.connected === false ? 'off' : 'unknown'} value={vpn?.connected === true ? vpn.adapter ?? 'Connected' : vpn?.connected === false ? 'Off' : '--'} />
                    </>
                  ) : null}
                  {s.showUptime ? (
                    <div className="fw-sys-uptime"><span>Uptime</span><strong>{uptime(system.uptimeSeconds)}</strong></div>
                  ) : null}
                </div>
              </>
            ) : (
              <EmptyState title="System data unavailable" detail="No provider snapshot. Unknown values stay unknown, never zero." />
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={system ? `Temp ${system.temperatureC == null ? '--' : `${Math.round(system.temperatureC)}°C`}` : 'Not available'} />
        </>
      );
      break;
    }
    case 'claude':
    case 'chatgpt': {
      const aiData = source as ClaudeData | ChatGPTData | undefined;
      const aiSettings = settingsFor(id, settings);
      body = <AiShell id={id} data={aiData} settings={aiSettings} preview={preview} status={status} onAction={onAction} />;
      break;
    }
    case 'agent': {
      const s = settingsFor('agent', settings);
      const agent = source as AgentData | undefined;
      const running = agent?.status === 'live';
      const empty = agent?.status === 'disconnected' || agent?.status === 'empty' || !agent;
      body = (
        <>
          <WidgetHeader
            id={id}
            status={statusFor(preview, status, agent?.status)}
            preview={preview}
            aside={<Chip>{running ? 'running' : agent?.status === 'empty' ? 'idle' : 'offline'}</Chip>}
          />
          <div className="fw-card-body">
            {empty ? (
              <EmptyState
                title={agent?.status === 'disconnected' ? 'Connect local agent' : 'No workspace agent'}
                detail="Start Claude Code in a workspace to see its task, branch and session metrics."
              />
            ) : (
              <>
                <div className="fw-agent-head">
                  <span className="fw-agent-workspace-icon"><Icon name="terminal" size={16} /></span>
                  <div>
                    <strong>{s.workspace || agent?.workspace || 'Workspace'}</strong>
                    <span><StatusDot state={running ? 'running' : 'idle'} /> {running ? 'Agent running' : 'Agent idle'}</span>
                  </div>
                  {agent?.branch ? <Chip>{agent.branch}</Chip> : null}
                </div>
                <div className="fw-agent-task">
                  <Icon name="spark" size={13} />
                  <span>{agent?.task ?? 'Waiting for a task'}</span>
                </div>
                <div className="fw-agent-grid">
                  {s.showTokens ? <div className="fw-agent-cell"><small>Tokens</small><strong>{agent?.tokens ?? '--'}</strong></div> : null}
                  {s.showFiles ? <div className="fw-agent-cell"><small>Files</small><strong>{agent?.filesChanged == null ? '--' : agent.filesChanged}</strong></div> : null}
                  {s.showToolCalls ? <div className="fw-agent-cell"><small>Tool calls</small><strong>{agent?.toolCalls == null ? '--' : agent.toolCalls}</strong></div> : null}
                  {s.showSession ? <div className="fw-agent-cell"><small>Session</small><strong>{agent?.duration ?? '--'}</strong></div> : null}
                </div>
                {agent?.branch ? <div className="fw-agent-branch"><Icon name="branch" size={12} /> {agent.branch}</div> : null}
              </>
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={agent?.status === 'live' ? 'Session active' : 'No session'} />
        </>
      );
      break;
    }
    case 'files': {
      const s = settingsFor('files', settings);
      const fileData = source as FilesData | undefined;
      const entries = (fileData?.recent ?? []).slice(0, s.maxItems);
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            <div className="fw-split-label"><span>{s.source}</span><span>{entries.length} of {fileData?.recent?.length ?? '--'}</span></div>
            {entries.length ? (
              <div className="fw-files">
                {entries.map((file) => (
                  <button type="button" className="fw-file-row" key={file.path} onClick={() => onAction?.(`folder:open:${s.source.toLowerCase()}`)} disabled={!onAction}>
                    {s.showType ? <span className="fw-file-badge">{fileBadge(file.kind)}</span> : <Icon name="doc" size={15} />}
                    <span className="fw-file-main"><strong>{file.name}</strong><small>{file.path}</small></span>
                    <span className="fw-file-meta"><span>{relativeTime(file.modifiedAt)}</span><small>{bytes(file.sizeBytes)}</small></span>
                  </button>
                ))}
              </div>
            ) : (
              <EmptyState title="No recent files" detail="Files you edit will appear here once the storage provider reports them." />
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview, s.source)} right={entries.length ? `${entries.length} files` : 'No files'} />
        </>
      );
      break;
    }
    case 'folders': {
      const s = settingsFor('folders', settings);
      const folderData = source as FoldersData | undefined;
      const folders = (folderData?.folders ?? []).slice(0, s.maxItems);
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            {folders.length ? (
              <div className="fw-folders">
                {folders.map((folder) => (
                  <button type="button" className="fw-folder-row" key={folder.id} onClick={() => onAction?.(`folder:open:${folder.id}`)} disabled={!onAction}>
                    <span className="fw-folder-icon"><Icon name="folder" size={16} /></span>
                    <span className="fw-folder-name">{folder.name}</span>
                    {s.showCounts ? <span className="fw-folder-count">{folder.itemCount == null ? '--' : folder.itemCount}</span> : null}
                  </button>
                ))}
              </div>
            ) : (
              <EmptyState title="No known folders" detail="Desktop, Documents and Downloads appear when the storage provider resolves them." />
            )}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={folders.length ? `${folders.length} locations` : 'Not available'} />
        </>
      );
      break;
    }
    case 'trash': {
      const s = settingsFor('trash', settings);
      const trash = source as TrashData | undefined;
      const isEmpty = trash?.itemCount === 0;
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            <div className="fw-trash-hero">
              <span className="fw-trash-icon"><Icon name="trash" size={30} /></span>
              <div>
                <strong className="fw-trash-count">{trash?.itemCount == null ? '--' : trash.itemCount}</strong>
                <span className="fw-trash-copy">{isEmpty ? 'Trash is empty' : 'items in Recycle Bin'}</span>
              </div>
              {s.showSize ? <strong className="fw-trash-size">{bytes(trash?.sizeBytes)}</strong> : null}
            </div>
            {isEmpty ? (
              <EmptyState title="Nothing to clear" detail="Deleted files will appear here with their reclaimable size." />
            ) : (
              <button type="button" className="fw-empty-btn" onClick={() => onAction?.('trash:empty')} disabled={!onAction || !trash || trash.itemCount == null}>
                <Icon name="trash" size={13} /> Empty Recycle Bin
              </button>
            )}
            {s.showLastEmptied ? (
              <div className="fw-trash-last"><span>Last emptied</span><strong>{isEmpty ? 'Just now' : trash?.lastEmptiedAt ? relativeTime(trash.lastEmptiedAt) : '--'}</strong></div>
            ) : null}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={trash ? 'Review before deleting' : 'Not available'} />
        </>
      );
      break;
    }
    case 'volume': {
      const s = settingsFor('volume', settings);
      const sound = source as VolumeData | undefined;
      const level = sound?.level;
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            <div className="fw-vol-hero">
              <span className="fw-vol-icon"><Icon name={sound?.muted ? 'volumeMute' : 'volume'} size={22} /></span>
              <strong className="fw-vol-read">{level == null ? '--' : sound?.muted ? 'Mute' : `${Math.round(level)}%`}</strong>
              {s.showMute ? (
                <button type="button" className="fw-mute-btn" onClick={() => onAction?.('volume:mute')} disabled={!onAction} aria-label={sound?.muted ? 'Unmute' : 'Mute'}>
                  <Icon name={sound?.muted ? 'volumeMute' : 'volume'} size={15} />
                </button>
              ) : null}
            </div>
            {s.showSlider ? (
              level == null ? <NoValue label="Level not available" /> : (
                <div className="fw-vol-slider-row">
                  <button type="button" className="fw-mini-btn" aria-label="Volume down" onClick={() => onAction?.('volume:down')} disabled={!onAction}><span>-</span></button>
                  <input
                    className="fw-range fw-range--volume"
                    type="range"
                    min={0}
                    max={100}
                    value={level}
                    disabled={!onAction}
                    aria-label="Volume level"
                    onChange={(event) => onAction?.(`volume:set:${Number(event.target.value)}`)}
                  />
                  <button type="button" className="fw-mini-btn" aria-label="Volume up" onClick={() => onAction?.('volume:up')} disabled={!onAction}><span>+</span></button>
                </div>
              )
            ) : null}
            {s.showDevice ? (
              <div className="fw-vol-device">
                <span><Icon name="device" size={13} /> Output</span>
                <strong>{sound?.device ?? 'No output device'}</strong>
                <Chip>Default</Chip>
              </div>
            ) : null}
            <div className="fw-vol-media">
              <span><Icon name="music" size={12} /> {sound?.mediaApp ?? 'No media app'}</span>
              <span>{sound ? 'Output active' : 'Not available'}</span>
            </div>
            {!sound ? <EmptyState title="Audio unavailable" detail="No audio provider snapshot. Level and device stay unknown." /> : null}
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={sound ? 'Output active' : 'Not available'} />
        </>
      );
      break;
    }
    case 'clock': {
      const s = settingsFor('clock', settings);
      const clock = source as ClockData | undefined;
      const parts = clockParts(clock?.now, s.hour12, s.showSeconds, clock?.timezone);
      const date = new Date(clock?.now ?? Date.now());
      const seconds = date.getSeconds();
      const minute = date.getMinutes() + seconds / 60;
      const hour = (date.getHours() % 12) + minute / 60;
      body = (
        <>
          <WidgetHeader id={id} status={statusFor(preview, status)} preview={preview} />
          <div className="fw-card-body">
            <div className="fw-clock-hero">
              <div className="fw-clock-copy">
                <strong className="fw-clock-time">{parts.time}{parts.suffix ? <small>{parts.suffix}</small> : null}</strong>
                {s.showDate ? <span className="fw-clock-date">{parts.date}</span> : null}
                {s.showTimezone ? <span className="fw-clock-tz">{parts.timezone}</span> : null}
              </div>
              <span className="fw-clock-dial" aria-hidden="true">
                <span className="fw-clock-face" />
                <span className="fw-clock-tick fw-clock-tick--12" />
                <span className="fw-clock-tick fw-clock-tick--3" />
                <span className="fw-clock-tick fw-clock-tick--6" />
                <span className="fw-clock-tick fw-clock-tick--9" />
                <span className="fw-clock-hand fw-clock-hand--hour" style={{ transform: `rotate(${hour * 30}deg)` }} />
                <span className="fw-clock-hand fw-clock-hand--minute" style={{ transform: `rotate(${minute * 6}deg)` }} />
                {s.showSeconds ? <span className="fw-clock-hand fw-clock-hand--second" style={{ transform: `rotate(${seconds * 6}deg)` }} /> : null}
              </span>
            </div>
            <div className="fw-clock-next">
              <span className="fw-clock-next-icon"><Icon name="calendar" size={13} /></span>
              <div><small>NEXT EVENT</small><strong>{clock?.nextEvent?.title ?? 'No event linked'}</strong></div>
              <span>{clock?.nextEvent ? String(clock.nextEvent.startsAt) : '--'}</span>
            </div>
          </div>
          <CardFooter left={sourceFoot(id, preview)} right={clock?.nextEvent ? 'Next event' : 'No calendar'} />
        </>
      );
      break;
    }
    default:
      body = null;
  }

  return (
    <section className={`fw-card fw-tint-${meta.tint} fw-card--${id}`} style={styleFor(id)} aria-label={`${meta.title} widget card`} data-widget-card={id} data-preview={preview ? 'true' : undefined}>
      {body}
    </section>
  );
}

export { WifiBars };
