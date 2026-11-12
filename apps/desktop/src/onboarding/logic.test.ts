import { describe, expect, it } from "vitest";
import type { OnboardingState, RootConfig, Settings } from "../bridge/types";
import {
  ONBOARDING_STEPS,
  buildOnboardingInput,
  buildSkippedOnboardingInput,
  canAdvance,
  createOnboardingDraft,
  deepestMode,
  legacyDetails,
  parsePersistedOnboarding,
  primaryLabel,
  serializeOnboardingDraft,
  selectedRoots,
  stepEyebrow,
  summarizeDraft,
} from "./logic";

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

function makeOnboardingState(overrides: Partial<OnboardingState> = {}): OnboardingState {
  return {
    complete: false,
    legacy: {
      detected: false,
      appDataPath: null,
      indexedFiles: 0,
      contentDocuments: 0,
      browserItems: 0,
      quicklinks: 0,
      snippets: 0,
      settings: 0,
    },
    suggestedRoots: [
      {
        path: "C:\\Users\\Me\\Documents",
        label: "Documents",
        exists: true,
        defaultMode: "content",
      },
      { path: "C:\\Users\\Me\\Desktop", label: "Desktop", exists: true, defaultMode: "names_only" },
      {
        path: "C:\\Users\\Me\\Pictures",
        label: "Pictures",
        exists: false,
        defaultMode: "content_and_ocr",
      },
    ],
    defaults: makeSettings(),
    ...overrides,
  };
}

describe("onboarding step model", () => {
  it("has the seven binding steps in order", () => {
    expect(ONBOARDING_STEPS.map((step) => step.id)).toEqual([
      "welcome",
      "name",
      "locations",
      "depth",
      "launcher",
      "agent",
      "ready",
    ]);
  });

  it("labels the primary action per step", () => {
    expect(primaryLabel("welcome")).toBe("Get started");
    expect(primaryLabel("agent")).toBe("Continue");
    expect(primaryLabel("ready")).toBe("Start using ProtonSearch");
  });

  it("formats the step eyebrow", () => {
    expect(stepEyebrow(0)).toBe("Getting started / 01 of 07");
    expect(stepEyebrow(6)).toBe("Getting started / 07 of 07");
  });
});

describe("createOnboardingDraft", () => {
  it("starts with suggested folders present but unchecked (explicit choice)", () => {
    const draft = createOnboardingDraft(makeOnboardingState(), makeSettings());
    expect(draft.roots).toEqual([
      { path: "C:\\Users\\Me\\Documents", mode: "content", enabled: false },
      { path: "C:\\Users\\Me\\Desktop", mode: "names_only", enabled: false },
      { path: "C:\\Users\\Me\\Pictures", mode: "content_and_ocr", enabled: false },
    ]);
    expect(selectedRoots(draft.roots)).toEqual([]);
    expect(draft.agentOptIn).toBe(false);
    expect(draft.hotkey).toBe("Alt+Space");
    expect(draft.launchAtLogin).toBe(true);
  });

  it("carries an existing name and a valid stored shortcut", () => {
    const settings = makeSettings({ userName: "Pranshul", hotkey: "Super+Space" });
    const draft = createOnboardingDraft(makeOnboardingState(), settings);
    expect(draft.userName).toBe("Pranshul");
    expect(draft.hotkey).toBe("Win+Space");
  });

  it("falls back to the default shortcut when the stored value is invalid", () => {
    const draft = createOnboardingDraft(makeOnboardingState(), makeSettings({ hotkey: "bogus" }));
    expect(draft.hotkey).toBe("Alt+Space");
  });

  it("uses configured roots when the engine suggests none", () => {
    const settings = makeSettings({
      indexing: {
        roots: [{ path: "D:\\Work", mode: "names_only", enabled: true }],
        excludedPaths: [],
        contentEnabled: false,
        ocrEnabled: false,
        respectPowerState: true,
        paused: false,
      },
    });
    const draft = createOnboardingDraft(makeOnboardingState({ suggestedRoots: [] }), settings);
    expect(draft.roots).toEqual([{ path: "D:\\Work", mode: "names_only", enabled: false }]);
  });
});

describe("depth derivation", () => {
  const roots: RootConfig[] = [
    { path: "C:\\A", mode: "names_only", enabled: true },
    { path: "C:\\B", mode: "content", enabled: true },
    { path: "C:\\C", mode: "content_and_ocr", enabled: false },
  ];

  it("returns the deepest selected mode only", () => {
    expect(deepestMode(roots)).toBe("content");
    expect(deepestMode([])).toBeNull();
  });

  it("derives global content/OCR flags from selected roots", () => {
    const state = makeOnboardingState();
    const draft = {
      ...createOnboardingDraft(state, makeSettings()),
      roots: [
        { path: "C:\\A", mode: "names_only" as const, enabled: true },
        { path: "C:\\B", mode: "content_and_ocr" as const, enabled: true },
      ],
    };
    const summary = summarizeDraft(draft);
    expect(summary.contentEnabled).toBe(true);
    expect(summary.ocrEnabled).toBe(true);
    expect(summary.deepestMode).toBe("content_and_ocr");
    expect(summary.locationCount).toBe(2);
  });
});

describe("summarizeDraft", () => {
  it("warns honestly when no locations are selected", () => {
    const draft = createOnboardingDraft(makeOnboardingState(), makeSettings());
    const summary = summarizeDraft(draft);
    expect(summary.locationCount).toBe(0);
    expect(summary.warnings[0]).toMatch(/No locations are selected/);
    expect(summary.contentEnabled).toBe(false);
    expect(summary.ocrEnabled).toBe(false);
  });

  it("reports an invalid shortcut without pretending it is fine", () => {
    const draft = { ...createOnboardingDraft(makeOnboardingState(), makeSettings()), hotkey: "K" };
    const summary = summarizeDraft(draft);
    expect(summary.hotkeyValid).toBe(false);
    expect(summary.hotkeyError).toMatch(/modifier/);
    expect(summary.hotkeyDisplay).toBe("K");
  });

  it("shows name, agent opt-in and Windows start-up state", () => {
    const draft = {
      ...createOnboardingDraft(makeOnboardingState(), makeSettings()),
      userName: "  Alex  ",
      agentOptIn: true,
      launchAtLogin: false,
    };
    const summary = summarizeDraft(draft);
    expect(summary.agentOptIn).toBe(true);
    expect(summary.launchAtLogin).toBe(false);
  });
});

describe("canAdvance", () => {
  it("blocks only the shortcut step while the shortcut is invalid", () => {
    const draft = { ...createOnboardingDraft(makeOnboardingState(), makeSettings()), hotkey: "bogus" };
    expect(canAdvance("launcher", draft)).toBe(false);
    expect(canAdvance("welcome", draft)).toBe(true);
    expect(canAdvance("name", draft)).toBe(true);
    expect(canAdvance("locations", draft)).toBe(true);
    expect(canAdvance("agent", draft)).toBe(true);
    expect(canAdvance("ready", draft)).toBe(true);
    expect(canAdvance("launcher", { ...draft, hotkey: "Alt+Space" })).toBe(true);
  });
});

describe("buildOnboardingInput", () => {
  it("persists exactly the selected roots and canonical values", () => {
    const draft = {
      ...createOnboardingDraft(makeOnboardingState(), makeSettings()),
      userName: "  Alex  ",
      hotkey: "win+shift+s",
      agentOptIn: true,
      launchAtLogin: false,
      roots: [
        { path: "C:\\Users\\Me\\Documents", mode: "content" as const, enabled: true },
        { path: "C:\\Users\\Me\\Desktop", mode: "content_and_ocr" as const, enabled: true },
        { path: "C:\\Users\\Me\\Pictures", mode: "content_and_ocr" as const, enabled: false },
      ],
    };
    expect(buildOnboardingInput(draft)).toEqual({
      userName: "Alex",
      roots: [
        { path: "C:\\Users\\Me\\Documents", mode: "content", enabled: true },
        { path: "C:\\Users\\Me\\Desktop", mode: "content_and_ocr", enabled: true },
      ],
      contentEnabled: true,
      ocrEnabled: true,
ocrEngine: "windows",
      hotkey: "Shift+Super+S",
      agentOptIn: true,
    });
  });

  it("stores null for an empty name", () => {
    const draft = {
      ...createOnboardingDraft(makeOnboardingState(), makeSettings()),
      userName: "   ",
    };
    expect(buildOnboardingInput(draft).userName).toBeNull();
  });

  it("does not set skipped for a real completion", () => {
    const draft = createOnboardingDraft(makeOnboardingState(), makeSettings());
    expect(buildOnboardingInput(draft).skipped).toBeUndefined();
  });

  it("builds the mission-defined skip payload with no roots", () => {
    const draft = { hotkey: "win+space" };
    expect(buildSkippedOnboardingInput(draft)).toEqual({
      userName: null,
      roots: [],
      contentEnabled: false,
      ocrEnabled: false,
ocrEngine: "windows",
      hotkey: "Super+Space",
      agentOptIn: false,
      skipped: true,
    });
  });

  it("falls back to the default shortcut when skipping with invalid text", () => {
    expect(buildSkippedOnboardingInput({ hotkey: "bogus" }).hotkey).toBe("Alt+Space");
    expect(buildSkippedOnboardingInput().hotkey).toBe("Alt+Space");
  });
});

describe("legacyDetails", () => {
  it("returns nothing when no legacy install is detected", () => {
    expect(legacyDetails(null)).toEqual([]);
    expect(legacyDetails(makeOnboardingState().legacy)).toEqual([]);
  });

  it("lists only nonzero legacy counts", () => {
    const details = legacyDetails({
      detected: true,
      appDataPath: "C:\\Users\\Me\\AppData\\Roaming\\protonsearch",
      indexedFiles: 89813,
      contentDocuments: 14609,
      browserItems: 4523,
      quicklinks: 0,
      snippets: 1,
      settings: 2,
    });
    expect(details.map((entry) => entry.label)).toEqual([
      "Indexed files",
      "Document texts",
      "Browser items",
      "Snippets",
      "Saved settings",
    ]);
    expect(details[0].value).toBe(89813);
  });
});

describe("draft persistence", () => {
  const draft = {
    ...createOnboardingDraft(makeOnboardingState(), makeSettings()),
    roots: [{ path: "C:\\Users\\Me\\Documents", mode: "content" as const, enabled: true }],
    userName: "Alex",
  };

  it("round-trips the current step and draft", () => {
    const raw = serializeOnboardingDraft(3, draft);
    expect(parsePersistedOnboarding(raw)).toEqual({ stepIndex: 3, draft });
  });

  it("rejects malformed or out-of-range state instead of half-applying it", () => {
    expect(parsePersistedOnboarding(null)).toBeNull();
    expect(parsePersistedOnboarding("not json")).toBeNull();
    expect(parsePersistedOnboarding(serializeOnboardingDraft(9, draft))).toBeNull();
    expect(parsePersistedOnboarding(JSON.stringify({ stepIndex: 1, draft: { roots: [] } }))).toBeNull();
    expect(parsePersistedOnboarding(JSON.stringify({ stepIndex: 1, draft: { ...draft, ocrEngine: "bogus" } }))?.draft.ocrEngine).toBe("windows");
    expect(
      parsePersistedOnboarding(
        JSON.stringify({
          stepIndex: 1,
          draft: { ...draft, roots: [{ path: "C:\\X", mode: "sideways", enabled: true }] },
        }),
      ),
    ).toBeNull();
  });
});
