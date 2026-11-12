import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { PiInstallPlan, PiInstallStatus } from "../../bridge/types";

export function PiSetup({ onChanged }: { onChanged: () => void }) {
  const { settings, updateSettings } = useApp();
  const [status, setStatus] = useState<PiInstallStatus | null>(null);
  const [plan, setPlan] = useState<PiInstallPlan | null>(null);
  const [busy, setBusy] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [path, setPath] = useState(settings.agent.piPath ?? "");
  const [provider, setProvider] = useState(settings.agent.provider ?? "");
  const refresh = useCallback(async () => {
    setStatus(await api.piInstallStatus(true));
    onChanged();
  }, [onChanged]);
  useEffect(() => {
    let disposed = false;
    void api.piInstallStatus().then((value) => { if (!disposed) setStatus(value); })
      .catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => { disposed = true; };
  }, []);
  const perform = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try { await action(); await refresh(); }
    catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  };
  return (
    <section aria-label="Pi setup" className="config-box">
      <p role="status">{installing ? "Installing Pi…" : status?.detail ?? "Checking Pi…"}</p>
      {status?.version ? <p>Pi {status.version} · {status.path} · {status.installationMethod}</p> : null}
      {status ? <p>Node.js {status.nodeOk ? "available" : "required"} · Git Bash {status.gitBashAvailable ? "available" : "not detected"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <form onSubmit={(event) => {
        event.preventDefault();
        void perform(async () => {
          await updateSettings({ agent: { ...settings.agent,
            piPath: path.trim() || null, provider: provider.trim() || null } });
        });
      }}>
        <label>Pi executable path<input className="input" value={path} onChange={(event) => setPath(event.target.value)} placeholder="Automatic detection" /></label>
        <label>Provider<input className="input" value={provider} onChange={(event) => setProvider(event.target.value)} placeholder="Pi default" /></label>
        <button type="submit" className="btn" disabled={busy}>Save Pi configuration</button>
      </form>
      <div className="config-actions">
        <button type="button" className="btn" disabled={busy} onClick={() => void perform(async () => undefined)}>Refresh Pi</button>
        <button type="button" className="btn" disabled={busy || !status?.path} onClick={() => void perform(() => api.piConnect())}>Connect model</button>
        <button type="button" className="btn" disabled={busy} onClick={() => void perform(async () => setPlan(await api.piInstallPlan()))}>Review Pi installation</button>
      </div>
      {plan ? <div role="group" aria-label="Confirm Pi installation">
        <p>Install Pi {plan.version} · {plan.license}</p>
        <p>Package size {(plan.unpackedBytes / 1048576).toFixed(1)} MB unpacked. Additional dependency downloads vary.</p>
        <p>Destination {plan.destination}</p>
        <a href={plan.sourceUrl} target="_blank" rel="noreferrer">Official Pi installation instructions</a>
        <p>npm verifies package integrity. Dependency lifecycle scripts are disabled.</p>
        <button type="button" className="btn" disabled={busy} onClick={() => void perform(async () => {
          setInstalling(true);
          try { await api.piInstall(plan.version, true); setPlan(null); }
          finally { setInstalling(false); }
        })}>Confirm installation</button>
        <button type="button" className="btn" disabled={!installing} onClick={() => { void api.piInstallCancel().catch((reason) => setError(String(reason))); }}>Cancel installation</button>
      </div> : null}
    </section>
  );
}
