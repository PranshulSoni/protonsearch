// Pure agent-run state machine for the optional Pi workflow panel.
// Events arrive from the Tauri "agent:event" channel (or the browser preview
// test seam). Raw provider/engine error text is never stored on the view: the
// surface shows product copy and only a sanitized reference code in Details.

import type { AgentEvent, AgentRunRequest } from "../bridge/types";

export type AgentPhase = "idle" | "starting" | "running" | "finished" | "error" | "cancelled";
export type AgentLogTone = "neutral" | "ok" | "warn" | "error";

export interface AgentLogEntry {
  id: number;
  label: string;
  text: string;
  tone: AgentLogTone;
}

export interface AgentRunView {
  phase: AgentPhase;
  runId: string | null;
  prompt: string;
  entries: AgentLogEntry[];
  finalAnswer: string | null;
  errorTitle: string | null;
  errorBody: string | null;
  /** Sanitized reference only (never a raw provider message). */
  errorReference: string | null;
  startedAt: number | null;
  finishedAt: number | null;
  durationMs: number | null;
}

export const AGENT_ERROR_TITLE = "Pi couldn't finish this workflow.";
export const AGENT_ERROR_BODY =
  "Search still works. Nothing outside the workspace was changed.";
export const AGENT_START_ERROR_TITLE = "Agent workflows are unavailable.";
export const AGENT_START_ERROR_BODY =
  "Search still works. The optional runtime didn't start.";

const MAX_ENTRIES = 200;
const MAX_TEXT = 2000;

function clampText(text: string): string {
  const clean = text.replace(/\r\n/g, "\n").trim();
  return clean.length > MAX_TEXT ? clean.slice(0, MAX_TEXT) + "…" : clean;
}

export function initialAgentView(): AgentRunView {
  return {
    phase: "idle",
    runId: null,
    prompt: "",
    entries: [],
    finalAnswer: null,
    errorTitle: null,
    errorBody: null,
    errorReference: null,
    startedAt: null,
    finishedAt: null,
    durationMs: null,
  };
}

function withEntry(view: AgentRunView, label: string, text: string, tone: AgentLogTone): AgentRunView {
  const entry: AgentLogEntry = {
    id: (view.entries[view.entries.length - 1]?.id ?? 0) + 1,
    label,
    text: clampText(text),
    tone,
  };
  const entries = [...view.entries, entry].slice(-MAX_ENTRIES);
  return { ...view, entries };
}

export function beginAgentRun(view: AgentRunView, runId: string, prompt: string, now = Date.now()): AgentRunView {
  void view;
  return {
    ...initialAgentView(),
    phase: "starting",
    runId,
    prompt: prompt.trim(),
    startedAt: now,
  };
}

export function markAgentStartFailed(view: AgentRunView): AgentRunView {
  return {
    ...view,
    phase: "error",
    errorTitle: AGENT_START_ERROR_TITLE,
    errorBody: AGENT_START_ERROR_BODY,
    errorReference: null,
  };
}

export function cancelAgentView(view: AgentRunView, now = Date.now()): AgentRunView {
  if (view.phase !== "running" && view.phase !== "starting") return view;
  return withEntry(
    {
      ...view,
      phase: "cancelled",
      finishedAt: now,
    },
    "Cancelled",
    "The workflow was cancelled.",
    "warn",
  );
}

export function applyAgentEvent(view: AgentRunView, event: AgentEvent, now = Date.now()): AgentRunView {
  switch (event.type) {
    case "started": {
      if (view.runId && view.runId !== event.runId) return view;
      return withEntry(
        {
          ...view,
          phase: "running",
          runId: event.runId,
          startedAt: view.startedAt ?? now,
        },
        "Workflow started",
        [event.runtime, event.model].filter(Boolean).join(" · "),
        "ok",
      );
    }
    case "plan":
      return withEntry(
        view,
        "Plan",
        event.steps.map((step, index) => index + 1 + ". " + step).join("\n"),
        "neutral",
      );
    case "progress":
      return withEntry(view, "Progress", event.message, "neutral");
    case "tool_result":
      return withEntry(
        view,
        "Tool · " + event.tool,
        event.ok ? event.preview : "The tool couldn't complete.",
        event.ok ? "ok" : "error",
      );
    case "approval_request":
      return withEntry(view, "Approval requested", event.title, "warn");
    case "output":
      if (event.finalAnswer) {
        return withEntry({ ...view, finalAnswer: clampText(event.text) }, "Result", "Final answer ready.", "ok");
      }
      return withEntry(view, "Output", event.text, "neutral");
    case "error":
      // Never surface event.message: it can contain provider traces or paths.
      return withEntry(
        {
          ...view,
          phase: "error",
          finishedAt: now,
          errorTitle: AGENT_ERROR_TITLE,
          errorBody: AGENT_ERROR_BODY,
          errorReference: event.code ? "Reference: " + event.code : null,
        },
        "Failed",
        "The workflow stopped before finishing.",
        "error",
      );
    case "finished": {
      const ok = event.ok;
      const next: AgentRunView = {
        ...view,
        phase: ok ? "finished" : "error",
        finishedAt: now,
        durationMs: event.durationMs,
      };
      if (!ok && !next.errorTitle) {
        next.errorTitle = AGENT_ERROR_TITLE;
        next.errorBody = AGENT_ERROR_BODY;
      }
      return withEntry(
        next,
        ok ? "Finished" : "Failed",
        ok ? "Completed in " + formatElapsed(event.durationMs) + "." : "The workflow finished with an error.",
        ok ? "ok" : "error",
      );
    }
    default:
      return view;
  }
}

export function elapsedMs(view: AgentRunView, now = Date.now()): number {
  if (!view.startedAt) return view.durationMs ?? 0;
  const end = view.finishedAt ?? now;
  return Math.max(0, end - view.startedAt);
}

export function formatElapsed(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "0.0 s";
  if (ms < 60_000) return (Math.round(ms / 100) / 10).toFixed(1) + " s";
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1000);
  return minutes + " m " + String(seconds).padStart(2, "0") + " s";
}

export function buildAgentRunRequest(
  runId: string,
  prompt: string,
  model?: string | null,
): AgentRunRequest {
  return {
    runId,
    prompt: prompt.trim(),
    model: model ?? null,
    maxSteps: null,
  };
}

export function createAgentRunId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return "run-" + Date.now().toString(36) + "-" + Math.random().toString(36).slice(2, 10);
}

export function agentIsActive(phase: AgentPhase): boolean {
  return phase === "starting" || phase === "running";
}