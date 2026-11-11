// Search & indexes route: locations, per-root depth, index status, exclusions,
// sources and storage diagnostics. Mutations are real Settings patches and
// index commands; the UI never invents progress or counts.

import { useCallback, useEffect, useRef, useState, type FormEvent } from "react";
import { useApp } from "../../App";
import type {
  IndexCommand,
  IndexingSettings,
  RootMode,
  RootStatus,
  SourceStatus,
} from "../../bridge/types";
import { formatNumber } from "../../lib/format";
import {
  describeSource,
  indexCountText,
  indexHealth,
  relativeTime,
  rootStatusText,
  sourceStatusText,
  stateLabel,
  summarizeWarnings,
  type SourceInfo,
} from "../describe";
import {
  removeRoot,
  rootDisplayName,
  samePath,
  setRootMode,
  unusedSuggestions,
  validateExcludedPath,
  validateRootPath,
} from "../roots";
import { SECTION_EYEBROWS } from "../sections";
import { sourceIconName } from "../sourceIcons";
import { OcrEngineSelector } from "../OcrEngineSelector";
import {
  Callout,
  Dot,
  EmptyState,
  Icon,
  Panel,
  RouteHeading,
  StateNotice,
  SavedStamp,
  SegmentedControl,
  Switch,
  useSavedStamp,
} from "../ui";

type RebuildCommand = Extract<IndexCommand, "rebuild" | "rebuild_content" | "rebuild_ocr">;

const DEPTH_OPTIONS: { value: RootMode; label: string }[] = [
  { value: "names_only", label: "File names" },
  { value: "content", label: "Inside files" },
  { value: "content_and_ocr", label: "Inside files & images (OCR)" },
];

const REBUILD: Record<RebuildCommand, { label: string; confirm: string }> = {
  rebuild: {
    label: "Rebuild index…",
    confirm:
      "Rebuild the index? Search stays available while filenames are re-scanned. This can take a few minutes and uses more CPU.",
  },
  rebuild_content: {
    label: "Rebuild content…",
    confirm:
      "Rebuild the content index? Extracted document text is read again in the background and search stays available.",
  },
  rebuild_ocr: {
    label: "Rebuild OCR…",
    confirm:
      "Rebuild the OCR index? Screenshots and images are read again in the background; this uses the most CPU.",
  },
};

const HEALTHY_ROOT_STATES = new Set(["ready", "indexing", "starting", "paused", "off"]);

function rootTone(root: RootStatus): "ok" | "busy" | "warn" | "error" {
  if (root.state === "error" || root.errors > 0) return "error";
  if (root.state === "ready") return "ok";
  if (root.state === "paused") return "warn";
  return "busy";
}

export default function IndexesView({ focusSource }: { focusSource?: string | null }) {
  const { settings, status, onboarding, updateSettings, indexCommand, refreshStatus } = useApp();
  const { savedKey, markSaved } = useSavedStamp();
  const [actionError, setActionError] = useState<{
    title: string;
    body: string;
    retry?: () => void;
  } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [rootDraft, setRootDraft] = useState("");
  const [rootError, setRootError] = useState<string | null>(null);
  const [excludedDraft, setExcludedDraft] = useState("");
  const [excludedError, setExcludedError] = useState<string | null>(null);
  const [removeTarget, setRemoveTarget] = useState<string | null>(null);
  const [rebuildTarget, setRebuildTarget] = useState<RebuildCommand | null>(null);
  const sourceRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const confirmTriggerRef = useRef<HTMLElement | null>(null);

  const roots = settings.indexing.roots;
  const excludedPaths = settings.indexing.excludedPaths;
  const health = indexHealth(status);
  const paused = settings.indexing.paused || status?.state === "paused";
  const sources = status?.sources ?? [];
  const warningSummary = summarizeWarnings(status?.warnings ?? []);
  const unavailableRoots = (status?.roots ?? []).filter(
    (root) => root.enabled && !HEALTHY_ROOT_STATES.has(root.state),
  );
  const suggestions = unusedSuggestions(onboarding?.suggestedRoots ?? [], roots);

  useEffect(() => {
    if (!focusSource) return;
    const node = sourceRefs.current[focusSource];
    if (node) {
      node.scrollIntoView({ block: "center" });
      node.focus();
    }
  }, [focusSource]);

  const patchIndexing = useCallback(
    async (patch: Partial<IndexingSettings>, stamp = "locations") => {
      setActionError(null);
      try {
        await updateSettings({ indexing: { ...settings.indexing, ...patch } });
        markSaved(stamp);
      } catch {
        setActionError({
          title: "Couldn't save that change.",
          body: "Your previous setting is still active.",
          retry: () => void patchIndexing(patch, stamp),
        });
      }
    },
    [settings.indexing, updateSettings, markSaved],
  );

  const changeMode = (path: string, mode: RootMode) => {
    void patchIndexing({ roots: setRootMode(roots, path, mode) });
  };

  const addLocation = (event: FormEvent) => {
    event.preventDefault();
    const validation = validateRootPath(roots, rootDraft);
    if (!validation.ok) {
      setRootError(validation.error ?? "Check the folder path.");
      return;
    }
    const path = validation.path;
    void (async () => {
      await patchIndexing({ roots: [...roots, { path, mode: "names_only", enabled: true }] });
      setRootDraft("");
      setRootError(null);
      setAdding(false);
    })();
  };

  const addSuggested = (path: string, mode: RootMode) => {
    void patchIndexing({ roots: [...roots, { path, mode, enabled: true }] });
  };

  const rememberTrigger = () => {
    confirmTriggerRef.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
  };

  const closeRemoveConfirm = () => {
    setRemoveTarget(null);
    confirmTriggerRef.current?.focus();
  };

  const closeRebuildConfirm = () => {
    setRebuildTarget(null);
    confirmTriggerRef.current?.focus();
  };

  const confirmRemove = async () => {
    if (!removeTarget) return;
    setBusy("remove");
    setActionError(null);
    try {
      await updateSettings({
        indexing: { ...settings.indexing, roots: removeRoot(roots, removeTarget) },
      });
      markSaved("locations");
    } catch {
      setActionError({
        title: "Couldn't save that change.",
        body: "Your previous setting is still active.",
        retry: () => void confirmRemove(),
      });
    } finally {
      setBusy(null);
      closeRemoveConfirm();
    }
  };

  const addExclusion = (event: FormEvent) => {
    event.preventDefault();
    const validation = validateExcludedPath(excludedPaths, excludedDraft);
    if (!validation.ok) {
      setExcludedError(validation.error ?? "Check the folder path.");
      return;
    }
    const path = validation.path;
    void (async () => {
      await patchIndexing({ excludedPaths: [...excludedPaths, path] }, "exclusions");
      setExcludedDraft("");
      setExcludedError(null);
    })();
  };

  const removeExclusion = (path: string) => {
    void patchIndexing(
      { excludedPaths: excludedPaths.filter((entry) => !samePath(entry, path)) },
      "exclusions",
    );
  };

  const togglePaused = async () => {
    setBusy("pause");
    setActionError(null);
    setNotice(null);
    try {
      await indexCommand(paused ? "resume" : "pause");
      await updateSettings({ indexing: { ...settings.indexing, paused: !paused } });
      markSaved("status");
    } catch {
      setActionError({
        title: "Couldn't reach the local engine.",
        body: "Indexing state is unchanged.",
        retry: () => void togglePaused(),
      });
    } finally {
      setBusy(null);
    }
  };

  const runRebuild = async () => {
    if (!rebuildTarget) return;
    setBusy(rebuildTarget);
    setActionError(null);
    setNotice(null);
    try {
      await indexCommand(rebuildTarget);
      setNotice("Rebuild requested from the local engine.");
      markSaved("status");
    } catch {
      setActionError({
        title: "Couldn't start the rebuild.",
        body: "Your existing index is unchanged.",
      });
    } finally {
      setBusy(null);
      closeRebuildConfirm();
    }
  };

  const setContentEnabled = (next: boolean) => {
    void patchIndexing({ contentEnabled: next }, "capabilities");
  };

  const setOcrEnabled = (next: boolean) => {
    void patchIndexing({ ocrEnabled: next }, "capabilities");
  };

  const toggleSource = async (info: SourceInfo) => {
    if (!info.privacyKey) return;
    const key = info.privacyKey;
    setActionError(null);
    try {
      await updateSettings({ privacy: { ...settings.privacy, [key]: !settings.privacy[key] } });
      markSaved("sources");
    } catch {
      setActionError({
        title: "Couldn't save that change.",
        body: "Your previous setting is still active.",
        retry: () => void toggleSource(info),
      });
    }
  };

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.indexes}>Search &amp; Indexing</RouteHeading>
      <p className="page-lead">
        Choose what ProtonSearch can find, without blocking the fast filename index.
      </p>
      <Callout tone="note">
        Names-only search keeps working even when content and OCR are off.
      </Callout>

      {actionError ? (
        <StateNotice
          role="alert"
          title={actionError.title}
          body={actionError.body}
          actionLabel={actionError.retry ? "Try again" : undefined}
          onAction={actionError.retry}
        />
      ) : null}

      <Panel
        id="indexes-locations"
        title="Locations"
        hint="Filename search works everywhere. Content and OCR run in bounded background jobs and never block the launcher."
        actions={<SavedStamp show={savedKey === "locations"} />}
      >
        {roots.length === 0 ? (
          <EmptyState
            title="No locations yet."
            body="Add a folder so ProtonSearch can find your files."
            action={
              <button type="button" className="btn" onClick={() => setAdding(true)}>
                Add location
              </button>
            }
          />
        ) : null}
        <div className="config-list">
          {roots.map((root) => {
            const live = status?.roots.find((entry) => samePath(entry.path, root.path));
            return (
              <div className="config-row root-row" key={root.path}>
                <div className="config-main">
                  <div className="config-title">{rootDisplayName(root.path)}</div>
                  <div className="config-path" title={root.path}>
                    {root.path}
                  </div>
                  <div className="config-sub">
                    {live ? (
                      <>
                        <Dot tone={rootTone(live)} />
                        {rootStatusText(live)}
                        {live.errors > 0 ? " · " + formatNumber(live.errors) + " errors" : ""}
                      </>
                    ) : root.enabled === false ? (
                      "Not included in the index"
                    ) : (
                      "Waiting for the engine to report this location"
                    )}
                  </div>
                </div>
                <div className="config-actions">
                  <SegmentedControl
                    label={"Index depth for " + root.path}
                    value={root.mode}
                    options={DEPTH_OPTIONS}
                    onChange={(mode) => changeMode(root.path, mode)}
                  />
                  <button
                    type="button"
                    className="icon-btn"
                    aria-label={"Remove " + rootDisplayName(root.path) + " from the index"}
                    title="Remove location"
                    onClick={() => {
                      rememberTrigger();
                      setRemoveTarget(root.path);
                    }}
                  >
                    <Icon name="close" size={16} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>

        {removeTarget ? (
          <div
            className="confirm-bar"
            role="group"
            aria-label="Confirm removal"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                closeRemoveConfirm();
              }
            }}
          >
            <p className="confirm-text">
              Remove {rootDisplayName(removeTarget)} from the index? Results from this folder stop
              appearing. Your files are not changed.
            </p>
            <div className="confirm-actions">
              <button
                type="button"
                className="btn btn-ghost"
                autoFocus
                onClick={closeRemoveConfirm}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-danger"
                disabled={busy !== null}
                onClick={() => void confirmRemove()}
              >
                Remove
              </button>
            </div>
          </div>
        ) : null}

        {adding ? (
          <form className="inline-add" onSubmit={addLocation} noValidate>
            <label className="sr-only" htmlFor="new-root">
              Folder path
            </label>
            <input
              id="new-root"
              className="input input-sm"
              placeholder="C:\Users\You\Projects"
              value={rootDraft}
              autoFocus
              spellCheck={false}
              aria-invalid={rootError ? true : undefined}
              aria-describedby={rootError ? "new-root-error" : "new-root-help"}
              onChange={(event) => {
                setRootDraft(event.target.value);
                setRootError(null);
              }}
            />
            <button type="submit" className="btn">
              Add location
            </button>
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => {
                setAdding(false);
                setRootDraft("");
                setRootError(null);
              }}
            >
              Cancel
            </button>
          </form>
        ) : (
          <div className="section-actions">
            <button type="button" className="btn" onClick={() => setAdding(true)}>
              Add location…
            </button>
            {suggestions.map((suggestion) => (
              <button
                key={suggestion.path}
                type="button"
                className="linkbtn"
                onClick={() => addSuggested(suggestion.path, suggestion.defaultMode)}
              >
                Add {suggestion.label}
              </button>
            ))}
          </div>
        )}
        {rootError ? (
          <p className="field-error" id="new-root-error" role="alert">
            {rootError}
          </p>
        ) : (
          <p className="field-help" id="new-root-help">
            The native folder picker is not available in this build — paste an absolute path.
          </p>
        )}
      </Panel>

      <Panel
        id="indexes-capabilities"
        title="Search depth"
        hint="The launcher never waits for extraction. These jobs run in bounded background work and can be paused."
        actions={<SavedStamp show={savedKey === "capabilities"} />}
      >
        <div className="config-list">
          <div className="config-row">
            <div className="config-main">
              <div className="config-title">Document and code content</div>
              <p className="config-desc">
                Extracts text from PDF, Office documents, Markdown and source code in the
                background. Adds CPU and disk reads while it runs.
              </p>
            </div>
            <Switch
              checked={settings.indexing.contentEnabled}
              label="Document and code content indexing"
              onChange={setContentEnabled}
            />
          </div>
          <div className="config-row">
            <div className="config-main">
              <div className="config-title">Screenshot OCR</div>
              <p className="config-desc">
                Reads text from images and screenshots on this PC, then searches the stored text.
                The heaviest mode; OCR never runs during a search.
              </p>
            </div>
            <Switch
              checked={settings.indexing.ocrEnabled}
              label="Screenshot OCR indexing"
              onChange={setOcrEnabled}
            />
          </div>
          {settings.indexing.ocrEnabled && (
            <div style={{ marginTop: "1rem", paddingTop: "0.75rem", borderTop: "1px solid var(--ps-border-subtle, rgba(255,255,255,0.08))" }}>
              <div style={{ marginBottom: "0.5rem", fontSize: "0.85rem", fontWeight: 500, color: "var(--ps-text-dim)" }}>OCR Engine &amp; Models</div>
              <OcrEngineSelector
                value={settings.ocrEngine}
                onChange={(ocrEngine) => void updateSettings({ ocrEngine })}
              />
            </div>
          )}
        </div>
        {!settings.indexing.contentEnabled ? (
          <Callout tone="warn" role="status">
            Content extraction is off, so locations set to content are indexed by filename only.
          </Callout>
        ) : null}
        {!settings.indexing.ocrEnabled ? (
          <Callout tone="warn" role="status">
            OCR is off, so images and screenshots are indexed by filename only.
          </Callout>
        ) : null}
      </Panel>

      <Panel
        id="indexes-status"
        title="Index status"
        hint="Status comes directly from the local engine. No estimated percentages are shown."
        actions={<SavedStamp show={savedKey === "status"} />}
      >
        {status ? (
          <>
            <div className="status-line">
              <Dot tone={health.tone} />
              <span className="status-word">{health.label}</span>
              <span className="status-detail">
                Last scan completed {relativeTime(status.lastUpdated)}
              </span>
              <button type="button" className="linkbtn" onClick={() => void refreshStatus()}>
                Refresh
              </button>
            </div>
            <p className="muted">
              {status.state === "error"
                ? "New files aren't being added. Existing search still works."
                : status.state === "paused"
                  ? "Existing results stay searchable, but new files are not being picked up."
                  : status.state === "starting"
                    ? "The local engine is warming up."
                    : "Everything you enabled is searchable on this device."}
            </p>
            <dl className="metric-list">
              <div className="metric-row">
                <dt>Files</dt>
                <dd>{indexCountText(status)}</dd>
              </div>
              <div className="metric-row">
                <dt>Documents</dt>
                <dd>{formatNumber(status.contentDocuments)}</dd>
              </div>
              <div className="metric-row">
                <dt>OCR documents</dt>
                <dd>{formatNumber(status.ocrDocuments)}</dd>
              </div>
              <div className="metric-row">
                <dt>Sources</dt>
                <dd>{formatNumber(status.sources.length)}</dd>
              </div>
            </dl>
            {warningSummary ? (
              <details className="skipped-items-disclosure" style={{ marginTop: "1rem", padding: "0.75rem 1rem", background: "var(--ps-surface-subtle, rgba(255,255,255,0.03))", borderRadius: "8px", border: "1px solid var(--ps-border-subtle, rgba(255,255,255,0.06))" }}>
                <summary style={{ cursor: "pointer", fontWeight: 500, fontSize: "0.85rem", color: "var(--ps-text-muted)" }}>
                  Review skipped items ({status.warnings.length})
                </summary>
                <div style={{ marginTop: "0.5rem", fontSize: "0.825rem", color: "var(--ps-text-dim)", lineHeight: 1.5 }}>
                  <p>{warningSummary.body}</p>
                  {warningSummary.details && <p style={{ marginTop: "0.25rem", opacity: 0.85 }}>{warningSummary.details}</p>}
                </div>
              </details>
            ) : null}
          </>
        ) : (
          <StateNotice
            title="Couldn't read the index status."
            body="The local search engine didn't respond. Your settings are unaffected."
            actionLabel="Try again"
            onAction={() => void refreshStatus()}
          />
        )}

        <div className="btn-row">
          <button
            type="button"
            className="btn"
            disabled={busy !== null}
            onClick={() => void togglePaused()}
          >
            {paused ? "Resume indexing" : "Pause indexing"}
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy !== null}
            onClick={() => {
              rememberTrigger();
              setRebuildTarget("rebuild");
            }}
          >
            {REBUILD.rebuild.label}
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy !== null || !settings.indexing.contentEnabled}
            onClick={() => {
              rememberTrigger();
              setRebuildTarget("rebuild_content");
            }}
          >
            {REBUILD.rebuild_content.label}
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy !== null || !settings.indexing.ocrEnabled}
            onClick={() => {
              rememberTrigger();
              setRebuildTarget("rebuild_ocr");
            }}
          >
            {REBUILD.rebuild_ocr.label}
          </button>
        </div>

        {rebuildTarget ? (
          <div
            className="confirm-bar"
            role="group"
            aria-label="Confirm rebuild"
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                closeRebuildConfirm();
              }
            }}
          >
            <p className="confirm-text">{REBUILD[rebuildTarget].confirm}</p>
            <div className="confirm-actions">
              <button
                type="button"
                className="btn btn-ghost"
                autoFocus
                onClick={closeRebuildConfirm}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn"
                disabled={busy !== null}
                onClick={() => void runRebuild()}
              >
                Rebuild
              </button>
            </div>
          </div>
        ) : null}
        {notice ? (
          <p className="inline-note" role="status">
            {notice}
          </p>
        ) : null}
      </Panel>

      <Panel
        id="indexes-exclusions"
        title="Exclusions"
        hint="These folders are skipped for content and OCR."
        actions={<SavedStamp show={savedKey === "exclusions"} />}
      >
        {excludedPaths.length === 0 ? (
          <EmptyState
            title="No exclusions."
            body="Everything in your locations is indexed."
          />
        ) : (
          <ul className="path-list">
            {excludedPaths.map((path) => (
              <li className="path-row" key={path}>
                <span className="config-path" title={path}>
                  {path}
                </span>
                <button
                  type="button"
                  className="icon-btn"
                  aria-label={"Stop excluding " + path}
                  title="Remove exclusion"
                  onClick={() => removeExclusion(path)}
                >
                  <Icon name="close" size={16} />
                </button>
              </li>
            ))}
          </ul>
        )}
        <form className="inline-add" onSubmit={addExclusion} noValidate>
          <label className="sr-only" htmlFor="new-exclusion">
            Folder path to exclude
          </label>
          <input
            id="new-exclusion"
            className="input input-sm"
            placeholder="C:\Users\You\Archive"
            value={excludedDraft}
            spellCheck={false}
            aria-invalid={excludedError ? true : undefined}
            aria-describedby={excludedError ? "exclusion-error" : "exclusion-help"}
            onChange={(event) => {
              setExcludedDraft(event.target.value);
              setExcludedError(null);
            }}
          />
          <button type="submit" className="btn">
            Add exclusion
          </button>
        </form>
        {excludedError ? (
          <p className="field-error" id="exclusion-error" role="alert">
            {excludedError}
          </p>
        ) : (
          <p className="field-help" id="exclusion-help">
            Excluded folders are still searchable by name unless they are also removed as locations.
          </p>
        )}
      </Panel>

      <Panel
        id="indexes-sources"
        title="Sources"
        hint="Every source is read locally from this PC. Privacy switches apply immediately."
        actions={<SavedStamp show={savedKey === "sources"} />}
      >
        {sources.length === 0 ? (
          <EmptyState
            title="No sources reporting."
            body="The local engine will list them when available."
          />
        ) : (
          <div className="config-list">
            {sources.map((source: SourceStatus) => {
              const info = describeSource(source, settings);
              return (
                <div
                  key={source.id}
                  ref={(node) => {
                    sourceRefs.current[source.id] = node;
                  }}
                  tabIndex={-1}
                  className={"config-row source-config-row" + (focusSource === source.id ? " is-focused" : "")}
                >
                  <div className="config-main">
                    <div className="config-title">
                      <span className="source-icon" aria-hidden="true">
                        <Icon name={sourceIconName(source.id)} size={16} />
                      </span>
                      {info.title}
                      <Dot tone={info.tone} />
                    </div>
                    <div className="config-sub">{sourceStatusText(source, info.enabled)}</div>
                    <p className="config-desc">{info.description}</p>
                    {source.state === "error" ? (
                      <p className="row-state">
                        <Icon name="alert" size={16} />
                        Couldn&apos;t read {info.title}. ProtonSearch will try again
                        automatically.
                      </p>
                    ) : null}
                  </div>
                  {info.privacyKey ? (
                    <Switch
                      checked={info.enabled}
                      label={info.title + " enabled"}
                      onChange={() => void toggleSource(info)}
                    />
                  ) : (
                    <span className="badge">Always on</span>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </Panel>

      {unavailableRoots.length > 0 ? (
        <Panel
          id="indexes-unavailable"
          title="Cloud & removable"
          hint="Placeholders are indexed by name. Content is read only when available."
        >
          <ul className="path-list">
            {unavailableRoots.map((root) => (
              <li className="path-row" key={root.path}>
                <span className="config-path" title={root.path}>
                  {root.path}
                </span>
                <span className="row-state">
                  <Icon name="alert" size={16} />
                  {root.state === "unavailable"
                    ? "Network location unavailable."
                    : "Couldn't read this location."}
                </span>
              </li>
            ))}
          </ul>
          <p className="field-help">
            Local files are still searchable. Reconnect the drive to index it again.
          </p>
        </Panel>
      ) : null}

      <Panel
        id="indexes-storage"
        title="Storage & diagnostics"
        hint="Counts and states are reported by the local engine; no estimates are added."
      >
        <dl className="metric-list">
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
            <dt>Index state</dt>
            <dd>{status ? stateLabel(status.state) : "Unknown"}</dd>
          </div>
          <div className="metric-row">
            <dt>Last updated</dt>
            <dd>{relativeTime(status?.lastUpdated)}</dd>
          </div>
        </dl>
        <p className="field-help">
          Index data is stored under %APPDATA%\protonsearch\v2 and never leaves this PC.
        </p>
      </Panel>
    </div>
  );
}