// Settings: a section list on the left and the selected section on the right.
// Every change saves as it is made; one inline status line reports the result.

import { type CSSProperties, useCallback, useEffect, useRef, useState } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { AppInfo, IslandSettings, SettingsPatch, ThemeMode, WebSearchEngine } from "../../bridge/types";
import { formatNumber } from "../../lib/format";
import { lookFrom, writeIslandLook } from "../../lib/islandLook";
import { checkForUpdate, RELEASES_PAGE, type UpdateCheck } from "../../lib/updates";
import { indexCountText, relativeTime, stateLabel } from "../describe";
import { HotkeyCapture } from "../HotkeyInput";
import { normalizeIslandSettings } from "../islandLogic";
import { SECTION_EYEBROWS, type Section } from "../sections";
import { RouteHeading, SegmentedControl, StateNotice, Switch } from "../ui";
import IntegrationsSettings from "./IntegrationsSettings";
import IslandCardsSettings from "./IslandSettingsView";
import { PrefRow } from "./PrefRow";
import "./settings.css";

type SectionId = "general" | "appearance" | "privacy" | "cards" | "windows" | "diagnostics";

const SECTIONS: { id: SectionId; label: string }[] = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "privacy", label: "Privacy & history" },
  { id: "cards", label: "Cards & order" },
  { id: "windows", label: "Windows integration" },
  { id: "diagnostics", label: "Diagnostics" },
];

const THEME_OPTIONS: { value: ThemeMode; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const WEB_SEARCH_ENGINES: { value: WebSearchEngine; label: string }[] = [
  { value: "google", label: "Google" },
  { value: "bing", label: "Bing" },
  { value: "duck_duck_go", label: "DuckDuckGo" },
  { value: "you_tube", label: "YouTube" },
];

const SAVED = "Saved";
const SAVE_FAILED = "Couldn't save. The previous setting is still active.";

interface Notice {
  tone: "ok" | "error";
  text: string;
}

function ValueRow({ title, value }: { title: string; value: string }) {
  return (
    <PrefRow title={title}>
      <span className="prefs-row-value">{value}</span>
    </PrefRow>
  );
}

// GitHub is asked only when the button is clicked.
function UpdateRow({ version }: { version?: string }) {
  const [check, setCheck] = useState<UpdateCheck | { kind: "checking" } | null>(null);

  const run = async () => {
    if (!version) return;
    setCheck({ kind: "checking" });
    setCheck(await checkForUpdate(version));
  };

  const download = async () => {
    const outcome = await api.openResult({ type: "open_url", url: RELEASES_PAGE }).catch(() => null);
    if (!outcome?.ok) setCheck({ kind: "error", text: "Couldn't open the download page." });
  };

  const text = !check
    ? ""
    : check.kind === "checking"
      ? "Checking…"
      : check.kind === "current"
        ? "You're up to date"
        : check.kind === "available"
          ? `Version ${check.version} is available`
          : check.text;

  return (
    <PrefRow title="Check for updates" description={"Current version " + (version ?? "unknown")}>
      <span className="prefs-row-value" role="status">
        {text}
      </span>
      {check?.kind === "available" ? (
        <button type="button" className="btn btn-primary" onClick={() => void download()}>
          Download
        </button>
      ) : (
        <button
          type="button"
          className="btn"
          disabled={!version || check?.kind === "checking"}
          onClick={() => void run()}
        >
          Check now
        </button>
      )}
    </PrefRow>
  );
}

export default function PreferencesView({ onNavigate }: { onNavigate: (section: Section) => void }) {
  const { settings, status, updateSettings, refreshStatus } = useApp();
  const [section, setSection] = useState<SectionId>("general");
  const [notice, setNotice] = useState<Notice | null>(null);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const noticeTimer = useRef<number | null>(null);

  const flash = useCallback((text: string, tone: Notice["tone"] = "ok") => {
    if (noticeTimer.current) window.clearTimeout(noticeTimer.current);
    setNotice({ text, tone });
    noticeTimer.current = window.setTimeout(() => setNotice(null), tone === "error" ? 5000 : 2500);
  }, []);

  useEffect(() => {
    return () => {
      if (noticeTimer.current) window.clearTimeout(noticeTimer.current);
    };
  }, []);

  useEffect(() => {
    let alive = true;
    void api
      .appInfo()
      .then((info) => {
        if (alive) setAppInfo(info);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);

  const apply = useCallback(
    async (patch: SettingsPatch) => {
      try {
        await updateSettings(patch);
        flash(SAVED);
      } catch {
        flash(SAVE_FAILED, "error");
      }
    },
    [updateSettings, flash],
  );

  // Island changes apply at once: the view updates immediately and the writes run
  // one at a time, each sending the newest state, so a slider drag never leaves
  // an older value saved.
  const launcherMode = settings.launcher.primaryMode;
  const [island, setIsland] = useState<IslandSettings>(() => normalizeIslandSettings(settings.launcher.island));
  const islandRef = useRef(island);
  const confirmedRef = useRef(island);
  const saveQueue = useRef<Promise<void>>(Promise.resolve());
  const saveQueued = useRef(false);

  const saveIsland = useCallback(
    (patch: Partial<IslandSettings>) => {
      const next = normalizeIslandSettings({ ...islandRef.current, ...patch });
      islandRef.current = next;
      setIsland(next);
      if (saveQueued.current) return;
      saveQueued.current = true;
      saveQueue.current = saveQueue.current.then(async () => {
        saveQueued.current = false;
        try {
          const saved = await updateSettings({
            launcher: { primaryMode: launcherMode, island: islandRef.current },
          });
          confirmedRef.current = normalizeIslandSettings(saved.launcher.island);
          flash(SAVED);
        } catch {
          islandRef.current = confirmedRef.current;
          setIsland(confirmedRef.current);
          flash(SAVE_FAILED, "error");
        }
      });
    },
    [updateSettings, launcherMode, flash],
  );

  // The island window reads this cache for its first frame (see lib/islandLook).
  useEffect(() => {
    writeIslandLook(lookFrom(island, settings.theme));
  }, [island, settings.theme]);

  // api.setHotkey re-registers the shortcut and rejects it when another app owns it.
  const applyHotkey = async (accelerator: string) => {
    await api.setHotkey(accelerator);
    await updateSettings({});
  };

  const glassLevel = island.liquidGlassLevel ?? 60;
  const failed = status?.state === "error";
  const title = SECTIONS.find((entry) => entry.id === section)?.label ?? "";

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.preferences}>Settings</RouteHeading>
      <div className="prefs">
        <nav className="prefs-nav" aria-label="Settings sections">
          {SECTIONS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              className="prefs-nav-item"
              aria-current={section === entry.id ? "page" : undefined}
              onClick={() => setSection(entry.id)}
            >
              {entry.label}
            </button>
          ))}
        </nav>

        <div className="prefs-pane">
          <div className="prefs-pane-head">
            <h2 className="prefs-pane-title">{title}</h2>
            <span className="prefs-status" role="status" data-tone={notice?.tone ?? "ok"}>
              {notice?.text ?? ""}
            </span>
          </div>

          {section === "general" ? (
            <>
              <div className="prefs-group">
                <h3 className="prefs-group-title">Launcher shortcut</h3>
                <HotkeyCapture value={settings.hotkey} onCommit={applyHotkey} />
              </div>
              <div className="prefs-rows prefs-group">
                <PrefRow title="Start with Windows">
                  <Switch
                    checked={settings.launchAtLogin}
                    label="Start ProtonSearch with Windows"
                    onChange={(next) => void apply({ launchAtLogin: next })}
                  />
                </PrefRow>
              </div>
              <div className="prefs-group">
                <h3 className="prefs-group-title">Search</h3>
                <div className="prefs-rows">
                  <PrefRow
                    title="Offer web search results"
                    description="Adds a “Search the web for …” result to the launcher."
                  >
                    <Switch
                      checked={settings.webSearchEnabled}
                      label="Offer web search results"
                      onChange={(next) => void apply({ webSearchEnabled: next })}
                    />
                  </PrefRow>
                  <PrefRow title="Default search engine" description="Used by the web search result.">
                    <SegmentedControl
                      label="Default web search engine"
                      value={settings.webSearchEngine}
                      options={WEB_SEARCH_ENGINES}
                      disabled={!settings.webSearchEnabled}
                      onChange={(engine) => void apply({ webSearchEngine: engine })}
                    />
                  </PrefRow>
                </div>
              </div>
              <div className="prefs-group">
                <h3 className="prefs-group-title">Updates</h3>
                <div className="prefs-rows">
                  <UpdateRow version={appInfo?.version ?? status?.version} />
                </div>
              </div>
            </>
          ) : null}

          {section === "appearance" ? (
            <div className="prefs-rows">
              <PrefRow title="Theme" description="System follows the Windows app theme.">
                <SegmentedControl
                  label="Theme"
                  value={settings.theme}
                  options={THEME_OPTIONS}
                  onChange={(theme) => void apply({ theme })}
                />
              </PrefRow>
              <PrefRow title="Liquid glass" description="Applies to the Dynamic Island only.">
                <Switch
                  checked={island.liquidGlass === true}
                  label="Liquid glass"
                  onChange={(next) => saveIsland({ liquidGlass: next })}
                />
              </PrefRow>
              {island.liquidGlass ? (
                <PrefRow title="Glass amount" description="How much the island hides what's behind it.">
                  <div className="glass-amount">
                    <div className="glass-amount-slider">
                      <span>Clear</span>
                      <input
                        type="range"
                        className="prefs-range"
                        min={0}
                        max={100}
                        step={1}
                        aria-label="Glass amount"
                        value={glassLevel}
                        onChange={(event) => saveIsland({ liquidGlassLevel: Number(event.target.value) })}
                      />
                      <span>Frosted</span>
                    </div>
                    {/* Preview: the island's own tint over a sample background. */}
                    <div
                      className="glass-preview"
                      aria-hidden="true"
                      style={{ "--glass": glassLevel / 100 } as CSSProperties}
                    >
                      <div className="glass-preview-island">
                        <div className="glass-preview-row glass-preview-row--search" />
                        <div className="glass-preview-row" />
                        <div className="glass-preview-row" />
                      </div>
                    </div>
                  </div>
                </PrefRow>
              ) : null}
              <PrefRow title="Page dots" description="One dot per enabled card.">
                <Switch
                  checked={island.showDots}
                  label="Show page dots"
                  onChange={(next) => saveIsland({ showDots: next })}
                />
              </PrefRow>
            </div>
          ) : null}

          {section === "privacy" ? (
            <div className="prefs-rows">
              <PrefRow title="Record activity" description="Search and launch history.">
                <Switch
                  checked={settings.privacy.activityHistory}
                  label="Record activity"
                  onChange={(next) =>
                    void apply({ privacy: { ...settings.privacy, activityHistory: next } })
                  }
                />
              </PrefRow>
              <PrefRow
                title="Clipboard history"
                description="Save copied text so it can appear in search. Capture starts now; earlier Windows clipboard history is not imported."
              >
                <Switch
                  checked={settings.privacy.clipboardHistory}
                  label="Save clipboard history"
                  onChange={(next) =>
                    void apply({ privacy: { ...settings.privacy, clipboardHistory: next } })
                  }
                />
              </PrefRow>
            </div>
          ) : null}

          {section === "cards" ? <IslandCardsSettings island={island} onChange={saveIsland} /> : null}

          {section === "windows" ? <IntegrationsSettings onStatus={flash} /> : null}

          {section === "diagnostics" ? (
            <>
              {failed ? (
                <StateNotice
                  tone="error"
                  title="Indexing stopped with an error."
                  actionLabel="Search & indexes"
                  onAction={() => onNavigate("indexes")}
                />
              ) : null}
              <div className="prefs-rows">
                <ValueRow title="Version" value={appInfo?.version ?? status?.version ?? "Unknown"} />
                <ValueRow title="Protocol" value={"v" + (appInfo?.protocol ?? status?.protocol ?? 1)} />
                <ValueRow title="Data schema" value={"v" + (appInfo?.dataSchema ?? 1)} />
                <ValueRow
                  title="Platform"
                  value={appInfo ? appInfo.platform + " · " + appInfo.arch : "Unknown"}
                />
                <ValueRow title="Index state" value={status ? stateLabel(status.state) : "Unknown"} />
                <ValueRow title="Filename entries" value={indexCountText(status)} />
                <ValueRow title="Document texts" value={formatNumber(status?.contentDocuments ?? 0)} />
                <ValueRow title="OCR texts" value={formatNumber(status?.ocrDocuments ?? 0)} />
                <ValueRow title="Last updated" value={relativeTime(status?.lastUpdated)} />
                <ValueRow title="Data folder" value={"%APPDATA%\\protonsearch\\v2"} />
              </div>
              <div className="prefs-actions">
                <button type="button" className="btn" onClick={() => void refreshStatus()}>
                  Refresh
                </button>
                <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
                  Search &amp; indexes
                </button>
              </div>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}
