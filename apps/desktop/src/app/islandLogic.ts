// Pure Dynamic Island settings logic shared by the placement/card editors and
// tests. Production values are the typed IPC shape: DIP offsets, stable card
// ids/order, nine centre-named anchors. The editor-only percentage mapping is
// derived here and never persisted.

import type {
  IslandAnchor,
  IslandCardConfig,
  IslandCardId,
  IslandSettings,
  MonitorInfo,
} from "../bridge/types";
import type { IconName } from "./ui";

export const V9_CARD_ORDER: IslandCardId[] = [
  "search",
  "media",
  "clipboard",
  "agent",
  "controls",
  "browser",
  "git",
  "bell",
];

export interface CardMeta {
  title: string;
  description: string;
  icon: IconName;
  required?: boolean;
}

export const CARD_META: Record<IslandCardId, CardMeta> = {
  search: { title: "Search", description: "Type to search files, apps and the web.", icon: "search", required: true },
  media: { title: "Media", description: "Now playing controls and track info.", icon: "play" },
  clipboard: { title: "Clipboard", description: "Recent copied text and images.", icon: "clipboard" },
  agent: { title: "Agents", description: "Pi workflow runs and quick agent actions.", icon: "workflows" },
  controls: { title: "Quick Settings", description: "Volume, Wi-Fi, Bluetooth, brightness.", icon: "grid" },
  browser: { title: "Browser", description: "Open tabs from supported browsers.", icon: "browser" },
  git: { title: "Git", description: "Repository status, branches and changes.", icon: "git" },
  bell: { title: "To-do", description: "Quick tasks, checklists and active to-dos.", icon: "todo" },
};

export interface AnchorMeta {
  id: IslandAnchor;
  label: string;
  x: number;
  y: number;
}

export const ANCHORS: AnchorMeta[] = [
  { id: "top_left", label: "Top left", x: 12, y: 12 },
  { id: "top_center", label: "Top centre", x: 50, y: 12 },
  { id: "top_right", label: "Top right", x: 88, y: 12 },
  { id: "center_left", label: "Middle left", x: 12, y: 48 },
  { id: "center", label: "Middle centre", x: 50, y: 48 },
  { id: "center_right", label: "Middle right", x: 88, y: 48 },
  { id: "bottom_left", label: "Bottom left", x: 12, y: 83 },
  { id: "bottom_center", label: "Bottom centre", x: 50, y: 83 },
  { id: "bottom_right", label: "Bottom right", x: 88, y: 83 },
];

export function anchorMeta(anchor: IslandAnchor): AnchorMeta {
  return ANCHORS.find((entry) => entry.id === anchor) ?? ANCHORS[1];
}

/** Editor drag bounds; anchor presets keep their own 12/88 percentages. */
export const DRAG_X_MIN = 15;
export const DRAG_X_MAX = 85;
export const DRAG_Y_MIN = 10;
export const DRAG_Y_MAX = 87;

export function clampNumber(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, value));
}

export function clampOffset(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.round(clampNumber(value, -500, 500));
}

/** Editor-only: DIP offset -> preview percentage, anchored on the monitor. */
export function percentFromOffsets(
  anchor: IslandAnchor,
  offsetX: number,
  offsetY: number,
  monitor: Pick<MonitorInfo, "workWidth" | "workHeight">,
): { x: number; y: number } {
  const meta = anchorMeta(anchor);
  const width = Math.max(1, monitor.workWidth);
  const height = Math.max(1, monitor.workHeight);
  return {
    x: clampNumber(meta.x + (offsetX / width) * 100, 0, 100),
    y: clampNumber(meta.y + (offsetY / height) * 100, 0, 100),
  };
}

/** Editor-only: dragged percentage -> DIP offset relative to the anchor. */
export function offsetsFromPercent(
  anchor: IslandAnchor,
  xPercent: number,
  yPercent: number,
  monitor: Pick<MonitorInfo, "workWidth" | "workHeight">,
): { offsetX: number; offsetY: number } {
  const meta = anchorMeta(anchor);
  const clampedX = clampNumber(xPercent, DRAG_X_MIN, DRAG_X_MAX);
  const clampedY = clampNumber(yPercent, DRAG_Y_MIN, DRAG_Y_MAX);
  return {
    offsetX: clampOffset(Math.round(((clampedX - meta.x) / 100) * Math.max(1, monitor.workWidth))),
    offsetY: clampOffset(Math.round(((clampedY - meta.y) / 100) * Math.max(1, monitor.workHeight))),
  };
}

export function normalizeCards(raw: readonly IslandCardConfig[] | null | undefined): IslandCardConfig[] {
  const seen = new Set<IslandCardId>();
  const cards: IslandCardConfig[] = [];
  for (const entry of raw ?? []) {
    if (!entry || !V9_CARD_ORDER.includes(entry.id) || seen.has(entry.id)) continue;
    seen.add(entry.id);
    cards.push({ id: entry.id, enabled: entry.id === "search" ? true : Boolean(entry.enabled) });
  }
  for (const id of V9_CARD_ORDER) {
    if (!seen.has(id)) cards.push({ id, enabled: id === "search" });
  }
  return cards;
}

export function normalizeIslandSettings(island: IslandSettings): IslandSettings {
  const cards = normalizeCards(island.cards);
  const enabled = new Set(cards.filter((card) => card.enabled).map((card) => card.id));
  const defaultCard = enabled.has(island.defaultCard) ? island.defaultCard : "search";
  return {
    ...island,
    cards,
    defaultCard: enabled.has(defaultCard) ? defaultCard : "search",
    placement: {
      ...island.placement,
      offsetX: clampOffset(island.placement.offsetX),
      offsetY: clampOffset(island.placement.offsetY),
    },
  };
}

export function moveCard(cards: IslandCardConfig[], from: number, to: number): IslandCardConfig[] {
  if (from === to || from < 0 || from >= cards.length || to < 0 || to >= cards.length) return cards;
  const next = [...cards];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

export function setCardEnabled(
  cards: IslandCardConfig[],
  id: IslandCardId,
  enabled: boolean,
): IslandCardConfig[] {
  if (id === "search") return cards.map((card) => (card.id === "search" ? { ...card, enabled: true } : card));
  return cards.map((card) => (card.id === id ? { ...card, enabled } : card));
}

/** Default always points at an enabled card; Search when nothing valid. */
export function resolveDefaultCard(cards: IslandCardConfig[], preferred: IslandCardId): IslandCardId {
  const enabled = new Set(cards.filter((card) => card.enabled).map((card) => card.id));
  return enabled.has(preferred) ? preferred : "search";
}

export function setDefaultCard(cards: IslandCardConfig[], preferred: IslandCardId): IslandCardId {
  return resolveDefaultCard(cards, preferred);
}

export function resetIslandToV9(island: IslandSettings): IslandSettings {
  return normalizeIslandSettings({
    ...island,
    cards: V9_CARD_ORDER.map((id) => ({ id, enabled: true })),
    defaultCard: "search",
  });
}

export function enabledCards(cards: IslandCardConfig[]): IslandCardConfig[] {
  return cards.filter((card) => card.enabled);
}

export function chooseMonitor(
  monitors: MonitorInfo[],
  preferredId: string | null,
): { monitor: MonitorInfo | null; lost: boolean } {
  if (monitors.length === 0) return { monitor: null, lost: false };
  const preferred = preferredId ? monitors.find((monitor) => monitor.id === preferredId) : undefined;
  if (preferred) return { monitor: preferred, lost: false };
  const primary = monitors.find((monitor) => monitor.primary) ?? monitors[0];
  return { monitor: primary, lost: Boolean(preferredId) };
}

export function monitorAspect(monitor: MonitorInfo | null): number {
  if (!monitor || monitor.width <= 0 || monitor.height <= 0) return 1.85;
  return monitor.width / monitor.height;
}

export function dpiLabel(monitor: MonitorInfo | null): string {
  if (!monitor) return "100%";
  return Math.round(monitor.scaleFactor * 100) + "% · " + monitor.width + " × " + monitor.height;
}