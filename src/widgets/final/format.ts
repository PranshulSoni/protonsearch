export const UNKNOWN = '--';

export function pct(value: number | null | undefined): string {
  return value == null || !Number.isFinite(value) ? UNKNOWN : `${Math.round(value)}%`;
}

export function num(value: number | null | undefined): string {
  return value == null || !Number.isFinite(value) ? UNKNOWN : new Intl.NumberFormat('en-US').format(value);
}

export function compactNumber(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return UNKNOWN;
  if (Math.abs(value) >= 1_000_000) return `${(value / 1_000_000).toFixed(1).replace(/\.0$/, '')}M`;
  if (Math.abs(value) >= 1_000) return `${(value / 1_000).toFixed(1).replace(/\.0$/, '')}K`;
  return String(value);
}

export function bytes(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return UNKNOWN;
  if (value < 1024) return `${Math.max(0, Math.round(value))} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let next = value / 1024;
  let index = 0;
  while (next >= 1024 && index < units.length - 1) {
    next /= 1024;
    index += 1;
  }
  return `${next >= 10 ? Math.round(next) : next.toFixed(1)} ${units[index]}`;
}

export function temperature(value: number | null | undefined, units: 'C' | 'F'): string {
  if (value == null || !Number.isFinite(value)) return UNKNOWN;
  const shown = units === 'F' ? (value * 9) / 5 + 32 : value;
  return `${Math.round(shown)}°`;
}

export function uptime(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return UNKNOWN;
  const totalMinutes = Math.floor(seconds / 60);
  const days = Math.floor(totalMinutes / 1440);
  const hours = Math.floor((totalMinutes % 1440) / 60);
  const minutes = totalMinutes % 60;
  if (days > 0) return `${days}d ${String(hours).padStart(2, '0')}h`;
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, '0')}m`;
  return `${minutes}m`;
}

export function durationMs(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value) || value < 0) return UNKNOWN;
  const total = Math.floor(value / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}:${String(seconds).padStart(2, '0')}`;
}

export function clockParts(
  now: number | null | undefined,
  hour12: boolean,
  showSeconds: boolean,
  timezone?: string | null,
): { time: string; suffix: string; seconds: string; date: string; timezone: string } {
  const date = new Date(now ?? Date.now());
  const rawHours = date.getHours();
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');
  const hours = hour12 ? rawHours % 12 || 12 : rawHours;
  const time = showSeconds ? `${hours}:${minutes}:${seconds}` : `${hours}:${minutes}`;
  const suffix = hour12 ? (rawHours >= 12 ? 'PM' : 'AM') : '';
  const dateLabel = new Intl.DateTimeFormat('en-US', { weekday: 'short', month: 'short', day: 'numeric' }).format(date);
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes >= 0 ? '+' : '-';
  const absolute = Math.abs(offsetMinutes);
  const computed = `GMT${sign}${Math.floor(absolute / 60)}:${String(absolute % 60).padStart(2, '0')}`;
  return {
    time,
    suffix,
    seconds,
    date: dateLabel,
    timezone: timezone?.trim() || computed,
  };
}

export function relativeTime(value: string | null | undefined): string {
  if (!value) return UNKNOWN;
  const numeric = Date.parse(value);
  if (!Number.isFinite(numeric)) return value;
  const diff = Date.now() - numeric;
  if (diff < 60_000) return 'Just now';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} min ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  const days = Math.floor(diff / 86_400_000);
  return days === 1 ? 'Yesterday' : `${days} days ago`;
}

export function fileBadge(kind: string | null | undefined): string {
  const clean = kind?.trim().toLowerCase() ?? '';
  if (!clean) return 'FILE';
  if (clean.includes('tsx') || clean.includes('jsx')) return 'TSX';
  if (clean.includes('typescript')) return 'TS';
  if (clean === 'rs' || clean.includes('rust')) return 'RS';
  if (clean === 'md' || clean.includes('markdown')) return 'MD';
  if (clean === 'js' || clean.includes('javascript')) return 'JS';
  if (clean === 'json') return 'JSON';
  if (clean === 'png' || clean === 'jpg' || clean === 'jpeg' || clean === 'webp') return 'IMG';
  if (clean === 'pdf') return 'PDF';
  return clean.slice(0, 4).toUpperCase();
}

export function clampPercent(value: number | null | undefined): number {
  if (value == null || !Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

export function usageTone(value: number | null | undefined, warnAt: number): 'normal' | 'warn' | 'critical' {
  if (value == null || !Number.isFinite(value)) return 'normal';
  if (value >= 95) return 'critical';
  if (value >= warnAt) return 'warn';
  return 'normal';
}

export function highLow(high: number | null | undefined, low: number | null | undefined, units: 'C' | 'F'): string {
  if (high == null && low == null) return 'High / low unavailable';
  return `High ${temperature(high, units)} · Low ${temperature(low, units)}`;
}
