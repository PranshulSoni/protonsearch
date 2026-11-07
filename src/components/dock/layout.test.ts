import { describe, expect, it } from 'vitest';
import {
  PROFILE_DEFAULTS,
  defaultLayout,
  disableWidget,
  enableWidget,
  isLayout,
  itemsFor,
  moveWidget,
  parseLayout,
  patchItem,
  type DockLayout,
} from './layout';

function ids(layout: DockLayout, profile = layout.profile) {
  return layout.profiles[profile].map((item) => item.id);
}

describe('dock layout v2', () => {
  it('exposes the four profile defaults from the brief', () => {
    const layout = defaultLayout();
    expect(layout.version).toBe(2);
    expect(layout.profile).toBe('daily');
    expect(ids(layout, 'daily')).toEqual([...PROFILE_DEFAULTS.daily]);
    expect(ids(layout, 'work')).toEqual([...PROFILE_DEFAULTS.work]);
    expect(ids(layout, 'founder')).toEqual([...PROFILE_DEFAULTS.founder]);
    expect(ids(layout, 'focus')).toEqual([...PROFILE_DEFAULTS.focus]);
    expect(layout.unknown).toEqual([]);
  });

  it('migrates v1 ids, sizes, appearance and unknown widgets', () => {
    const parsed = parseLayout({
      version: 1,
      profile: 'daily',
      theme: 'dark',
      appearances: { daily: 'lines', founder: 'capsules' },
      profiles: {
        daily: [
          { id: 'windows', size: 'wide' },
          { id: 'protonsearch' },
          { id: 'time' },
          { id: 'weather', settings: { units: 'F', forecastHours: 6 } },
          { id: 'status' },
          { id: 'media' },
          { id: 'calendar' },
        ],
        founder: [{ id: 'git' }, { id: 'revenue' }],
      },
    });

    expect(parsed).not.toBeNull();
    expect(parsed?.version).toBe(2);
    expect(ids(parsed!, 'daily')).toEqual(['windows', 'search', 'clock', 'weather', 'system', 'spotify']);
    expect(parsed?.profiles.daily[0].size).toBe('wide');
    expect(parsed?.profiles.daily[3].settings).toMatchObject({ units: 'F' });
    expect(ids(parsed!, 'founder')).toEqual([]);
    expect(ids(parsed!, 'work')).toEqual([...PROFILE_DEFAULTS.work]);
    expect(parsed?.unknown).toEqual(['calendar', 'git', 'revenue']);
    expect(parsed?.appearances).toEqual({ daily: 'lines', founder: 'capsules' });
    expect(parsed?.theme).toBe('dark');
  });

  it('clamps saved settings and drops unknown keys instead of failing the layout', () => {
    const parsed = parseLayout({
      version: 2,
      profile: 'focus',
      unknown: ['legacy-thing'],
      profiles: {
        daily: [{ id: 'apps', size: 'wide', settings: { maxApps: 99, showTitles: false, mystery: true } }],
        work: [],
        founder: [],
        focus: [{ id: 'weather', settings: { weatherEnabled: true, weatherLatitude: -200, weatherLongitude: 72.5 } }],
      },
    });

    expect(parsed).not.toBeNull();
    expect(parsed?.profile).toBe('focus');
    expect(parsed?.profiles.daily[0]).toMatchObject({
      id: 'apps',
      size: 'wide',
      settings: { showTitles: false, maxApps: 8 },
    });
    expect(parsed?.profiles.focus[0].settings).toMatchObject({
      weatherEnabled: true,
      weatherLatitude: -90,
      weatherLongitude: 72.5,
    });
  });

  it('returns null for unknown or malformed schemas so callers can preserve saved data', () => {
    expect(parseLayout(null)).toBeNull();
    expect(parseLayout({ version: 3, profiles: {} })).toBeNull();
    expect(parseLayout({ version: 1, profile: 'focus', profiles: { daily: [], founder: [] } })).toBeNull();
    expect(parseLayout({ version: 2, profile: 'daily' })).toBeNull();
  });

  it('validates v2 structurally and rejects duplicate ids or extra item keys', () => {
    const valid = defaultLayout();
    expect(isLayout(valid)).toBe(true);
    expect(isLayout({ ...valid, extra: true })).toBe(false);
    expect(isLayout({ ...valid, unknown: [1] })).toBe(false);
    expect(isLayout({
      ...valid,
      profiles: { ...valid.profiles, daily: [{ id: 'clock' }, { id: 'clock' }] },
    })).toBe(false);
    expect(isLayout({
      ...valid,
      profiles: { ...valid.profiles, daily: [{ id: 'clock', width: 300 }] },
    })).toBe(false);
  });

  it('supports profile switching, enable/disable, size, settings and reorder operations', () => {
    let layout = defaultLayout();
    layout = enableWidget(layout, 'trash');
    expect(ids(layout).at(-1)).toBe('trash');
    expect(ids(layout).filter((id) => id === 'trash')).toHaveLength(1);

    layout = disableWidget(layout, 'clock');
    expect(ids(layout)).not.toContain('clock');

    layout = patchItem(layout, 'apps', { size: 'wide', settings: { maxApps: 6 } });
    expect(layout.profiles.daily.find((item) => item.id === 'apps')).toMatchObject({
      size: 'wide',
      settings: { maxApps: 6 },
    });

    layout = moveWidget(layout, 0, 2);
    expect(ids(layout).slice(0, 3)).toEqual(['search', 'apps', 'windows']);
    const beforeInvalid = ids(layout);
    expect(ids(moveWidget(layout, -1, 2))).toEqual(beforeInvalid);

    const work = { ...layout, profile: 'work' as const };
    expect(itemsFor(work)).toBe(work.profiles.work);
    expect(itemsFor(work, 'focus').map((item) => item.id)).toEqual([...PROFILE_DEFAULTS.focus]);
  });
});
