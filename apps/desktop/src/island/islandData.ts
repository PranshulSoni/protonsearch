import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../bridge/api";
import { SEARCH_LIMIT } from "../launcher/launcherLayout";
import type {
  AgentStatus,
  AgentTask,
  DetectedAgent,
  EngineStatus,
  IslandSettings,
  MonitorInfo,
  SearchResult,
  SearchScope,
  Settings,
} from "../bridge/types";

/** Real-data hooks for the island cards. No sample rows anywhere. */

export interface IslandBootstrap {
  settings: Settings | null;
  status: EngineStatus | null;
  monitors: MonitorInfo[];
  agents: DetectedAgent[];
  agentStatus: AgentStatus | null;
  agentTasks: AgentTask[];
  ready: boolean;
}

export function useIslandBootstrap(): IslandBootstrap & { refresh: () => void } {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [agents, setAgents] = useState<DetectedAgent[]>([]);
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);
  const [agentTasks, setAgentTasks] = useState<AgentTask[]>([]);
  const [ready, setReady] = useState(false);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    let cancelled = false;
    void Promise.allSettled([
      api.getSettings(),
      api.getStatus(),
      api.listMonitors(),
      api.detectAgents(),
      api.agentStatus(),
      api.agentTasks(50),
    ]).then((results) => {
      if (cancelled) return;
      if (results[0].status === "fulfilled") setSettings(results[0].value);
      if (results[1].status === "fulfilled") setStatus(results[1].value);
      if (results[2].status === "fulfilled") setMonitors(results[2].value);
      if (results[3].status === "fulfilled") setAgents(results[3].value);
      if (results[4].status === "fulfilled") setAgentStatus(results[4].value);
      if (results[5]?.status === "fulfilled") setAgentTasks(results[5].value);
      setReady(true);
    });
    return () => {
      cancelled = true;
    };
  }, [nonce]);

  // Do not refresh the entire bootstrap on every island focus: that fired six
  // IPC calls + React commits during the open path and showed up as a 30-54 ms
  // ui-stall. Explicit card actions still refresh via the returned callback.

  const refresh = useCallback(() => setNonce((value) => value + 1), []);

  return {
    settings,
    status,
    monitors,
    agents,
    agentStatus,
    agentTasks,
    ready,
    refresh,
  };
}

export interface ScopedSearchState {
  results: SearchResult[];
  total: number;
  pending: boolean;
  error: string | null;
}

export function useIslandSearch(
  scope: SearchScope,
  query: string,
  enabled: boolean,
  revision = 0,
): ScopedSearchState {
  const [state, setState] = useState<ScopedSearchState & { scope: SearchScope }>({
    scope,
    results: [],
    total: 0,
    pending: enabled,
    error: null,
  });
  const tokenRef = useRef(0);

  useEffect(() => {
    if (!enabled) return;
    const token = ++tokenRef.current;
    // Keystroke path: no artificial debounce (the launcher searches immediately).
    // Keep the previous rows on screen while the next response is in flight; a
    // pending-only state update is avoided when results already exist, which
    // removes a full list unmount/remount per keystroke.
    setState((current) =>
      current.pending || current.results.length > 0
        ? current
        : { ...current, pending: true, error: null },
    );
    void api
      .search({ requestId: Date.now() + token, query, scope, limit: SEARCH_LIMIT })
      .then((response) => {
        if (token !== tokenRef.current) return;
        setState({
          scope,
          results: response.results,
          total: response.total,
          pending: false,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (token !== tokenRef.current) return;
        setState({
          scope,
          results: [],
          total: 0,
          pending: false,
          error: error instanceof Error ? error.message : "Search is unavailable right now.",
        });
      });
  }, [enabled, scope, query, revision]);

  return state.scope === scope ? state : { results: [], total: 0, pending: enabled, error: null };
}

export type ClipTab = "all" | "text" | "images" | "links" | "code" | "files" | "pinned";

export const CLIP_TABS: ReadonlyArray<{ id: ClipTab; label: string }> = [
  { id: "all", label: "All" },
  { id: "text", label: "Text" },
  { id: "images", label: "Images" },
  { id: "links", label: "Links" },
  { id: "code", label: "Code" },
  { id: "files", label: "Files" },
  { id: "pinned", label: "Pinned" },
];

const LINK = /^https?:\/\/\S+$/i;
const PATH = /^(?:[A-Za-z]:\\|\\\\|%[A-Za-z_]+%\\)/;

export function clipboardKind(result: SearchResult): Exclude<ClipTab, "all" | "pinned"> {
  const label = (result.meta?.label ?? "").toUpperCase();
  if (label === "IMAGE") return "images";
  if (label === "LINK") return "links";
  if (label === "CODE") return "code";
  if (label === "PATH") return "files";
  const first = (result.title || "").split(/\r?\n/)[0].trim();
  if (LINK.test(first)) return "links";
  if (PATH.test(first)) return "files";
  if (/[{};]|=>|\b(?:const|let|var|function|class|import|return)\b/.test(first)) return "code";
  return "text";
}

export function filterClipboard(results: SearchResult[], tab: ClipTab): SearchResult[] {
  if (tab === "all") return results;
  if (tab === "pinned") return [];
  return results.filter((result) => clipboardKind(result) === tab);
}

export function relativeTime(timestamp?: number | null): string {
  if (!timestamp || !Number.isFinite(timestamp)) return "";
  const diff = Math.max(0, Date.now() - timestamp);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (diff < minute) return "just now";
  if (diff < hour) return Math.floor(diff / minute) + " min ago";
  if (diff < day) return Math.floor(diff / hour) + " hr ago";
  if (diff < 2 * day) return "Yesterday";
  if (diff < 7 * day) return Math.floor(diff / day) + " days ago";
  return new Date(timestamp).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function isIslandSettings(settings: Settings | null): IslandSettings | null {
  return settings?.launcher?.island ?? null;
}
