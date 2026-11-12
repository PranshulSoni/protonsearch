// Pure first-run onboarding logic (7 steps, docs/design/SCREENS.md §4):
// step model, draft state, derived depth, summary and the exact OnboardingInput
// persisted through the bridge. No React, unit-tested.

import type {
  LegacySummary,
  OnboardingInput,
  OnboardingState,
  OcrEngine,
  RootConfig,
  RootMode,
  Settings,
} from "../bridge/types";
import { OCR_ENGINE_OPTIONS } from "../bridge/types";
import { DEFAULT_HOTKEY, displayHotkey, parseHotkey } from "../app/hotkey";
import { ROOT_MODE_ORDER, ROOT_MODE_SHORT } from "../app/roots";

export const ONBOARDING_STEPS = [
  { id: "welcome", label: "Welcome" },
  { id: "name", label: "Your name" },
  { id: "locations", label: "Locations" },
  { id: "depth", label: "Search depth" },
  { id: "launcher", label: "Shortcut" },
  { id: "agent", label: "Agent" },
  { id: "ready", label: "Ready" },
] as const;

export type OnboardingStepId = (typeof ONBOARDING_STEPS)[number]["id"];

export interface OnboardingDraft {
  userName: string;
  /** Every suggested root, with enabled marking the explicit choice. */
  roots: RootConfig[];
  agentOptIn: boolean;
  /** Raw text in the shortcut field; validated on the shortcut step. */
  hotkey: string;
  launchAtLogin: boolean;
  ocrEngine: OcrEngine;
}

function initialHotkey(settings: Settings): string {
  return parseHotkey(settings.hotkey).ok ? displayHotkey(settings.hotkey) : DEFAULT_HOTKEY;
}

/**
 * Initial draft. Per SCREENS section 4.4 the suggested folders start unchecked:
 * nothing is indexed without an explicit choice. OCR is opt-in via per-root
 * depth, so there are no hidden global toggles.
 */
export function createOnboardingDraft(
  state: OnboardingState | null,
  settings: Settings,
): OnboardingDraft {
  const suggestions = state?.suggestedRoots ?? [];
  const roots: RootConfig[] =
    suggestions.length > 0
      ? suggestions.map((suggestion) => ({
          path: suggestion.path,
          mode: suggestion.defaultMode,
          enabled: false,
        }))
      : settings.indexing.roots.map((root) => ({ ...root, enabled: false }));
  return {
    userName: settings.userName ?? "",
    roots,
    agentOptIn: false,
    hotkey: initialHotkey(settings),
    launchAtLogin: settings.launchAtLogin,
    ocrEngine: settings.ocrEngine ?? "windows",
  };
}

export function selectedRoots(roots: RootConfig[]): RootConfig[] {
  return roots.filter((root) => root.enabled !== false);
}

export function countRootsByMode(roots: RootConfig[]): Record<RootMode, number> {
  const counts: Record<RootMode, number> = { names_only: 0, content: 0, content_and_ocr: 0 };
  for (const root of roots) counts[root.mode] += 1;
  return counts;
}

/** The deepest mode the user enabled. Drives the trade-off copy. */
export function deepestMode(roots: RootConfig[]): RootMode | null {
  const selected = selectedRoots(roots);
  if (selected.length === 0) return null;
  for (let index = ROOT_MODE_ORDER.length - 1; index >= 0; index -= 1) {
    const mode = ROOT_MODE_ORDER[index];
    if (selected.some((root) => root.mode === mode)) return mode;
  }
  return null;
}

/** Content extraction is on when any selected location goes beyond names. */
export function draftContentEnabled(draft: OnboardingDraft): boolean {
  return selectedRoots(draft.roots).some((root) => root.mode !== "names_only");
}

/** OCR is on only when a selected location uses the content + OCR depth. */
export function draftOcrEnabled(draft: OnboardingDraft): boolean {
  return selectedRoots(draft.roots).some((root) => root.mode === "content_and_ocr");
}

export interface OnboardingSummary {
  locationCount: number;
  roots: RootConfig[];
  contentEnabled: boolean;
  ocrEnabled: boolean;
  ocrEngine: OcrEngine;
  agentOptIn: boolean;
  launchAtLogin: boolean;
  hotkeyDisplay: string;
  hotkeyValid: boolean;
  hotkeyError: string | null;
  deepestMode: RootMode | null;
  warnings: string[];
}

/** Everything the Ready step shows, including honest partial-setup warnings. */
export function summarizeDraft(draft: OnboardingDraft): OnboardingSummary {
  const roots = selectedRoots(draft.roots);
  const hotkey = parseHotkey(draft.hotkey);
  const warnings: string[] = [];
  if (roots.length === 0) {
    warnings.push(
      "No locations are selected. Filename search will still cover applications and actions; add folders later in Search & Indexes.",
    );
  }
  return {
    locationCount: roots.length,
    roots,
    contentEnabled: draftContentEnabled(draft),
    ocrEnabled: draftOcrEnabled(draft),
    ocrEngine: draft.ocrEngine ?? "windows",
    agentOptIn: draft.agentOptIn,
    launchAtLogin: draft.launchAtLogin,
    hotkeyDisplay: hotkey.ok ? hotkey.display : draft.hotkey.trim() || DEFAULT_HOTKEY,
    hotkeyValid: hotkey.ok,
    hotkeyError: hotkey.ok ? null : hotkey.error,
    deepestMode: deepestMode(draft.roots),
    warnings,
  };
}

/** Trade-off copy for the deepest mode chosen (SCREENS section 4.5). */
export const DEPTH_TRADEOFF: Record<RootMode, string> = {
  names_only: "Instant. Smallest index. No file contents are read.",
  content:
    "Extracts text from documents, code, and text files in the background. Uses more CPU and disk.",
  content_and_ocr:
    "Reads text in screenshots and images. Uses the most CPU; results arrive later.",
};

/** Depth labels on the per-location segmented control (SCREENS section 4.5). */
export const DEPTH_LABEL: Record<RootMode, string> = {
  names_only: "Names only",
  content: "+ Content",
  content_and_ocr: "+ OCR",
};

/** Short mode label for the Ready summary. */
export const DEPTH_SUMMARY_LABEL: Record<RootMode, string> = ROOT_MODE_SHORT;

/** Whether the primary button may advance from this step. */
export function canAdvance(step: OnboardingStepId, draft: OnboardingDraft): boolean {
  if (step === "launcher") return parseHotkey(draft.hotkey).ok;
  return true;
}

/** Persist exactly this shape once, via completeOnboarding. */
export function buildOnboardingInput(draft: OnboardingDraft): OnboardingInput {
  const hotkey = parseHotkey(draft.hotkey);
  return {
    userName: draft.userName.trim() || null,
    roots: selectedRoots(draft.roots).map((root) => ({
      path: root.path,
      mode: root.mode,
      enabled: true,
    })),
    contentEnabled: draftContentEnabled(draft),
    ocrEnabled: draftOcrEnabled(draft),
    ocrEngine: draft.ocrEngine,
    hotkey: hotkey.ok ? hotkey.accelerator : DEFAULT_HOTKEY,
    agentOptIn: draft.agentOptIn,
  };
}

/** Skip setup: complete the flag, configure nothing, keep the current shortcut. */
export function buildSkippedOnboardingInput(draft?: Pick<OnboardingDraft, "hotkey">): OnboardingInput {
  const parsed = draft ? parseHotkey(draft.hotkey) : null;
  return {
    userName: null,
    roots: [],
    contentEnabled: false,
    ocrEnabled: false,
    ocrEngine: "windows",
    hotkey: parsed && parsed.ok ? parsed.accelerator : DEFAULT_HOTKEY,
    agentOptIn: false,
    skipped: true,
  };
}

export interface LegacyDetail {
  label: string;
  value: number;
}

/** Truthful legacy counts; only nonzero entries. */
export function legacyDetails(legacy: LegacySummary | null | undefined): LegacyDetail[] {
  if (!legacy?.detected) return [];
  const entries: [string, number][] = [
    ["Indexed files", legacy.indexedFiles],
    ["Document texts", legacy.contentDocuments],
    ["Browser items", legacy.browserItems],
    ["Quicklinks", legacy.quicklinks],
    ["Snippets", legacy.snippets],
    ["Saved settings", legacy.settings],
  ];
  return entries
    .filter(([, value]) => value > 0)
    .map(([label, value]) => ({ label, value }));
}

export function stepEyebrow(index: number): string {
  return "Getting started / 0" + (index + 1) + " of 0" + ONBOARDING_STEPS.length;
}

/** Exact primary-button label per step (SCREENS section 4.1). */
export function primaryLabel(step: OnboardingStepId): string {
  if (step === "welcome") return "Get started";
  if (step === "ready") return "Start using ProtonSearch";
  return "Continue";
}

/* ------------------------------------------------------------------------- */
/* UI-local draft persistence                                                */
/* ------------------------------------------------------------------------- */

/**
 * The frozen IPC contract has no per-step onboarding state, so resumability
 * lives here: the draft and current step are kept in this WebView's
 * localStorage (never sent anywhere) and cleared when setup completes.
 */
export const ONBOARDING_DRAFT_KEY = "protonsearch.onboarding.draft.v1";

export interface PersistedOnboarding {
  stepIndex: number;
  draft: OnboardingDraft;
}

export function serializeOnboardingDraft(stepIndex: number, draft: OnboardingDraft): string {
  return JSON.stringify({ stepIndex, draft });
}

/** Strict parser: unknown or malformed state is discarded, never half-applied. */
export function parsePersistedOnboarding(
  raw: string | null,
  stepCount: number = ONBOARDING_STEPS.length,
): PersistedOnboarding | null {
  if (!raw) return null;
  try {
    const data = JSON.parse(raw) as Partial<PersistedOnboarding> | null;
    if (!data || typeof data !== "object") return null;
    if (
      typeof data.stepIndex !== "number" ||
      !Number.isInteger(data.stepIndex) ||
      data.stepIndex < 0 ||
      data.stepIndex >= stepCount
    ) {
      return null;
    }
    const draft = data.draft as Partial<OnboardingDraft> | undefined;
    if (
      !draft ||
      typeof draft.userName !== "string" ||
      typeof draft.hotkey !== "string" ||
      typeof draft.agentOptIn !== "boolean" ||
      typeof draft.launchAtLogin !== "boolean" ||
      !Array.isArray(draft.roots)
    ) {
      return null;
    }
    const roots: RootConfig[] = [];
    for (const root of draft.roots) {
      if (
        !root ||
        typeof root.path !== "string" ||
        typeof root.enabled !== "boolean" ||
        !ROOT_MODE_ORDER.includes(root.mode)
      ) {
        return null;
      }
      roots.push({ path: root.path, mode: root.mode, enabled: root.enabled });
    }
    return {
      stepIndex: data.stepIndex,
      draft: {
        userName: draft.userName,
        roots,
        agentOptIn: draft.agentOptIn,
        hotkey: draft.hotkey,
        launchAtLogin: draft.launchAtLogin,
        ocrEngine: OCR_ENGINE_OPTIONS.some(
          (option) => option.engine === draft.ocrEngine && option.availability === "available",
        ) ? draft.ocrEngine! : "windows",
      },
    };
  } catch {
    return null;
  }
}
