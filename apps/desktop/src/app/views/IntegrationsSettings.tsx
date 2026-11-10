// Preferences → Windows integrations. Read-only capability snapshot from the
// shell plus the real source/privacy switches; unsupported capabilities offer
// only the allow-listed Windows settings fallback.

import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { ControlsStatus } from "../../bridge/types";
import { Icon, StateNotice, Switch } from "../ui";

const CAPABILITIES: { key: keyof ControlsStatus; label: string; description: string; page: string }[] = [
  { key: "mediaCapability", label: "Media controls", description: "Play, pause and skip from the island.", page: "sound" },
  { key: "wifi", label: "Wi-Fi", description: "Show and switch the current Wi-Fi radio.", page: "wifi" },
  { key: "bluetooth", label: "Bluetooth", description: "Show paired devices and the Bluetooth radio.", page: "bluetooth" },
  { key: "brightnessCapability", label: "Display brightness", description: "Adjust the brightness of supported displays.", page: "display" },
  { key: "notifications", label: "Notifications", description: "Read recent Windows notifications in the island.", page: "notifications" },
  { key: "nightLight", label: "Night light", description: "Open the Windows night-light control.", page: "night_light" },
];

export default function IntegrationsSettings() {
  const { settings, updateSettings } = useApp();
  const [status, setStatus] = useState<ControlsStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [actionError, setActionError] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(false);
    try {
      setStatus(await api.getControlsStatus());
    } catch {
      setStatus(null);
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const patch = async (next: Parameters<typeof updateSettings>[0]) => {
    setActionError(false);
    try {
      await updateSettings(next);
    } catch {
      setActionError(true);
    }
  };

  const openWindows = async (page: string) => {
    try {
      await api.openSettings(page);
    } catch {
      setActionError(true);
    }
  };

  return (
    <div className="settings-grid settings-grid--single">
      <section className="settings-section" aria-labelledby="prefs-integrations-windows">
        <div className="settings-label">
          <h2 id="prefs-integrations-windows">Windows integrations</h2>
        </div>
        <div className="panel">
          {actionError ? (
            <StateNotice
              tone="error"
              title="Couldn't save that change."
              body="Your previous setting is still active."
            />
          ) : null}
          {loading ? (
            <p className="field-help" aria-busy="true">Checking Windows capabilities…</p>
          ) : loadError ? (
            <StateNotice
              title="Couldn't read Windows capabilities."
              body="Your settings are unaffected. Island cards keep their current state."
              actionLabel="Try again"
              onAction={() => void load()}
            />
          ) : (
            <ul className="integration-list">
              {CAPABILITIES.map((entry) => {
                const capability = status?.[entry.key];
                const supported =
                  typeof capability === "object" &&
                  capability !== null &&
                  "supported" in capability &&
                  Boolean((capability as { supported?: boolean }).supported);
                return (
                  <li className="integration-row" key={entry.label}>
                    <span className="integration-icon" aria-hidden="true">
                      <Icon name="settings" size={16} />
                    </span>
                    <span className="integration-main">
                      <span className="integration-title">{entry.label}</span>
                      <span className="integration-desc">{entry.description}</span>
                    </span>
                    <span className={"badge " + (supported ? "badge-quiet" : "badge-muted")}>
                      {supported ? "Available" : "Unavailable"}
                    </span>
                    {!supported ? (
                      <button
                        type="button"
                        className="linkbtn"
                        onClick={() => void openWindows(entry.page)}
                      >
                        Windows settings
                      </button>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          )}
          <p className="field-help">
            Capabilities are reported by the desktop shell. Unsupported features are never rendered
            as if they were on.
          </p>
        </div>
      </section>

      <section className="settings-section" aria-labelledby="prefs-integrations-sources">
        <div className="settings-label">
          <h2 id="prefs-integrations-sources">Sources &amp; capture</h2>
        </div>
        <div className="panel">
          <div className="pref-row">
            <div className="pref-copy">
              <span className="pref-title">Browser history &amp; bookmarks</span>
              <span className="pref-desc">Read local browser profiles; nothing is uploaded.</span>
            </div>
            <Switch
              checked={settings.privacy.browserHistory}
              label="Browser history and bookmarks"
              onChange={(next) =>
                void patch({ privacy: { ...settings.privacy, browserHistory: next } })
              }
            />
          </div>
          <div className="pref-row">
            <div className="pref-copy">
              <span className="pref-title">Git repositories</span>
              <span className="pref-desc">Index commits and TODOs inside indexed folders.</span>
            </div>
            <Switch
              checked={settings.privacy.gitIndexing}
              label="Git repositories"
              onChange={(next) =>
                void patch({ privacy: { ...settings.privacy, gitIndexing: next } })
              }
            />
          </div>
          <div className="pref-row">
            <div className="pref-copy">
              <span className="pref-title">Clipboard images</span>
              <span className="pref-desc">Keep image clipboard entries so they can be searched.</span>
            </div>
            <Switch
              checked={settings.clipboard.captureImages}
              label="Clipboard images"
              onChange={(next) =>
                void patch({ clipboard: { ...settings.clipboard, captureImages: next } })
              }
            />
          </div>
          <p className="field-help">
            Capture starts now; previous Windows clipboard history is not imported or backfilled.
            Activity and clipboard clearing remain in Privacy &amp; history.
          </p>
        </div>
      </section>
    </div>
  );
}
