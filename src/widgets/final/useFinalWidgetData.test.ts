// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import {
  isPreviewMode,
  mapAgent,
  mapChatGPT,
  mapClaude,
  mapClock,
  mapKnownFolders,
  mapMedia,
  mapRunningApps,
  mapSystem,
  mapTrash,
  mapVolume,
  mapWeather,
  mapWindows,
} from './useFinalWidgetData';

const live = (data: unknown) => ({ status: 'live' as const, data, updatedAt: 1 });

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe('final widget data mapping', () => {
  it('keeps missing and malformed system fields null instead of inventing values', () => {
    const data = mapSystem(live({
      cpu: 'fast',
      ram: 42,
      battery: null,
      batteryCharging: 'yes',
      temperatureC: undefined,
      uptimeSeconds: 3600,
      connectivity: { wifi: { connected: true, ssid: '', signalPercent: 'strong' }, vpn: { connected: false } },
    }));
    expect(data).toMatchObject({
      cpu: null,
      ram: 42,
      battery: null,
      batteryCharging: null,
      temperatureC: null,
      uptimeSeconds: 3600,
    });
    expect(data?.connectivity).toMatchObject({
      wifi: { connected: true, ssid: null, signalPercent: null, state: null },
      ethernet: null,
      vpn: { connected: false, adapter: null },
      bluetooth: null,
    });
    expect(mapSystem(live(null))).toBeUndefined();
  });

  it('merges volume data into media only when media lacks its own values', () => {
    const media = mapMedia(
      live({ title: 'Track', playing: 'yes', volume: null, device: null }),
      live({ level: 68, muted: false, device: 'Speakers (Realtek)' }),
    );
    expect(media).toMatchObject({
      title: 'Track',
      playing: null,
      volume: 68,
      device: 'Speakers (Realtek)',
    });
    expect(mapMedia(live(null))).toBeUndefined();
  });

  it('maps running apps, ignoring malformed entries and reporting absent metrics as null', () => {
    const data = mapRunningApps(live({
      items: [
        { id: 'a', title: 'VS Code', active: true },
        { id: 2, title: 'bad' },
        { id: 'b', title: 'Slack' },
      ],
    }));
    expect(data?.items).toEqual([
      { id: 'a', title: 'VS Code', icon: null, active: true },
      { id: 'b', title: 'Slack', icon: null, active: false },
    ]);
    expect(data?.activeTitle).toBe('VS Code');
    expect(data?.windowCount).toBeNull();
    expect(data?.memoryBytes).toBeNull();
    expect(mapRunningApps(live({ items: 'none' }))).toBeUndefined();
  });

  it('maps running app titles into the Windows recent list without inventing pinned apps', () => {
    const data = mapWindows(live({
      items: [
        { id: 'a', title: 'VS Code', active: true },
        { id: 'b', title: 'Slack' },
      ],
    }));
    expect(data).toEqual({ pinnedApps: null, recentApps: ['VS Code', 'Slack'] });
    expect(mapWindows(live(null))).toBeUndefined();
  });

  it('maps known folders, converting epoch modifiedAt to ISO and leaving counts null', () => {
    const data = mapKnownFolders(live({
      folders: [{ id: 'projects', name: 'Projects', path: 'C:\\Projects', icon: 'folder' }],
      recent: [{ name: 'main.ts', path: 'C:\\Projects\\main.ts', kind: 'code', modifiedAt: 1700000000000 }],
    }));
    expect(data?.folders).toEqual([
      { id: 'projects', name: 'Projects', path: 'C:\\Projects', icon: 'folder', itemCount: null },
    ]);
    expect(data?.recent?.[0]).toMatchObject({
      name: 'main.ts',
      kind: 'code',
      modifiedAt: new Date(1700000000000).toISOString(),
    });
    expect(data?.recent?.[0].sizeBytes).toBeNull();
  });

  it('maps weather hourlies and nulls unavailable detail fields', () => {
    const data = mapWeather(live({
      temperature: 24,
      weatherCode: 2,
      time: 1700000000000,
      utcOffsetSeconds: 19800,
      hours: [{ time: 1, temperature: 23, weatherCode: 2 }, { time: 'bad' }],
    }));
    expect(data).toMatchObject({
      temperature: 24,
      weatherCode: 2,
      hours: [{ time: 1, temperature: 23, weatherCode: 2 }],
      highC: null,
      lowC: null,
      feelsLikeC: null,
      windKph: null,
      humidityPercent: null,
      uvIndex: null,
      visibilityKm: null,
    });
    expect(mapWeather({ status: 'disconnected', updatedAt: 1 })).toBeUndefined();
  });

  it('maps trash, volume and clock without fabricating state', () => {
    expect(mapTrash(live({ itemCount: 18, sizeBytes: 2, lastEmptiedAt: null }))).toEqual({
      itemCount: 18,
      sizeBytes: 2,
      lastEmptiedAt: null,
    });
    expect(mapVolume(live({ level: 68, muted: false, device: 'Speakers' }))).toEqual({
      level: 68,
      muted: false,
      device: 'Speakers',
      mediaApp: null,
    });
    expect(mapClock(123)).toEqual({ now: 123, timezone: null, nextEvent: null });
  });

  it('maps local AI usage without inventing missing limits or agent metrics', () => {
    const snapshot = live({
      chatgpt: {
        status: 'live',
        model: 'gpt-6-luna',
        sessionPercent: 83,
        weeklyPercent: 13,
        resetIn: '4h 12m',
        weeklyResetIn: '6d 3h',
        tokensUsed: 34769,
        tokensLimit: null,
        plan: 'plus',
        creditsPercent: null,
        creditsUsed: null,
        creditsLimit: null,
      },
      claude: { status: 'disconnected' },
      agent: {
        status: 'live',
        workspace: 'C:\\Projects\\demo',
        branch: null,
        task: 'Fix live weather and AI usage',
        duration: '1h 3m',
        tokens: '34.8K',
        filesChanged: null,
        toolCalls: null,
      },
    });
    expect(mapChatGPT(snapshot)).toMatchObject({
      status: 'live',
      sessionPercent: 83,
      weeklyPercent: 13,
      tokensUsed: 34769,
      tokensLimit: null,
      plan: 'plus',
      creditsPercent: null,
    });
    expect(mapClaude(snapshot)).toMatchObject({ status: 'disconnected' });
    expect(mapAgent(snapshot)).toMatchObject({
      status: 'live',
      workspace: 'C:\\Projects\\demo',
      branch: null,
      tokens: '34.8K',
      filesChanged: null,
      toolCalls: null,
    });
    expect(mapAgent(live({ agent: { status: 'idle', workspace: 'C:\\Projects\\demo' } }))).toMatchObject({
      status: 'empty',
      workspace: 'C:\\Projects\\demo',
    });
    expect(mapChatGPT({ status: 'error', updatedAt: 1 })).toBeUndefined();
    expect(mapChatGPT(undefined)).toBeUndefined();
    expect(mapClaude(undefined)).toBeUndefined();
    expect(mapAgent(undefined)).toBeUndefined();
  });

  it('only enables sample preview mode outside the native app', () => {
    expect(isPreviewMode('?preview=1')).toBe(true);
    expect(isPreviewMode('?preview=0')).toBe(false);
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {}, configurable: true });
    expect(isPreviewMode('?preview=1')).toBe(false);
  });
});
