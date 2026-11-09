// Preferences route: shortcut, appearance, privacy, indexing, agent, diagnostics
// and about. Autosave with a 2 s "Saved" stamp; every toggle persists through
// update_settings and surfaces bridge errors instead of pretending success.

import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type {
  AppInfo,
  Density,
  MotionPreference,
  SettingsPatch,
  ThemeMode,
  WebSearchEngine,
} from "../../bridge/types";
import { formatNumber } from "../../lib/format";
import { indexCountText, relativeTime, stateLabel, summarizeWarnings } from "../describe";
import { HotkeyCapture } from "../HotkeyInput";
import { SECTION_EYEBROWS, type Section } from "../sections";
import {
  RouteHeading,
  SavedStamp,
  SegmentedControl,
  StateNotice,
  Switch,
  useSavedStamp,
} from "../ui";

const THEME_OPTIONS: { value: ThemeMode; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const DENSITY_OPTIONS: { value: Density; label: string }[] = [
  { value: "comfortable", label: "Comfortable" },
  { value: "compact", label: "Compact" },
];

const WEB_SEARCH_ENGINES: { value: WebSearchEngine; label: string }[] = [
  { value: "google", label: "Google" },
  { value: "bing", label: "Bing" },
  { value: "duck_duck_go", label: "DuckDuckGo" },
  { value: "you_tube", label: "YouTube" },
];

const MOTION_OPTIONS: { value: MotionPreference; label: string }[] = [
  { value: "system", label: "System" },
  { value: "full", label: "Full" },
  { value: "reduced", label: "Reduced" },
];

export default function PreferencesView({
  onNavigate,
}: {
  onNavigate: (section: Section) => void;
}) {
  const { settings, status, updateSettings, refreshStatus } = useApp();
  const { savedKey, markSaved } = useSavedStamp();
  const [actionError, setActionError] = useState<{
    title: string;
    body: string;
    retry?: () => void;
  } | null>(null);
  const [limitDraft, setLimitDraft] = useState(String(settings.resultLimit));
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const warningSummary = summarizeWarnings(status?.warnings ?? []);

  useEffect(() => {
    setLimitDraft(String(settings.resultLimit));
  }, [settings.resultLimit]);

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
    async function apply(patch: SettingsPatch, stamp: string) {
      setActionError(null);
      try {
        await updateSettings(patch);
        markSaved(stamp);
      } catch {
        setActionError({
          title: "Couldn't save that change.",
          body: "Your previous setting is still active.",
          retry: () => void apply(patch, stamp),
        });
      }
    },
    [updateSettings, markSaved],
  );

  // M2: re-register through the shell (api.setHotkey) so conflicts surface
  // immediately, then pull the saved Settings back into app context.
  const applyHotkey = useCallback(
    async (accelerator: string) => {
      await api.setHotkey(accelerator);
      await updateSettings({});
      markSaved("hotkey");
    },
    [updateSettings, markSaved],
  );

  const commitLimit = () => {
    const value = Number(limitDraft);
    if (!Number.isFinite(value)) {
      setLimitDraft(String(settings.resultLimit));
      return;
    }
    const clamped = Math.min(100, Math.max(5, Math.round(value)));
    setLimitDraft(String(clamped));
    if (clamped !== settings.resultLimit) {
      void apply({ resultLimit: clamped }, "launcher");
    }
  };

  const togglePrivacy = (key: keyof typeof settings.privacy, value: boolean) => {
    void apply({ privacy: { ...settings.privacy, [key]: value } }, "privacy");
  };

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.preferences}>Make it yours</RouteHeading>
      <p className="page-lead">
        Customize behavior, appearance and privacy independently of search performance.
      </p>

      {actionError ? (
        <StateNotice
          role="alert"
          title={actionError.title}
          body={actionError.body}
          actionLabel={actionError.retry ? "Try again" : undefined}
          onAction={actionError.retry}
        />
      ) : null}

      <div className="settings-grid">
        <section className="settings-section" aria-labelledby="prefs-hotkey">
          <div className="settings-label">
            <h2 id="prefs-hotkey">Launcher shortcut</h2>
            <SavedStamp show={savedKey === "hotkey"} />
          </div>
          <div className="panel">
            <HotkeyCapture
              value={settings.hotkey}
              onCommit={applyHotkey}
              help="Use at least one of Ctrl, Alt, Shift, or Win. The shortcut applies immediately; Windows rejects a combination another app already owns."
            />

            <dl className="metric-list">
              <div className="metric-row">
                <dt>Results per query</dt>
                <dd>
                  <input
                    className="input input-xs"
                    type="number"
                    min={5}
                    max={100}
                    step={5}
                    value={limitDraft}
                    aria-label="Results per query"
                    onChange={(event) => setLimitDraft(event.target.value)}
                    onBlur={commitLimit}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        event.preventDefault();
                        commitLimit();
                      }
                    }}
                  />
                </dd>
              </div>
              <div className="metric-row">
                <dt>Start with Windows</dt>
                <dd>
                  <Switch
                    checked={settings.launchAtLogin}
                    label="Start ProtonSearch with Windows"
                    onChange={(next) => void apply({ launchAtLogin: next }, "hotkey")}
                  />
                </dd>
              </div>
            </dl>
            <p className="field-help">
              Between 5 and 100 results. Larger lists take longer to render; the engine stays the
              same.
            </p>
          </div>
        </section>



        <section className="settings-section" aria-labelledby="prefs-search">
          <div className="settings-label">
            <h2 id="prefs-search">Search</h2>
            <SavedStamp show={savedKey === "search"} />
          </div>
          <div className="panel">
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Offer web search results</span>
                <span className="pref-desc">
                  Adds a “Search the web for …” action to launcher results. Nothing is sent until
                  you choose it.
                </span>
              </div>
              <Switch
                checked={settings.webSearchEnabled}
                label="Offer web search results"
                onChange={(next) => void apply({ webSearchEnabled: next }, "search")}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Default search engine</span>
                <span className="pref-desc">Used by the “Search the web for …” result.</span>
              </div>
              <SegmentedControl
                label="Default web search engine"
                value={settings.webSearchEngine}
                options={WEB_SEARCH_ENGINES}
                disabled={!settings.webSearchEnabled}
                onChange={(engine) => void apply({ webSearchEngine: engine }, "search")}
              />
            </div>
            <p className="field-help">
              Opens the result in your browser; nothing is sent until you choose it.
            </p>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-appearance">
          <div className="settings-label">
            <h2 id="prefs-appearance">Appearance</h2>
            <SavedStamp show={savedKey === "appearance"} />
          </div>
          <div className="panel">
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Theme</span>
                <span className="pref-desc">System follows the Windows app theme.</span>
              </div>
              <SegmentedControl
                label="Theme"
                value={settings.theme}
                options={THEME_OPTIONS}
                onChange={(theme) => void apply({ theme }, "appearance")}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Density</span>
                <span className="pref-desc">Compact tightens row pitch and control heights.</span>
              </div>
              <SegmentedControl
                label="Density"
                value={settings.density}
                options={DENSITY_OPTIONS}
                onChange={(density) => void apply({ density }, "appearance")}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Motion</span>
                <span className="pref-desc">System follows your Windows animation setting.</span>
              </div>
              <SegmentedControl
                label="Motion"
                value={settings.motion}
                options={MOTION_OPTIONS}
                onChange={(motion) => void apply({ motion }, "appearance")}
              />
            </div>
            <div className="preview-row" aria-hidden="true">
              <span className="preview-tile">AB</span>
              <span className="preview-text">
                <span className="preview-title">Result preview</span>
                <span className="preview-sub">Title and path use the selected density</span>
              </span>
              <kbd className="kbd">Enter</kbd>
            </div>
            <p className="field-help">Preview only — real rows come from the engine.</p>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-privacy">
          <div className="settings-label">
            <h2 id="prefs-privacy">Privacy &amp; history</h2>
            <SavedStamp show={savedKey === "privacy"} />
          </div>
          <div className="panel">
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Record activity</span>
                <span className="pref-desc">Search and launch history stays on this PC.</span>
              </div>
              <Switch
                checked={settings.privacy.activityHistory}
                label="Record activity"
                onChange={(next) => togglePrivacy("activityHistory", next)}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Clipboard history</span>
                <span className="pref-desc">
                  Save copied text locally so it can appear in search. Capture starts now; previous
                  Windows clipboard history is not imported or backfilled.
                </span>
              </div>
              <Switch
                checked={settings.privacy.clipboardHistory}
                label="Save clipboard history"
                onChange={(next) => togglePrivacy("clipboardHistory", next)}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Browser history &amp; bookmarks</span>
                <span className="pref-desc">Read local browser profiles; nothing is uploaded.</span>
              </div>
              <Switch
                checked={settings.privacy.browserHistory}
                label="Index browser history and bookmarks"
                onChange={(next) => togglePrivacy("browserHistory", next)}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Git repositories</span>
                <span className="pref-desc">Index commits and TODO/FIXME tasks inside indexed folders.</span>
              </div>
              <Switch
                checked={settings.privacy.gitIndexing}
                label="Index Git repositories"
                onChange={(next) => togglePrivacy("gitIndexing", next)}
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Telemetry</span>
                <span className="pref-desc">There is no telemetry pipeline in this build.</span>
              </div>
              <span className="badge badge-quiet">Never sent</span>
            </div>
            <p className="field-help">
              No telemetry. Nothing leaves this PC unless you opt into an agent provider. Clearing
              history is not available in this build.
            </p>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-indexing">
          <div className="settings-label">
            <h2 id="prefs-indexing">Indexing</h2>
          </div>
          <div className="panel">
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Background content indexing</span>
                <span className="pref-desc">Extract text from documents and code in the background.</span>
              </div>
              <Switch
                checked={settings.indexing.contentEnabled}
                label="Background content indexing"
                onChange={(next) =>
                  void apply(
                    { indexing: { ...settings.indexing, contentEnabled: next } },
                    "indexing",
                  )
                }
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Screenshot OCR</span>
                <span className="pref-desc">Extract text from images; the heaviest background mode.</span>
              </div>
              <Switch
                checked={settings.indexing.ocrEnabled}
                label="Screenshot OCR"
                onChange={(next) =>
                  void apply({ indexing: { ...settings.indexing, ocrEnabled: next } }, "indexing")
                }
              />
            </div>
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Pause indexing on battery</span>
                <span className="pref-desc">Respect the power-aware indexing policy.</span>
              </div>
              <Switch
                checked={settings.indexing.respectPowerState}
                label="Pause indexing on battery"
                onChange={(next) =>
                  void apply(
                    { indexing: { ...settings.indexing, respectPowerState: next } },
                    "indexing",
                  )
                }
              />
            </div>
            <div className="section-actions">
              <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
                Index health
              </button>
              <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
                Rebuild index…
              </button>
            </div>
            <p className="field-help">Filename search always works, even with content and OCR off.</p>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-agent">
          <div className="settings-label">
            <h2 id="prefs-agent">Agent &amp; models</h2>
          </div>
          <div className="panel">
            <div className="pref-row">
              <div className="pref-copy">
                <span className="pref-title">Enable agent workflows</span>
                <span className="pref-desc">Off by default; the runtime starts only for a workflow.</span>
              </div>
              <Switch
                checked={settings.agent.enabled}
                label="Enable agent workflows"
                onChange={(next) =>
                  void apply({ agent: { ...settings.agent, enabled: next } }, "agent")
                }
              />
            </div>
            <dl className="metric-list">
              <div className="metric-row">
                <dt>Provider</dt>
                <dd>{settings.agent.provider ?? "No provider configured"}</dd>
              </div>
              <div className="metric-row">
                <dt>Model</dt>
                <dd>{settings.agent.model ?? "No model configured"}</dd>
              </div>
            </dl>
            <p className="field-help">
              No model is downloaded without asking. Installing and configuring the runtime is not
              available in this build.
            </p>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-diagnostics">
          <div className="settings-label">
            <h2 id="prefs-diagnostics">Diagnostics</h2>
          </div>
          <div className="panel">
            <dl className="metric-list">
              <div className="metric-row">
                <dt>Version</dt>
                <dd>{appInfo?.version ?? status?.version ?? "Unknown"}</dd>
              </div>
              <div className="metric-row">
                <dt>Protocol</dt>
                <dd>v{appInfo?.protocol ?? status?.protocol ?? 1}</dd>
              </div>
              <div className="metric-row">
                <dt>Data schema</dt>
                <dd>v{appInfo?.dataSchema ?? 1}</dd>
              </div>
              <div className="metric-row">
                <dt>Platform</dt>
                <dd>
                  {appInfo ? appInfo.platform + " · " + appInfo.arch : "Unknown"}
                </dd>
              </div>
              <div className="metric-row">
                <dt>Index state</dt>
                <dd>{status ? stateLabel(status.state) : "Unknown"}</dd>
              </div>
              <div className="metric-row">
                <dt>Filename entries</dt>
                <dd>{indexCountText(status)}</dd>
              </div>
              <div className="metric-row">
                <dt>Document texts</dt>
                <dd>{formatNumber(status?.contentDocuments ?? 0)}</dd>
              </div>
              <div className="metric-row">
                <dt>OCR texts</dt>
                <dd>{formatNumber(status?.ocrDocuments ?? 0)}</dd>
              </div>
              <div className="metric-row">
                <dt>Last updated</dt>
                <dd>{relativeTime(status?.lastUpdated)}</dd>
              </div>
            </dl>
            {warningSummary ? (
              <StateNotice
                tone="warn"
                title="Index needs attention."
                body={warningSummary.body}
                details={warningSummary.details}
                actionLabel="Open Search & indexes"
                onAction={() => onNavigate("indexes")}
              />
            ) : null}
            <div className="section-actions">
              <button type="button" className="btn" onClick={() => void refreshStatus()}>
                Refresh diagnostics
              </button>
              <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
                Search &amp; indexes
              </button>
            </div>
            <p className="field-help">
              Data is stored under %APPDATA%\protonsearch\v2. First-run setup runs once; change
              those choices in Search &amp; indexes, Privacy &amp; history, and Appearance.
            </p>
          </div>
        </section>

        <section className="settings-section" aria-labelledby="prefs-about">
          <div className="settings-label">
            <h2 id="prefs-about">About</h2>
          </div>
          <div className="panel">
            <p className="about-line">
              ProtonSearch {appInfo?.version ?? status?.version ?? ""} — local-first. No account.
            </p>
            <p className="field-help">
              Search, indexing, OCR and history run on this PC. Optional agent workflows are the
              only surface that can reach a provider, and only after you configure one.
            </p>
          </div>
        </section>
      </div>
    </div>
  );
}