// Settings → Windows integration. Capture sources each have their own switch.
// Capability rows report what this PC supports; no setting backs them, so they
// have no switch.

import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { ControlsStatus, SettingsPatch } from "../../bridge/types";
import { StateNotice, Switch } from "../ui";
import { PrefRow } from "./PrefRow";

const CAPABILITIES: { key: keyof ControlsStatus; label: string; page: string }[] = [
  { key: "mediaCapability", label: "Media controls", page: "sound" },
  { key: "wifi", label: "Wi-Fi", page: "wifi" },
  { key: "bluetooth", label: "Bluetooth", page: "bluetooth" },
  { key: "brightnessCapability", label: "Display brightness", page: "display" },
  { key: "notifications", label: "Notifications", page: "notifications" },
  { key: "nightLight", label: "Night light", page: "night_light" },
];

export default function IntegrationsSettings({
  onStatus,
}: {
  onStatus: (text: string, tone?: "ok" | "error") => void;
}) {
  const { settings, updateSettings } = useApp();
  const [capabilities, setCapabilities] = useState<ControlsStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(false);
    try {
      setCapabilities(await api.getControlsStatus());
    } catch {
      setCapabilities(null);
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const save = async (patch: SettingsPatch) => {
    try {
      await updateSettings(patch);
      onStatus("Saved");
    } catch {
      onStatus("Couldn't save. The previous setting is still active.", "error");
    }
  };

  const openWindows = async (page: string) => {
    try {
      await api.openSettings(page);
    } catch {
      onStatus("Couldn't open Windows settings.", "error");
    }
  };

  return (
    <>
      <div className="prefs-group">
        <h3 className="prefs-group-title">Capabilities</h3>
        {loading ? (
          <p className="prefs-row-desc" aria-busy="true">Checking Windows…</p>
        ) : loadError ? (
          <StateNotice
            title="Couldn't read Windows capabilities."
            actionLabel="Try again"
            onAction={() => void load()}
          />
        ) : (
          <div className="prefs-rows">
            {CAPABILITIES.map((entry) => {
              const capability = capabilities?.[entry.key];
              const supported =
                typeof capability === "object" &&
                capability !== null &&
                "supported" in capability &&
                Boolean((capability as { supported?: boolean }).supported);
              return (
                <PrefRow key={entry.label} title={entry.label}>
                  <span className={"badge " + (supported ? "badge-quiet" : "badge-muted")}>
                    {supported ? "Available" : "Unavailable"}
                  </span>
                  {!supported ? (
                    <button type="button" className="linkbtn" onClick={() => void openWindows(entry.page)}>
                      Windows settings
                    </button>
                  ) : null}
                </PrefRow>
              );
            })}
          </div>
        )}
      </div>

      <div className="prefs-group">
        <h3 className="prefs-group-title">Capture sources</h3>
        <div className="prefs-rows">
          <PrefRow title="Browser history & bookmarks" description="Read local browser profiles.">
            <Switch
              checked={settings.privacy.browserHistory}
              label="Browser history and bookmarks"
              onChange={(next) => void save({ privacy: { ...settings.privacy, browserHistory: next } })}
            />
          </PrefRow>
          <PrefRow title="Git repositories" description="Commits and TODO lines in indexed folders.">
            <Switch
              checked={settings.privacy.gitIndexing}
              label="Git repositories"
              onChange={(next) => void save({ privacy: { ...settings.privacy, gitIndexing: next } })}
            />
          </PrefRow>
          <PrefRow title="Clipboard images" description="Keep image clipboard entries so they can be searched.">
            <Switch
              checked={settings.clipboard.captureImages}
              label="Clipboard images"
              onChange={(next) => void save({ clipboard: { ...settings.clipboard, captureImages: next } })}
            />
          </PrefRow>
        </div>
      </div>
    </>
  );
}
