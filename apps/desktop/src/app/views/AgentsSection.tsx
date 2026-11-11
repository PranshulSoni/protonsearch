// Pi-only Agent runtime section at the top of Workflows (argon branch).
// Exclusively surfaces the Pi coding-agent runtime detected by the shell
// (detect_agents) and recent local Pi workflow runs.

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
import { EmptyState, Icon, Panel, StateNotice } from "../ui";
import { PiSetup } from "./PiSetup";

/** Middle-ellipsis for long runtime paths; full value stays in the title. */
function middleEllipsis(value: string, max = 72): string {
  if (value.length <= max) return value;
  const half = Math.floor((max - 1) / 2);
  return value.slice(0, half) + "…" + value.slice(value.length - (max - 1 - half));
}

export interface AgentsSectionProps {
  onUsePi?: () => void;
}

export default function AgentsSection({ onUsePi }: AgentsSectionProps = {}) {
  const [piAgent, setPiAgent] = useState<DetectedAgent | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [selectedPi, setSelectedPi] = useState(true);
  const [showSetup, setShowSetup] = useState(false);
  const [runs, setRuns] = useState<AgentRunRecord[]>(() =>
    loadAgentRunHistory().filter((r) => r.runtime.toLowerCase().includes("pi"))
  );
  const [activity, setActivity] = useState<AgentActivity>(() => getAgentActivity());

  const loadPiAgent = useCallback(async () => {
    setLoading(true);
    setLoadError(false);
    try {
      const detected = await api.detectAgents();
      const pi = detected.find((agent) => agent.id === "pi") ?? null;
      setPiAgent(pi);
    } catch {
      setPiAgent(null);
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadPiAgent();
  }, [loadPiAgent]);

  useEffect(() => {
    return subscribeAgentRuns(() =>
      setRuns(loadAgentRunHistory().filter((r) => r.runtime.toLowerCase().includes("pi")))
    );
  }, []);

  useEffect(() => {
    return subscribeAgentActivity(() => setActivity(getAgentActivity()));
  }, []);

  const handleUsePi = () => {
    setSelectedPi(true);
    if (onUsePi) {
      onUsePi();
    } else {
      const panelEl = document.querySelector(".workflows-agent-panel, [data-panel='pi-workflow']");
      panelEl?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }
  };

  return (
    <Panel
      id="workflows-agents"
      title="Agents"
      hint="Local Pi coding-agent runtime and execution status on this machine."
    >
      {loading ? (
        <p className="muted" aria-busy="true">
          Checking Pi runtime…
        </p>
      ) : null}

      {!loading && loadError ? (
        <StateNotice
          tone="error"
          title="Pi runtime check failed"
          body="Could not query local Pi installation."
          actionLabel="Retry"
          onAction={() => void loadPiAgent()}
        />
      ) : null}

      {!loading && !loadError && (!piAgent || !piAgent.available) ? (
        <div className="pi-agent-missing">
          <EmptyState
            title="Pi agent runtime not detected"
            body="Install or configure the Pi CLI runtime below to enable local agent workflows."
          />
          <div style={{ marginTop: 12 }}>
            <PiSetup onChanged={() => void loadPiAgent()} />
          </div>
        </div>
      ) : null}

      {!loading && !loadError && piAgent && piAgent.available ? (
        <>
          <ul className="agent-catalog">
            <li key={piAgent.id} className="agent-catalog-row">
              <span className="agent-catalog-icon" aria-hidden="true">
                <Icon name="workflows" />
              </span>
              <span className="agent-catalog-main">
                <span className="agent-catalog-title">{piAgent.name || "Pi"}</span>
                <span className="agent-default-tag">Primary Runtime</span>
                {piAgent.version ? (
                  <span className="agent-badge">v{piAgent.version}</span>
                ) : null}
                {activity.active ? (
                  <span className="agent-badge agent-badge--running">
                    <span className="agent-run-dot agent-run-dot--ok" aria-hidden="true" />
                    Running
                  </span>
                ) : (
                  <span className="agent-badge agent-badge--available">Available</span>
                )}
              </span>
              {piAgent.path ? (
                <span className="agent-catalog-path" title={piAgent.path}>
                  {middleEllipsis(piAgent.path)}
                </span>
              ) : null}
              <span className="agent-catalog-detail">
                {piAgent.detail ?? "Pi coding-agent runtime ready for local workflows."}
              </span>
              <span className="agent-catalog-actions" style={{ display: "flex", gap: 8 }}>
                <button
                  type="button"
                  className="btn"
                  onClick={() => setShowSetup((prev) => !prev)}
                >
                  {showSetup ? "Hide setup" : "Configure"}
                </button>
                {selectedPi ? (
                  <span className="badge badge-quiet">Active</span>
                ) : (
                  <button type="button" className="btn" onClick={handleUsePi}>
                    Use Pi
                  </button>
                )}
              </span>
            </li>
          </ul>

          {showSetup ? (
            <div style={{ marginTop: 12 }}>
              <PiSetup onChanged={() => void loadPiAgent()} />
            </div>
          ) : null}
        </>
      ) : null}

      <div className="agent-runs">
        <h3 className="agent-section-label">Recent Pi runs</h3>
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
                  Pi · {relativeTime(run.endedAt)} · {formatElapsed(run.durationMs)}
                </span>
              </li>
            ))}
          </ol>
        )}
        <p className="field-help">
          Run history stays on this PC and keeps status, time and duration only – never prompts,
          tool output or paths.
        </p>
      </div>
    </Panel>
  );
}