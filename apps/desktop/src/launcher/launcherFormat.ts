/**
 * Locale-aware recency formatting for the launcher and preview (M10).
 * Uses Intl.RelativeTimeFormat / Intl.DateTimeFormat — never hard-coded English.
 */

const RELATIVE = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });
const DATE_SHORT = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" });
const DATE_FULL = new Intl.DateTimeFormat(undefined, {
  year: "numeric",
  month: "short",
  day: "numeric",
});
const DATE_TIME = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Short localized recency: "now", "2 minutes ago", "yesterday", "Mar 4". */
export function formatRecency(
  timestampMs: number | null | undefined,
  now: number = Date.now(),
): string {
  if (!timestampMs || !Number.isFinite(timestampMs)) return "";
  const deltaSeconds = Math.round((timestampMs - now) / 1000);
  const absolute = Math.abs(deltaSeconds);
  if (absolute < MINUTE) return RELATIVE.format(deltaSeconds, "second");
  if (absolute < HOUR) return RELATIVE.format(Math.round(deltaSeconds / MINUTE), "minute");
  if (absolute < DAY) return RELATIVE.format(Math.round(deltaSeconds / HOUR), "hour");
  if (absolute < 7 * DAY) return RELATIVE.format(Math.round(deltaSeconds / DAY), "day");
  const date = new Date(timestampMs);
  const sameYear = date.getFullYear() === new Date(now).getFullYear();
  return (sameYear ? DATE_SHORT : DATE_FULL).format(date);
}

/** Full localized date + time for tooltips. */
export function formatFullDate(timestampMs: number | null | undefined): string | undefined {
  if (!timestampMs || !Number.isFinite(timestampMs)) return undefined;
  return DATE_TIME.format(timestampMs);
}
