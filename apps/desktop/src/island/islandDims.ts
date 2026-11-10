import type { IslandCardId, IslandSettings } from "../bridge/types";

/**
 * V9 island geometry (binding: docs/design/ISLAND_V9_SPEC.md §2).
 * The native window adds transparent shadow padding; we always report the
 * SURFACE size to setIslandGeometry.
 */

export type IslandCardView = IslandCardId;
export type IslandVariant =
  | "default"
  | "media-output"
  | "controls-wifi"
  | "controls-bluetooth"
  | "agent-hierarchy";

export interface IslandCardDims {
  width: number;
  height: number;
  radius: number;
}

const NOMINAL_WIDTH: Record<IslandCardId, number> = {
  search: 520,
  media: 490,
  clipboard: 510,
  agent: 520,
  controls: 515,
  browser: 500,
  git: 505,
  bell: 500,
};

const HEIGHTS: Record<IslandCardView, { wide: number; mid: number; narrow: number }> = {
  search: { wide: 66, mid: 66, narrow: 66 },
  media: { wide: 350, mid: 360, narrow: 360 },
  clipboard: { wide: 391, mid: 416, narrow: 446 },
  agent: { wide: 411, mid: 445, narrow: 445 },
  controls: { wide: 376, mid: 400, narrow: 483 },
  browser: { wide: 361, mid: 408, narrow: 408 },
  git: { wide: 330, mid: 355, narrow: 355 },
  bell: { wide: 345, mid: 385, narrow: 385 },
};

export const SEARCH_EXPANDED_WIDTH = 550;

export function isNarrow(stageWidth: number): boolean {
  return stageWidth <= 380;
}

export function isMid(stageWidth: number): boolean {
  return stageWidth > 380 && stageWidth <= 520;
}

export function clampIslandWidth(nominal: number, stageWidth: number): number {
  // Captured V9 widths are stage - 20 (520->500, 430->410, 335->315); the spec's
  // "actual = min(nominal, stage-18)" is the CSS rule including the 1 px edges.
  return Math.max(240, Math.min(nominal, stageWidth - 20));
}

/**
 * Surface width the native window is sized for: the widest card. It stays
 * constant so a card switch never moves or re-centres the window sideways.
 */
export function islandWindowWidth(stageWidth: number): number {
  return clampIslandWidth(
    Math.max(SEARCH_EXPANDED_WIDTH, ...Object.values(NOMINAL_WIDTH)),
    stageWidth,
  );
}

export function breakpointHeight(view: IslandCardView, stageWidth: number): number {
  const row = HEIGHTS[view];
  if (isNarrow(stageWidth)) return row.narrow;
  if (isMid(stageWidth)) return row.mid;
  return row.wide;
}

export function expandedSearchHeight(matches: number): number {
  return Math.min(460, 185 + 53 * Math.max(0, Math.min(matches, 5)));
}

export function islandCardDims(input: {
  card: IslandCardView;
  variant?: IslandVariant;
  stageWidth: number;
  compact: boolean;
  searchMatches?: number;
}): IslandCardDims {
  const { card, variant = "default", stageWidth, compact } = input;
  if (card === "search" && compact) {
    return { width: clampIslandWidth(520, stageWidth), height: 66, radius: 999 };
  }
  const baseWidth = card === "search" ? SEARCH_EXPANDED_WIDTH : NOMINAL_WIDTH[card];
  let height = breakpointHeight(card, stageWidth);
  if (card === "search") height = expandedSearchHeight(input.searchMatches ?? 0);
  if (card === "media" && variant === "media-output") {
    height = stageWidth > 520 ? 342 : 365;
  }
  if (card === "controls" && (variant === "controls-wifi" || variant === "controls-bluetooth")) {
    height = stageWidth > 520 ? 366 : 418;
  }
  return {
    width: clampIslandWidth(baseWidth, stageWidth),
    height,
    radius: card === "media" ? 41 : 34,
  };
}

/** Enabled cards in the user's saved order (the 8-dot nav contract). */
export function visibleIslandCards(settings: IslandSettings | null): IslandCardId[] {
  const cards = settings?.cards ?? [];
  const enabled = cards.filter((card) => card.enabled).map((card) => card.id);
  return enabled.length > 0 ? enabled : ["search"];
}

export const ISLAND_CARD_LABEL: Record<IslandCardId, string> = {
  search: "Search",
  media: "Media",
  clipboard: "Clipboard",
  agent: "Agents",
  controls: "Quick Settings",
  browser: "Browser",
  git: "Git",
  bell: "Notifications",
};

export function islandCardLabel(card: IslandCardId): string {
  return ISLAND_CARD_LABEL[card] ?? "Card";
}
