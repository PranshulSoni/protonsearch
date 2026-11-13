// First-run onboarding: welcome, name, OCR, appearance, shortcut, done. Every
// choice goes through the settings API: name and look when their step is left,
// OCR and the shortcut as they change. Finishing marks onboarding complete.

import { useEffect, useRef, useState, type CSSProperties, type FormEvent } from "react";
import { useApp } from "../App";
import { api } from "../bridge/api";
import type { SettingsPatch, ThemeMode } from "../bridge/types";
import { Logo } from "../components/Logo";
import { hotkeyCapTokens } from "../app/hotkey";
import { HotkeyCapture } from "../app/HotkeyInput";
import { OcrEngineSelector } from "../app/OcrEngineSelector";
import { SegmentedControl, StateNotice, Switch } from "../app/ui";
import {
  beginOnboardingCrossfade,
  endOnboardingCrossfade,
  nextPaint,
  prefersReducedMotion,
  supportsViewTransition,
} from "../app/transition";
import { lookFrom, useResolvedTheme, writeIslandLook } from "../lib/islandLook";
import {
  ONBOARDING_STEPS,
  buildOnboardingInput,
  completionRoots,
  primaryLabel,
  type OnboardingStepId,
} from "./logic";
import "./onboarding.css";

const COPY: Record<OnboardingStepId, { title: string; lead?: string }> = {
  welcome: {
    title: "Welcome to ProtonSearch",
    lead: "Find files, apps and history on this PC from one shortcut.",
  },
  name: {
    title: "What should we call you?",
    lead: "Home greets you by this name. Leave it blank to use your Windows name.",
  },
  ocr: {
    title: "Read text in images?",
    lead: "OCR lets search find words inside screenshots and scanned documents.",
  },
  appearance: {
    title: "Choose a look",
    lead: "The preview shows the Dynamic Island with your choices.",
  },
  shortcut: {
    title: "Set your shortcut",
    lead: "Press it anywhere to open the launcher.",
  },
  done: { title: "You're all set" },
};

const OCR_OPTIONS: { value: "on" | "off"; label: string }[] = [
  { value: "on", label: "Use OCR" },
  { value: "off", label: "No OCR" },
];

const THEME_OPTIONS: { value: ThemeMode; label: string }[] = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export default function OnboardingView() {
  const { settings, onboarding, completeOnboarding, updateSettings } = useApp();
  const suggestions = onboarding?.suggestedRoots;
  const [stepIndex, setStepIndex] = useState(0);
  const [back, setBack] = useState(false);
  const [name, setName] = useState(settings.userName ?? "");
  const [look, setLook] = useState(() => lookFrom(settings.launcher.island, settings.theme));
  const [finishing, setFinishing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [finishFailed, setFinishFailed] = useState(false);
  const lastSkipRef = useRef(false);
  const savingRef = useRef(false);
  const stepRef = useRef<HTMLDivElement>(null);
  const primaryRef = useRef<HTMLButtonElement>(null);
  const previewTheme = useResolvedTheme(look.theme);

  const step = ONBOARDING_STEPS[stepIndex];
  const copy = COPY[step.id];

  // Keyboard flow: each step starts on its first control, else on the primary button.
  useEffect(() => {
    const first = stepRef.current?.querySelector<HTMLElement>(
      "input:not(:disabled), button:not(:disabled):not([tabindex='-1'])",
    );
    (first ?? primaryRef.current)?.focus();
  }, [stepIndex]);

  const fail = () => setError("Couldn't save that. Try again.");

  const save = (patch: SettingsPatch) => updateSettings(patch).then(() => setError(null), fail);

  // api.setHotkey re-registers the shortcut and rejects it when another app owns it.
  const applyHotkey = async (accelerator: string) => {
    await api.setHotkey(accelerator);
    await updateSettings({});
  };

  const move = (delta: 1 | -1) => {
    setBack(delta < 0);
    setError(null);
    setStepIndex((index) => index + delta);
  };

  const saveStep = async () => {
    if (step.id === "name") {
      await updateSettings({ userName: name.trim() });
    } else if (step.id === "appearance") {
      const saved = await updateSettings({
        theme: look.theme,
        launcher: {
          ...settings.launcher,
          island: {
            ...settings.launcher.island,
            liquidGlass: look.liquidGlass,
            liquidGlassLevel: look.liquidGlassLevel,
          },
        },
      });
      // The island window paints its first frame from this cache (lib/islandLook).
      writeIslandLook(lookFrom(saved.launcher.island, saved.theme));
    }
  };

  const finish = async (skipped: boolean) => {
    if (finishing) return;
    lastSkipRef.current = skipped;
    setFinishing(true);
    setError(null);
    setFinishFailed(false);

    const complete = async () => {
      await completeOnboarding(buildOnboardingInput(settings, suggestions, skipped));
      if (settings.launcher.primaryMode !== "dynamic_island") {
        void updateSettings({
          launcher: { ...settings.launcher, primaryMode: "dynamic_island" },
        }).catch(() => undefined);
      }
      if (!skipped) void api.showIsland().catch(() => undefined);
    };

    // Onboarding → DesktopApp: a 160 ms root crossfade via the View Transition
    // API when available; falls back to the app shell's own 160 ms fade.
    const runCompletion = async () => {
      if (!supportsViewTransition() || prefersReducedMotion()) {
        await complete();
        return;
      }
      let completionError: unknown = null;
      beginOnboardingCrossfade();
      try {
        let transition: ViewTransition | null = null;
        try {
          transition = document.startViewTransition(async () => {
            try {
              await complete();
              await nextPaint();
            } catch (err) {
              completionError = err;
              throw err;
            }
          });
        } catch {
          // A transition could not start (e.g. one is already active): finish
          // without it rather than blocking setup.
          endOnboardingCrossfade();
          await complete();
          return;
        }
        await transition.finished.catch(() => undefined);
        if (completionError) throw completionError;
      } finally {
        endOnboardingCrossfade();
      }
    };

    try {
      await runCompletion();
    } catch {
      setFinishFailed(true);
      setFinishing(false);
    }
  };

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (finishing || savingRef.current) return;
    if (step.id === "done") {
      void finish(false);
      return;
    }
    savingRef.current = true;
    try {
      await saveStep();
      move(1);
    } catch {
      fail();
    } finally {
      savingRef.current = false;
    }
  };

  const caps = hotkeyCapTokens(settings.hotkey);

  const renderStep = () => {
    switch (step.id) {
      case "name":
        return (
          <input
            className="onboard-input"
            type="text"
            value={name}
            maxLength={40}
            autoComplete="off"
            spellCheck={false}
            placeholder="Your name"
            aria-labelledby="onboard-title"
            onChange={(event) => setName(event.target.value)}
          />
        );

      case "ocr":
        return (
          <>
            <SegmentedControl
              label="OCR"
              value={settings.indexing.ocrEnabled ? "on" : "off"}
              options={OCR_OPTIONS}
              onChange={(value) =>
                void save({ indexing: { ...settings.indexing, ocrEnabled: value === "on" } })
              }
            />
            {settings.indexing.ocrEnabled ? (
              <OcrEngineSelector
                value={settings.ocrEngine ?? "windows"}
                onChange={(ocrEngine) => updateSettings({ ocrEngine })}
              />
            ) : (
              <p className="onboard-note">Search will skip text inside images.</p>
            )}
          </>
        );

      case "appearance":
        return (
          <>
            <div className="onboard-preview" data-look={previewTheme} aria-hidden="true">
              <div
                className={"onboard-pill" + (look.liquidGlass ? " is-glass" : "")}
                style={{ "--glass": look.liquidGlassLevel / 100 } as CSSProperties}
              >
                <Logo size={18} />
                <span className="onboard-pill-hint">Search</span>
              </div>
            </div>
            <div className="onboard-rows">
              <div className="onboard-row">
                <span>Theme</span>
                <SegmentedControl
                  label="Theme"
                  value={look.theme}
                  options={THEME_OPTIONS}
                  onChange={(theme) => setLook((current) => ({ ...current, theme }))}
                />
              </div>
              <div className="onboard-row">
                <span>Liquid glass</span>
                <Switch
                  checked={look.liquidGlass}
                  label="Liquid glass"
                  onChange={(liquidGlass) => setLook((current) => ({ ...current, liquidGlass }))}
                />
              </div>
              {look.liquidGlass ? (
                <div className="onboard-row">
                  <span>Glass amount</span>
                  <span className="onboard-range">
                    <span aria-hidden="true">Clear</span>
                    <input
                      type="range"
                      min={0}
                      max={100}
                      step={1}
                      aria-label="Glass amount"
                      value={look.liquidGlassLevel}
                      onChange={(event) => {
                        const liquidGlassLevel = Number(event.target.value);
                        setLook((current) => ({ ...current, liquidGlassLevel }));
                      }}
                    />
                    <span aria-hidden="true">Frosted</span>
                  </span>
                </div>
              ) : null}
            </div>
          </>
        );

      case "shortcut":
        return (
          <div className="onboard-shortcut">
            <HotkeyCapture value={settings.hotkey} onCommit={applyHotkey} />
          </div>
        );

      case "done":
        return completionRoots(settings, suggestions).length === 0 ? (
          <p className="onboard-note">Add folders to search in Search &amp; indexes.</p>
        ) : null;

      default:
        return null;
    }
  };

  return (
    <div className="onboarding-stage">
      <div className="onboard">
        <ol className="onboard-progress" aria-label="Setup progress">
          {ONBOARDING_STEPS.map((item, index) => (
            <li
              key={item.id}
              className={index < stepIndex ? "is-done" : index === stepIndex ? "is-current" : undefined}
              aria-current={index === stepIndex ? "step" : undefined}
            >
              <span className="onboard-progress-bar" aria-hidden="true" />
              <span className="onboard-progress-label">{item.label}</span>
            </li>
          ))}
        </ol>

        <form className="onboard-form" onSubmit={handleSubmit} noValidate>
          <div
            className={"onboard-step" + (step.id === "welcome" || step.id === "done" ? " is-hero" : "")}
            key={step.id}
            ref={stepRef}
            data-dir={back ? "back" : "forward"}
            role="group"
            aria-labelledby="onboard-title"
          >
            {step.id === "welcome" ? <Logo size={56} /> : null}
            <div>
              <h1 id="onboard-title">{copy.title}</h1>
              {step.id === "done" ? (
                <p className="onboard-lead">
                  Press{" "}
                  <span className="kbd-row">
                    {(caps.length > 0 ? caps : [settings.hotkey]).map((cap, index) => (
                      <span key={cap + index}>
                        {index > 0 ? <span className="kbd-plus">+</span> : null}
                        <kbd className="kbd">{cap}</kbd>
                      </span>
                    ))}
                  </span>{" "}
                  anywhere to open the launcher.
                </p>
              ) : (
                <p className="onboard-lead">{copy.lead}</p>
              )}
            </div>
            {renderStep()}
          </div>

          {error ? (
            <p className="field-error" role="alert">
              {error}
            </p>
          ) : null}
          {finishFailed ? (
            <StateNotice
              tone="error"
              role="alert"
              title="Couldn't finish setup."
              body="Your choices are saved on this PC."
              actionLabel="Try again"
              onAction={() => void finish(lastSkipRef.current)}
            />
          ) : null}

          <div className="onboard-nav">
            {stepIndex > 0 ? (
              <button type="button" className="btn btn-ghost" disabled={finishing} onClick={() => move(-1)}>
                Back
              </button>
            ) : null}
            {step.id !== "done" ? (
              <button type="button" className="linkbtn" disabled={finishing} onClick={() => void finish(true)}>
                Skip setup
              </button>
            ) : null}
            <button ref={primaryRef} type="submit" className="btn btn-primary" disabled={finishing}>
              {finishing ? "Saving…" : primaryLabel(step.id)}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
