// Optional Pi agent workflow panel (task-13). Optional, on-demand runtime:
// nothing runs while idle, Run is gated on settings.agent.enabled, write/shell
// access is per-run consent (persisted but asked again every run), and streamed
// events render as a quiet log. Product copy only on the surface.

import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { AgentStatus } from "../../bridge/types";
import { isAbsoluteWindowsPath, normalizeWindowsPath } from "../roots";
import {
  AGENT_ERROR_BODY,
  AGENT_ERROR_TITLE,
  agentIsActive,
  applyAgentEvent,
  beginAgentRun,
  buildAgentRunRequest,
  cancelAgentView,
  createAgentRunId,
  elapsedMs,
  formatElapsed,
  initialAgentView,
  markAgentStartFailed,
} from "../agentRun";
import { readMockAgentStatus, subscribeAgentEvents } from "../agentEvents";
import {
  Dot,
  Icon,
  Panel,
  SavedStamp,
  StateNotice,
  Switch,
  useSavedStamp,
  type IconName,
} from "../ui";

const DEFAULT_WORKSPACE = "%APPDATA%\\protonsearch\\v2\\agent-workspace";

function runtimeLabel(status: AgentStatus | null): string {
  switch (status?.state) {
    case "running":
      return "Running";
    case "stopped":
      return "Not running";
    case "error":
      return "Pi unavailable";
    default:
      return "Pi not detected";
  }
}

function runtimeTone(status: AgentStatus | null): "ok" | "warn" | "error" | "off" {
  switch (status?.state) {
    case "running":
    case "stopped":
      return "ok";
    case "error":
      return "error";
    default:
      return "off";
  }
}

function logIcon(tone: string): IconName {
  if (tone === "error") return "alert";
  if (tone === "warn") return "alert";
  if (tone === "ok") return "check";
  return "activity";
}

export default function PiAgentPanel() {
  const { settings, updateSettings } = useApp();
  const { savedKey, markSaved } = useSavedStamp();
  const [status, setStatus] = useState<AgentStatus | null>(null);
  const [statusLoading, setStatusLoading] = useState(true);
  const [view, setView] = useState(initialAgentView);
  const [prompt, setPrompt] = useState("");
  const [starting, setStarting] = useState(false);
  const [now, setNow] = useState(Date.now());
  const [actionNotice, setActionNotice] = useState<{
    title: string;
    body: string;
    retry?: () => void;
  } | null>(null);
  const [workspaceDraft, setWorkspaceDraft] = useState(settings.agent.workspace ?? "");
  const [editingWorkspace, setEditingWorkspace] = useState(false);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const phaseRef = useRef(view.phase);
  phaseRef.current = view.phase;

  const refreshStatus = useCallback(async () => {
    setStatusLoading(true);
    try {
      setStatus(readMockAgentStatus() ?? (await api.agentStatus()));
    } catch {
      setStatus(null);
    } finally {
      setStatusLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    return subscribeAgentEvents((event) => {
      setView((current) => {
        if (
          current.phase !== "starting" &&
          current.phase !== "running" &&
          event.type !== "started"
        ) {
          return current;
        }
        return applyAgentEvent(current, event);
      });
      if (event.type === "started" || event.type === "finished" || event.type === "error") {
        void refreshStatus();
      }
    });
  }, [refreshStatus]);

  useEffect(() => {
    if (!agentIsActive(view.phase)) return;
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, [view.phase]);

  // One run at a time; after any terminal state make sure no Pi process is left
  // running while idle (the runtime is strictly on demand).
  useEffect(() => {
    if (view.phase !== "finished" && view.phase !== "error" && view.phase !== "cancelled") return;
    void api.agentStop().catch(() => undefined);
    const timer = window.setTimeout(() => void refreshStatus(), 350);
    return () => window.clearTimeout(timer);
  }, [view.phase, refreshStatus]);

  // Navigating away mid-run must not leave an orphaned agent process.
  useEffect(() => {
    return () => {
      if (phaseRef.current === "running" || phaseRef.current === "starting") {
        void api.agentCancel().catch(() => undefined);
        void api.agentStop().catch(() => undefined);
      }
    };
  }, []);

  useEffect(() => {
    setWorkspaceDraft(settings.agent.workspace ?? "");
  }, [settings.agent.workspace]);

  const detected = status?.state === "running" || status?.state === "stopped";
  const active = agentIsActive(view.phase);
  const statusLabel = active
    ? "Running"
    : status?.state === "stopped"
      ? "Not running · " + (status.runtime ?? "Pi")
      : runtimeLabel(status);
  const statusDescription = active
    ? "Pi is running this workflow. It stops when the run finishes."
    : detected
      ? "The Pi CLI agent is available on this PC. It runs only for a prompt you start here."
      : "Install the Pi CLI, then refresh. Nothing is downloaded automatically.";
  const disabled = !settings.agent.enabled;
  const elapsed = elapsedMs(view, now);
  const canRun =
    !disabled && detected && prompt.trim().length > 0 && !starting && !active;

  const toggleEnabled = async (next: boolean) => {
    setActionNotice(null);
    try {
      await updateSettings({ agent: { ...settings.agent, enabled: next } });
      markSaved("agent");
      if (!next) {
        // Disabling the feature must never leave a Pi process running.
        if (agentIsActive(view.phase)) void cancelRun();
        else void api.agentStop().catch(() => undefined);
      }
      void refreshStatus();
    } catch {
      setActionNotice({
        title: "Couldn't save that change.",
        body: "Your previous setting is still active.",
        retry: () => void toggleEnabled(next),
      });
    }
  };

  // RR-02: read-only only in this alpha. The Tauri shell enforces read-only
  // server-side; the UI never persists a write preference and has no consent flow.
  const startRun = async (promptOverride?: string) => {
    const text = (promptOverride ?? prompt).trim();
    if (!settings.agent.enabled || !detected || !text || active || starting) return;
    setStarting(true);
    setActionNotice(null);
    try {
      const runId = createAgentRunId();
      setView(beginAgentRun(initialAgentView(), runId, text));
      setNow(Date.now());
      await api.agentStart(buildAgentRunRequest(runId, text, settings.agent.model));
    } catch {
      setView((current) => markAgentStartFailed(current));
    } finally {
      setStarting(false);
    }
  };

  const cancelRun = async () => {
    try {
      await api.agentCancel(view.runId ?? undefined);
    } catch {
      /* cancel best effort */
    }
    setView((current) => cancelAgentView(current));
    void api.agentStop().catch(() => undefined);
    void refreshStatus();
  };

  const retryRun = () => {
    const text = view.prompt || prompt;
    setPrompt(text);
    void startRun(text);
  };

  const workspacePath = settings.agent.workspace?.trim() || null;

  const openWorkspace = async () => {
    if (!workspacePath) {
      setWorkspaceError("Set an explicit workspace folder to open it here.");
      return;
    }
    setWorkspaceError(null);
    try {
      const outcome = await api.openResult({ type: "reveal_path", path: workspacePath });
      if (!outcome.ok) setWorkspaceError("The workspace folder isn't available yet.");
    } catch {
      setWorkspaceError("The workspace folder isn't available yet.");
    }
  };

  const saveWorkspace = async (event: FormEvent) => {
    event.preventDefault();
    const normalized = normalizeWindowsPath(workspaceDraft);
    if (!isAbsoluteWindowsPath(normalized)) {
      setWorkspaceError("Enter a full folder path, for example C:\\Users\\You\\AgentWorkspace.");
      return;
    }
    setWorkspaceError(null);
    setActionNotice(null);
    try {
      await updateSettings({ agent: { ...settings.agent, workspace: normalized } });
      setEditingWorkspace(false);
      markSaved("agent");
    } catch {
      setActionNotice({
        title: "Couldn't save that change.",
        body: "Your previous setting is still active.",
      });
    }
  };

  return (
    <Panel
      id="workflows-agent"
      title="Pi agent"
      hint="Optional. Starts only when you run a workflow, then stops; no process runs while idle."
      actions={<SavedStamp show={savedKey === "agent"} />}
    >
      <div className="config-row">
        <div className="config-main">
          <div className="config-title">
            <Dot tone={active ? "ok" : runtimeTone(status)} />
            {statusLoading && !active ? "Checking Pi…" : statusLabel}
          </div>
          <p className="config-desc">{statusDescription}</p>
        </div>
        <Switch
          checked={settings.agent.enabled}
          label="Enable agent workflows"
          onChange={(next) => void toggleEnabled(next)}
        />
      </div>

      <dl className="metric-list">
        <div className="metric-row">
          <dt>Runtime</dt>
          <dd>{status?.runtime ?? "Pi CLI"}</dd>
        </div>
        <div className="metric-row">
          <dt>Provider</dt>
          <dd>{settings.agent.provider ?? "Pi default"}</dd>
        </div>
        <div className="metric-row">
          <dt>Model</dt>
          <dd>{settings.agent.model ?? "Pi default"}</dd>
        </div>
        <div className="metric-row">
          <dt>Idle process</dt>
          <dd>{active ? "Running this workflow" : "None"}</dd>
        </div>
      </dl>

      <div className="agent-workspace">
        <div className="agent-workspace-main">
          <span className="field-label">Workspace</span>
          <span className="config-path" title={workspacePath ?? DEFAULT_WORKSPACE}>
            {workspacePath ?? DEFAULT_WORKSPACE}
          </span>
          <span className="config-sub">
            {workspacePath ? "Custom folder" : "Default folder managed by ProtonSearch"}
          </span>
        </div>
        <div className="agent-workspace-actions">
          <button
            type="button"
            className="btn"
            disabled={!workspacePath}
            title={
              workspacePath
                ? "Open the agent workspace in File Explorer"
                : "Set an explicit workspace folder to open it here"
            }
            onClick={() => void openWorkspace()}
          >
            Open in Explorer
          </button>
          <button
            type="button"
            className="linkbtn"
            onClick={() => {
              setWorkspaceDraft(workspacePath ?? "");
              setEditingWorkspace(true);
              setWorkspaceError(null);
            }}
          >
            Set workspace…
          </button>
        </div>
      </div>
      {editingWorkspace ? (
        <form className="inline-add agent-workspace-form" onSubmit={saveWorkspace} noValidate>
          <label className="sr-only" htmlFor="agent-workspace-input">
            Agent workspace folder
          </label>
          <input
            id="agent-workspace-input"
            className="input input-sm"
            value={workspaceDraft}
            autoFocus
            spellCheck={false}
            placeholder="C:\Users\You\AgentWorkspace"
            aria-invalid={workspaceError ? true : undefined}
            aria-describedby={workspaceError ? "agent-workspace-error" : "agent-workspace-help"}
            onChange={(event) => {
              setWorkspaceDraft(event.target.value);
              setWorkspaceError(null);
            }}
          />
          <button type="submit" className="btn">
            Save workspace
          </button>
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => {
              setEditingWorkspace(false);
              setWorkspaceError(null);
            }}
          >
            Cancel
          </button>
        </form>
      ) : null}
      {workspaceError ? (
        <p className="field-error" id="agent-workspace-error" role="alert">
          {workspaceError}
        </p>
      ) : (
        <p className="field-help" id="agent-workspace-help">
          {workspacePath
            ? "Default working folder only — not a sandbox. Pi runs with your Windows permissions and read tools can open any file you can read."
            : "The default working folder is created under the ProtonSearch data folder on the first run. It is not a sandbox."}
        </p>
      )}

      {!settings.agent.enabled ? (
        <StateNotice
          title="Agent workflows are off."
          body="ProtonSearch works fully without them. Turn the switch on to run a Pi workflow."
        />
      ) : !detected ? (
        <StateNotice
          tone="warn"
          title="Pi isn't detected."
          body="Install the Pi CLI, then refresh. Nothing is downloaded automatically."
          actionLabel="Refresh"
          onAction={() => void refreshStatus()}
        />
      ) : null}

      <div className="agent-prompt-wrap">
        <label className="field-label" htmlFor="agent-prompt">
          Workflow prompt
        </label>
        <textarea
          id="agent-prompt"
          className="agent-prompt"
          rows={3}
          maxLength={2000}
          value={prompt}
          disabled={disabled || !detected || active}
          placeholder="Describe the workflow you want, for example: list the TODOs in this workspace and propose a plan."
          onChange={(event) => setPrompt(event.target.value)}
        />
      </div>

      <div className="agent-safety">
        <p className="agent-safety-line">
          Pi runs with your Windows permissions. Read tools can open any file you can read; the
          workspace is the default working folder, not a sandbox.
        </p>
        <p className="agent-safety-line">
          There is no per-tool approval. File edits and shell access are disabled in this alpha.
        </p>
        <p className="agent-safety-line">
          When a run starts, your prompt, context and tool output are sent to the configured
          provider.
        </p>
      </div>

      <div className="agent-actions">
        {active ? (
          <button type="button" className="btn" onClick={() => void cancelRun()}>
            Cancel workflow
          </button>
        ) : (
          <button
            type="button"
            className="btn btn-primary"
            disabled={!canRun}
            onClick={() => void startRun()}
          >
            {starting ? "Starting…" : "Run workflow"}
          </button>
        )}
        {!active && (view.phase === "finished" || view.phase === "cancelled") ? (
          <button
            type="button"
            className="btn"
            disabled={!settings.agent.enabled || !detected}
            onClick={retryRun}
          >
            Try again
          </button>
        ) : null}
        <span className="agent-elapsed">
          {view.phase === "finished" || view.phase === "error" || view.phase === "cancelled"
            ? "Last run · " + formatElapsed(elapsed)
            : active
              ? "Running · " + formatElapsed(elapsed)
              : ""}
        </span>
        <button type="button" className="linkbtn agent-refresh" onClick={() => void refreshStatus()}>
          Refresh status
        </button>
      </div>

      {actionNotice ? (
        <StateNotice
          className="state-inline"
          title={actionNotice.title}
          body={actionNotice.body}
          actionLabel={actionNotice.retry ? "Try again" : undefined}
          onAction={actionNotice.retry}
        />
      ) : null}

      {view.phase === "error" ? (
        <StateNotice
          tone="error"
          role="alert"
          title={view.errorTitle ?? AGENT_ERROR_TITLE}
          body={view.errorBody ?? AGENT_ERROR_BODY}
          details={view.errorReference ?? undefined}
          actionLabel="Try again"
          onAction={retryRun}
        />
      ) : null}

      {view.finalAnswer ? (
        <div className="agent-result">
          <h3 className="agent-section-label">Result</h3>
          <p className="agent-result-text">{view.finalAnswer}</p>
        </div>
      ) : null}

      {view.entries.length > 0 ? (
        <div className="agent-log-wrap">
          <h3 className="agent-section-label">Workflow activity</h3>
          <ol className="agent-log" aria-live="polite">
            {view.entries.map((entry) => (
              <li key={entry.id} className={"agent-log-entry agent-log-" + entry.tone}>
                <span className="agent-log-icon" aria-hidden="true">
                  <Icon name={logIcon(entry.tone)} size={14} />
                </span>
                <span className="agent-log-label">{entry.label}</span>
                <span className="agent-log-text">{entry.text}</span>
              </li>
            ))}
          </ol>
        </div>
      ) : null}

      <p className="field-help">
        Pi is optional and on demand. Nothing is sent to a provider unless you run a workflow, and
        the runtime stops when the run finishes.
      </p>
    </Panel>
  );
}