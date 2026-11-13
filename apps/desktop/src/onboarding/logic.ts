// Pure first-run onboarding logic: the six-step model and the completion
// payload. Each choice is saved through the settings API as its step is left,
// so completion marks setup done with what is stored, plus default folders
// when there are none. No React.

import type { OnboardingInput, RootConfig, RootSuggestion, Settings } from "../bridge/types";
import { DEFAULT_HOTKEY, parseHotkey } from "../app/hotkey";

export const ONBOARDING_STEPS = [
  { id: "welcome", label: "Welcome" },
  { id: "name", label: "Name" },
  { id: "ocr", label: "OCR" },
  { id: "appearance", label: "Appearance" },
  { id: "shortcut", label: "Shortcut" },
  { id: "done", label: "Done" },
] as const;

export type OnboardingStepId = (typeof ONBOARDING_STEPS)[number]["id"];

export function primaryLabel(step: OnboardingStepId): string {
  if (step === "welcome") return "Get started";
  if (step === "done") return "Open launcher";
  return "Continue";
}

/**
 * Folders indexed on completion: the saved ones, or, for a profile with none
 * yet, the engine's suggested known folders that exist (Desktop, Documents,
 * Downloads, Pictures) in their default mode, so search finds files at once.
 */
export function completionRoots(settings: Settings, suggestions: RootSuggestion[] = []): RootConfig[] {
  if (settings.indexing.roots.length > 0) return settings.indexing.roots;
  return suggestions
    .filter((suggestion) => suggestion.exists)
    .map((suggestion) => ({ path: suggestion.path, mode: suggestion.defaultMode, enabled: true }));
}

/**
 * Completion payload from the saved settings. Saved folders and agent state
 * pass through unchanged instead of being reset.
 */
export function buildOnboardingInput(
  settings: Settings,
  suggestions: RootSuggestion[] = [],
  skipped = false,
): OnboardingInput {
  const hotkey = parseHotkey(settings.hotkey);
  const roots = completionRoots(settings, suggestions);
  return {
    userName: settings.userName?.trim() || null,
    roots,
    // Added default folders (Documents is content) need content indexing on.
    contentEnabled:
      settings.indexing.contentEnabled ||
      (settings.indexing.roots.length === 0 && roots.some((root) => root.mode !== "names_only")),
    ocrEnabled: settings.indexing.ocrEnabled,
    ocrEngine: settings.ocrEngine ?? "windows",
    hotkey: hotkey.ok ? hotkey.accelerator : DEFAULT_HOTKEY,
    agentOptIn: settings.agent.enabled,
    ...(skipped ? { skipped: true } : {}),
  };
}
