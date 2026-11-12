import { describe, expect, it } from "vitest";
import type { IslandCardConfig, IslandSettings, MonitorInfo } from "../bridge/types";
import {
  ANCHORS,
  DRAG_X_MIN,
  V9_CARD_ORDER,
  chooseMonitor,
  clampOffset,
  monitorAspect,
  moveCard,
  normalizeCards,
  normalizeIslandSettings,
  offsetsFromPercent,
  percentFromOffsets,
  resetIslandToV9,
  resolveDefaultCard,
  setCardEnabled,
} from "./islandLogic";

const monitor: MonitorInfo = {
  id: "DISPLAY1",
  name: "Built-in",
  primary: true,
  x: 0,
  y: 0,
  width: 2560,
  height: 1600,
  workX: 0,
  workY: 0,
  workWidth: 2560,
  workHeight: 1560,
  scaleFactor: 1.5,
};

function island(overrides: Partial<IslandSettings> = {}): IslandSettings {
  return {
    enabled: true,
    placement: {
      monitor: null,
      anchor: "top_center",
      offsetX: 0,
      offsetY: 22,
      keepInWorkArea: true,
      autoFlipExpanded: true,
    },
    cards: V9_CARD_ORDER.map((id) => ({ id, enabled: true })),
    defaultCard: "search",
    showDots: true,
    hideOnFocusLoss: true,
    skipDisabled: true,
    returnToSearch: true,
    ...overrides,
  };
}

describe("island card model", () => {
  it("keeps the exact V9 order and rejects unknown/duplicate ids", () => {
    const cards = normalizeCards([
      { id: "media", enabled: true },
      { id: "media", enabled: false },
      { id: "nope" as never, enabled: true },
      ...V9_CARD_ORDER.filter((id) => id !== "media").map((id) => ({ id, enabled: false })),
    ]);
    expect(cards).toHaveLength(8);
    expect(new Set(cards.map((card) => card.id)).size).toBe(8);
    expect(cards[0].id).toBe("media");
    expect(cards.find((card) => card.id === "media")?.enabled).toBe(true);
    expect(cards.find((card) => card.id === "search")?.enabled).toBe(true);
  });

  it("moves cards and keeps search required", () => {
    const cards = V9_CARD_ORDER.map((id) => ({ id, enabled: true }));
    const moved = moveCard(cards, 0, 2);
    expect(moved.map((card) => card.id).slice(0, 3)).toEqual(["media", "clipboard", "search"]);
    expect(setCardEnabled(moved, "search", false).find((card) => card.id === "search")?.enabled).toBe(true);
    expect(moveCard(cards, 0, 99)).toBe(cards);
  });

  it("falls back to Search when the default card is hidden or unknown", () => {
    const cards: IslandCardConfig[] = V9_CARD_ORDER.map((id) => ({
      id,
      enabled: id !== "media",
    }));
    expect(resolveDefaultCard(cards, "media")).toBe("search");
    expect(resolveDefaultCard(cards, "clipboard")).toBe("clipboard");
    expect(resolveDefaultCard(cards, "nope" as never)).toBe("search");
  });

  it("resets to V9 order, all cards enabled and Search default", () => {
    const dirty = island({
      cards: V9_CARD_ORDER.slice().reverse().map((id) => ({ id, enabled: id !== "bell" })),
      defaultCard: "bell",
    });
    const reset = resetIslandToV9(dirty);
    expect(reset.cards.map((card) => card.id)).toEqual(V9_CARD_ORDER);
    expect(reset.cards.every((card) => card.enabled)).toBe(true);
    expect(reset.defaultCard).toBe("search");
  });

  it("normalizes persisted settings: clamps offsets and repairs default", () => {
    const normalized = normalizeIslandSettings(
      island({
        placement: { ...island().placement, offsetX: 9999, offsetY: -9999 },
        defaultCard: "media",
        cards: V9_CARD_ORDER.map((id) => ({ id, enabled: id !== "media" })),
      }),
    );
    expect(normalized.placement.offsetX).toBe(500);
    expect(normalized.placement.offsetY).toBe(-500);
    expect(normalized.defaultCard).toBe("search");
  });
});

describe("island placement mapping", () => {
  it("maps every anchor around the monitor", () => {
    expect(ANCHORS).toHaveLength(9);
    for (const anchor of ANCHORS) {
      const percent = percentFromOffsets(anchor.id, 0, 0, monitor);
      expect(percent.x).toBeCloseTo(anchor.x, 5);
      expect(percent.y).toBeCloseTo(anchor.y, 5);
    }
  });

  it("converts DIP offsets to preview percentages and back", () => {
    const percent = percentFromOffsets("top_center", 128, 156, monitor);
    expect(percent.x).toBeCloseTo(55, 3);
    expect(percent.y).toBeCloseTo(22, 3);
    const offset = offsetsFromPercent("top_center", percent.x, percent.y, monitor);
    expect(offset.offsetX).toBe(128);
    expect(offset.offsetY).toBe(156);
  });

  it("clamps drag percentages and offsets", () => {
    const offset = offsetsFromPercent("top_left", 2, 99, monitor);
    const percent = percentFromOffsets("top_left", offset.offsetX, offset.offsetY, monitor);
    expect(Math.abs(offset.offsetX)).toBeLessThanOrEqual(500);
    expect(Math.abs(offset.offsetY)).toBeLessThanOrEqual(500);
    expect(percent.x).toBeGreaterThanOrEqual(DRAG_X_MIN);
    expect(percent.x).toBeLessThanOrEqual(100);
    expect(percent.y).toBeGreaterThanOrEqual(0);
    expect(clampOffset(Number.NaN)).toBe(0);
  });

  it("keeps settings when a monitor disappears but shows the primary", () => {
    const external = { ...monitor, id: "DISPLAY2", primary: false, x: 2560 };
    const kept = chooseMonitor([monitor, external], "DISPLAY2");
    expect(kept.monitor?.id).toBe("DISPLAY2");
    expect(kept.lost).toBe(false);
    const lost = chooseMonitor([monitor], "DISPLAY2");
    expect(lost.monitor?.id).toBe("DISPLAY1");
    expect(lost.lost).toBe(true);
    expect(chooseMonitor([], "DISPLAY2")).toEqual({ monitor: null, lost: false });
  });

  it("derives aspect ratios and DPI labels", () => {
    expect(monitorAspect(monitor)).toBeCloseTo(1.6, 2);
    expect(monitorAspect(null)).toBe(1.85);
    expect(monitor.width).toBe(2560);
  });
});