import { describe, expect, it } from "vitest";
import {
  AGENT_ERROR_BODY,
  AGENT_ERROR_TITLE,
  AGENT_START_ERROR_BODY,
  AGENT_START_ERROR_TITLE,
  applyAgentEvent,
  beginAgentRun,
  buildAgentRunRequest,
  cancelAgentView,
  createAgentRunId,
  elapsedMs,
  formatElapsed,
  formatRunStarted,
  initialAgentView,
  markAgentStartFailed,
} from "./agentRun";

const SECRET = "C:\\Users\\private\\provider-trace.log: HTTP 500 stack";

function runStarted() {
  return beginAgentRun(initialAgentView(), "run-1", "Summarise TODOs", 1000);
}

describe("agent run state machine", () => {
  it("starts idle and moves through a full successful stream", () => {
    let view = runStarted();
    expect(view.phase).toBe("starting");
    expect(view.startedAt).toBe(1000);

    view = applyAgentEvent(
      view,
      { type: "started", runId: "run-1", runtime: "Pi 1.5.0", model: "pi-default" },
      1200,
    );
    expect(view.phase).toBe("running");

    view = applyAgentEvent(view, { type: "plan", steps: ["Read files", "Write summary"] }, 1300);
    view = applyAgentEvent(view, { type: "progress", message: "Reading workspace…" }, 1400);
    view = applyAgentEvent(
      view,
      { type: "tool_result", tool: "read", ok: true, preview: "package.json" },
      1500,
    );
    view = applyAgentEvent(
      view,
      { type: "output", text: "Three TODOs found.", finalAnswer: true },
      1600,
    );
    view = applyAgentEvent(view, { type: "finished", ok: true, durationMs: 600 }, 1700);

    expect(view.phase).toBe("finished");
    expect(view.finalAnswer).toBe("Three TODOs found.");
    expect(view.durationMs).toBe(600);
    expect(view.entries.map((entry) => entry.label)).toEqual(
      expect.arrayContaining(["Workflow started", "Plan", "Progress", "Tool · read", "Result", "Finished"]),
    );
    expect(elapsedMs(view, 5000)).toBe(700);
  });

  it("never stores raw provider error text; only product copy and a reference", () => {
    let view = runStarted();
    view = applyAgentEvent(view, { type: "started", runId: "run-1", runtime: "Pi 1.5.0", model: "x" });
    view = applyAgentEvent(view, { type: "error", code: "provider_error", message: SECRET });

    expect(view.phase).toBe("error");
    expect(view.errorTitle).toBe(AGENT_ERROR_TITLE);
    expect(view.errorBody).toBe(AGENT_ERROR_BODY);
    expect(view.errorReference).toBe("Reference: provider_error");
    const serialized = JSON.stringify(view);
    expect(serialized).not.toContain(SECRET);
    expect(serialized).not.toContain("C:\\Users\\private");
  });

  it("genericises failed tool previews instead of showing raw error output", () => {
    let view = runStarted();
    view = applyAgentEvent(view, { type: "started", runId: "run-1", runtime: "Pi 1.5.0", model: "x" });
    view = applyAgentEvent(
      view,
      { type: "tool_result", tool: "shell", ok: false, preview: SECRET },
    );
    expect(JSON.stringify(view)).not.toContain(SECRET);
    expect(view.entries[view.entries.length - 1].text).toBe("The tool couldn't complete.");
  });

  it("ends as an error when a run finishes unsuccessfully", () => {
    let view = runStarted();
    view = applyAgentEvent(view, { type: "started", runId: "run-1", runtime: "Pi 1.5.0", model: "x" });
    view = applyAgentEvent(view, { type: "finished", ok: false, durationMs: 4200 });
    expect(view.phase).toBe("error");
    expect(view.errorTitle).toBe(AGENT_ERROR_TITLE);
    expect(view.entries[view.entries.length - 1].label).toBe("Failed");
  });

  it("ignores a started event from a different run", () => {
    let view = runStarted();
    view = applyAgentEvent(view, { type: "started", runId: "other", runtime: "Pi", model: "x" });
    expect(view.runId).toBe("run-1");
    expect(view.entries).toHaveLength(0);
  });

  it("supports start failure and cancellation with quiet copy", () => {
    const failed = markAgentStartFailed(runStarted());
    expect(failed.phase).toBe("error");
    expect(failed.errorTitle).toBe(AGENT_START_ERROR_TITLE);
    expect(failed.errorBody).toBe(AGENT_START_ERROR_BODY);

    let view = runStarted();
    view = applyAgentEvent(view, { type: "started", runId: "run-1", runtime: "Pi", model: "x" });
    view = cancelAgentView(view, 2000);
    expect(view.phase).toBe("cancelled");
    expect(view.finishedAt).toBe(2000);
  });

  it("builds the frozen AgentRunRequest and stable ids", () => {
    expect(buildAgentRunRequest("run-9", "  do it  ", "model-a")).toEqual({
      runId: "run-9",
      prompt: "do it",
      model: "model-a",
      maxSteps: null,
    });
    expect(createAgentRunId()).toMatch(/[a-z0-9-]/i);
    expect(formatElapsed(950)).toBe("1.0 s");
    expect(formatElapsed(65_000)).toBe("1 m 05 s");
    expect(formatElapsed(0)).toBe("0.0 s");
    expect(formatElapsed(Number.NaN)).toBe("0.0 s");
  });

  it("accepts a synthetic started event while idle (browser visual-QA seam)", () => {
    const view = applyAgentEvent(initialAgentView(), {
      type: "started",
      runId: "qa-run",
      runtime: "Pi 1.5.0",
      model: "qa",
    });
    expect(view.phase).toBe("running");
    expect(view.runId).toBe("qa-run");
  });
});

describe("run start formatting", () => {
  it("uses a localized Today time for same-day runs", () => {
    const now = new Date(2026, 4, 20, 18, 0, 0).getTime();
    const started = new Date(2026, 4, 20, 14, 32, 0).getTime();
    const label = formatRunStarted(started, now);
    expect(label).toMatch(/^Today, /);
    expect(label).toMatch(/\d/);
  });

  it("includes the date for older runs", () => {
    const now = new Date(2026, 4, 20, 18, 0, 0).getTime();
    const started = new Date(2026, 4, 18, 9, 15, 0).getTime();
    const label = formatRunStarted(started, now);
    expect(label).not.toContain("Today");
    expect(label).toMatch(/\d/);
  });
});
