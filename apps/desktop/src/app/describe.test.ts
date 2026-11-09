import { describe, expect, it } from "vitest";
import type {
  ActivityItem,
  EngineStatus,
  Settings,
  SourceStatus,
} from "../bridge/types";
import {
  describeAgent,
  describeSource,
  formatClock,
  greetingForTimestamp,
  greetingLine,
  groupActivityByDay,
  indexCounts,
  indexHealth,
  recentAccessed,
  relativeTime,
  stateLabel,
} from "./describe";

function makeSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    schemaVersion: 1,
    userName: null,
    hotkey: "Alt+Space",
    theme: "system",
    density: "comfortable",
    motion: "system",
    launchAtLogin: true,
    indexing: {
      roots: [],
      excludedPaths: [],
      contentEnabled: true,
      ocrEnabled: true,
      respectPowerState: true,
      paused: false,
    },
    privacy: {
      activityHistory: true,
      clipboardHistory: true,
      browserHistory: true,
      gitIndexing: true,
      telemetry: false,
    },
    agent: { enabled: false, provider: null, model: null, sidecarPath: null },
    semantic: { enabled: false, model: null },
    onboardingComplete: true,
    legacyMigrated: false,
    resultLimit: 30,
    ...overrides,
  };
}

function makeStatus(overrides: Partial<EngineStatus> = {}): EngineStatus {
  return {
    protocol: 1,
    version: "2.0.0-alpha.1",
    state: "ready",
    filenameEntries: 124318,
    contentDocuments: 14609,
    ocrDocuments: 8440,
    roots: [],
    sources: [
      { id: "apps", label: "Applications", enabled: true, itemCount: 126, state: "ready" },
      {
        id: "browser",
        label: "Browser history",
        enabled: true,
        itemCount: 4523,
        state: "ready",
      },
    ],
    agent: null,
    lastUpdated: 0,
    warnings: [],
    ...overrides,
  };
}

function item(id: number, kind: ActivityItem["kind"], timestamp: number, title = "Title"): ActivityItem {
  return { id, kind, timestamp, title };
}

describe("indexHealth", () => {
  it("reports ready, indexing, paused, starting and error honestly", () => {
    expect(indexHealth(makeStatus())).toMatchObject({ tone: "ok", label: "Index ready" });
    expect(indexHealth(makeStatus({ state: "indexing" }))).toMatchObject({ tone: "busy" });
    expect(indexHealth(makeStatus({ state: "paused" }))).toMatchObject({ tone: "warn", label: "Indexing paused" });
    expect(indexHealth(makeStatus({ state: "starting" }))).toMatchObject({ tone: "busy", label: "Starting" });
    expect(indexHealth(makeStatus({ state: "error" }))).toMatchObject({ tone: "error", label: "Index error" });
  });

  it("uses the first engine warning without inventing one", () => {
    const health = indexHealth(
      makeStatus({ warnings: ["Pictures root is unavailable (device disconnected)"] }),
    );
    expect(health).toMatchObject({ tone: "warn", label: "Needs attention" });
    expect(health.detail).toContain("Pictures root");
  });

  it("handles a missing status", () => {
    expect(indexHealth(null)).toMatchObject({ tone: "off", label: "Waiting for engine" });
  });
});

describe("stateLabel", () => {
  it("sentence-cases engine states", () => {
    expect(stateLabel("ready")).toBe("Ready");
    expect(stateLabel("unavailable")).toBe("Unavailable");
    expect(stateLabel("weird")).toBe("Weird");
    expect(stateLabel("")).toBe("Unknown");
  });
});

describe("sources", () => {
  const browser: SourceStatus = {
    id: "browser",
    label: "Browser history & bookmarks",
    enabled: true,
    itemCount: 4523,
    state: "ready",
  };

  it("maps browser privacy to settings and respects a disabled switch", () => {
    const enabled = describeSource(browser, makeSettings());
    expect(enabled).toMatchObject({ privacyKey: "browserHistory", tone: "ok", enabled: true });

    const disabled = describeSource(
      browser,
      makeSettings({
        privacy: {
          activityHistory: true,
          clipboardHistory: true,
          browserHistory: false,
          gitIndexing: true,
          telemetry: false,
        },
      }),
    );
    expect(disabled).toMatchObject({ tone: "off", enabled: false, stateLabel: "Off" });
  });

  it("treats apps/recent as always-on sources", () => {
    const apps = describeSource(
      { id: "apps", label: "Applications", enabled: true, itemCount: 126, state: "ready" },
      makeSettings(),
    );
    expect(apps.privacyKey).toBeNull();
    expect(apps.tone).toBe("ok");
  });

  it("falls back to engine labels for unknown sources", () => {
    const custom = describeSource(
      { id: "custom", label: "Custom source", enabled: true, itemCount: 1, state: "error", detail: "Boom" },
      makeSettings(),
    );
    expect(custom.title).toBe("Custom source");
    expect(custom.tone).toBe("error");
  });
});

describe("greeting", () => {
  const at = (hour: number, minute = 0) => new Date(2026, 4, 20, hour, minute, 0, 0).getTime();

  it("uses the 05-11 / 12-16 / 17-04 local buckets", () => {
    expect(greetingForTimestamp(at(5))).toBe("Good morning");
    expect(greetingForTimestamp(at(11, 59))).toBe("Good morning");
    expect(greetingForTimestamp(at(12))).toBe("Good afternoon");
    expect(greetingForTimestamp(at(16, 59))).toBe("Good afternoon");
    expect(greetingForTimestamp(at(17))).toBe("Good evening");
    expect(greetingForTimestamp(at(23))).toBe("Good evening");
    expect(greetingForTimestamp(at(4, 59))).toBe("Good evening");
  });

  it("adds the optional name with correct punctuation", () => {
    expect(greetingLine("Pranshul", at(9))).toBe("Good morning, Pranshul.");
    expect(greetingLine("  Alex  ", at(14))).toBe("Good afternoon, Alex.");
    expect(greetingLine(null, at(20))).toBe("Good evening.");
    expect(greetingLine("", at(20))).toBe("Good evening.");
  });
});

describe("relative time and clock", () => {
  const now = new Date(2026, 4, 20, 12, 0, 0, 0).getTime();

  it("formats recent timestamps relative to now", () => {
    expect(relativeTime(now - 5 * 60_000, now)).toMatch(/5/);
    expect(relativeTime(now, now)).toMatch(/now/i);
    expect(relativeTime(null)).toBe("Never");
  });

  it("formats a local clock time", () => {
    expect(formatClock(now)).toMatch(/\d/);
  });
});

describe("activity grouping", () => {
  const now = new Date(2026, 4, 20, 12, 0, 0, 0).getTime();
  const yesterday = new Date(2026, 4, 19, 9, 0, 0, 0).getTime();
  const older = new Date(2026, 4, 12, 9, 0, 0, 0).getTime();

  it("groups today, yesterday and older dates newest-first", () => {
    const groups = groupActivityByDay(
      [
        item(1, "search", now - 60_000),
        item(2, "open", now - 120_000),
        item(3, "launch", yesterday),
        item(4, "index", older),
      ],
      now,
    );
    expect(groups.map((group) => group.label)).toEqual(["Today", "Yesterday", groups[2].label]);
    expect(groups[0].items.map((entry) => entry.id)).toEqual([1, 2]);
    expect(groups[2].label.length).toBeGreaterThan(0);
  });

  it("sorts items inside a day newest-first", () => {
    const groups = groupActivityByDay(
      [item(1, "search", now - 5 * 60_000), item(2, "open", now - 60_000)],
      now,
    );
    expect(groups[0].items.map((entry) => entry.id)).toEqual([2, 1]);
  });
});

describe("recentAccessed", () => {
  const now = Date.now();
  it("includes only opened/launched items and caps at three by default", () => {
    const items: ActivityItem[] = [
      item(1, "search", now, "query"),
      item(2, "open", now - 1, "A"),
      item(3, "launch", now - 2, "B"),
      item(4, "open", now - 3, "C"),
      item(5, "open", now - 4, "D"),
    ];
    expect(recentAccessed(items).map((entry) => entry.id)).toEqual([2, 3, 4]);
  });

  it("deduplicates repeated opens of the same item", () => {
    const items: ActivityItem[] = [
      { ...item(1, "open", now, "Notes"), path: "C:\\Notes.txt" },
      { ...item(2, "open", now - 1, "Notes"), path: "C:\\Notes.txt" },
      { ...item(3, "open", now - 2, "Other"), path: "C:\\Other.txt" },
    ];
    expect(recentAccessed(items).map((entry) => entry.id)).toEqual([1, 3]);
  });
});

describe("indexCounts", () => {
  it("derives real counts and clamps the other-files bucket", () => {
    const counts = indexCounts(makeStatus());
    expect(counts.total).toBe(124318);
    expect(counts.documents).toBe(14609);
    expect(counts.applications).toBe(126);
    expect(counts.imagesOcr).toBe(8440);
    expect(counts.otherFiles).toBe(109709);
    expect(counts.sources).toBe(2);

    const clamped = indexCounts(makeStatus({ filenameEntries: 10, contentDocuments: 20 }));
    expect(clamped.otherFiles).toBe(0);
  });
});

describe("describeAgent", () => {
  it("never pretends the runtime exists when it is not configured", () => {
    const view = describeAgent(null, makeSettings());
    expect(view).toMatchObject({ tone: "off", stateLabel: "Not set up", installed: false });
  });

  it("warns when the preference is saved but no runtime is installed", () => {
    const view = describeAgent(null, makeSettings({ agent: { enabled: true } }));
    expect(view.tone).toBe("warn");
    expect(view.detail).toMatch(/preference is saved/);
  });

  it("reports running and error states from the engine", () => {
    expect(
      describeAgent(
        { state: "running", runtime: "python 3.13", model: "local" },
        makeSettings({ agent: { enabled: true } }),
      ),
    ).toMatchObject({ tone: "ok", stateLabel: "Running", installed: true });

    expect(
      describeAgent({ state: "error", detail: "Sidecar exited with code 1" }, makeSettings()),
    ).toMatchObject({ tone: "error", stateLabel: "Couldn't start" });
  });
});
