import type { SearchScope } from "../bridge/types";

/**
 * Launcher category/filter strip. Single-select, session-remembered, wired to
 * SearchRequest.scope. Ctrl+1..9 selects a chip without moving focus out of the
 * search input (KEYBOARD_AND_A11Y keeps the input as the primary focus target).
 */

export interface ScopeChip {
  id: string;
  label: string;
  scope: SearchScope;
  shortcut: string;
}

export const SCOPE_CHIPS: readonly ScopeChip[] = [
  { id: "all", label: "ALL", scope: "all", shortcut: "Ctrl+1" },
  { id: "files", label: "FILES", scope: "files", shortcut: "Ctrl+2" },
  { id: "folders", label: "FOLDERS", scope: "folders", shortcut: "Ctrl+3" },
  { id: "content", label: "DOCUMENTS", scope: "content", shortcut: "Ctrl+4" },
  { id: "apps", label: "APPS", scope: "apps", shortcut: "Ctrl+5" },
  { id: "clipboard", label: "CLIPBOARD", scope: "clipboard", shortcut: "Ctrl+6" },
  { id: "browser", label: "WEB", scope: "browser", shortcut: "Ctrl+7" },
  { id: "git", label: "GIT", scope: "git", shortcut: "Ctrl+8" },
  { id: "actions", label: "ACTIONS", scope: "actions", shortcut: "Ctrl+9" },
];

export const DEFAULT_SCOPE: SearchScope = "all";

export const SCOPE_STORAGE_KEY = "protonsearch.launcher.scope";

export function scopeIndex(scope: SearchScope): number {
  const index = SCOPE_CHIPS.findIndex((chip) => chip.scope === scope);
  return index >= 0 ? index : 0;
}

export function readStoredScope(): SearchScope {
  try {
    const stored = window.sessionStorage.getItem(SCOPE_STORAGE_KEY);
    if (stored && SCOPE_CHIPS.some((chip) => chip.scope === stored)) {
      return stored as SearchScope;
    }
  } catch {
    /* sessionStorage can be unavailable in hardened contexts */
  }
  return DEFAULT_SCOPE;
}

export function storeScope(scope: SearchScope): void {
  try {
    window.sessionStorage.setItem(SCOPE_STORAGE_KEY, scope);
  } catch {
    /* non-fatal: the filter simply is not remembered */
  }
}