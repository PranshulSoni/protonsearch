import type { IslandCardId, IslandSettings } from "../bridge/types";

/**
 * V9 island geometry (binding: docs/design/ISLAND_V9_SPEC.md §2).
 * The native window adds transparent shadow padding; we always report the
 * SURFACE size to setIslandGeometry.
 */

export type IslandCardView = IslandCardId;
export type IslandVariant = "default";

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

/** The page-dots footer under every expanded card (--island-dots-h in island.css). */
const DOTS_FOOTER = 31;

/** Surface heights. Each card body fills what is above the footer and scrolls inside it. */
const HEIGHTS: Record<IslandCardView, number> = {
  search: 66,
  media: 320 + DOTS_FOOTER,
  clipboard: 416,
  agent: 460,
  controls: 400 + DOTS_FOOTER,
  browser: 460,
  git: 355,
  bell: 460,
};

export const SEARCH_EXPANDED_WIDTH = 550;

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

/** Result rows the expanded search shows at once. */
export const ISLAND_VISIBLE_ROWS = 5;

/** No island surface grows past this; card bodies scroll inside it. */
export const MAX_ISLAND_HEIGHT = 460;

/**
 * Expanded search height for `matches` rows under `labels` group labels. One
 * label is part of the base height; rows stay in ranked order, so another label
 * starts wherever the result type changes, and each needs its own line.
 */
export function expandedSearchHeight(matches: number, labels = 1): number {
  const rows = Math.max(0, Math.min(matches, ISLAND_VISIBLE_ROWS));
  return Math.min(MAX_ISLAND_HEIGHT, 185 + 53 * rows + 24 * Math.max(0, labels - 1));
}

export function islandCardDims(input: {
  card: IslandCardView;
  variant?: IslandVariant;
  stageWidth: number;
  compact: boolean;
  searchMatches?: number;
  /** Group labels among the shown search rows (see expandedSearchHeight). */
  searchLabels?: number;
}): IslandCardDims {
  const { card, stageWidth, compact } = input;
  if (card === "search" && compact) {
    return { width: clampIslandWidth(520, stageWidth), height: 66, radius: 999 };
  }
  const baseWidth = card === "search" ? SEARCH_EXPANDED_WIDTH : NOMINAL_WIDTH[card];
  const height =
    card === "search" ? expandedSearchHeight(input.searchMatches ?? 0, input.searchLabels ?? 1) : HEIGHTS[card];
  return {
    width: clampIslandWidth(baseWidth, stageWidth),
    height: Math.min(MAX_ISLAND_HEIGHT, height),
    radius: 28,
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
  bell: "To-do",
};

export function islandCardLabel(card: IslandCardId): string {
  return ISLAND_CARD_LABEL[card] ?? "Card";
}
