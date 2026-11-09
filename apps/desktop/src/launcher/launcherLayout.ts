import type { Density, SearchResult, SourceKind } from "../bridge/types";
import type { ResultGroup } from "../lib/results";

/**
 * Launcher geometry, selection and scroll math.
 *
 * Binding spec: docs/design/SCREENS.md §2.1/§2.2 and
 * docs/design/KEYBOARD_AND_A11Y.md §2/§4. Everything here is pure so the
 * keyboard/selection rules can be unit-tested without a DOM.
 */

export const SEARCHLINE_HEIGHT = 72;
export const FOOTER_HEIGHT = 44;
/** Results container padding: 10 px top + 13 px bottom (reference:34). */
export const RESULTS_PADDING = 23;
export const GROUP_LABEL_HEIGHT = 34;
export const GROUP_LABEL_HEIGHT_COMPACT = 30;
export const ROW_HEIGHT = 58;
export const ROW_HEIGHT_COMPACT = 48;
export const MIN_LAUNCHER_HEIGHT = 256;
export const MAX_LAUNCHER_HEIGHT = 480;
export const MAX_RESULTS_VIEWPORT = 365;
export const PREVIEW_DRAWER_HEIGHT = 240;
export const PREVIEW_DRAWER_MAX_HEIGHT = 640;
export const PREVIEW_SIDE_MIN_WORK_WIDTH = 1240;
/** PageUp/PageDown move about one viewport (KEYBOARD_AND_A11Y §2). */
export const PAGE_STEP = 6;
/** Engine request limit and rendered-row bound; never render hundreds of rows. */
export const SEARCH_LIMIT = 50;
export const MAX_RENDERED_ROWS = 50;

export interface WorkAreaSize {
  width: number;
  height: number;
}

/** Screen work area in CSS px. Injected in tests via the workArea prop. */
export function getWorkAreaSize(): WorkAreaSize {
  if (typeof window === "undefined" || !window.screen) {
    return { width: 1280, height: 800 };
  }
  const screen = window.screen as Screen;
  return {
    width: screen.availWidth || screen.width || 1280,
    height: screen.availHeight || screen.height || 800,
  };
}

export type PreviewMode = "side" | "drawer";

export function rowHeightFor(density: Density | undefined): number {
  return density === "compact" ? ROW_HEIGHT_COMPACT : ROW_HEIGHT;
}

export function labelHeightFor(density: Density | undefined): number {
  return density === "compact" ? GROUP_LABEL_HEIGHT_COMPACT : GROUP_LABEL_HEIGHT;
}

export interface LauncherMetricsInput {
  /** Number of rows in each rendered group, in group order. */
  groupRowCounts: readonly number[];
  density?: Density;
  previewOpen: boolean;
  workArea: WorkAreaSize;
}

export interface LauncherMetrics {
  rowHeight: number;
  labelHeight: number;
  /** Height of search row + results + footer, before preview drawer growth. */
  baseHeight: number;
  /** Actual window height to request (includes a bottom preview drawer). */
  totalHeight: number;
  /** Height available to the scrolling results list. */
  listViewport: number;
  previewMode: PreviewMode;
}

function snapTo2(value: number): number {
  return Math.round(value / 2) * 2;
}

export function computeLauncherMetrics(input: LauncherMetricsInput): LauncherMetrics {
  const rowHeight = rowHeightFor(input.density);
  const labelHeight = labelHeightFor(input.density);
  const content = input.groupRowCounts.reduce(
    (sum, rows) => sum + labelHeight + rows * rowHeight,
    0,
  );
  const raw = SEARCHLINE_HEIGHT + RESULTS_PADDING + content;
  const baseHeight = snapTo2(
    Math.min(MAX_LAUNCHER_HEIGHT, Math.max(MIN_LAUNCHER_HEIGHT, raw)),
  );
  const previewMode: PreviewMode =
    input.workArea.width >= PREVIEW_SIDE_MIN_WORK_WIDTH ? "side" : "drawer";
  let totalHeight = baseHeight;
  if (input.previewOpen && previewMode === "drawer") {
    const cap = Math.min(PREVIEW_DRAWER_MAX_HEIGHT, input.workArea.height - 24);
    totalHeight = snapTo2(Math.min(baseHeight + PREVIEW_DRAWER_HEIGHT, cap));
  }
  const listViewport = Math.max(
    0,
    Math.min(MAX_RESULTS_VIEWPORT, baseHeight - SEARCHLINE_HEIGHT - FOOTER_HEIGHT),
  );
  return { rowHeight, labelHeight, baseHeight, totalHeight, listViewport, previewMode };
}

export interface FlatEntry {
  type: "group" | "row";
  /** Stable DOM id fragment. */
  key: string;
  label?: string;
  result?: SearchResult;
  /** Flat keyboard index across all results. */
  index?: number;
  /** Offset from the scroll container's content origin, including top padding. */
  top: number;
  height: number;
}

/** Flat, fixed-height entry list used for scroll-into-view math. */
export function buildFlatEntries(
  groups: readonly ResultGroup[],
  rowHeight: number,
  labelHeight: number,
): { entries: FlatEntry[]; contentHeight: number } {
  const entries: FlatEntry[] = [];
  let top = 10;
  let visualIndex = 0;
  for (const group of groups) {
    entries.push({
      type: "group",
      key: "group:" + group.kind,
      label: group.label,
      top,
      height: labelHeight,
    });
    top += labelHeight;
    for (const item of group.items) {
      entries.push({
        type: "row",
        key: "row:" + item.result.id,
        result: item.result,
        index: visualIndex,
        top,
        height: rowHeight,
      });
      visualIndex += 1;
      top += rowHeight;
    }
  }
  return { entries, contentHeight: top - 10 };
}

/**
 * Nearest scrollTop that fully reveals the selected row, or null when it is
 * already visible. Pure arithmetic: never measures in an event handler.
 */
export function scrollTopForIndex(
  entries: readonly FlatEntry[],
  index: number,
  viewportHeight: number,
  currentScrollTop: number,
): number | null {
  const entry = entries.find((candidate) => candidate.index === index);
  if (!entry) return null;
  const visibleTop = currentScrollTop;
  const visibleBottom = currentScrollTop + viewportHeight;
  if (entry.top < visibleTop) return Math.max(0, entry.top - 10);
  if (entry.top + entry.height > visibleBottom) {
    return entry.top + entry.height - viewportHeight + 13;
  }
  return null;
}

/** Wrap-around movement for ↑/↓ (reference:137). */
export function wrappedIndex(index: number, delta: number, count: number): number {
  if (count <= 0) return -1;
  return ((index + delta) % count + count) % count;
}

/** Clamped movement for PageUp/PageDown/Ctrl+Home/Ctrl+End. */
export function clampedIndex(index: number, count: number): number {
  if (count <= 0) return -1;
  return Math.max(0, Math.min(index, count - 1));
}

export interface SelectionState {
  resultId: string | null;
  index: number;
  epoch: number;
}

export const EMPTY_SELECTION: SelectionState = { resultId: null, index: 0, epoch: 0 };

/**
 * Commit a response's results into the selection (KEYBOARD_AND_A11Y §4.3).
 * New query epoch → first row. Same epoch (late tier / retry) → keep the
 * selected id when present, otherwise clamp the previous index.
 */
export function commitSelection(
  previous: SelectionState,
  results: readonly SearchResult[],
  epoch: number,
): SelectionState {
  if (results.length === 0) return { resultId: null, index: 0, epoch };
  if (previous.epoch === epoch && previous.resultId) {
    const kept = results.findIndex((result) => result.id === previous.resultId);
    if (kept >= 0) return { resultId: results[kept].id, index: kept, epoch };
    const clamped = clampedIndex(previous.index, results.length);
    return { resultId: results[clamped].id, index: clamped, epoch };
  }
  return { resultId: results[0].id, index: 0, epoch };
}

/** DOM ids are derived from the stable result id (KEYBOARD_AND_A11Y §7.1). */
export function optionDomId(resultId: string): string {
  return "ps-opt-" + resultId.replace(/[^a-zA-Z0-9_-]/g, "_");
}

export function descriptionDomId(resultId: string): string {
  return "ps-desc-" + resultId.replace(/[^a-zA-Z0-9_-]/g, "_");
}

export function metaDomId(resultId: string): string {
  return "ps-meta-" + resultId.replace(/[^a-zA-Z0-9_-]/g, "_");
}

export function groupLabelDomId(kind: SourceKind): string {
  return "ps-group-" + kind;
}

export function countMatches(total: number): string {
  if (total === 0) return "No matches";
  if (total === 1) return "1 match";
  return total.toLocaleString() + " matches";
}
