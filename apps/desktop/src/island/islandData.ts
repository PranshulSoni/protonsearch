import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../bridge/api";
import { SEARCH_LIMIT } from "../launcher/launcherLayout";
import type {
  AgentStatus,
  AgentTask,
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
  agentStatus: AgentStatus | null;
  agentTasks: AgentTask[];
  ready: boolean;
}

export function useIslandBootstrap(): IslandBootstrap & { refresh: () => void } {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
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
      api.agentStatus(),
      api.agentTasks(50),
    ]).then((results) => {
      if (cancelled) return;
      if (results[0].status === "fulfilled") setSettings(results[0].value);
      if (results[1].status === "fulfilled") setStatus(results[1].value);
      if (results[2].status === "fulfilled") setMonitors(results[2].value);
      if (results[3].status === "fulfilled") setAgentStatus(results[3].value);
      if (results[4]?.status === "fulfilled") setAgentTasks(results[4].value);
      setReady(true);
    });
    return () => {
      cancelled = true;
    };
  }, [nonce]);

  // The shell emits settings:changed after each saved change. The island window
  // is prewarmed, so without this it keeps the snapshot it loaded at startup.
  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    void import("@tauri-apps/api/event")
      .then(({ listen }) => listen<Settings>("settings:changed", (event) => setSettings(event.payload)))
      .then((unlisten) => {
        if (disposed) unlisten();
        else off = unlisten;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      off?.();
    };
  }, []);

  // Do not refresh the entire bootstrap on every island focus: that fired six
  // IPC calls + React commits during the open path and showed up as a 30-54 ms
  // ui-stall. Explicit card actions still refresh via the returned callback.

  const refresh = useCallback(() => setNonce((value) => value + 1), []);

  return {
    settings,
    status,
    monitors,
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

/** How long the previous rows may stand in while the next answer is pending. */
export const STALE_RESULTS_MS = 250;

export function useIslandSearch(
  scope: SearchScope,
  query: string,
  enabled: boolean,
  revision = 0,
): ScopedSearchState {
  const [state, setState] = useState<ScopedSearchState & { scope: SearchScope; query: string }>({
    scope,
    query,
    results: [],
    total: 0,
    pending: enabled,
    error: null,
  });
  const tokenRef = useRef(0);

  useEffect(() => {
    if (!enabled) {
      // An answer still in flight for the old request must not land later.
      tokenRef.current += 1;
      return;
    }
    const token = ++tokenRef.current;
    // Keystroke path: no artificial debounce (the launcher searches immediately).
    // The previous rows stay while the next answer is in flight, so a fast
    // answer swaps them in place (no flicker, no height jump, also when the
    // filter changes). They never pass for current results: rows of the empty
    // query (the compact pill showed none) are dropped at once, and any rows
    // still standing after STALE_RESULTS_MS give way to the loading state.
    setState((current) => {
      if (current.results.length > 0 && !current.query.trim() && query.trim()) {
        return { ...current, scope, query, results: [], total: 0, pending: true, error: null };
      }
      return current.pending || current.results.length > 0
        ? current
        : { ...current, pending: true, error: null };
    });
    const staleTimer = window.setTimeout(() => {
      if (token !== tokenRef.current) return;
      setState((current) =>
        current.results.length > 0
          ? { ...current, scope, query, results: [], total: 0, pending: true, error: null }
          : current,
      );
    }, STALE_RESULTS_MS);
    void api
      .search({ requestId: Date.now() + token, query, scope, limit: SEARCH_LIMIT })
      .then((response) => {
        if (token !== tokenRef.current) return;
        window.clearTimeout(staleTimer);
        setState({
          scope,
          query,
          results: response.results,
          total: response.total,
          pending: false,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (token !== tokenRef.current) return;
        window.clearTimeout(staleTimer);
        setState({
          scope,
          query,
          results: [],
          total: 0,
          pending: false,
          error: error instanceof Error ? error.message : "Search is unavailable right now.",
        });
      });
    return () => window.clearTimeout(staleTimer);
  }, [enabled, scope, query, revision]);

  return state;
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
  const filtered =
    tab === "all"
      ? results
      : tab === "pinned"
        ? results.filter((result) => result.meta?.pinned)
        : results.filter((result) => clipboardKind(result) === tab);
  // Pinned clips lead, then newest first.
  return [...filtered].sort(
    (a, b) =>
      Number(b.meta?.pinned ?? false) - Number(a.meta?.pinned ?? false) ||
      (b.meta?.modified ?? 0) - (a.meta?.modified ?? 0),
  );
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
