import type { SearchResult, SearchScope } from "../bridge/types";

/**
 * Launcher category/filter strip. Single-select, not persisted across summons
 * (default ALL). Ctrl+1..7 selects a chip without moving focus out of the search
 * input. Removed DOCUMENTS and GIT scopes stay reachable through query prefixes
 * and are no longer chips (SCREENS §2.11).
 */

export interface ScopeChip {
  id: string;
  label: string;
  scope: SearchScope;
  shortcut: string;
}

// v10 normal launcher: only four visible filters (PROTONSEARCH_V2_FINAL_MASTER
// §5). Every other source stays reachable through prefixes/quick destinations.
const BASE_SCOPES: Array<{ id: string; label: string; scope: SearchScope }> = [
  { id: "all", label: "ALL", scope: "all" },
  { id: "files", label: "FILES", scope: "files" },
  { id: "folders", label: "FOLDERS", scope: "folders" },
  { id: "apps", label: "APPS", scope: "apps" },
];

export const SCOPE_CHIPS: readonly ScopeChip[] = BASE_SCOPES.map((chip, index) => ({
  ...chip,
  shortcut: "Ctrl+" + (index + 1),
}));

export const DEFAULT_SCOPE: SearchScope = "all";

/** Query prefixes supported by the engine parser (SCREENS §2.11 table). */
const PREFIXES: ReadonlyArray<{ prefix: string; scope: SearchScope }> = [
  { prefix: "files:", scope: "files" },
  { prefix: "file:", scope: "files" },
  { prefix: "folders:", scope: "folders" },
  { prefix: "folder:", scope: "folders" },
  { prefix: "apps:", scope: "apps" },
  { prefix: "app:", scope: "apps" },
  { prefix: "clipboard:", scope: "clipboard" },
  { prefix: "clip:", scope: "clipboard" },
  { prefix: "web:", scope: "browser" },
  { prefix: "history:", scope: "browser" },
  { prefix: "bookmarks:", scope: "browser" },
  { prefix: "google:", scope: "browser" },
  { prefix: "content:", scope: "content" },
  { prefix: "ocr:", scope: "ocr" },
  { prefix: "git:", scope: "git" },
  { prefix: "commits:", scope: "git" },
  { prefix: "todos:", scope: "git" },
  { prefix: "calc:", scope: "actions" },
  { prefix: "command:", scope: "actions" },
];

/**
 * Choosing a chip clears a leading scope prefix from the query text.
 * A typed prefix alone does not select a chip (the strip stays ALL).
 */
export function stripScopePrefix(query: string): string {
  const trimmed = query.replace(/^\s+/, "");
  const lower = trimmed.toLowerCase();
  for (const { prefix } of PREFIXES) {
    if (lower.startsWith(prefix)) {
      return trimmed.slice(prefix.length).trim();
    }
  }
  return query;
}

/**
 * Effective scope for rendering/guarding when the user typed a scope prefix.
 * The visible chip does not move (v10 normal launcher); the prefix is the only
 * indicator for the additional sources.
 */
export function prefixScope(query: string): SearchScope | null {
  const trimmed = query.replace(/^\s+/, "");
  const lower = trimmed.toLowerCase();
  for (const { prefix, scope } of PREFIXES) {
    if (lower.startsWith(prefix)) return scope;
  }
  return null;
}

/** Footer/empty-state syntax examples kept in sync with the engine prefixes. */
export const PREFIX_EXAMPLES = [
  "bookmarks:",
  "history:",
  "clipboard:",
  "git:",
  "ocr:",
] as const;

const SCOPE_LABELS: Readonly<Record<string, string>> = {
  all: "ALL",
  files: "FILES",
  folders: "FOLDERS",
  apps: "APPS",
  clipboard: "CLIPBOARD",
  browser: "WEB",
  git: "GIT",
  ocr: "OCR",
  actions: "ACTIONS",
  commands: "COMMANDS",
  content: "DOCUMENTS",
  settings: "SETTINGS",
  agent: "AGENTS",
};

export function scopeLabel(scope: SearchScope): string {
  return SCOPE_LABELS[scope] ?? "ALL";
}

export function scopeIndex(scope: SearchScope): number {
  const index = SCOPE_CHIPS.findIndex((chip) => chip.scope === scope);
  return index >= 0 ? index : 0;
}

/** Strict FILES/FOLDERS guard until ps-core task-19 lands (alpha debt). */
export function guardScopeResults(
  results: readonly SearchResult[],
  scope: SearchScope,
): SearchResult[] {
  if (scope === "files") return results.filter((result) => result.kind !== "folder");
  if (scope === "folders") return results.filter((result) => result.kind === "folder");
  if (scope === "ocr") {
    return results.filter(
      (result) =>
        result.kind === "ocr_text" ||
        result.kind === "content_match" ||
        result.kind === "document" ||
        result.kind === "recent" ||
        (result.kind === "file" && !result.path?.endsWith("/")),
    );
  }
  return [...results];
}

export type ClipboardTag = "LINK" | "EMAIL" | "PATH" | "CODE" | "IMAGE" | "TEXT";

const ENGINE_TAGS = new Set<ClipboardTag>([
  "LINK",
  "EMAIL",
  "PATH",
  "CODE",
  "IMAGE",
  "TEXT",
]);

const SINGLE_URL = /^https?:\/\/\S+$/i;
const SINGLE_EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;
const WINDOWS_PATH = /^(?:[A-Za-z]:\\|\\\\|%[A-Za-z_]+%\\)/;
const CODE_SIGNAL = /[{};]|=>|->|\b(?:def|function|class|import|const|let|var|return|#include)\b|^\s{2,}\S/m;

function firstExposedText(result: SearchResult): string {
  return (result.title || "")
    .split(/\r?\n/)[0]
    .slice(0, 120)
    .trim();
}

function textTag(result: SearchResult): ClipboardTag {
  const text = firstExposedText(result);
  const unquoted = text.replace(/^["'\u201c\u201d]+|["'\u201c\u201d]+$/g, "").trim();
  if (SINGLE_URL.test(unquoted)) return "LINK";
  if (SINGLE_EMAIL.test(unquoted)) return "EMAIL";
  if (WINDOWS_PATH.test(unquoted)) return "PATH";
  const lines = result.title.split(/\r?\n/);
  if (lines.length >= 2) {
    const codeLines = lines.filter((line) => CODE_SIGNAL.test(line)).length;
    if (codeLines >= 2) return "CODE";
  }
  return "TEXT";
}

/**
 * Clipboard tag: engine meta labels win (task-20 contract), otherwise the
 * deterministic text rules classify the exposed text.
 */
export function clipboardCategoryTag(result: SearchResult): ClipboardTag | null {
  if (result.kind !== "clipboard") return null;
  const label = (result.meta?.label ?? "").trim().toUpperCase();
  if (ENGINE_TAGS.has(label as ClipboardTag)) return label as ClipboardTag;
  return textTag(result);
}

export type ClipboardSectionId = "links" | "emails" | "paths" | "code" | "text" | "images";

export interface ClipboardSection {
  id: ClipboardSectionId;
  label: string;
  items: SearchResult[];
}

/** Fixed section order for CLIPBOARD results (task-25). */
export const CLIPBOARD_SECTIONS: ReadonlyArray<{ id: ClipboardSectionId; label: string }> = [
  { id: "links", label: "LINKS" },
  { id: "emails", label: "EMAILS" },
  { id: "paths", label: "PATHS" },
  { id: "code", label: "CODE" },
  { id: "text", label: "TEXT" },
  { id: "images", label: "IMAGES" },
];

export function clipboardSectionIdForResult(result: SearchResult): ClipboardSectionId {
  switch (clipboardCategoryTag(result)) {
    case "LINK":
      return "links";
    case "EMAIL":
      return "emails";
    case "PATH":
      return "paths";
    case "CODE":
      return "code";
    case "IMAGE":
      return "images";
    default:
      return "text";
  }
}

/**
 * Group clipboard rows into clean, non-empty sections while preserving the
 * engine order inside each section (pinned first, newest first).
 */
export function groupClipboardSections(
  results: readonly SearchResult[],
): ClipboardSection[] {
  const buckets = new Map<ClipboardSectionId, SearchResult[]>();
  for (const result of results) {
    const id = clipboardSectionIdForResult(result);
    const bucket = buckets.get(id);
    if (bucket) bucket.push(result);
    else buckets.set(id, [result]);
  }
  return CLIPBOARD_SECTIONS.flatMap(({ id, label }) => {
    const items = buckets.get(id);
    return items && items.length > 0 ? [{ id, label, items }] : [];
  });
}

export function flattenClipboardSections(sections: readonly ClipboardSection[]): SearchResult[] {
  return sections.flatMap((section) => section.items);
}

/** Section label id used for aria-labelledby on the section group. */
export function clipboardSectionDomId(id: ClipboardSectionId): string {
  return "ps-clipboard-section-" + id;
}
