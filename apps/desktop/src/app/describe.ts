// Pure presentation logic for engine status, sources, activity, greetings and
// the optional agent runtime. No React: unit-tested in describe.test.ts.
// All dates/times go through Intl so they are locale-aware.

import type {
  ActivityItem,
  ActivityKind,
  ActivityQuery,
  AgentStatus,
  EngineStatus,
  PrivacySettings,
  RootStatus,
  Settings,
  SourceStatus,
} from "../bridge/types";
import { formatNumber } from "../lib/format";

export type Tone = "ok" | "busy" | "warn" | "error" | "off";

export interface IndexHealth {
  tone: Tone;
  label: string;
  detail: string;
}

/** Honest one-line index health derived only from what the engine reports. */
export function indexHealth(status: EngineStatus | null): IndexHealth {
  if (!status) {
    return {
      tone: "off",
      label: "Waiting for engine",
      detail: "The local engine has not reported its state yet.",
    };
  }
  const warning = status.warnings.length > 0;
  switch (status.state) {
    case "ready":
      return warning
        ? {
            tone: "warn",
            label: "Needs attention",
            detail: "Some items couldn't be indexed. Search still works with what is available.",
          }
        : {
            tone: "ok",
            label: "Index ready",
            detail: "Everything you enabled is searchable on this device.",
          };
    case "indexing":
      return {
        tone: "busy",
        label: "Indexing",
        detail:
          "Background indexing is running. Search results keep working while it continues (" +
          formatNumber(status.filenameEntries) +
          " names so far).",
      };
    case "paused":
      return {
        tone: "warn",
        label: "Indexing paused",
        detail: "Existing results stay searchable, but new files are not being picked up.",
      };
    case "starting":
      return {
        tone: "busy",
        label: "Starting",
        detail:
          "The local engine is warming up. Filename search becomes available as soon as it reports ready.",
      };
    case "error":
      return {
        tone: "error",
        label: "Index error",
        detail: "New files aren't being added. Existing search still works.",
      };
    default:
      return {
        tone: "off",
        label: "Unknown state",
        detail: "The engine reported an unrecognised index state.",
      };
  }
}

export function stateLabel(state: string): string {
  switch (state) {
    case "ready":
      return "Ready";
    case "starting":
      return "Starting";
    case "indexing":
      return "Indexing";
    case "paused":
      return "Paused";
    case "error":
      return "Error";
    case "off":
      return "Off";
    case "unavailable":
      return "Unavailable";
    default:
      return state ? state.charAt(0).toUpperCase() + state.slice(1) : "Unknown";
  }
}

export interface SourceInfo {
  title: string;
  description: string;
  privacyKey: keyof PrivacySettings | null;
  tone: Tone;
  stateLabel: string;
  enabled: boolean;
}

interface SourceMeta {
  title: string;
  description: string;
  privacyKey: keyof PrivacySettings | null;
}

const SOURCE_META: Record<string, SourceMeta> = {
  apps: {
    title: "Applications",
    description: "Installed apps discovered from the Start menu and app paths.",
    privacyKey: null,
  },
  browser: {
    title: "Browser history & bookmarks",
    description: "Reads local Chrome, Edge and Firefox profiles. Nothing is uploaded.",
    privacyKey: "browserHistory",
  },
  clipboard: {
    title: "Clipboard history",
    description: "Keeps text you copy locally so you can paste it from the launcher.",
    privacyKey: "clipboardHistory",
  },
  recent: {
    title: "Recent files",
    description: "Files you opened recently, read from the Windows Recent folder.",
    privacyKey: null,
  },
  git: {
    title: "Git repositories",
    description: "Commits and TODO/FIXME tasks from repositories inside your indexed folders.",
    privacyKey: "gitIndexing",
  },
};

export function describeSource(source: SourceStatus, settings: Settings): SourceInfo {
  const meta: SourceMeta =
    SOURCE_META[source.id] ?? {
      title: source.label,
      description: "Reported by the local engine.",
      privacyKey: null,
    };
  const enabled = meta.privacyKey ? settings.privacy[meta.privacyKey] && source.enabled : source.enabled;
  const tone: Tone = !enabled
    ? "off"
    : source.state === "error"
      ? "error"
      : source.state === "ready"
        ? "ok"
        : source.state === "indexing"
          ? "busy"
          : "warn";
  return {
    title: meta.title,
    description: meta.description,
    privacyKey: meta.privacyKey,
    tone,
    stateLabel: enabled ? stateLabel(source.state) : "Off",
    enabled,
  };
}

export const ACTIVITY_KIND_LABEL: Record<ActivityKind, string> = {
  search: "Search",
  open: "Opened",
  launch: "Launched",
  index: "Index",
  workflow: "Workflow",
  error: "Error",
  system: "System",
};

export interface ActivityDay {
  key: string;
  label: string;
  items: ActivityItem[];
}

function dayKey(timestamp: number): string {
  const date = new Date(timestamp);
  return date.getFullYear() + "-" + (date.getMonth() + 1) + "-" + date.getDate();
}

function dayLabel(timestamp: number, todayKey: string, yesterdayKey: string): string {
  const key = dayKey(timestamp);
  if (key === todayKey) return "Today";
  if (key === yesterdayKey) return "Yesterday";
  return new Date(timestamp).toLocaleDateString(undefined, {
    weekday: "long",
    month: "short",
    day: "numeric",
  });
}

/** Group activity into local-time days, newest first, newest item first. */
export function groupActivityByDay(items: ActivityItem[], now = Date.now()): ActivityDay[] {
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  const todayKey = dayKey(now);
  const yesterdayKey = dayKey(yesterday.getTime());

  const days = new Map<string, ActivityDay>();
  for (const item of items) {
    const key = dayKey(item.timestamp);
    let day = days.get(key);
    if (!day) {
      day = { key, label: dayLabel(item.timestamp, todayKey, yesterdayKey), items: [] };
      days.set(key, day);
    }
    day.items.push(item);
  }
  const ordered = [...days.values()];
  for (const day of ordered) day.items.sort((a, b) => b.timestamp - a.timestamp);
  ordered.sort((a, b) => b.items[0].timestamp - a.items[0].timestamp);
  return ordered;
}

const ACCESS_KINDS: ActivityKind[] = ["open", "launch"];

/**
 * "Recently accessed" means things the user opened or launched (not searches),
 * newest first, without repeating the same path/title.
 */
export function recentAccessed(items: ActivityItem[], limit = 3): ActivityItem[] {
  const seen = new Set<string>();
  const result: ActivityItem[] = [];
  for (const item of items) {
    if (!ACCESS_KINDS.includes(item.kind)) continue;
    const identity = (item.path ?? "") + "|" + item.title;
    if (seen.has(identity)) continue;
    seen.add(identity);
    result.push(item);
    if (result.length >= limit) break;
  }
  return result;
}

export function activityQuery(limit = 100): ActivityQuery {
  return { limit };
}

/* ------------------------------------------------------------------------- */
/* Greeting, relative time and clock strings (localised via Intl)            */
/* ------------------------------------------------------------------------- */

/**
 * Time-of-day greeting buckets (local): 05:00–11:59 morning,
 * 12:00–16:59 afternoon, 17:00–04:59 evening. Uses Intl for the hour, so the
 * user's locale/clock format decides what "local hour" means.
 */
export function greetingForTimestamp(timestamp = Date.now(), locale?: string): string {
  const parts = new Intl.DateTimeFormat(locale, {
    hour: "numeric",
    hourCycle: "h23",
  }).formatToParts(new Date(timestamp));
  const hourPart = parts.find((part) => part.type === "hour");
  const hour = hourPart ? Number(hourPart.value) : new Date(timestamp).getHours();
  if (hour >= 5 && hour <= 11) return "Good morning";
  if (hour >= 12 && hour <= 16) return "Good afternoon";
  return "Good evening";
}

/** "Good morning, Pranshul." or the neutral "Good morning." fallback. */
export function greetingLine(userName: string | null | undefined, now = Date.now()): string {
  const greeting = greetingForTimestamp(now);
  const name = (userName ?? "").trim();
  return name ? greeting + ", " + name + "." : greeting + ".";
}

const RELATIVE_UNITS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["day", 86_400_000],
  ["hour", 3_600_000],
  ["minute", 60_000],
  ["second", 1_000],
];

/** Locale-aware relative time ("3 minutes ago", "yesterday"). */
export function relativeTime(timestamp: number | null | undefined, now = Date.now()): string {
  if (!timestamp || !Number.isFinite(timestamp)) return "Never";
  const delta = timestamp - now;
  const abs = Math.abs(delta);
  for (const [unit, size] of RELATIVE_UNITS) {
    if (abs >= size || unit === "second") {
      const value = Math.round(delta / size);
      return new Intl.RelativeTimeFormat(undefined, { numeric: "auto" }).format(value, unit);
    }
  }
  return "just now";
}

export function formatClock(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" }).format(
    new Date(timestamp),
  );
}

/* ------------------------------------------------------------------------- */
/* Home / index aggregates                                                   */
/* ------------------------------------------------------------------------- */


/* ------------------------------------------------------------------------- */
/* Indexed-count truthfulness (task-2)                                       */
/* ------------------------------------------------------------------------- */

/**
 * True only once the engine has reported its first real filename count.
 * "Starting…" is reserved for before the first status / while the first scan
 * has not produced entries; a numeric 0 is shown once the engine reports 0.
 */
export function hasIndexCount(status: EngineStatus | null): boolean {
  if (!status) return false;
  return !(status.state === "starting" && status.filenameEntries === 0);
}

/** The only app-level indexed-item count: EngineStatus.filenameEntries. */
export function indexCountText(status: EngineStatus | null): string {
  if (!hasIndexCount(status) || status === null) return "Starting…";
  return formatNumber(status.filenameEntries);
}

/** Source row count text; never "0 items" before a source reports. */
export function sourceStatusText(source: SourceStatus, enabled: boolean): string {
  if (!enabled) return "Off";
  if (source.state === "starting" && source.itemCount === 0) return "Starting…";
  return formatNumber(source.itemCount) + " items · " + stateLabel(source.state);
}

export interface WarningSummary {
  body: string;
  details: string;
}

/**
 * Engine content warnings can arrive as many per-file strings (e.g. malformed
 * PDFs). The surface shows one human notice; raw strings may contain paths, so
 * only an aggregate count is exposed in the Details disclosure (task-7/lead).
 */
export function summarizeWarnings(warnings: string[]): WarningSummary | null {
  if (warnings.length === 0) return null;
  const count = warnings.length;
  const plural = new Intl.PluralRules(undefined, { type: "cardinal" }).select(count);
  const unit = plural === "one" ? "item" : "items";
  return {
    body: "Some items couldn't be indexed. Search still works with what is available.",
    details:
      count +
      " " +
      unit +
      " could not be read during the last index update. They are retried automatically on the next update; malformed documents are skipped so they cannot block other results.",
  };
}

/** Root row text; "Starting…" instead of "0 items" before the root reports. */
export function rootStatusText(root: RootStatus): string {
  if (root.state === "starting" && root.indexed === 0) return "Starting…";
  return stateLabel(root.state) + " · " + formatNumber(root.indexed) + " items";
}

export interface IndexCounts {
  total: number;
  documents: number;
  applications: number;
  imagesOcr: number;
  otherFiles: number;
  sources: number;
}

/**
 * Real counts only. "Applications" comes from the app source status; "Other
 * files" is filenames without extracted text (derived, clamped at zero) — it is
 * never a fabricated bucket.
 */
export function indexCounts(status: EngineStatus | null): IndexCounts {
  if (!status) {
    return { total: 0, documents: 0, applications: 0, imagesOcr: 0, otherFiles: 0, sources: 0 };
  }
  const applications = status.sources.find((source) => source.id === "apps")?.itemCount ?? 0;
  const documents = status.contentDocuments;
  const total = status.filenameEntries;
  return {
    total,
    documents,
    applications,
    imagesOcr: status.ocrDocuments,
    otherFiles: Math.max(0, total - documents),
    sources: status.sources.length,
  };
}

export interface AgentView {
  tone: Tone;
  stateLabel: string;
  detail: string;
  runtime: string;
  model: string;
  installed: boolean;
}

/**
 * Pi is optional and may not exist in this build. Never report
 * "running" unless the engine actually says so.
 */
export function describeAgent(
  agent: AgentStatus | null | undefined,
  settings: Settings,
): AgentView {
  const state = agent?.state ?? "not_configured";
  // Engine detail strings are never rendered: they can contain paths, codes or
  // provider traces. The UI shows product copy only (SCREENS §3.7 / REVIEW_2 F1).
  const runtime = agent?.runtime ?? "Not installed";
  const model = agent?.model ?? "Not configured";

  if (state === "running") {
    return {
      tone: "ok",
      stateLabel: "Running",
      detail: "Pi is running the current workflow.",
      runtime,
      model,
      installed: true,
    };
  }
  if (state === "stopped") {
    return {
      tone: "ok",
      stateLabel: "Configured, idle",
      detail: "The runtime is installed and starts only when you run a workflow.",
      runtime,
      model,
      installed: true,
    };
  }
  if (state === "error") {
    return {
      tone: "error",
      stateLabel: "Unavailable",
      detail: "Search still works. The optional runtime didn't start.",
      runtime,
      model,
      installed: true,
    };
  }
  if (settings.agent.enabled) {
    return {
      tone: "warn",
      stateLabel: "Not set up",
      detail:
        "Your preference is saved, but no agent runtime is installed in this build. Nothing was downloaded or started.",
      runtime,
      model,
      installed: false,
    };
  }
  return {
    tone: "off",
    stateLabel: "Off",
    detail: "ProtonSearch works fully without them.",
    runtime,
    model,
    installed: false,
  };
}