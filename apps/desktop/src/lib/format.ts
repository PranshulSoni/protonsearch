export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes)) return "";
  if (bytes < 1024) return bytes + " B";
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return (value >= 10 ? value.toFixed(0) : value.toFixed(1)) + " " + units[unit];
}

export function timeAgo(timestampMs: number | null | undefined, now = Date.now()): string {
  if (!timestampMs) return "";
  const delta = Math.max(0, now - timestampMs);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (delta < minute) return "Just now";
  if (delta < hour) {
    const m = Math.round(delta / minute);
    return m + " min ago";
  }
  if (delta < day) {
    const h = Math.round(delta / hour);
    return h + (h === 1 ? " hour ago" : " hours ago");
  }
  if (delta < 2 * day) return "Yesterday";
  if (delta < 7 * day) return Math.round(delta / day) + " days ago";
  return new Date(timestampMs).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function greetingForHour(hour: number): string {
  if (hour < 5) return "Good evening";
  if (hour < 12) return "Good morning";
  if (hour < 17) return "Good afternoon";
  return "Good evening";
}

export function formatNumber(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  return value.toLocaleString();
}

export function formatRelativeDate(timestampMs: number | null | undefined): string {
  if (!timestampMs) return "";
  return new Date(timestampMs).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
