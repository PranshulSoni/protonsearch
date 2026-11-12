import { describe, expect, it } from "vitest";
import { formatFullDate, formatRecency } from "./launcherFormat";

const NOW = Date.UTC(2026, 10, 9, 12, 0, 0);
const relative = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

describe("formatRecency", () => {
  it("uses Intl.RelativeTimeFormat for seconds/minutes/hours/days", () => {
    expect(formatRecency(NOW - 30_000, NOW)).toBe(relative.format(-30, "second"));
    expect(formatRecency(NOW - 60_000, NOW)).toBe(relative.format(-1, "minute"));
    expect(formatRecency(NOW - 2 * 3_600_000, NOW)).toBe(relative.format(-2, "hour"));
    expect(formatRecency(NOW - 3 * 86_400_000, NOW)).toBe(relative.format(-3, "day"));
  });

  it("falls back to a localized date after a week", () => {
    const older = NOW - 30 * 86_400_000;
    expect(formatRecency(older, NOW)).toBe(
      new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" }).format(older),
    );
  });

  it("returns empty strings without a timestamp", () => {
    expect(formatRecency(null)).toBe("");
    expect(formatRecency(undefined)).toBe("");
    expect(formatFullDate(null)).toBeUndefined();
    expect(typeof formatFullDate(NOW)).toBe("string");
  });
});
