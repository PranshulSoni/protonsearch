import { describe, expect, it, vi } from "vitest";
import {
  AGENT_RUN_HISTORY_KEY,
  MAX_AGENT_RUNS,
  agentRunStatusLabel,
  loadAgentRunHistory,
  parseAgentRunHistory,
  recordAgentRun,
  subscribeAgentRuns,
  type AgentRunRecord,
  type RunHistoryStorage,
} from "./agentRunHistory";

function fakeStorage(initial: Record<string, string> = {}): RunHistoryStorage & { data: Map<string, string> } {
  const data = new Map(Object.entries(initial));
  return {
    data,
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => {
      data.set(key, value);
    },
  };
}

function run(runId: string, status: AgentRunRecord["status"] = "ok"): AgentRunRecord {
  return { runId, runtime: "Pi 1.5.0", status, startedAt: 1000, endedAt: 5200, durationMs: 4200 };
}

describe("agent run history", () => {
  it("parses only valid records and caps the list", () => {
    const valid = Array.from({ length: 14 }, (_, index) => run("r" + index));
    const parsed = parseAgentRunHistory(JSON.stringify([...valid, { runId: "bad" }, null, "nope"]));
    expect(parsed).toHaveLength(MAX_AGENT_RUNS);
    expect(parsed[0].runId).toBe("r0");
    expect(parseAgentRunHistory("not json")).toEqual([]);
    expect(parseAgentRunHistory(null)).toEqual([]);
  });

  it("records newest-first, dedupes by runId and persists", () => {
    const storage = fakeStorage();
    recordAgentRun(run("r1"), storage);
    recordAgentRun(run("r2", "error"), storage);
    recordAgentRun(run("r1", "cancelled"), storage);
    const loaded = loadAgentRunHistory(storage);
    expect(loaded.map((entry) => entry.runId)).toEqual(["r2", "r1"]);
    expect(loaded[1].status).toBe("ok");
    expect(storage.data.get(AGENT_RUN_HISTORY_KEY)).toContain("r2");
  });

  it("caps stored history", () => {
    const storage = fakeStorage();
    for (let index = 0; index < MAX_AGENT_RUNS + 5; index += 1) {
      recordAgentRun(run("run-" + index), storage);
    }
    expect(loadAgentRunHistory(storage)).toHaveLength(MAX_AGENT_RUNS);
  });

  it("notifies subscribers and labels statuses without raw detail", () => {
    const storage = fakeStorage();
    const listener = vi.fn();
    const unsubscribe = subscribeAgentRuns(listener);
    recordAgentRun(run("r3"), storage);
    expect(listener).toHaveBeenCalledTimes(1);
    unsubscribe();
    recordAgentRun(run("r4"), storage);
    expect(listener).toHaveBeenCalledTimes(1);
    expect(agentRunStatusLabel("ok")).toBe("Completed");
    expect(agentRunStatusLabel("error")).toBe("Failed");
    expect(agentRunStatusLabel("cancelled")).toBe("Cancelled");
  });

  it("degrades safely to a session-only history without storage", () => {
    expect(loadAgentRunHistory(null)).toEqual([]);
    const stored = recordAgentRun(run("r5"), null);
    expect(stored).toHaveLength(1);
    expect(loadAgentRunHistory(null)[0].runId).toBe("r5");
  });
});