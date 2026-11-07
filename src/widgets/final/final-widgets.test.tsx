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

const sizes = ['compact', 'standard', 'wide'] as const;

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

  const search = render(<DockWidget id="search" size="standard" />);
  expect(search.queryByText('Alt')).toBeNull();
  search.unmount();
});

test('all dock sizes and all cards render without emoji or mojibake', () => {
  for (const id of FINAL_WIDGET_IDS) {
    for (const size of sizes) {
      const view = render(<DockWidget id={id} size={size} preview />);
      expect(view.container.querySelector(`[data-widget="${id}"][data-size="${size}"]`)).not.toBeNull();
      expect(view.container.textContent ?? '').not.toMatch(/[\u{1F000}-\u{1FAFF}\u{2600}-\u{27BF}]/u);
      expect(view.container.textContent ?? '').not.toMatch(/[\u00C2\u00E2\uFFFD]/);
      view.unmount();
    }
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

test('AI docks show an honest disconnected state instead of fake limits', () => {
  for (const id of ['claude', 'chatgpt'] as const) {
    const label = id === 'claude' ? 'Claude' : 'ChatGPT';
    const disconnected = render(
      <DockWidget
        id={id}
        size="standard"
        data={{ status: 'disconnected', sessionPercent: 62, weeklyPercent: 41 }}
        settings={settingsFor(id)}
      />,
    );
    expect(disconnected.getByText('Not connected')).toBeTruthy();
    expect(disconnected.getByText(`Connect ${label}`)).toBeTruthy();
    expect(disconnected.container.textContent ?? '').not.toContain('62%');
    expect(disconnected.container.textContent ?? '').not.toContain('Max 20x');
    disconnected.unmount();

    const missing = render(<DockWidget id={id} size="standard" />);
    expect(missing.getByText('Not connected')).toBeTruthy();
    missing.unmount();
  }
  const live = render(
    <DockWidget
      id="chatgpt"
      size="standard"
      data={{ status: 'live', sessionPercent: 62, weeklyPercent: 41 }}
      settings={settingsFor('chatgpt')}
    />,
  );
  expect(live.container.textContent ?? '').toContain('62%');
  expect(live.getByText('Week 41%')).toBeTruthy();
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

  const dock = render(<DockWidget id="apps" size="standard" data={apps} settings={settingsFor('apps')} />);
  expect(dock.container.querySelectorAll('img.fw-app-img')).toHaveLength(1);
  dock.unmount();

  const unsafe: typeof apps = {
    items: [{ id: 'code', title: 'Visual Studio Code', icon: 'https://example.com/icon.png', active: true }],
    activeTitle: 'Visual Studio Code',
    windowCount: 1,
  };
  const fallback = render(<WidgetCard id="apps" data={unsafe} settings={settingsFor('apps')} />);
  expect(fallback.container.querySelector('img.fw-app-img')).toBeNull();
  fallback.unmount();
});

test('system dock shows Wi-Fi and Bluetooth state and respects visibility settings', () => {
  const data = {
    cpu: 20,
    ram: 40,
    battery: 80,
    batteryCharging: false,
    temperatureC: 45,
    uptimeSeconds: 3600,
    connectivity: {
      wifi: { connected: true, ssid: 'HomeNet', signalPercent: 90, state: 'connected' },
      bluetooth: { available: true, enabled: true, name: 'Buds' },
      ethernet: { connected: false, adapter: null },
      vpn: { connected: false, adapter: null },
    },
  };
  const view = render(<DockWidget id="system" size="standard" data={data} settings={settingsFor('system')} />);
  expect(view.getByText('HomeNet')).toBeTruthy();
  expect(view.getByText('Buds')).toBeTruthy();
  expect(view.getByText('CPU')).toBeTruthy();
  expect(view.getByText('RAM')).toBeTruthy();
  expect(view.getByText('BAT')).toBeTruthy();
  view.unmount();

  const hidden = render(
    <DockWidget
      id="system"
      size="standard"
      data={data}
      settings={{ ...settingsFor('system'), showWifi: false, showBluetooth: false }}
    />,
  );
  expect(hidden.queryByText('HomeNet')).toBeNull();
  expect(hidden.queryByText('Buds')).toBeNull();
  expect(hidden.container.querySelector('.fw-sys-net')).toBeNull();
  hidden.unmount();

  const offline = render(
    <DockWidget
      id="system"
      size="standard"
      data={{
        ...data,
        connectivity: {
          wifi: { connected: false, ssid: null, signalPercent: null, state: 'disconnected' },
          bluetooth: { available: false, enabled: false, name: null },
        },
      }}
      settings={settingsFor('system')}
    />,
  );
  expect(offline.getByText('Off')).toBeTruthy();
  expect(offline.getByText('Unavailable')).toBeTruthy();
  offline.unmount();

  const empty = render(<DockWidget id="system" size="standard" data={{ ...data, connectivity: null }} settings={settingsFor('system')} />);
  expect(empty.container.querySelector('.fw-sys-net')).toBeNull();
  empty.unmount();

  const compact = render(<DockWidget id="system" size="compact" data={data} settings={settingsFor('system')} />);
  expect(compact.container.querySelectorAll('.fw-sys-net-ic')).toHaveLength(2);
  compact.unmount();
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
  const disabled = render(
    <DockWidget
      id="weather"
      size="standard"
      settings={{ ...settingsFor('weather'), weatherEnabled: false }}
    />,
  );
  expect(disabled.getByText('--°')).toBeTruthy();
  expect(disabled.getByText('Set location')).toBeTruthy();
  expect(disabled.queryByText('Partly cloudy')).toBeNull();
  expect(disabled.queryByText(/High \/ low unavailable/)).toBeNull();
  disabled.unmount();

  const enabled = render(<DockWidget id="weather" size="wide" settings={settingsFor('weather')} />);
  expect(enabled.getByText('--°')).toBeTruthy();
  expect(enabled.getByText('No data')).toBeTruthy();
  expect(enabled.getByText('Ahmedabad')).toBeTruthy();
  expect(enabled.queryByText('Partly cloudy')).toBeNull();
  expect(enabled.queryByText(/High \/ low unavailable/)).toBeNull();
  enabled.unmount();
});

test('weather dock renders live data and preview samples without placeholder copy', () => {
  const live = render(
    <DockWidget
      id="weather"
      size="wide"
      data={{ temperature: 31, weatherCode: 0, time: 0, utcOffsetSeconds: 19800, hours: [], highC: 34, lowC: 22 }}
      settings={{ ...settingsFor('weather'), units: 'F' }}
    />,
  );
  expect(live.getByText('88°')).toBeTruthy();
  expect(live.getByText('Clear')).toBeTruthy();
  expect(live.getByText('High 93° · Low 72°')).toBeTruthy();
  expect(live.getByText('Ahmedabad')).toBeTruthy();
  expect(live.queryByText('No data')).toBeNull();
  live.unmount();

  const preview = render(
    <DockWidget
      id="weather"
      size="wide"
      preview
      settings={{ ...settingsFor('weather'), weatherEnabled: false }}
    />,
  );
  expect(preview.getByText('24°')).toBeTruthy();
  expect(preview.getByText('Partly cloudy')).toBeTruthy();
  expect(preview.getByText('High 27° · Low 18°')).toBeTruthy();
  expect(preview.getByText('Ahmedabad')).toBeTruthy();
  expect(preview.queryByText('Set location')).toBeNull();
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
