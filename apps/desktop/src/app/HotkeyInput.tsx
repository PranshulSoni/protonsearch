// Press-the-chord shortcut capture control (KEYBOARD_AND_A11Y §6, SCREENS
// §3.6/§4.6). Validates with the same parser as the Rust shell, applies the
// shortcut live through api.setHotkey (parent callback) and surfaces OS
// rejection text inline. Escape cancels; Tab moves focus without disarming.

import { useEffect, useRef, useState } from "react";
import { StateNotice } from "./ui";
import {
  DEFAULT_HOTKEY,
  chordFromKeyboardEvent,
  displayHotkey,
  hotkeyCapTokens,
  parseHotkey,
} from "./hotkey";

export interface HotkeyCaptureProps {
  /** Currently stored accelerator string. */
  value: string;
  /** Apply the validated accelerator; reject with a descriptive error to show inline. */
  onCommit: (accelerator: string, display: string) => Promise<void>;
  help?: string;
  disabled?: boolean;
}

export function HotkeyCapture({ value, onCommit, help, disabled }: HotkeyCaptureProps) {
  const [capturing, setCapturing] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [commitFailed, setCommitFailed] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [liveMessage, setLiveMessage] = useState("");
  const triggerRef = useRef<HTMLButtonElement>(null);
  const commitRef = useRef(onCommit);
  commitRef.current = onCommit;
  const valueRef = useRef(value);
  valueRef.current = value;

  const caps = hotkeyCapTokens(value);
  const currentDisplay = displayHotkey(value);

  useEffect(() => {
    if (!status) return;
    const timer = window.setTimeout(() => setStatus(null), 2500);
    return () => window.clearTimeout(timer);
  }, [status]);

  const startCapture = () => {
    if (disabled || applying) return;
    setCapturing(true);
    setError(null);
    setCommitFailed(false);
    setStatus(null);
    setLiveMessage("Press the shortcut you want. Press Escape to cancel.");
    triggerRef.current?.focus();
  };

  const cancelCapture = () => {
    setCapturing(false);
    setError(null);
    setCommitFailed(false);
    setLiveMessage("Shortcut capture cancelled.");
    triggerRef.current?.focus();
  };

  const finish = async (accelerator: string, display: string) => {
    setCapturing(false);
    if (accelerator === valueRef.current) {
      setStatus("That shortcut is already active.");
      triggerRef.current?.focus();
      return;
    }
    setApplying(true);
    setError(null);
    setCommitFailed(false);
    try {
      await commitRef.current(accelerator, display);
      setStatus("Shortcut applied and active now.");
      setLiveMessage("Shortcut applied and active now.");
    } catch {
      setCommitFailed(true);
      setLiveMessage("That shortcut isn't available.");
    } finally {
      setApplying(false);
      triggerRef.current?.focus();
    }
  };

  const reset = () => {
    const parsed = parseHotkey(DEFAULT_HOTKEY);
    if (parsed.ok) void finish(parsed.accelerator, parsed.display);
  };

  useEffect(() => {
    if (!capturing) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        cancelCapture();
        return;
      }
      const result = chordFromKeyboardEvent(event);
      if (result === null) return;
      const withoutModifier = !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
      // Tab still moves focus while capture is armed (KEYBOARD §3).
      if (withoutModifier && event.key === "Tab") return;
      event.preventDefault();
      event.stopPropagation();
      if (!result.ok) {
        setError(result.error);
        setLiveMessage(result.error);
        return;
      }
      void finish(result.accelerator, result.display);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
    // finish/cancelCapture read refs and setState only; re-arming on each
    // capture start is enough.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [capturing]);

  return (
    <div className="hotkey-control">
      <div className="hotkey-display">
        <span className="hotkey-label">Current shortcut</span>
        <span className="kbd-row" aria-label={"Current shortcut " + currentDisplay}>
          {caps.length > 0 ? (
            caps.map((cap, index) => (
              <span key={cap + index}>
                {index > 0 ? <span className="kbd-plus">+</span> : null}
                <kbd className="kbd">{cap}</kbd>
              </span>
            ))
          ) : (
            <span className="hotkey-value">{currentDisplay}</span>
          )}
        </span>
      </div>
      <div className="hotkey-actions">
        <button
          ref={triggerRef}
          type="button"
          className={"btn" + (capturing ? " is-capturing" : "")}
          aria-pressed={capturing}
          aria-describedby="hotkey-capture-help"
          disabled={disabled || applying}
          onClick={() => (capturing ? cancelCapture() : startCapture())}
        >
          {applying ? "Applying…" : capturing ? "Listening…" : "Change"}
        </button>
        <button
          type="button"
          className="btn btn-ghost"
          disabled={disabled || applying || capturing}
          onClick={reset}
        >
          Reset to Alt+Space
        </button>
      </div>
      {capturing || help ? (
        <p className="field-help" id="hotkey-capture-help">
          {capturing ? "Press the new shortcut. Esc cancels." : help}
        </p>
      ) : null}
      <span className="sr-only" role="status" aria-live="polite">
        {liveMessage}
      </span>
      {status ? (
        <p className="inline-note" role="status">
          {status}
        </p>
      ) : null}
      {error ? (
        <StateNotice className="state-inline" role="alert" title="That shortcut isn't valid." body={error} />
      ) : null}
      {commitFailed ? (
        <StateNotice
          className="state-inline"
          title="That shortcut isn't available."
          body="Another app may already be using it."
        />
      ) : null}
    </div>
  );
}