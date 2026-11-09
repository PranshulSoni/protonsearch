import { describe, expect, it } from "vitest";
import type { SearchResult } from "../bridge/types";
import type { ResultGroup } from "../lib/results";
import {
  EMPTY_SELECTION,
  MAX_LAUNCHER_HEIGHT,
  MIN_LAUNCHER_HEIGHT,
  PREVIEW_SIDE_PANEL_WIDTH,
  PREVIEW_WINDOW_WIDTH,
  buildFlatEntries,
  clampedIndex,
  commitSelection,
  computeLauncherMetrics,
  countMatches,
  descriptionDomId,
  metaDomId,
  optionDomId,
  scrollTopForIndex,
  wrappedIndex,
} from "./launcherLayout";

const WORK_AREA = { width: 1600, height: 900 };

function result(id: string): SearchResult {
  return {
    id,
    kind: "file",
    title: id,
    subtitle: "Documents",
    icon: { type: "file_type", ext: "txt" },
    score: 1,
    action: { type: "open_path", path: "C:\\docs\\" + id },
  };
}

function group(kind: ResultGroup["kind"], ids: string[]): ResultGroup {
  return {
    label: kind.toUpperCase(),
    kind,
    items: ids.map((id, index) => ({ result: result(id), index })),
  };
}

describe("computeLauncherMetrics", () => {
  it("uses the 256 px floor for an empty launcher", () => {
    const metrics = computeLauncherMetrics({
      groupRowCounts: [],
      density: "comfortable",
      previewOpen: false,
      workArea: WORK_AREA,
    });
    expect(metrics.baseHeight).toBe(MIN_LAUNCHER_HEIGHT);
    expect(metrics.totalHeight).toBe(MIN_LAUNCHER_HEIGHT);
    expect(metrics.searchlineHeight).toBe(72);
    expect(metrics.listViewport).toBe(138);
  });

  it("adds a 34 px group label and 58 px rows, snapped to 2 px", () => {
    const metrics = computeLauncherMetrics({
      groupRowCounts: [1],
      density: "comfortable",
      previewOpen: false,
      workArea: WORK_AREA,
    });
    // 72 + 23 + 34 + 58 = 187 -> floor 256.
    expect(metrics.baseHeight).toBe(MIN_LAUNCHER_HEIGHT);
    expect(metrics.rowHeight).toBe(58);
    expect(metrics.labelHeight).toBe(34);

    const taller = computeLauncherMetrics({
      groupRowCounts: [2, 1],
      density: "comfortable",
      previewOpen: false,
      workArea: WORK_AREA,
    });
    // B2 + 2 px border: 72 + 23 + 242 + 44 + 2 = 383 -> snapped 384.
    expect(taller.baseHeight).toBe(384);
    expect(taller.listViewport).toBe(266);
  });

  it("uses 48 px rows and 30 px labels in compact density", () => {
    const metrics = computeLauncherMetrics({
      groupRowCounts: [2, 1],
      density: "compact",
      previewOpen: false,
      workArea: WORK_AREA,
    });
    // 72 + 23 + 204 + 44 + 2 = 345 -> snapped 346.
    expect(metrics.rowHeight).toBe(48);
    expect(metrics.labelHeight).toBe(30);
    expect(metrics.baseHeight).toBe(346);
    expect(metrics.listViewport).toBe(228);
  });

  it("clamps the launcher to 480 px for long result lists", () => {
    const metrics = computeLauncherMetrics({
      groupRowCounts: [20, 20],
      density: "comfortable",
      previewOpen: false,
      workArea: WORK_AREA,
    });
    expect(metrics.baseHeight).toBe(MAX_LAUNCHER_HEIGHT);
    expect(metrics.listViewport).toBe(362);
  });

  it("grows for a bottom preview drawer but not a side panel", () => {
    const side = computeLauncherMetrics({
      groupRowCounts: [2],
      density: "comfortable",
      previewOpen: true,
      workArea: { width: 1600, height: 900 },
    });
    expect(side.previewMode).toBe("side");
    expect(side.totalHeight).toBe(side.baseHeight);

    const drawer = computeLauncherMetrics({
      groupRowCounts: [2],
      density: "comfortable",
      previewOpen: true,
      workArea: { width: 1024, height: 768 },
    });
    expect(drawer.previewMode).toBe("drawer");
    expect(drawer.totalHeight).toBe(drawer.baseHeight + 240);
  });

  it("uses the 1180/360/1200 preview contract boundaries", () => {
    const boundary = computeLauncherMetrics({
      groupRowCounts: [2],
      density: "comfortable",
      previewOpen: false,
      workArea: { width: 1200, height: 900 },
    });
    expect(boundary.previewMode).toBe("side");

    const below = computeLauncherMetrics({
      groupRowCounts: [2],
      density: "comfortable",
      previewOpen: false,
      workArea: { width: 1199, height: 900 },
    });
    expect(below.previewMode).toBe("drawer");
    expect(PREVIEW_WINDOW_WIDTH).toBe(1180);
    expect(PREVIEW_SIDE_PANEL_WIDTH).toBe(360);
  });

  it("shrinks the search row to 61 px on a ≤700 px surface (SCREENS §6.1)", () => {
    const metrics = computeLauncherMetrics({
      groupRowCounts: [2],
      density: "comfortable",
      previewOpen: false,
      workArea: { width: 650, height: 600 },
    });
    expect(metrics.surfaceWidth).toBe(618);
    expect(metrics.searchlineHeight).toBe(61);
    // 61 + 23 + 150 + 44 + 2 = 280.
    expect(metrics.baseHeight).toBe(280);
    expect(metrics.listViewport).toBe(173);
  });
});

describe("selection rules", () => {
  it("resets to the first row for a new query epoch", () => {
    const previous = { resultId: "b", index: 1, epoch: 3 };
    const next = commitSelection(previous, [result("a"), result("b")], 4);
    expect(next).toEqual({ resultId: "a", index: 0, epoch: 4 });
  });

  it("keeps the selected result id for a late tier in the same epoch", () => {
    const previous = { resultId: "b", index: 1, epoch: 3 };
    const next = commitSelection(previous, [result("c"), result("a"), result("b")], 3);
    expect(next).toEqual({ resultId: "b", index: 2, epoch: 3 });
  });

  it("clamps the previous index when the selected id disappears in the same epoch", () => {
    const previous = { resultId: "gone", index: 2, epoch: 3 };
    const next = commitSelection(previous, [result("a"), result("b")], 3);
    expect(next).toEqual({ resultId: "b", index: 1, epoch: 3 });
  });

  it("clears selection when there are no results", () => {
    expect(commitSelection({ resultId: "a", index: 0, epoch: 2 }, [], 2)).toEqual({
      resultId: null,
      index: 0,
      epoch: 2,
    });
    expect(EMPTY_SELECTION.resultId).toBeNull();
  });

  it("wraps arrow navigation and clamps page movement", () => {
    expect(wrappedIndex(0, 1, 3)).toBe(1);
    expect(wrappedIndex(2, 1, 3)).toBe(0);
    expect(wrappedIndex(0, -1, 3)).toBe(2);
    expect(clampedIndex(8, 3)).toBe(2);
    expect(clampedIndex(-4, 3)).toBe(0);
    expect(clampedIndex(0, 0)).toBe(-1);
  });
});

describe("scroll + identity helpers", () => {
  const groups = [group("file", ["a", "b", "c", "d", "e", "f", "g"]), group("folder", ["h"])];
  const flat = buildFlatEntries(groups, 58, 34);

  it("builds fixed-height entries with the 10 px top padding", () => {
    expect(flat.entries[0]).toMatchObject({ type: "group", top: 10, height: 34 });
    expect(flat.entries[1]).toMatchObject({ type: "row", index: 0, top: 44, height: 58 });
    expect(flat.entries[flat.entries.length - 1].type).toBe("row");
    expect(flat.contentHeight).toBe(34 + 7 * 58 + 34 + 58);
  });

  it("returns null when the selected row is already visible", () => {
    expect(scrollTopForIndex(flat.entries, 0, 200, 0)).toBeNull();
  });

  it("scrolls down just enough to reveal a row beyond the viewport", () => {
    const target = scrollTopForIndex(flat.entries, 5, 200, 0);
    expect(target).not.toBeNull();
    expect(target as number).toBeGreaterThan(0);
  });

  it("derives stable, DOM-safe option ids from result ids", () => {
    expect(optionDomId("file:Architecture notes")).toBe("ps-opt-file_Architecture_notes");
    expect(descriptionDomId("a b")).toBe("ps-desc-a_b");
    expect(metaDomId("a b")).toBe("ps-meta-a_b");
  });

  it("formats the truthful match count copy", () => {
    expect(countMatches(0)).toBe("No matches");
    expect(countMatches(1)).toBe("1 match");
    expect(countMatches(1243)).toBe((1243).toLocaleString() + " matches");
  });
});
