// UI-local history of agent runs (task-22). Deliberately contains only
// non-sensitive run metadata: id, runtime label, status, start/end, duration.
// Never prompts, tool output, provider messages or paths. Persisted in
// localStorage so the Agents section can show recent runs; no bridge/IPC data
// is added or changed.

export type AgentRunStatus = "ok" | "error" | "cancelled";

export interface AgentRunRecord {
  runId: string;
  runtime: string;
  status: AgentRunStatus;
  startedAt: number;
  endedAt: number;
  durationMs: number;
}

export const AGENT_RUN_HISTORY_KEY = "protonsearch.agent.runs.v1";
export const MAX_AGENT_RUNS = 10;

export interface RunHistoryStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

type Listener = () => void;
const listeners = new Set<Listener>();
// Session fallback when localStorage is unavailable (private mode, etc.).
let memoryFallback: AgentRunRecord[] = [];

export function defaultRunHistoryStorage(): RunHistoryStorage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function isValidRecord(value: unknown): value is AgentRunRecord {
  if (!value || typeof value !== "object") return false;
  const record = value as Partial<AgentRunRecord>;
  return (
    typeof record.runId === "string" &&
    record.runId.length > 0 &&
    typeof record.runtime === "string" &&
    (record.status === "ok" || record.status === "error" || record.status === "cancelled") &&
    typeof record.startedAt === "number" &&
    Number.isFinite(record.startedAt) &&
    typeof record.endedAt === "number" &&
    Number.isFinite(record.endedAt) &&
    typeof record.durationMs === "number" &&
    Number.isFinite(record.durationMs) &&
    record.durationMs >= 0
  );
}

export function parseAgentRunHistory(raw: string | null): AgentRunRecord[] {
  if (!raw) return [];
  try {
    const data = JSON.parse(raw) as unknown;
    if (!Array.isArray(data)) return [];
    return data.filter(isValidRecord).slice(0, MAX_AGENT_RUNS);
  } catch {
    return [];
  }
}

export function loadAgentRunHistory(storage: RunHistoryStorage | null = defaultRunHistoryStorage()): AgentRunRecord[] {
  if (!storage) return memoryFallback;
  try {
    return parseAgentRunHistory(storage.getItem(AGENT_RUN_HISTORY_KEY));
  } catch {
    return memoryFallback;
  }
}

export function recordAgentRun(
  record: AgentRunRecord,
  storage: RunHistoryStorage | null = defaultRunHistoryStorage(),
): AgentRunRecord[] {
  if (!isValidRecord(record)) return loadAgentRunHistory(storage);
  const existing = loadAgentRunHistory(storage);
  if (existing.some((entry) => entry.runId === record.runId)) return existing;
  const next = [record, ...existing].slice(0, MAX_AGENT_RUNS);
  if (storage) {
    try {
      storage.setItem(AGENT_RUN_HISTORY_KEY, JSON.stringify(next));
    } catch {
      memoryFallback = next;
    }
  } else {
    memoryFallback = next;
  }
  for (const listener of listeners) listener();
  return next;
}

export function subscribeAgentRuns(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function agentRunStatusLabel(status: AgentRunStatus): string {
  if (status === "ok") return "Completed";
  if (status === "error") return "Failed";
  return "Cancelled";
}