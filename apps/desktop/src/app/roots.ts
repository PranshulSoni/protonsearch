// Pure index-root helpers shared by the desktop app and first-run onboarding.
// Path rules: Windows absolute paths only (drive or UNC); native folder pickers
// are not available yet, so the UI validates pasted text honestly.

import type { RootConfig, RootMode, RootSuggestion } from "../bridge/types";

export const ROOT_MODE_ORDER: readonly RootMode[] = ["names_only", "content", "content_and_ocr"];

export const ROOT_MODE_LABEL: Record<RootMode, string> = {
  names_only: "Names only",
  content: "Names + content",
  content_and_ocr: "Names + content + OCR",
};

export const ROOT_MODE_SHORT: Record<RootMode, string> = {
  names_only: "Names",
  content: "Content",
  content_and_ocr: "Content + OCR",
};

export const ROOT_MODE_HINT: Record<RootMode, string> = {
  names_only: "Filenames and folder paths only. No file contents are read, so this is the lightest mode.",
  content: "Adds text extracted from documents and code. Extraction runs in the background; searches stay fast.",
  content_and_ocr: "Also extracts text from images and screenshots. The heaviest mode; OCR never runs during a search.",
};

export function cycleRootMode(mode: RootMode): RootMode {
  const index = ROOT_MODE_ORDER.indexOf(mode);
  return ROOT_MODE_ORDER[(index + 1) % ROOT_MODE_ORDER.length];
}

export function setRootMode(roots: RootConfig[], path: string, mode: RootMode): RootConfig[] {
  return roots.map((root) => (samePath(root.path, path) ? { ...root, mode } : root));
}

export function toggleRootEnabled(roots: RootConfig[], path: string): RootConfig[] {
  return roots.map((root) =>
    samePath(root.path, path) ? { ...root, enabled: !(root.enabled !== false) } : root,
  );
}

export function removeRoot(roots: RootConfig[], path: string): RootConfig[] {
  return roots.filter((root) => !samePath(root.path, path));
}

/**
 * Normalise a pasted path: trim, drop Explorer's surrounding quotes, use
 * backslashes, collapse duplicate separators and trailing separators (except a
 * drive root such as C:\).
 */
export function normalizeWindowsPath(raw: string): string {
  let path = raw.trim();
  if (path.length >= 2 && path.startsWith('"') && path.endsWith('"')) {
    path = path.slice(1, -1).trim();
  }
  if (!path) return "";
  path = path.replace(/\//g, "\\");

  if (/^[a-zA-Z]:$/.test(path)) return path + "\\";

  if (path.startsWith("\\\\")) {
    const rest = path.slice(2).replace(/\\{2,}/g, "\\");
    let result = "\\\\" + rest;
    while (result.length > 2 && result.endsWith("\\")) result = result.slice(0, -1);
    return result;
  }

  path = path.replace(/\\{2,}/g, "\\");
  while (path.length > 3 && path.endsWith("\\")) path = path.slice(0, -1);
  return path;
}

export function isAbsoluteWindowsPath(path: string): boolean {
  return /^[a-zA-Z]:\\/.test(path) || /^\\\\[^\\]+\\[^\\]+/.test(path);
}

/** Windows paths compare case-insensitively; ignore a trailing separator. */
export function pathKey(path: string): string {
  return normalizeWindowsPath(path).replace(/\\+$/, "").toLowerCase();
}

export function samePath(a: string, b: string): boolean {
  return pathKey(a) === pathKey(b);
}

/** Human label for a configured root ("Documents"), falling back to the path. */
export function rootDisplayName(path: string): string {
  const normalized = normalizeWindowsPath(path);
  const parts = normalized.split("\\").filter(Boolean);
  return parts[parts.length - 1] ?? normalized;
}

export interface PathValidation {
  ok: boolean;
  path: string;
  error?: string;
}

export function validateAbsolutePath(
  existing: string[],
  raw: string,
  duplicateMessage: string,
): PathValidation {
  const path = normalizeWindowsPath(raw);
  if (!path) {
    return { ok: false, path, error: "Enter a folder path." };
  }
  if (!isAbsoluteWindowsPath(path)) {
    return {
      ok: false,
      path,
      error: "Enter a full folder path, for example C:\\Users\\You\\Documents.",
    };
  }
  if (existing.some((entry) => samePath(entry, path))) {
    return { ok: false, path, error: duplicateMessage };
  }
  return { ok: true, path };
}

export function validateRootPath(roots: RootConfig[], raw: string): PathValidation {
  return validateAbsolutePath(
    roots.map((root) => root.path),
    raw,
    "That folder is already in the list.",
  );
}

export function validateExcludedPath(excludedPaths: string[], raw: string): PathValidation {
  return validateAbsolutePath(excludedPaths, raw, "That folder is already excluded.");
}

export interface AddRootResult {
  ok: boolean;
  roots: RootConfig[];
  path?: string;
  error?: string;
}

export function addRoot(
  roots: RootConfig[],
  raw: string,
  mode: RootMode = "names_only",
): AddRootResult {
  const validation = validateRootPath(roots, raw);
  if (!validation.ok) return { ok: false, roots, error: validation.error };
  return {
    ok: true,
    roots: [...roots, { path: validation.path, mode, enabled: true }],
    path: validation.path,
  };
}

/** Suggested roots that are not configured yet, for quick-add chips. */
export function unusedSuggestions(
  suggestions: RootSuggestion[],
  roots: RootConfig[],
): RootSuggestion[] {
  return suggestions.filter(
    (suggestion) => suggestion.exists && !roots.some((root) => samePath(root.path, suggestion.path)),
  );
}

/** Default selection when first-run setup opens: every existing suggestion. */
export function rootsFromSuggestions(suggestions: RootSuggestion[]): RootConfig[] {
  return suggestions.map((suggestion) => ({
    path: suggestion.path,
    mode: suggestion.defaultMode,
    enabled: suggestion.exists,
  }));
}