// Pi on the Workflows page: its setup, the default model the island Agents card
// runs, and the providers Pi can use. Keys go to Pi's own config and are never
// shown again.

import { useCallback, useEffect, useState, type FormEvent } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { AgentModel, AgentSettings, PiProvider } from "../../bridge/types";
import { Panel, Switch } from "../ui";
import { PiSetup } from "./PiSetup";

/** The add form's choice for an OpenAI-compatible endpoint. */
const ENDPOINT = "endpoint";

function Providers({ onChanged }: { onChanged: () => void }) {
  const [providers, setProviders] = useState<PiProvider[]>([]);
  const [choice, setChoice] = useState("");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [modelIds, setModelIds] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.piProviders().then(setProviders, (reason) => setError(String(reason)));
  }, []);

  const apply = async (action: () => Promise<PiProvider[]>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    try {
      setProviders(await action());
      onChanged();
      return true;
    } catch (reason) {
      setError(String(reason));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const add = async (event: FormEvent) => {
    event.preventDefault();
    const saved = await apply(() =>
      choice === ENDPOINT
        ? api.piEndpointSave(name, baseUrl, apiKey, modelIds.split(/[\s,]+/).filter(Boolean))
        : api.piProviderSave(choice, apiKey),
    );
    if (saved) {
      setChoice("");
      setName("");
      setBaseUrl("");
      setModelIds("");
      setApiKey("");
    }
  };

  const listed = providers.filter((provider) => provider.saved || provider.fromEnv);
  return (
    <Panel id="workflows-providers" title="Providers" hint="Models from these providers appear in the Agents card.">
      {listed.length > 0 ? (
        <ul className="agent-list">
          {listed.map((provider) => (
            <li key={provider.id} className="agent-row">
              <div className="agent-row-head">
                <span className="agent-row-name">{provider.name}</span>
                <span className="agent-row-state">
                  {provider.custom
                    ? `${provider.baseUrl ?? ""} · ${provider.models.join(", ")}`
                    : provider.saved
                      ? "API key ••••••••"
                      : "Key set in the environment"}
                </span>
                {provider.saved ? (
                  <button
                    type="button"
                    className="btn"
                    aria-label={`Remove ${provider.name}`}
                    disabled={busy}
                    onClick={() => void apply(() => api.piProviderRemove(provider.id))}
                  >
                    Remove
                  </button>
                ) : (
                  <span />
                )}
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted">No providers yet.</p>
      )}
      <form className="agent-config" onSubmit={(event) => void add(event)}>
        <label className="agent-field">
          <span>Add a provider</span>
          <select className="input" value={choice} onChange={(event) => setChoice(event.target.value)}>
            <option value="">Choose a provider</option>
            {providers
              .filter((provider) => !provider.custom)
              .map((provider) => (
                <option key={provider.id} value={provider.id}>
                  {provider.name}
                </option>
              ))}
            <option value={ENDPOINT}>OpenAI-compatible endpoint</option>
          </select>
        </label>
        {choice === ENDPOINT ? (
          <>
            <label className="agent-field">
              <span>Name</span>
              <input className="input" value={name} onChange={(event) => setName(event.target.value)} placeholder="LM Studio" />
            </label>
            <label className="agent-field">
              <span>Base URL</span>
              <input
                className="input"
                value={baseUrl}
                onChange={(event) => setBaseUrl(event.target.value)}
                placeholder="http://localhost:1234/v1"
              />
            </label>
            <label className="agent-field">
              <span>Model ids</span>
              <input
                className="input"
                value={modelIds}
                onChange={(event) => setModelIds(event.target.value)}
                placeholder="Separate with commas"
              />
            </label>
          </>
        ) : null}
        {choice ? (
          <>
            <label className="agent-field">
              <span>{choice === ENDPOINT ? "API key (optional)" : "API key"}</span>
              <input
                className="input"
                type="password"
                autoComplete="off"
                spellCheck={false}
                value={apiKey}
                onChange={(event) => setApiKey(event.target.value)}
              />
            </label>
            <div className="config-actions">
              <button type="submit" className="btn" disabled={busy}>
                Save provider
              </button>
            </div>
          </>
        ) : null}
      </form>
      {error ? (
        <p role="alert" className="agent-notice">
          {error}
        </p>
      ) : null}
    </Panel>
  );
}

export default function AgentsSection() {
  const { settings, updateSettings } = useApp();
  const [models, setModels] = useState<AgentModel[]>([]);
  const [notice, setNotice] = useState<string | null>(null);

  const loadModels = useCallback(() => {
    api.agentModels().then(setModels, () => setModels([]));
  }, []);
  useEffect(loadModels, [loadModels]);

  const save = async (agent: AgentSettings) => {
    setNotice(null);
    try {
      await updateSettings({ agent });
    } catch (reason) {
      setNotice(String(reason));
    }
  };

  const model = settings.agent.model ?? null;
  const options = models.map((item) => `${item.provider}/${item.id}`);
  if (model && !options.includes(model)) options.unshift(model);

  return (
    <>
      <Panel
        id="workflows-agents"
        title="Pi agent"
        actions={
          <div className="privacy-control">
            <span>Agent runs</span>
            <Switch
              checked={settings.agent.enabled}
              label="Agent runs"
              onChange={(enabled) => void save({ ...settings.agent, enabled })}
            />
          </div>
        }
      >
        <PiSetup onChanged={loadModels} />
        <label className="agent-field">
          <span>Default model</span>
          {models.length > 0 ? (
            <select
              className="input"
              value={model ?? ""}
              onChange={(event) => void save({ ...settings.agent, model: event.target.value || null })}
            >
              <option value="">Pi default</option>
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
              placeholder="Pi default"
              onBlur={(event) => {
                const value = event.target.value.trim() || null;
                if (value !== model) void save({ ...settings.agent, model: value });
              }}
            />
          )}
        </label>
        <div className="agent-safety">
          <p>
            Pi runs with your Windows permissions. Read tools can open any file you can read; the workspace is the default
            working folder, not a sandbox.
          </p>
          <p>There is no per-tool approval. File edits and shell access are disabled in this alpha.</p>
          <p>When a run starts, your prompt, context and tool output are sent to the configured provider.</p>
        </div>
        {notice ? (
          <p role="alert" className="agent-notice">
            {notice}
          </p>
        ) : null}
      </Panel>
      <Providers onChanged={loadModels} />
    </>
  );
}
