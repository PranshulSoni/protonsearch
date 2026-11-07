// @vitest-environment jsdom
import { useState } from 'react';
import { afterEach, expect, test } from 'vitest';
import { cleanup, fireEvent, render } from '@testing-library/react';
import {
  DEFAULT_DOCK_ORDER,
  DockWidget,
  FINAL_SETTING_FIELDS,
  FINAL_WIDGET_IDS,
  FINAL_WIDGET_META,
  STORE_WIDGET_IDS,
  WidgetCard,
  WidgetSettings,
  settingsFor,
  validateWidgetSettings,
} from './index';
import type { FinalWidgetId, FinalWidgetSettings } from './index';

afterEach(cleanup);

test('metadata covers the 14 widgets, dock order and store set', () => {
  expect(FINAL_WIDGET_IDS).toHaveLength(14);
  expect(DEFAULT_DOCK_ORDER).toEqual([
    'windows',
    'search',
    'apps',
    'spotify',
    'weather',
    'system',
    'claude',
    'agent',
    'clock',
  ]);
  expect(STORE_WIDGET_IDS).toEqual(['chatgpt', 'files', 'folders', 'trash', 'volume']);
  for (const id of FINAL_WIDGET_IDS) {
    const meta = FINAL_WIDGET_META.find((entry) => entry.id === id);
    expect(meta).toBeDefined();
    if (!meta) continue;
    expect(meta.id).toBe(id);
    expect(meta.title.trim().length).toBeGreaterThan(0);
    expect(meta.sizes).toContain(meta.defaultSize);
    expect(meta.widths.compact).toBeLessThanOrEqual(meta.widths.standard);
    expect(meta.widths.standard).toBeLessThanOrEqual(meta.widths.wide);
    expect(meta.defaultWidth).toBe(meta.widths[meta.defaultSize]);
    expect(meta.cardWidth).toBeGreaterThanOrEqual(440);
    expect(meta.cardWidth).toBeLessThanOrEqual(470);
  }
});

test('defaults enable live weather and hide the search shortcut badge', () => {
  const weather = settingsFor('weather');
  expect(weather.weatherEnabled).toBe(true);
  expect(weather.location).toBe('Ahmedabad');
  expect(weather.weatherLatitude).toBeCloseTo(23.0225);
  expect(weather.weatherLongitude).toBeCloseTo(72.5714);
  expect(settingsFor('search').showShortcut).toBe(false);

  const search = render(<DockWidget id="search" />);
  expect(search.queryByText('Alt')).toBeNull();
  search.unmount();
});

test('every dock tile and card renders without emoji or mojibake', () => {
  for (const id of FINAL_WIDGET_IDS) {
    const view = render(<DockWidget id={id} preview />);
    expect(view.container.querySelector(`[data-widget="${id}"]`)).not.toBeNull();
    expect(view.container.textContent ?? '').not.toMatch(/[\u{1F000}-\u{1FAFF}\u{2600}-\u{27BF}]/u);
    expect(view.container.textContent ?? '').not.toMatch(/[\u00C2\u00E2\uFFFD]/);
    view.unmount();
    const card = render(<WidgetCard id={id} preview />);
    expect(card.container.querySelector(`[data-widget-card="${id}"]`)).not.toBeNull();
    expect(card.container.textContent ?? '').not.toMatch(/[\u{1F000}-\u{1FAFF}\u{2600}-\u{27BF}]/u);
    expect(card.container.textContent ?? '').not.toMatch(/[\u00C2\u00E2\uFFFD]/);
    card.unmount();
  }
});

test('sample values only render behind preview', () => {
  const live = render(<WidgetCard id="spotify" />);
  expect(live.queryByText('Tunnel Vision')).toBeNull();
  live.unmount();

  const preview = render(<WidgetCard id="spotify" preview />);
  expect(preview.getByText('Tunnel Vision')).not.toBeNull();
  preview.unmount();
});

test('unknown system data renders honestly', () => {
  const view = render(
    <WidgetCard
      id="system"
      data={{
        cpu: null,
        ram: null,
        battery: null,
        batteryCharging: null,
        temperatureC: null,
        uptimeSeconds: null,
        connectivity: null,
      }}
    />,
  );
  const text = view.container.textContent ?? '';
  expect(text).toContain('--');
  expect(text).not.toMatch(/\b0%/);
  expect(text).toContain('Not available');
});

test('loading state renders a skeleton and never claims empty data', () => {
  const view = render(<WidgetCard id="system" status="loading" />);
  expect(view.getByText('SYNCING')).not.toBeNull();
  expect(view.container.querySelector('.fw-skeleton--hero')).not.toBeNull();
  expect(view.container.textContent ?? '').not.toContain('Not available');
  view.unmount();
});

test('AI widgets ask for a local connection without data', () => {
  for (const id of ['claude', 'chatgpt'] as const) {
    const view = render(<WidgetCard id={id} />);
    expect(view.getByText('Connect local agent')).not.toBeNull();
    expect(view.container.textContent ?? '').not.toContain('62%');
    view.unmount();
  }
});

test('an idle agent still shows its last workspace and session', () => {
  const view = render(
    <WidgetCard
      id="agent"
      status="empty"
      data={{
        status: 'empty',
        workspace: 'C:\\Users\\demo\\projects\\protonsearch',
        task: 'Port dock widgets to native providers',
        duration: '1h 24m',
        tokens: '34.8K',
      }}
    />,
  );
  expect(view.queryByText('No workspace agent')).toBeNull();
  expect(view.getByText('C:\\Users\\demo\\projects\\protonsearch')).not.toBeNull();
  expect(view.getByText('Port dock widgets to native providers')).not.toBeNull();
  expect(view.getByText('1h 24m')).not.toBeNull();
  view.unmount();
});

test('AI docks show an honest disconnected state instead of fake limits', () => {
  for (const id of ['claude', 'chatgpt'] as const) {
    const disconnected = render(
      <DockWidget
        id={id}
        data={{ status: 'disconnected', sessionPercent: 62, weeklyPercent: 41 }}
        settings={settingsFor(id)}
      />,
    );
    expect(disconnected.getByRole('img', { name: 'Not connected' })).toBeTruthy();
    expect(disconnected.container.textContent ?? '').not.toContain('62');
    expect(disconnected.container.querySelector('.dock-ring-fill')).toBeNull();
    disconnected.unmount();

    const missing = render(<DockWidget id={id} />);
    expect(missing.getByRole('img', { name: 'Not connected' })).toBeTruthy();
    expect(missing.container.querySelector('.dock-ring-fill')).toBeNull();
    missing.unmount();
  }
  const live = render(
    <DockWidget
      id="chatgpt"
      data={{ status: 'live', sessionPercent: 62, weeklyPercent: 41 }}
      settings={settingsFor('chatgpt')}
    />,
  );
  expect(live.getByRole('img', { name: 'Session 62%' })).toBeTruthy();
  expect(live.container.querySelector('.dock-ring-fill')).not.toBeNull();
  live.unmount();
});

test('app icons render only safe data URLs and fall back otherwise', () => {
  const apps = {
    items: [
      { id: 'chrome', title: 'Google Chrome', icon: 'data:image/png;base64,iVBORw0KGgo=', active: true },
      { id: 'code', title: 'Visual Studio Code', icon: 'https://example.com/icon.png', active: false },
    ],
    activeTitle: 'Google Chrome',
    windowCount: 2,
  };
  const card = render(<WidgetCard id="apps" data={apps} settings={settingsFor('apps')} />);
  const images = Array.from(card.container.querySelectorAll('img.fw-app-img'));
  expect(images).toHaveLength(2);
  expect(images[0]?.getAttribute('src')).toBe('data:image/png;base64,iVBORw0KGgo=');
  expect(card.getAllByText('Google Chrome').length).toBeGreaterThan(0);
  card.unmount();

  const unsafe: typeof apps = {
    items: [{ id: 'code', title: 'Visual Studio Code', icon: 'https://example.com/icon.png', active: true }],
    activeTitle: 'Visual Studio Code',
    windowCount: 1,
  };
  const fallback = render(<WidgetCard id="apps" data={unsafe} settings={settingsFor('apps')} />);
  expect(fallback.container.querySelector('img.fw-app-img')).toBeNull();
  fallback.unmount();
});

test('system tile shows CPU load honestly', () => {
  const data = {
    cpu: 20,
    ram: 40,
    battery: 80,
    batteryCharging: false,
    temperatureC: 45,
    uptimeSeconds: 3600,
    connectivity: null,
  };
  const view = render(<DockWidget id="system" data={data} />);
  expect(view.getByRole('img', { name: 'CPU 20%' })).toBeTruthy();
  expect(view.getByText('20')).toBeTruthy();
  expect(view.container.querySelector('.dock-ring-fill')).not.toBeNull();
  view.unmount();

  const unknown = render(<DockWidget id="system" data={{ ...data, cpu: null }} />);
  expect(unknown.getByRole('img', { name: 'CPU unavailable' })).toBeTruthy();
  expect(unknown.getByText('--')).toBeTruthy();
  expect(unknown.container.querySelector('.dock-ring-fill')).toBeNull();
  unknown.unmount();
});

test('volume, trash and clock tiles show live values only', () => {
  const volume = render(<DockWidget id="volume" data={{ level: 68, muted: false, device: null }} />);
  expect(volume.getByRole('img', { name: 'Volume 68%' })).toBeTruthy();
  expect(volume.container.querySelector('.dock-level')).not.toBeNull();
  volume.unmount();

  const muted = render(<DockWidget id="volume" data={{ level: 68, muted: true, device: null }} />);
  expect(muted.getByRole('img', { name: 'Muted' })).toBeTruthy();
  expect(muted.container.querySelector('.dock-level')).toBeNull();
  muted.unmount();

  const unknown = render(<DockWidget id="volume" data={{ level: null, muted: null, device: null }} />);
  expect(unknown.getByRole('img', { name: 'Volume unavailable' })).toBeTruthy();
  expect(unknown.container.querySelector('.dock-level')).toBeNull();
  unknown.unmount();

  const trash = render(<DockWidget id="trash" data={{ itemCount: 3, sizeBytes: null, lastEmptiedAt: null }} />);
  expect(trash.container.querySelector('.dock-badge')?.textContent).toBe('3');
  trash.unmount();

  const emptyTrash = render(<DockWidget id="trash" data={{ itemCount: 0, sizeBytes: null, lastEmptiedAt: null }} />);
  expect(emptyTrash.container.querySelector('.dock-badge')).toBeNull();
  emptyTrash.unmount();

  const clock = render(
    <DockWidget
      id="clock"
      data={{ now: new Date(2026, 10, 7, 16, 5).getTime() }}
      settings={{ ...settingsFor('clock'), hour12: false }}
    />,
  );
  expect(clock.getByRole('img', { name: '16:05' })).toBeTruthy();
  clock.unmount();
});

test('allowed actions dispatch through onAction', () => {
  const actions: string[] = [];
  const onAction = (action: string) => actions.push(action);

  const spotify = render(<WidgetCard id="spotify" preview onAction={onAction} />);
  fireEvent.click(spotify.getByRole('button', { name: 'Pause' }));
  spotify.unmount();

  const trash = render(<WidgetCard id="trash" preview onAction={onAction} />);
  fireEvent.click(trash.getByRole('button', { name: /Empty Recycle Bin/ }));
  trash.unmount();

  const volume = render(<WidgetCard id="volume" preview onAction={onAction} />);
  fireEvent.click(volume.getByRole('button', { name: 'Mute' }));
  volume.unmount();

  const folders = render(<WidgetCard id="folders" preview onAction={onAction} />);
  fireEvent.click(folders.getByRole('button', { name: /Desktop/ }));
  folders.unmount();

  expect(actions).toEqual(['media:toggle', 'trash:empty', 'volume:mute', 'folder:open:desktop']);
});

test('settings controls are labelled and validation catches bad values', () => {
  const view = render(<WidgetSettings id="spotify" onChange={() => undefined} />);
  expect(view.getByRole('group', { name: 'Widget settings' })).not.toBeNull();
  expect(view.getByRole('checkbox', { name: 'Progress bar' })).not.toBeNull();
  expect(view.getByRole('checkbox', { name: 'Transport controls' })).not.toBeNull();
  expect(validateWidgetSettings('apps', settingsFor('apps'))).toBe(true);
  expect(validateWidgetSettings('apps', { maxApps: 99 })).toBe(false);
  expect(validateWidgetSettings('clock', { hour12: true })).toBe(true);
  expect(validateWidgetSettings('clock', { hour12: 'yes' })).toBe(false);
});

function SettingsHarness({ id }: { id: FinalWidgetId }) {
  const [value, setValue] = useState<FinalWidgetSettings>(() => settingsFor(id));
  return (
    <>
      <WidgetSettings id={id} value={value} onChange={setValue} />
      <WidgetCard id={id} settings={value} preview />
    </>
  );
}

test('weather card asks for a location when live weather is disabled', () => {
  const view = render(
    <WidgetCard
      id="weather"
      settings={{ ...settingsFor('weather'), weatherEnabled: false }}
    />,
  );
  expect(view.getByText('Set weather location')).not.toBeNull();
  expect(view.getByText(/not estimated/)).not.toBeNull();
});

test('weather dock states stay honest without a snapshot', () => {
  const disabled = render(<DockWidget id="weather" settings={{ ...settingsFor('weather'), weatherEnabled: false }} />);
  expect(disabled.getByRole('img', { name: 'Weather location not set' })).toBeTruthy();
  expect(disabled.getByText('--°')).toBeTruthy();
  disabled.unmount();

  const enabled = render(<DockWidget id="weather" settings={settingsFor('weather')} />);
  expect(enabled.getByRole('img', { name: 'No weather data' })).toBeTruthy();
  expect(enabled.getByText('--°')).toBeTruthy();
  enabled.unmount();
});

test('weather dock renders live data and preview samples without placeholder copy', () => {
  const live = render(
    <DockWidget
      id="weather"
      data={{ temperature: 31, weatherCode: 0, time: 0, utcOffsetSeconds: 19800, hours: [], highC: 34, lowC: 22 }}
      settings={{ ...settingsFor('weather'), units: 'F' }}
    />,
  );
  expect(live.getByText('88°')).toBeTruthy();
  expect(live.getByRole('img', { name: '88°, Clear' })).toBeTruthy();
  live.unmount();

  const preview = render(
    <DockWidget
      id="weather"
      preview
      settings={{ ...settingsFor('weather'), weatherEnabled: false }}
    />,
  );
  expect(preview.getByText('24°')).toBeTruthy();
  expect(preview.getByRole('img', { name: '24°, Partly cloudy' })).toBeTruthy();
  preview.unmount();
});

test('every widget setting changes the rendered card', () => {
  for (const id of FINAL_WIDGET_IDS) {
    for (const field of FINAL_SETTING_FIELDS[id]) {
      const view = render(<SettingsHarness id={id} />);
      const card = view.container.querySelector('[data-widget-card]');
      expect(card).not.toBeNull();
      const before = card?.innerHTML;

      // These fields configure live weather acquisition; the sample card does not display coordinates.
      if (id === 'weather' && ['weatherEnabled', 'weatherLatitude', 'weatherLongitude'].includes(field.key)) {
        view.unmount();
        continue;
      }

      if (field.type === 'toggle') {
        fireEvent.click(view.getByRole('checkbox', { name: field.label }));
      } else if (field.type === 'range') {
        const input = view.getByLabelText(field.label) as HTMLInputElement;
        const next = Number(input.value) === field.min ? field.max : field.min;
        fireEvent.change(input, { target: { value: String(next) } });
      } else if (field.type === 'select') {
        const select = view.getByLabelText(field.label) as HTMLSelectElement;
        const next = field.options.find((option) => option.value !== select.value);
        expect(next).toBeDefined();
        fireEvent.change(select, { target: { value: next?.value } });
      } else if (field.type === 'number') {
        fireEvent.change(view.getByLabelText(field.label), { target: { value: String(field.max) } });
      } else {
        fireEvent.change(view.getByLabelText(field.label), { target: { value: 'Preview value' } });
      }

      const after = view.container.querySelector('[data-widget-card]')?.innerHTML;
      expect(after, `${id}.${field.key}`).not.toBe(before);
      view.unmount();
    }
  }
});

test('sources contain no em dashes, replacement characters or emoji', () => {
  const sources = import.meta.glob('./*.{ts,tsx,css}', {
    eager: true,
    query: '?raw',
    import: 'default',
  }) as Record<string, string>;
  for (const [file, source] of Object.entries(sources)) {
    if (file.endsWith('.test.tsx')) continue;
    expect(source, file).not.toMatch(/[\u2013\u2014]/);
    expect(source, file).not.toMatch(/[\u00C2\u00E2\uFFFD]/);
    expect(source, file).not.toMatch(/[\u{1F000}-\u{1FAFF}\u{2600}-\u{27BF}]/u);
  }
});
