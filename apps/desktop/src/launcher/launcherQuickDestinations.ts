import type { EngineStatus } from "../bridge/types";
import { formatNumber } from "../lib/format";

/**
 * Empty-state quick destinations for the v10 Normal Launcher (master prompt §5).
 * Only real EngineStatus data is shown: source item counts, enabled/off state and
 * the agent runtime state. No fake counts, no destinations for unavailable data.
 */

export type QuickDestinationAction =
  | { type: "search"; prefix: string }
  | { type: "main" };

export interface QuickDestination {
  id: string;
  label: string;
  subtitle: string;
  action: QuickDestinationAction;
  /** Primary destinations sit in the 2×2 grid; secondary are quiet links. */
  secondary?: boolean;
  prefix?: string;
  off?: boolean;
}

function source(status: EngineStatus | null, id: string) {
  return status?.sources?.find((candidate) => candidate.id === id) ?? null;
}

function sourceOff(status: EngineStatus | null, id: string): boolean {
  const found = source(status, id);
  if (!found) return true;
  return found.enabled === false || found.state === "disabled" || found.state === "off";
}

function sourceCountLabel(status: EngineStatus | null, id: string): string {
  const found = source(status, id);
  if (!found || sourceOff(status, id)) return "Off — enable in Preferences";
  if (!Number.isFinite(found.itemCount)) return "Search this source";
  if (found.itemCount <= 0) return "No items indexed yet";
  return formatNumber(found.itemCount) + " items";
}

export function buildQuickDestinations(status: EngineStatus | null): QuickDestination[] {
  const browserOff = sourceOff(status, "browser");
  const clipboardOff = sourceOff(status, "clipboard");
  const gitOff = sourceOff(status, "git");
  const agentState = status?.agent?.state ?? "not_configured";
  const agentSubtitle =
    agentState === "running"
      ? "Agent runtime running"
      : agentState === "error"
        ? "Agent runtime needs attention"
        : agentState === "stopped"
          ? "Agent runtime stopped"
          : "No agent configured";

  return [
    {
      id: "bookmarks",
      label: "Browser Bookmarks",
      subtitle: browserOff ? "Off — enable in Preferences" : "Search saved bookmarks",
      action: browserOff ? { type: "main" } : { type: "search", prefix: "bookmarks:" },
      prefix: "bookmarks:",
      off: browserOff,
    },
    {
      id: "history",
      label: "Browser History",
      subtitle: browserOff
        ? "Off — enable in Preferences"
        : sourceCountLabel(status, "browser"),
      action: browserOff ? { type: "main" } : { type: "search", prefix: "history:" },
      prefix: "history:",
      off: browserOff,
    },
    {
      id: "clipboard",
      label: "Clipboard History",
      subtitle: clipboardOff
        ? "Off — enable in Preferences"
        : sourceCountLabel(status, "clipboard"),
      action: clipboardOff ? { type: "main" } : { type: "search", prefix: "clipboard:" },
      prefix: "clipboard:",
      off: clipboardOff,
    },
    {
      id: "agents",
      label: "Agents",
      subtitle: agentSubtitle,
      action: { type: "main" },
    },
    {
      id: "git",
      label: "Git history",
      subtitle: gitOff ? "Off — enable in Preferences" : sourceCountLabel(status, "git"),
      action: gitOff ? { type: "main" } : { type: "search", prefix: "git:" },
      prefix: "git:",
      secondary: true,
      off: gitOff,
    },
    {
      id: "agent-history",
      label: "Agent History",
      subtitle: "Open Workflows history",
      action: { type: "main" },
      secondary: true,
    },
  ];
}

/** Prefix syntax documented in the empty state (engine query parser). */
export const SEARCH_SYNTAX_HINT =
  "Prefixes: bookmarks:, history:, clipboard:, git:, ocr:, content:, calc:";

/** Stable DOM id for the aria-activedescendant quick-destination options. */
export function quickDestinationDomId(id: string): string {
  return "ps-quick-" + id.replace(/[^a-zA-Z0-9_-]/g, "_");
}

/** Estimated panel height for the launcher geometry (grid + secondary + hint). */
export const QUICK_DESTINATIONS_HEIGHT = 248;
export const QUICK_DESTINATIONS_HEIGHT_COMPACT = 224;
