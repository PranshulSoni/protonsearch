// Preferences → Launcher: primary launcher mode, existing shortcut summary and
// the Dynamic Island master switch. Settings are real IPC fields; no fake
// normal-launcher options are exposed until the shell has them.

import { useApp } from "../../App";
import type { LauncherMode } from "../../bridge/types";
import { SavedStamp, useSavedStamp } from "../ui";

export default function LauncherPreferences({
  onNavigate,
}: {
  onNavigate: (tab: "general" | "launcher" | "island") => void;
}) {
  const { settings, updateSettings } = useApp();
  const { savedKey, markSaved } = useSavedStamp();

  const setMode = async (mode: LauncherMode) => {
    try {
      await updateSettings({ launcher: { ...settings.launcher, primaryMode: mode } });
      markSaved("launcher");
    } catch {
      /* the parent quiet notice covers rare failures on the General tab */
    }
  };

  const setIslandEnabled = async (enabled: boolean) => {
    try {
      await updateSettings({
        launcher: {
          ...settings.launcher,
          island: { ...settings.launcher.island, enabled },
        },
      });
      markSaved("launcher");
    } catch {
      /* keep the previous setting */
    }
  };

  return (
    <>
      <div className="settings-grid">
        <section className="settings-section" aria-labelledby="prefs-launcher-mode">
          <div className="settings-label">
            <h2 id="prefs-launcher-mode">Primary launcher</h2>
            <SavedStamp show={savedKey === "launcher"} />
          </div>
          <div className="panel">
            <p className="field-help">
              Choose what Alt + Space opens. You can switch at any time; the shortcut itself is
              unchanged.
            </p>
            <div className="launcher-mode-grid">
              {(
                [
                  {
                    value: "dynamic_island" as LauncherMode,
                    title: "Dynamic Island",
                    body: "A compact island at the top of the screen with cards, dots and the same search.",
                  },
                  {
                    value: "normal" as LauncherMode,
                    title: "Normal Launcher",
                    body: "The familiar centred launcher window with the full result list.",
                  },
                ]
              ).map((option) => (
                <label
                  key={option.value}
                  className={
                    "launcher-mode-card" +
                    (settings.launcher.primaryMode === option.value ? " is-selected" : "")
                  }
                >
                  <input
                    type="radio"
                    name="primary-launcher-preferences"
                    value={option.value}
                    checked={settings.launcher.primaryMode === option.value}
                    onChange={() => void setMode(option.value)}
                  />
                  <span className="launcher-mode-title">{option.title}</span>
                  <span className="launcher-mode-body">{option.body}</span>
                </label>
              ))}
            </div>
            <div className="section-actions">
              <button type="button" className="btn" onClick={() => onNavigate("island")}>
                Open Dynamic Island settings
              </button>
            </div>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-launcher-island">
          <div className="settings-label">
            <h2 id="prefs-launcher-island">Dynamic Island</h2>
          </div>
          <div className="panel">
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Enable Dynamic Island</span>
                <span className="pref-desc">
                  Keep the alternate launcher available. Placement and cards live in Dynamic Island
                  settings.
                </span>
              </div>
              <input
                className="pref-check"
                type="checkbox"
                checked={settings.launcher.island.enabled}
                aria-label="Enable Dynamic Island"
                onChange={(event) => void setIslandEnabled(event.target.checked)}
              />
            </div>
            <dl className="metric-list">
              <div className="metric-row">
                <dt>Shortcut</dt>
                <dd>
                  <kbd className="kbd">{settings.hotkey.replace(/\+/g, " + ")}</kbd>
                </dd>
              </div>
              <div className="metric-row">
                <dt>Primary launcher</dt>
                <dd>
                  {settings.launcher.primaryMode === "dynamic_island"
                    ? "Dynamic Island"
                    : "Normal Launcher"}
                </dd>
              </div>
            </dl>
            <div className="section-actions">
              <button type="button" className="btn" onClick={() => onNavigate("general")}>
                Change shortcut in General
              </button>
            </div>
            <p className="field-help">
              The shell applies window placement and shortcuts. These preferences are stored
              locally with the rest of your settings.
            </p>
          </div>
        </section>
      </div>
    </>
  );
}