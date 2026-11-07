import { isTheme, type Theme } from '../../design/theme';
import {
  FINAL_WIDGET_IDS,
  getWidgetMeta,
  isFinalWidgetId,
  settingsFor,
  validateWidgetSettings,
} from '../../widgets/final/meta';
import type { FinalWidgetId, FinalWidgetSettings } from '../../widgets/final/types';
import type { WidgetSize } from '../../widgets/sdk/types';

export const PROFILE_IDS = ['daily', 'work', 'founder', 'focus'] as const;
export type ProfileId = (typeof PROFILE_IDS)[number];

export const PROFILE_LABELS: Record<ProfileId, string> = {
  daily: 'Daily',
  work: 'Work',
  founder: 'Founder',
  focus: 'Focus',
};

export type AppearanceMode = 'capsules' | 'lines';

export interface DockItem {
  id: FinalWidgetId;
  size?: WidgetSize;
  settings?: Record<string, unknown>;
}

export interface DockLayout {
  version: 2;
  profile: ProfileId;
  profiles: Record<ProfileId, DockItem[]>;
  unknown: string[];
  appearances?: Partial<Record<ProfileId, AppearanceMode>>;
  theme?: Theme;
}

const SIZES: readonly WidgetSize[] = ['compact', 'standard', 'wide'];
const LEGACY_IDS: Record<string, FinalWidgetId> = {
  time: 'clock',
  status: 'system',
  media: 'spotify',
  protonsearch: 'search',
};

export const PROFILE_DEFAULTS: Record<ProfileId, readonly FinalWidgetId[]> = {
  daily: ['windows', 'search', 'apps', 'spotify', 'weather', 'system', 'claude', 'agent', 'clock'],
  work: ['windows', 'search', 'apps', 'system', 'claude', 'agent', 'clock', 'files', 'folders'],
  founder: ['windows', 'search', 'apps', 'spotify', 'weather', 'system', 'claude', 'chatgpt', 'agent', 'clock'],
  focus: ['windows', 'search', 'system', 'weather', 'clock'],
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

export function isProfileId(value: unknown): value is ProfileId {
  return typeof value === 'string' && (PROFILE_IDS as readonly string[]).includes(value);
}

export function isWidgetSize(value: unknown): value is WidgetSize {
  return typeof value === 'string' && (SIZES as readonly string[]).includes(value);
}

function normalizeSettings(id: FinalWidgetId, value: unknown): Record<string, unknown> | undefined {
  if (!isRecord(value)) return undefined;
  const clamped = settingsFor(id, value) as unknown as Record<string, unknown>;
  if (!validateWidgetSettings(id, clamped)) return undefined;
  const defaults = getWidgetMeta(id).settingsDefaults as unknown as Record<string, unknown>;
  return JSON.stringify(clamped) === JSON.stringify(defaults) ? undefined : clamped;
}

function normalizeItem(id: FinalWidgetId, raw: unknown): DockItem {
  if (!isRecord(raw)) return { id };
  const item: DockItem = { id };
  if (isWidgetSize(raw.size)) item.size = raw.size;
  const settings = normalizeSettings(id, raw.settings);
  if (settings) item.settings = settings;
  return item;
}

function uniqueItems(items: DockItem[]): DockItem[] {
  const seen = new Set<FinalWidgetId>();
  return items.filter((item) => {
    if (seen.has(item.id)) return false;
    seen.add(item.id);
    return true;
  });
}

function profileItems(value: unknown, unknown: string[]): DockItem[] {
  if (!Array.isArray(value)) return [];
  const items: DockItem[] = [];
  for (const raw of value) {
    if (!isRecord(raw) || typeof raw.id !== 'string') continue;
    if (isFinalWidgetId(raw.id)) {
      items.push(normalizeItem(raw.id, raw));
    } else if (!unknown.includes(raw.id)) {
      unknown.push(raw.id);
    }
  }
  return uniqueItems(items);
}

export function defaultLayout(profile: ProfileId = 'daily'): DockLayout {
  return {
    version: 2,
    profile,
    profiles: {
      daily: PROFILE_DEFAULTS.daily.map((id) => ({ id })),
      work: PROFILE_DEFAULTS.work.map((id) => ({ id })),
      founder: PROFILE_DEFAULTS.founder.map((id) => ({ id })),
      focus: PROFILE_DEFAULTS.focus.map((id) => ({ id })),
    },
    unknown: [],
    appearances: { daily: 'capsules', work: 'capsules', founder: 'capsules', focus: 'capsules' },
  };
}

function normalizeProfile(value: unknown, fallback: ProfileId): ProfileId {
  return isProfileId(value) ? value : fallback;
}

function parseV2(value: Record<string, unknown>): DockLayout | null {
  if (value.version !== 2) return null;
  if (!isRecord(value.profiles)) return null;
  const unknown: string[] = [];
  if (Array.isArray(value.unknown)) {
    for (const entry of value.unknown) {
      if (typeof entry === 'string' && !unknown.includes(entry)) unknown.push(entry);
    }
  }
  const profiles = {} as Record<ProfileId, DockItem[]>;
  for (const profile of PROFILE_IDS) {
    const raw = value.profiles[profile];
    profiles[profile] = Array.isArray(raw) ? profileItems(raw, unknown) : defaultLayout().profiles[profile];
  }
  const layout: DockLayout = {
    version: 2,
    profile: normalizeProfile(value.profile, 'daily'),
    profiles,
    unknown,
  };
  if (isTheme(value.theme)) layout.theme = value.theme;
  if (isRecord(value.appearances)) {
    const appearances: Partial<Record<ProfileId, AppearanceMode>> = {};
    for (const profile of PROFILE_IDS) {
      const mode = value.appearances[profile];
      if (mode === 'capsules' || mode === 'lines') appearances[profile] = mode;
    }
    if (Object.keys(appearances).length) layout.appearances = appearances;
  }
  return layout;
}

function parseV1(value: Record<string, unknown>): DockLayout | null {
  if (value.version !== 1) return null;
  if (!isRecord(value.profiles)) return null;
  if (!isProfileId(value.profile) || (value.profile !== 'daily' && value.profile !== 'founder')) return null;
  if (!Array.isArray(value.profiles.daily) || !Array.isArray(value.profiles.founder)) return null;

  const layout = defaultLayout(value.profile);
  const unknown: string[] = [];
  const migrate = (raw: unknown): DockItem[] => {
    if (!Array.isArray(raw)) return [];
    const items: DockItem[] = [];
    for (const entry of raw) {
      if (!isRecord(entry) || typeof entry.id !== 'string') continue;
      const mapped = LEGACY_IDS[entry.id] ?? (isFinalWidgetId(entry.id) ? entry.id : null);
      if (!mapped) {
        if (!unknown.includes(entry.id)) unknown.push(entry.id);
        continue;
      }
      items.push(normalizeItem(mapped, entry));
    }
    return uniqueItems(items);
  };

  layout.profiles.daily = migrate(value.profiles.daily);
  layout.profiles.founder = migrate(value.profiles.founder);
  layout.unknown = unknown;
  if (isTheme(value.theme)) layout.theme = value.theme;
  if (isRecord(value.appearances)) {
    const appearances: Partial<Record<ProfileId, AppearanceMode>> = {};
    for (const profile of ['daily', 'founder'] as const) {
      const mode = value.appearances[profile];
      if (mode === 'capsules' || mode === 'lines') appearances[profile] = mode;
    }
    if (Object.keys(appearances).length) layout.appearances = appearances;
  }
  return layout;
}

/** Parse saved settings from either schema version and return a sanitized v2 layout. */
export function parseLayout(value: unknown): DockLayout | null {
  if (!isRecord(value)) return null;
  return value.version === 2 ? parseV2(value) : value.version === 1 ? parseV1(value) : null;
}

export function isLayout(value: unknown): value is DockLayout {
  if (!isRecord(value)) return false;
  const parsed = parseV2(value);
  if (!parsed) return false;
  const keys = Object.keys(value);
  if (keys.some((key) => !['version', 'profile', 'profiles', 'unknown', 'appearances', 'theme'].includes(key))) {
    return false;
  }
  const profiles = value.profiles;
  if (!isRecord(profiles)) return false;
  if (!Array.isArray(profiles.daily) || !Array.isArray(profiles.work)
    || !Array.isArray(profiles.founder) || !Array.isArray(profiles.focus)) {
    return false;
  }
  for (const profile of PROFILE_IDS) {
    const seen = new Set<string>();
    for (const item of profiles[profile] as unknown[]) {
      if (!isRecord(item) || typeof item.id !== 'string' || !isFinalWidgetId(item.id)) return false;
      if (Object.keys(item).some((key) => !['id', 'size', 'settings'].includes(key))) return false;
      if (item.size !== undefined && !isWidgetSize(item.size)) return false;
      if (item.settings !== undefined && !isRecord(item.settings)) return false;
      if (seen.has(item.id)) return false;
      seen.add(item.id);
    }
  }
  if (!Array.isArray(value.unknown) || value.unknown.some((entry) => typeof entry !== 'string')) return false;
  if (value.appearances !== undefined) {
    if (!isRecord(value.appearances)) return false;
    for (const [key, mode] of Object.entries(value.appearances)) {
      if (!isProfileId(key) || (mode !== 'capsules' && mode !== 'lines')) return false;
    }
  }
  return value.theme === undefined || isTheme(value.theme);
}

export function normalizeLayout(value: unknown): DockLayout {
  return parseLayout(value) ?? defaultLayout();
}

export function itemsFor(layout: DockLayout, profile: ProfileId = layout.profile): DockItem[] {
  return layout.profiles[profile];
}

export function updateItems(
  layout: DockLayout,
  profile: ProfileId,
  update: (items: DockItem[]) => DockItem[],
): DockLayout {
  return { ...layout, profiles: { ...layout.profiles, [profile]: update(layout.profiles[profile]) } };
}

export function enableWidget(layout: DockLayout, id: FinalWidgetId): DockLayout {
  return updateItems(layout, layout.profile, (items) => (
    items.some((item) => item.id === id) ? items : [...items, normalizeItem(id, {})]
  ));
}

export function disableWidget(layout: DockLayout, id: FinalWidgetId): DockLayout {
  return updateItems(layout, layout.profile, (items) => items.filter((item) => item.id !== id));
}

export function patchItem(
  layout: DockLayout,
  id: FinalWidgetId,
  patch: Partial<Pick<DockItem, 'size' | 'settings'>>,
): DockLayout {
  return updateItems(layout, layout.profile, (items) => items.map((item) => (
    item.id === id ? { ...item, ...patch } : item
  )));
}

export function moveWidget(layout: DockLayout, from: number, to: number): DockLayout {
  return updateItems(layout, layout.profile, (items) => {
    if (from < 0 || from >= items.length || to < 0 || to >= items.length || from === to) return items;
    const next = items.slice();
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    return next;
  });
}

export function effectiveSize(item: DockItem): WidgetSize {
  return item.size ?? getWidgetMeta(item.id).defaultSize;
}

export function effectiveSettings(item: DockItem): FinalWidgetSettings {
  return settingsFor(item.id, item.settings);
}

export function layoutFingerprint(layout: DockLayout): string {
  return JSON.stringify(layout);
}

export { FINAL_WIDGET_IDS };
