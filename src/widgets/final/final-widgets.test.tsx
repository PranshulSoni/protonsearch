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

test('AI widgets ask for a local connection without data', () => {
  for (const id of ['claude', 'chatgpt'] as const) {
    const view = render(<WidgetCard id={id} />);
    expect(view.getByText('Connect local agent')).not.toBeNull();
    expect(view.container.textContent ?? '').not.toContain('62%');
    view.unmount();
  }
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

test('every widget setting changes the rendered card', () => {
  for (const id of FINAL_WIDGET_IDS) {
    for (const field of FINAL_SETTING_FIELDS[id]) {
      const view = render(<SettingsHarness id={id} />);
      const card = view.container.querySelector('[data-widget-card]');
      expect(card).not.toBeNull();
      const before = card?.innerHTML;

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
