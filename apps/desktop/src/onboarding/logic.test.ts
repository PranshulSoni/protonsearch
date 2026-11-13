import { describe, expect, it } from "vitest";
import type { Settings } from "../bridge/types";
import { ONBOARDING_STEPS, buildOnboardingInput, primaryLabel } from "./logic";

function makeSettings(overrides: Partial<Settings> = {}): Settings {
  return {
    schemaVersion: 1,
    userName: null,
    hotkey: "Alt+Space",
    theme: "system",
    density: "comfortable",
    motion: "system",
    launchAtLogin: true,
ocrEngine: "windows",
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
    onboardingComplete: false,
    legacyMigrated: false,
    webSearchEnabled: true,
    webSearchEngine: "google",
    launcher: {
    primaryMode: "normal",
    island: {
      enabled: true,
      placement: {
        monitor: null,
        anchor: "top_center",
        offsetX: 0,
        offsetY: 22,
        keepInWorkArea: true,
        autoFlipExpanded: true,
      },
      cards: [
        { id: "search", enabled: true },
        { id: "media", enabled: true },
        { id: "clipboard", enabled: true },
        { id: "agent", enabled: true },
        { id: "controls", enabled: true },
        { id: "browser", enabled: true },
        { id: "git", enabled: true },
        { id: "bell", enabled: true },
      ],
      defaultCard: "search",
      showDots: true,
      hideOnFocusLoss: true,
      skipDisabled: true,
      returnToSearch: true,
    },
  },
  clipboard: {
    captureText: true,
    captureImages: true,
    retentionDays: 30,
    maxItems: 500,
    pause: false,
    excludeApps: [],
    clearOnExit: false,
  },
  resultLimit: 30,
    ...overrides,
  };
}

describe("onboarding step model", () => {
  it("has the six steps in order", () => {
    expect(ONBOARDING_STEPS.map((step) => step.id)).toEqual([
      "welcome",
      "name",
      "ocr",
      "appearance",
      "shortcut",
      "done",
    ]);
  });

  it("labels the primary action per step", () => {
    expect(primaryLabel("welcome")).toBe("Get started");
    expect(primaryLabel("ocr")).toBe("Continue");
    expect(primaryLabel("done")).toBe("Open launcher");
  });
});

describe("buildOnboardingInput", () => {
  const roots = [{ path: "D:\\Work", mode: "content" as const, enabled: true }];
  const suggestions = [
    { path: "C:\\Users\\Me\\Desktop", label: "Desktop", exists: true, defaultMode: "names_only" as const },
    { path: "C:\\Users\\Me\\Documents", label: "Documents", exists: true, defaultMode: "content" as const },
    { path: "C:\\Users\\Me\\Pictures", label: "Pictures", exists: false, defaultMode: "content_and_ocr" as const },
  ];

  it("completes with the saved choices and keeps folders and agent state", () => {
    const settings = makeSettings({
      userName: "  Alex  ",
      hotkey: "win+shift+s",
      ocrEngine: "tesseract",
      agent: { enabled: true, provider: null, model: null, sidecarPath: null },
    });
    settings.indexing = { ...settings.indexing, roots, ocrEnabled: false };
    expect(buildOnboardingInput(settings, suggestions)).toEqual({
      userName: "Alex",
      roots,
      contentEnabled: true,
      ocrEnabled: false,
      ocrEngine: "tesseract",
      hotkey: "Shift+Super+S",
      agentOptIn: true,
    });
  });

  it("stores null for a blank name and the default for an invalid shortcut", () => {
    const input = buildOnboardingInput(makeSettings({ userName: "   ", hotkey: "bogus" }));
    expect(input.userName).toBeNull();
    expect(input.hotkey).toBe("Alt+Space");
  });

  it("adds the suggested folders that exist, in their default mode, when none are saved", () => {
    const settings = makeSettings();
    settings.indexing = { ...settings.indexing, roots: [], contentEnabled: false };
    const input = buildOnboardingInput(settings, suggestions);
    expect(input.roots).toEqual([
      { path: "C:\\Users\\Me\\Desktop", mode: "names_only", enabled: true },
      { path: "C:\\Users\\Me\\Documents", mode: "content", enabled: true },
    ]);
    expect(input.contentEnabled).toBe(true);
    expect(buildOnboardingInput(settings).roots).toEqual([]);
  });

  it("marks only a skip as skipped", () => {
    expect(buildOnboardingInput(makeSettings()).skipped).toBeUndefined();
    expect(buildOnboardingInput(makeSettings(), [], true).skipped).toBe(true);
  });
});
