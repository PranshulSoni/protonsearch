// First-run onboarding (docs/design/SCREENS.md section 4): seven steps, real
// configuration, truthful legacy notice, honest skip, UI-local resumability.
// Everything persisted here is exactly the frozen OnboardingInput shape.

import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { useApp } from "../App";
import type { RootConfig, RootMode } from "../bridge/types";
import { formatNumber } from "../lib/format";
import { Logo } from "../components/Logo";
import { parseHotkey } from "../app/hotkey";
import { HotkeyCapture } from "../app/HotkeyInput";
import { api } from "../bridge/api";
import { rootDisplayName, samePath, validateRootPath } from "../app/roots";
import { Callout, Icon, SegmentedControl } from "../app/ui";
import {
  DEPTH_LABEL,
  DEPTH_SUMMARY_LABEL,
  DEPTH_TRADEOFF,
  ONBOARDING_DRAFT_KEY,
  ONBOARDING_STEPS,
  buildOnboardingInput,
  buildSkippedOnboardingInput,
  createOnboardingDraft,
  legacyDetails,
  parsePersistedOnboarding,
  primaryLabel,
  selectedRoots,
  serializeOnboardingDraft,
  stepEyebrow,
  summarizeDraft,
  type OnboardingDraft,
  type OnboardingStepId,
} from "./logic";
import "./onboarding.css";

const STEP_COPY: Record<OnboardingStepId, { title: string; lead: string }> = {
  welcome: {
    title: "Find anything, without the friction.",
    lead: "A fast, private Windows search experience built around your work. We only need a few preferences to get started.",
  },
  name: {
    title: "What should we call you?",
    lead: "We use it only for the greeting in your local app. You can change or remove it later.",
  },
  locations: {
    title: "Choose where to search.",
    lead: "Pick folders to index. Filename search is instant; content and OCR enrich in the background.",
  },
  depth: {
    title: "Decide how deep to index.",
    lead: "Basic filename search never needs AI or an account. Choose optional content search and OCR; model downloads remain opt-in.",
  },
  launcher: {
    title: "Set your shortcut.",
    lead: "Press it anywhere to summon ProtonSearch.",
  },
  agent: {
    title: "Add an agent, only if you want one.",
    lead: "ProtonSearch works fully without AI. Agent workflows are optional, start on demand, and ask before they act.",
  },
  ready: {
    title: "Ready when you are.",
    lead: "Filename search is available now. Content and OCR will fill in the background; you can change everything later.",
  },
};

const DEPTH_OPTIONS: { value: RootMode; label: string }[] = [
  { value: "names_only", label: DEPTH_LABEL.names_only },
  { value: "content", label: DEPTH_LABEL.content },
  { value: "content_and_ocr", label: DEPTH_LABEL.content_and_ocr },
];

const SUGGESTION_HINT: Record<string, string> = {
  Documents: "Notes and documents",
  Desktop: "Files close at hand",
  Downloads: "Recent downloads",
  Pictures: "Screenshots and images",
};

function loadPersisted(complete: boolean) {
  if (complete || typeof window === "undefined") return null;
  try {
    return parsePersistedOnboarding(window.localStorage.getItem(ONBOARDING_DRAFT_KEY));
  } catch {
    return null;
  }
}

export default function OnboardingView() {
  const { settings, onboarding, completeOnboarding, updateSettings } = useApp();
  const persisted = useMemo(() => loadPersisted(Boolean(onboarding?.complete)), [onboarding?.complete]);
  const [draft, setDraft] = useState<OnboardingDraft>(
    () => persisted?.draft ?? createOnboardingDraft(onboarding, settings),
  );
  const [stepIndex, setStepIndex] = useState(() => persisted?.stepIndex ?? 0);
  const [finishing, setFinishing] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [addingRoot, setAddingRoot] = useState(false);
  const [rootDraft, setRootDraft] = useState("");
  const [rootError, setRootError] = useState<string | null>(null);
  const headingRef = useRef<HTMLHeadingElement>(null);

  const step = ONBOARDING_STEPS[stepIndex];
  const copy = STEP_COPY[step.id];
  const summary = useMemo(() => summarizeDraft(draft), [draft]);
  const chosen = selectedRoots(draft.roots);
  const legacy = onboarding?.legacy;
  const legacyInfo = legacyDetails(legacy);

  useEffect(() => {
    headingRef.current?.focus();
  }, [stepIndex]);

  useEffect(() => {
    if (finishing) return;
    try {
      window.localStorage.setItem(
        ONBOARDING_DRAFT_KEY,
        serializeOnboardingDraft(stepIndex, draft),
      );
    } catch {
      /* Storage can be unavailable; setup still works without resumability. */
    }
  }, [stepIndex, draft, finishing]);

  const patchDraft = (patch: Partial<OnboardingDraft>) => {
    setDraft((current) => ({ ...current, ...patch }));
    setFormError(null);
  };

  const toggleRoot = (path: string) => {
    setDraft((current) => ({
      ...current,
      roots: current.roots.map((root) =>
        samePath(root.path, path) ? { ...root, enabled: !(root.enabled !== false) } : root,
      ),
    }));
  };

  const setDepth = (path: string, mode: RootMode) => {
    setDraft((current) => ({
      ...current,
      roots: current.roots.map((root) => (samePath(root.path, path) ? { ...root, mode } : root)),
    }));
  };

  const addRoot = (event: FormEvent) => {
    event.preventDefault();
    const validation = validateRootPath(draft.roots, rootDraft);
    if (!validation.ok) {
      setRootError(validation.error ?? "Check the folder path.");
      return;
    }
    const path = validation.path;
    setDraft((current) => ({
      ...current,
      roots: [...current.roots, { path, mode: "names_only", enabled: true }],
    }));
    setRootDraft("");
    setRootError(null);
    setAddingRoot(false);
  };

  const finish = async (skipped: boolean) => {
    if (finishing) return;
    setFinishing(true);
    setFormError(null);
    try {
      await completeOnboarding(
        skipped ? buildSkippedOnboardingInput(draft) : buildOnboardingInput(draft),
      );
      try {
        window.localStorage.removeItem(ONBOARDING_DRAFT_KEY);
      } catch {
        /* ignore storage failures */
      }
      if (!skipped && draft.launchAtLogin !== settings.launchAtLogin) {
        void updateSettings({ launchAtLogin: draft.launchAtLogin }).catch(() => undefined);
      }
    } catch (err) {
      setFormError(err instanceof Error ? err.message : String(err));
      setFinishing(false);
    }
  };

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (finishing) return;
    setFormError(null);
    if (step.id === "launcher" && !parseHotkey(draft.hotkey).ok) {
      setFormError("Choose a valid shortcut before continuing.");
      return;
    }
    if (step.id === "ready") {
      void finish(false);
      return;
    }
    setStepIndex((index) => Math.min(index + 1, ONBOARDING_STEPS.length - 1));
  };

  const goBack = () => {
    setFormError(null);
    setStepIndex((index) => Math.max(0, index - 1));
  };

  const suggestionFor = (path: string) =>
    onboarding?.suggestedRoots.find((suggestion) => samePath(suggestion.path, path));

  const rootName = (root: RootConfig) =>
    suggestionFor(root.path)?.label ?? rootDisplayName(root.path);

  const rootHint = (root: RootConfig) => {
    const suggestion = suggestionFor(root.path);
    if (suggestion && SUGGESTION_HINT[suggestion.label]) {
      return SUGGESTION_HINT[suggestion.label];
    }
    return DEPTH_LABEL[root.mode] + " · " + DEPTH_TRADEOFF[root.mode];
  };

  const renderStep = () => {
    switch (step.id) {
      case "welcome":
        return (
          <>
            <ul className="value-list">
              <li className="value-row">
                <span className="value-title">Local-first</span>
                <span className="value-desc">Your index, history, and settings stay on this PC.</span>
              </li>
              <li className="value-row">
                <span className="value-title">No account</span>
                <span className="value-desc">Nothing to sign in to, and no telemetry.</span>
              </li>
              <li className="value-row">
                <span className="value-title">You choose</span>
                <span className="value-desc">Content, OCR, and agent workflows are opt-in.</span>
              </li>
            </ul>
            {legacy?.detected ? (
              <section className="legacy-note" aria-labelledby="legacy-title">
                <h2 className="legacy-title" id="legacy-title">
                  Existing ProtonSearch data found
                </h2>
                <dl className="legacy-counts">
                  {legacyInfo.map((detail) => (
                    <div key={detail.label}>
                      <dt>{detail.label}</dt>
                      <dd>{formatNumber(detail.value)}</dd>
                    </div>
                  ))}
                </dl>
                {legacy.appDataPath ? <p className="legacy-path">{legacy.appDataPath}</p> : null}
                <p className="legacy-copy">
                  The previous data folder is opened read-only and is never modified. Nothing has
                  been imported yet; whether the engine can reuse it is reported after setup.
                </p>
              </section>
            ) : null}
          </>
        );

      case "name":
        return (
          <>
            <label className="field-label" htmlFor="onboarding-name">
              First name (optional)
            </label>
            <input
              id="onboarding-name"
              className="field"
              type="text"
              value={draft.userName}
              maxLength={40}
              autoComplete="off"
              placeholder="Your first name"
              aria-describedby="onboarding-name-help"
              onChange={(event) => patchDraft({ userName: event.target.value })}
            />
            <p className="field-help" id="onboarding-name-help">
              Leave blank if you&apos;d rather not say. It is only used in the greeting on Home.
            </p>
          </>
        );

      case "locations":
        return (
          <>
            <div className="choices">
              {draft.roots.map((root, index) => {
                const hintId = "location-hint-" + index;
                return (
                  <label className="choice" key={root.path}>
                    <input
                      type="checkbox"
                      checked={root.enabled !== false}
                      onChange={() => toggleRoot(root.path)}
                      aria-describedby={hintId}
                    />
                    <span className="choice-text">
                      <span className="choice-name">{rootName(root)}</span>
                      <span className="choice-detail" title={root.path}>
                        {root.path}
                      </span>
                      <span className="choice-hint" id={hintId}>
                        {rootHint(root)}
                      </span>
                    </span>
                  </label>
                );
              })}
            </div>
            {draft.roots.length === 0 ? (
              <Callout tone="note">
                No suggested folders were reported by the engine. Add a folder below.
              </Callout>
            ) : null}

            {addingRoot ? (
              <form className="add-root" onSubmit={addRoot} noValidate>
                <label className="field-label" htmlFor="onboarding-new-root">
                  Folder path
                </label>
                <div className="add-root-row">
                  <input
                    id="onboarding-new-root"
                    className="field"
                    value={rootDraft}
                    autoFocus
                    spellCheck={false}
                    placeholder="C:\Users\You\Projects"
                    aria-invalid={rootError ? true : undefined}
                    aria-describedby={rootError ? "onboarding-root-error" : "onboarding-root-help"}
                    onChange={(event) => {
                      setRootDraft(event.target.value);
                      setRootError(null);
                    }}
                  />
                  <button type="submit" className="btn">
                    Add folder
                  </button>
                  <button
                    type="button"
                    className="btn btn-ghost"
                    onClick={() => {
                      setAddingRoot(false);
                      setRootDraft("");
                      setRootError(null);
                    }}
                  >
                    Cancel
                  </button>
                </div>
                {rootError ? (
                  <p className="field-error" id="onboarding-root-error" role="alert">
                    {rootError}
                  </p>
                ) : (
                  <p className="field-help" id="onboarding-root-help">
                    The native folder picker is not available in this build — paste an absolute
                    path.
                  </p>
                )}
              </form>
            ) : (
              <button type="button" className="linkbtn" onClick={() => setAddingRoot(true)}>
                Add another folder…
              </button>
            )}
            <p className="field-help">You can change these later in Search &amp; indexes.</p>
          </>
        );

      case "depth":
        return chosen.length === 0 ? (
          <>
            <p className="empty-title">No locations selected.</p>
            <p className="field-help">
              Filename search has nothing to scan until you choose a folder.
            </p>
            <button type="button" className="btn" onClick={() => setStepIndex(2)}>
              Back to locations
            </button>
          </>
        ) : (
          <>
            <div className="depth-list">
              {chosen.map((root) => (
                <div className="depth-row" key={root.path}>
                  <div className="depth-main">
                    <span className="depth-name">{rootName(root)}</span>
                    <span className="depth-path" title={root.path}>
                      {root.path}
                    </span>
                  </div>
                  <SegmentedControl
                    label={"Index depth for " + root.path}
                    value={root.mode}
                    options={DEPTH_OPTIONS}
                    onChange={(mode) => setDepth(root.path, mode)}
                  />
                </div>
              ))}
            </div>
            <p className="depth-tradeoff">
              {DEPTH_TRADEOFF[summary.deepestMode ?? "names_only"]}
            </p>
            <p className="field-help">
              Filename search is always on. Extracted text only comes from the depth you choose.
              Nothing is uploaded.
            </p>
          </>
        );

      case "launcher":
        return (
          <>
            <HotkeyCapture
              value={draft.hotkey}
              help="Use at least one of Ctrl, Alt, Shift, or Win. The shortcut applies immediately; Windows rejects a combination another app already owns. You can change it later in Preferences."
              onCommit={async (accelerator, display) => {
                await api.setHotkey(accelerator);
                setDraft((current) => ({ ...current, hotkey: display }));
              }}
            />
            <label className="choice choice-single">
              <input
                type="checkbox"
                checked={draft.launchAtLogin}
                onChange={(event) => patchDraft({ launchAtLogin: event.target.checked })}
              />
              <span className="choice-text">
                <span className="choice-name">Start ProtonSearch with Windows</span>
                <span className="choice-hint">
                  Starts in the tray so the shortcut is available after sign-in.
                </span>
              </span>
            </label>
          </>
        );

      case "agent":
        return (
          <>
            <label className="choice choice-single">
              <input
                type="checkbox"
                checked={draft.agentOptIn}
                onChange={(event) => patchDraft({ agentOptIn: event.target.checked })}
              />
              <span className="choice-text">
                <span className="choice-name">Set up agent workflows later</span>
                <span className="choice-hint">
                  The runtime stays off until you run one. ProtonSearch works fully without it.
                </span>
              </span>
            </label>
            <Callout tone="note">
              No models are downloaded automatically. You choose a provider and a model in
              Preferences.
            </Callout>
            <details className="disclosure">
              <summary>What leaves this PC?</summary>
              <p>
                Nothing leaves this PC unless you configure a provider and run an agent workflow.
                Then the prompt and the data you approve are sent to that provider.
              </p>
            </details>
          </>
        );

      case "ready":
        return (
          <>
            <dl className="summary-list">
              <div className="summary-row">
                <dt>Included locations</dt>
                <dd>{summary.locationCount}</dd>
              </div>
              <div className="summary-row">
                <dt>Filename search</dt>
                <dd>Always available</dd>
              </div>
              <div className="summary-row">
                <dt>Document content</dt>
                <dd>{summary.contentEnabled ? "Enabled" : "Off"}</dd>
              </div>
              <div className="summary-row">
                <dt>OCR search</dt>
                <dd>{summary.ocrEnabled ? "Enabled" : "Off"}</dd>
              </div>
              <div className="summary-row">
                <dt>Agent workflows</dt>
                <dd>{summary.agentOptIn ? "Set up later" : "Off"}</dd>
              </div>
              <div className="summary-row">
                <dt>Start with Windows</dt>
                <dd>{draft.launchAtLogin ? "On" : "Off"}</dd>
              </div>
              <div className="summary-row">
                <dt>Shortcut</dt>
                <dd>{summary.hotkeyDisplay}</dd>
              </div>
            </dl>
            {summary.roots.length > 0 ? (
              <ul className="summary-roots">
                {summary.roots.map((root) => (
                  <li key={root.path}>
                    <span className="summary-path" title={root.path}>
                      {root.path}
                    </span>
                    <span className="summary-mode">{DEPTH_SUMMARY_LABEL[root.mode]}</span>
                  </li>
                ))}
              </ul>
            ) : null}
            {summary.warnings.map((warning) => (
              <Callout tone="warn" role="status" key={warning}>
                {warning}
              </Callout>
            ))}
            {legacy?.detected ? (
              <p className="field-help">
                Existing ProtonSearch data was found and is opened read-only; nothing has been
                imported. Migration status is reported by the engine.
              </p>
            ) : null}
            <p className="field-help">
              Indexing starts after you continue. You can keep using search while it runs.
            </p>
          </>
        );

      default:
        return null;
    }
  };

  return (
    <div className="onboarding-stage">
      <div className="onboarding">
        <aside className="setup-rail">
          <div className="setup-brand">
            <span aria-hidden="true">
              <Logo size={28} />
            </span>
            <span>ProtonSearch</span>
          </div>
          <nav aria-label="Setup steps">
            <ol className="steps">
              {ONBOARDING_STEPS.map((item, index) => (
                <li
                  key={item.id}
                  className={
                    "step" +
                    (index === stepIndex ? " is-current" : "") +
                    (index < stepIndex ? " is-done" : "")
                  }
                  aria-current={index === stepIndex ? "step" : undefined}
                >
                  <span className="step-num" aria-hidden="true">
                    {index < stepIndex ? <Icon name="check" size={12} /> : index + 1}
                  </span>
                  <span className="step-label">{item.label}</span>
                </li>
              ))}
            </ol>
          </nav>
          <div className="setup-foot">
            <p>
              No account required.
              <br />
              Your files remain on your PC.
            </p>
            <p className="setup-save-note">
              Setup saved. You can close this window and continue later.
            </p>
          </div>
        </aside>

        <form className="setup-main" onSubmit={handleSubmit} noValidate>
          <p className="eyebrow" id="onboarding-step-eyebrow">
            {stepEyebrow(stepIndex)}
          </p>
          <h1 id="onboarding-step-title" ref={headingRef} tabIndex={-1} aria-describedby="onboarding-step-eyebrow">
            {copy.title}
          </h1>
          <p className="setup-lead">{copy.lead}</p>
          <div className="step-content" key={step.id}>
            {renderStep()}
          </div>
          {formError ? (
            <Callout tone="danger" role="alert">
              {formError}
            </Callout>
          ) : null}
          <div className="setup-bottom">
            <div className="setup-bottom-left">
              {stepIndex > 0 ? (
                <button type="button" className="btn btn-ghost" disabled={finishing} onClick={goBack}>
                  Back
                </button>
              ) : null}
              <button
                type="button"
                className="linkbtn"
                disabled={finishing}
                onClick={() => void finish(true)}
              >
                Skip setup
              </button>
            </div>
            <button type="submit" className="btn btn-primary" disabled={finishing}>
              {finishing ? "Saving…" : primaryLabel(step.id)}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}