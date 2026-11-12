// Agents on the Workflows page: Pi and the four agent CLIs, one row each. The
// "Default agent" choice sets the agent the island Agents card runs; a row shows
// the install state, runs a one-word Test, and sets that agent's default model.

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { AgentModel, AgentRuntime, AgentSettings, DetectedAgent } from "../../bridge/types";
import { AGENT_RUNTIMES } from "../agentRun";
import { Panel, StateNotice } from "../ui";
import { PiSetup } from "./PiSetup";

type TestResult =
  | { status: "running" }
  | { status: "passed"; detail: string }
  | { status: "failed"; message: string };

/** Middle-ellipsis for long paths; the full value stays in the title. */
function middleEllipsis(value: string, max = 72): string {
  if (value.length <= max) return value;
  const half = Math.floor((max - 1) / 2);
  return value.slice(0, half) + "…" + value.slice(value.length - (max - 1 - half));
}

/** Pi keeps its default model in `model`; each CLI keeps its own in `cliModels`. */
function modelOf(agent: AgentSettings, id: AgentRuntime): string | null {
  return id === "pi" ? (agent.model ?? null) : (agent.cliModels?.[id] ?? null);
}

function withModel(agent: AgentSettings, id: AgentRuntime, value: string | null): AgentSettings {
  if (id === "pi") return { ...agent, model: value };
  const cliModels = { ...agent.cliModels };
  if (value) cliModels[id] = value;
  else delete cliModels[id];
  return { ...agent, cliModels };
}

interface AgentRowProps {
  name: string;
  found: DetectedAgent | null;
  stateLabel: string;
  isDefault: boolean;
  open: boolean;
  test: TestResult | undefined;
  model: string | null;
  models: AgentModel[];
  onToggle: () => void;
  onTest: () => void;
  onUseDefault: () => void;
  onModel: (value: string | null) => void;
  children: ReactNode;
}

function AgentRow({
  name,
  found,
  stateLabel,
  isDefault,
  open,
  test,
  model,
  models,
  onToggle,
  onTest,
  onUseDefault,
  onModel,
  children,
}: AgentRowProps) {
  const running = test?.status === "running";
  const options = models.map((item) => `${item.provider}/${item.id}`);
  if (model && !options.includes(model)) options.unshift(model);
  return (
    <li className="agent-row">
      <div className="agent-row-head">
        <button type="button" className="agent-row-toggle" aria-expanded={open} onClick={onToggle}>
          <span className="agent-row-name">{name}</span>
          <span className={found ? "agent-row-state" : "agent-row-state is-missing"}>{stateLabel}</span>
          {isDefault ? <span className="agent-row-default">Default</span> : null}
        </button>
        <span className="agent-row-result" aria-live="polite">
          {running ? <span className="muted">Testing…</span> : null}
          {test?.status === "passed" ? <span className="agent-test-pass">Passed · {test.detail}</span> : null}
          {test?.status === "failed" ? (
            <span className="agent-test-fail" title={test.message}>
              Failed · {test.message}
            </span>
          ) : null}
        </span>
        <button type="button" className="btn" aria-label={`Test ${name}`} disabled={!found || running} onClick={onTest}>
          Test
        </button>
      </div>
      {open ? (
        <div className="agent-config">
          {found?.path ? (
            <p className="agent-path" title={found.path}>
              {middleEllipsis(found.path)}
            </p>
          ) : null}
          <div className="agent-config-row">
            <label className="agent-field">
              <span>Default model</span>
              {models.length > 0 ? (
                <select className="input" value={model ?? ""} onChange={(event) => onModel(event.target.value || null)}>
                  <option value="">Agent default</option>
                  {options.map((option) => (
                    <option key={option} value={option}>
                      {option}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  key={model ?? ""}
                  className="input"
                  defaultValue={model ?? ""}
                  placeholder="Agent default"
                  onBlur={(event) => onModel(event.target.value.trim() || null)}
                />
              )}
            </label>
            <button type="button" className="btn" disabled={!found || isDefault} onClick={onUseDefault}>
              {isDefault ? "Default agent" : "Use as default agent"}
            </button>
          </div>
          {children}
        </div>
      ) : null}
    </li>
  );
}

export default function AgentsSection() {
  const { settings, updateSettings } = useApp();
  const [detected, setDetected] = useState<DetectedAgent[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [openId, setOpenId] = useState<AgentRuntime | null>(null);
  const [tests, setTests] = useState<Partial<Record<AgentRuntime, TestResult>>>({});
  const [models, setModels] = useState<Partial<Record<AgentRuntime, AgentModel[]>>>({});
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async (refresh: boolean) => {
    setLoadError(null);
    try {
      setDetected(await api.detectAgents(refresh));
    } catch (reason) {
      setLoadError(String(reason));
    }
  }, []);

  const refreshAll = useCallback(() => {
    setTests({});
    setModels({});
    void load(true);
  }, [load]);

  useEffect(() => {
    void load(false);
  }, [load]);

  // Model lists load once per row, when the row is first opened.
  useEffect(() => {
    if (openId === null || models[openId] !== undefined) return;
    let current = true;
    api.agentModels(openId).then(
      (list) => {
        if (current) setModels((previous) => ({ ...previous, [openId]: list }));
      },
      () => {
        if (current) setModels((previous) => ({ ...previous, [openId]: [] }));
      },
    );
    return () => {
      current = false;
    };
  }, [openId, models]);

  const runTest = async (id: AgentRuntime) => {
    setTests((previous) => ({ ...previous, [id]: { status: "running" } }));
    try {
      const detail = await api.agentTest(id);
      setTests((previous) => ({ ...previous, [id]: { status: "passed", detail } }));
    } catch (reason) {
      setTests((previous) => ({ ...previous, [id]: { status: "failed", message: String(reason) } }));
    }
  };

  const useAsDefault = async (id: AgentRuntime) => {
    setNotice(null);
    try {
      await updateSettings({ agent: { ...settings.agent, enabled: true, runtime: id } });
    } catch (reason) {
      setNotice(String(reason));
    }
  };

  const setModel = async (id: AgentRuntime, value: string | null) => {
    if (value === modelOf(settings.agent, id)) return;
    setNotice(null);
    try {
      await updateSettings({ agent: withModel(settings.agent, id, value) });
    } catch (reason) {
      setNotice(String(reason));
    }
  };

  const defaultId: AgentRuntime = settings.agent.runtime ?? "pi";
  const piDetails = (
    <>
      <PiSetup onChanged={refreshAll} />
      <div className="agent-safety">
        <p>
          Pi runs with your Windows permissions. Read tools can open any file you can read; the workspace is the default
          working folder, not a sandbox.
        </p>
        <p>There is no per-tool approval. File edits and shell access are disabled in this alpha.</p>
        <p>When a run starts, your prompt, context and tool output are sent to the configured provider.</p>
      </div>
    </>
  );

  return (
    <Panel
      id="workflows-agents"
      title="Agents"
      actions={
        <button type="button" className="btn" onClick={refreshAll}>
          Refresh
        </button>
      }
    >
      {loadError ? (
        <StateNotice
          tone="error"
          title="Agents could not be checked"
          body={loadError}
          actionLabel="Retry"
          onAction={refreshAll}
        />
      ) : null}
      <label className="agent-field">
        <span>Default agent</span>
        <select className="input" value={defaultId} onChange={(event) => void useAsDefault(event.target.value as AgentRuntime)}>
          {AGENT_RUNTIMES.map((entry) => {
            const installed = detected?.some((item) => item.id === entry.id && item.available) ?? false;
            return (
              <option key={entry.id} value={entry.id} disabled={!installed}>
                {installed ? entry.name : `${entry.name} (not installed)`}
              </option>
            );
          })}
        </select>
      </label>
      <ul className="agent-list">
        {AGENT_RUNTIMES.map((entry) => {
          const found = detected?.find((item) => item.id === entry.id && item.available) ?? null;
          const stateLabel =
            detected === null
              ? loadError
                ? "Not checked"
                : "Checking…"
              : found
                ? found.version
                  ? `Installed · ${found.version}`
                  : "Installed"
                : "Not installed";
          return (
            <AgentRow
              key={entry.id}
              name={entry.name}
              found={found}
              stateLabel={stateLabel}
              isDefault={defaultId === entry.id}
              open={openId === entry.id}
              test={tests[entry.id]}
              model={modelOf(settings.agent, entry.id)}
              models={models[entry.id] ?? []}
              onToggle={() => setOpenId((current) => (current === entry.id ? null : entry.id))}
              onTest={() => void runTest(entry.id)}
              onUseDefault={() => void useAsDefault(entry.id)}
              onModel={(value) => void setModel(entry.id, value)}
            >
              {entry.id === "pi" ? (
                piDetails
              ) : (
                <p className="agent-safety">
                  {entry.name} runs with your Windows permissions. Your prompt is sent to its model provider.
                </p>
              )}
            </AgentRow>
          );
        })}
      </ul>
      {notice ? (
        <p role="alert" className="agent-notice">
          {notice}
        </p>
      ) : null}
    </Panel>
  );
}
