// Agents section at the top of Workflows (task-22). Lists real runtimes detected
// by the shell (detect_agents) — no fake rows; unavailable entries are shown as
// unavailable, and only Pi is wired to the run panel in this alpha. Recent runs
// come from the UI-local run history (metadata only, no prompts/output/paths).

import { useCallback, useEffect, useState } from "react";
import { api } from "../../bridge/api";
import type { DetectedAgent } from "../../bridge/types";
import {
  agentRunStatusLabel,
  loadAgentRunHistory,
  subscribeAgentRuns,
  type AgentRunRecord,
} from "../agentRunHistory";
import { formatElapsed } from "../agentRun";
import {
  getAgentActivity,
  subscribeAgentActivity,
  type AgentActivity,
} from "../agentActivity";
import { relativeTime } from "../describe";
import { EmptyState, Icon, Panel, StateNotice, type IconName } from "../ui";

function agentIcon(id: string): IconName {
  if (id === "pi") return "workflows";
  return "terminal";
}

function availabilityDetail(agent: DetectedAgent): string {
  if (agent.id === "pi" && agent.available) {
    return "Default runtime. Read-only workflows in this alpha.";
  }
  if (agent.available) {
    return "Detected · not wired in this build";
  }
  return "Not found on PATH.";
}

/** Middle-ellipsis for long runtime paths; full value stays in the title. */
function middleEllipsis(value: string, max = 72): string {
  if (value.length <= max) return value;
  const half = Math.floor((max - 1) / 2);
  return value.slice(0, half) + "…" + value.slice(value.length - (max - 1 - half));
}

export default function AgentsSection({ onUsePi }: { onUsePi: () => void }) {
  const [agents, setAgents] = useState<DetectedAgent[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [selectedPi, setSelectedPi] = useState(false);
  const [runs, setRuns] = useState<AgentRunRecord[]>(() => loadAgentRunHistory());
  const [activity, setActivity] = useState<AgentActivity>(() => getAgentActivity());

  const loadAgents = useCallback(async () => {
    setLoading(true);
    setLoadError(false);
    try {
      setAgents(await api.detectAgents());
    } catch {
      setAgents([]);
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAgents();
  }, [loadAgents]);

  useEffect(() => subscribeAgentRuns(() => setRuns(loadAgentRunHistory())), []);
  useEffect(() => subscribeAgentActivity(() => setActivity(getAgentActivity())), []);

  const usePi = () => {
    setSelectedPi(true);
    onUsePi();
  };

  return (
    <Panel
      id="workflows-agents"
      title="Agents"
      hint="Agent runtimes detected on this PC. Optional; nothing runs until you start a workflow."
      actions={
        <button type="button" className="linkbtn" disabled={loading} onClick={() => void loadAgents()}>
          {loading ? "Checking…" : "Refresh"}
        </button>
      }
    >
      {loading && agents === null ? (
        <p className="muted" aria-busy="true">
          Looking for agent runtimes…
        </p>
      ) : loadError ? (
        <StateNotice
          title="Couldn't look for agents."
          body="Nothing runs until an agent runtime is available."
          actionLabel="Try again"
          onAction={() => void loadAgents()}
        />
      ) : agents && agents.length === 0 ? (
        <EmptyState
          title="No agent runtimes found."
          body="Install the Pi CLI, then refresh. Nothing is downloaded automatically."
          action={
            <button type="button" className="btn" onClick={() => void loadAgents()}>
              Refresh
            </button>
          }
        />
      ) : (
        <ul className="agent-catalog">
          {(agents ?? []).map((agent) => {
            const wired = agent.id === "pi" && agent.available;
            return (
              <li
                key={agent.id}
                className={"agent-catalog-row" + (agent.id === "pi" ? " is-primary" : "")}
              >
                <span className="agent-catalog-icon" aria-hidden="true">
                  <Icon name={agentIcon(agent.id)} size={20} />
                </span>
                <span className="agent-catalog-main">
                  <span className="agent-catalog-head">
                    <span className="agent-catalog-name">{agent.name}</span>
                    {agent.id === "pi" ? (
                      <span className="agent-default-tag">Default</span>
                    ) : null}
                    {agent.version ? (
                      <span className="agent-badge">v{agent.version}</span>
                    ) : null}
                    {agent.id === "pi" && activity.active ? (
                      <span className="agent-badge agent-badge--running">
                        <span className="agent-run-dot agent-run-dot--ok" aria-hidden="true" />
                        Running
                      </span>
                    ) : (
                      <span
                        className={
                          "agent-badge " +
                          (agent.available ? "agent-badge--available" : "agent-badge--missing")
                        }
                      >
                        {agent.available ? "Available" : "Not found"}
                      </span>
                    )}
                  </span>
                  {agent.path ? (
                    <span className="agent-catalog-path" title={agent.path}>
                      {middleEllipsis(agent.path)}
                    </span>
                  ) : null}
                  <span className="agent-catalog-detail">{availabilityDetail(agent)}</span>
                </span>
                <span className="agent-catalog-actions">
                  {wired ? (
                    selectedPi ? (
                      <span className="badge badge-quiet">In use</span>
                    ) : (
                      <button type="button" className="btn" onClick={usePi}>
                        Use
                      </button>
                    )
                  ) : null}
                </span>
              </li>
            );
          })}
        </ul>
      )}

      <div className="agent-runs">
        <h3 className="agent-section-label">Recent runs</h3>
        {runs.length === 0 ? (
          <p className="muted">No runs yet.</p>
        ) : (
          <ol className="agent-run-list">
            {runs.slice(0, 4).map((run) => (
              <li key={run.runId} className="agent-run-row">
                <span className={"agent-run-status agent-run-" + run.status}>
                  <span className={"agent-run-dot agent-run-dot--" + run.status} aria-hidden="true" />
                  {agentRunStatusLabel(run.status)}
                </span>
                <span className="agent-run-meta">
                  {run.runtime} · {relativeTime(run.endedAt)} · {formatElapsed(run.durationMs)}
                </span>
              </li>
            ))}
          </ol>
        )}
        <p className="field-help">
          Run history stays on this PC and keeps status, time and duration only — never prompts,
          tool output or paths.
        </p>
      </div>
    </Panel>
  );
}