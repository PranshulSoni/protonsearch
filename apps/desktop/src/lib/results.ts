import type { SearchResult, SourceKind } from "../bridge/types";

/**
 * Launcher grouping contract (docs/design/SCREENS.md §2.2):
 * fixed order APPLICATIONS · FILES · FOLDERS · DOCUMENTS · CLIPBOARD · WEB ·
 * CODE · ACTIONS. RECENT is the empty-input group (§2.3). Only non-empty groups
 * render. Labels are literal uppercase content labels.
 */
const GROUP_ORDER: SourceKind[] = [
  "recent",
  "application",
  "file",
  "folder",
  "document",
  "content_match",
  "ocr_text",
  "clipboard",
  "browser_history",
  "bookmark",
  "git_commit",
  "git_repository",
  "git_todo",
  "action",
  "command",
  "calculator",
  "quicklink",
  "snippet",
  "setting",
  "window",
  "agent",
  "system",
];

const GROUP_LABELS: Partial<Record<SourceKind, string>> = {
  recent: "RECENT",
  application: "APPLICATIONS",
  file: "FILES",
  folder: "FOLDERS",
  document: "DOCUMENTS",
  content_match: "DOCUMENTS",
  ocr_text: "DOCUMENTS",
  clipboard: "CLIPBOARD",
  browser_history: "WEB",
  bookmark: "WEB",
  git_commit: "CODE",
  git_repository: "CODE",
  git_todo: "CODE",
  action: "ACTIONS",
  command: "ACTIONS",
  calculator: "ACTIONS",
  quicklink: "ACTIONS",
  snippet: "ACTIONS",
  setting: "SETTINGS",
  window: "ACTIONS",
  agent: "ACTIONS",
  system: "ACTIONS",
};

export interface ResultGroup {
  /** Stable key for rendering: the first SourceKind represented in the group. */
  kind: SourceKind;
  label: string;
  items: { result: SearchResult; index: number }[];
}

function orderFor(kind: SourceKind): number {
  const index = GROUP_ORDER.indexOf(kind);
  return index === -1 ? GROUP_ORDER.length : index;
}

/**
 * Groups results for display while keeping a single flat keyboard index.
 * Multiple result kinds can share one visible group (e.g. CONTENT_MATCH rows are
 * rendered inside DOCUMENTS; browser history and bookmarks share WEB).
 */
export function groupResults(results: SearchResult[]): ResultGroup[] {
  const groups = new Map<string, ResultGroup>();
  results.forEach((result, index) => {
    const label = GROUP_LABELS[result.kind] ?? "RESULTS";
    let group = groups.get(label);
    if (!group) {
      group = { kind: result.kind, label, items: [] };
      groups.set(label, group);
    }
    group.items.push({ result, index });
  });
  return [...groups.values()].sort((a, b) => orderFor(a.kind) - orderFor(b.kind));
}

/**
 * Island grouping: rows keep the engine's ranked order (relevance first, then
 * Apps / Folders / Files / documents, then history) and a group label starts
 * wherever the result type changes. Grouping never moves a row, so a weaker app
 * can not be shown above a better file just because apps form a group. Only the
 * first `limit` ranked rows are grouped (the rows the island shows).
 */
export function groupResultsInRankOrder(results: SearchResult[], limit = results.length): ResultGroup[] {
  const groups: ResultGroup[] = [];
  results.slice(0, limit).forEach((result, index) => {
    const label = GROUP_LABELS[result.kind] ?? "RESULTS";
    const last = groups[groups.length - 1];
    if (last && last.label === label) {
      last.items.push({ result, index });
    } else {
      groups.push({ kind: result.kind, label, items: [{ result, index }] });
    }
  });
  return groups;
}

export function resultIdentity(result: SearchResult): string {
  return result.id;
}
