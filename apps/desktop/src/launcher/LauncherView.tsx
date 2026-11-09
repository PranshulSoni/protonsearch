import { useCallback, useEffect, useRef } from "react";
import { api, isTauriRuntime } from "../bridge/api";
import { useApp } from "../App";
import LauncherSurface, { type LauncherSurfaceHandle } from "./LauncherSurface";

/**
 * Quick launcher window container: owns the bridge/runtime wiring, the Tauri
 * window events and launcher geometry. All interaction lives in LauncherSurface
 * so it can be tested against a mock bridge.
 */
export default function LauncherView() {
  const { status, settings, hideLauncher, showMain } = useApp();
  const surfaceRef = useRef<LauncherSurfaceHandle>(null);
  // A hide event emitted by an earlier dismissal can arrive after a re-summon
  // (Tauri events are async). Ignore that stale event so re-summon really cancels
  // the close instead of clearing the freshly opened surface.
  const summonGuardRef = useRef<number | null>(null);
  const justSummonedRef = useRef(false);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const offSummoned = await listen("launcher:summoned", () => {
          justSummonedRef.current = true;
          if (summonGuardRef.current != null) window.clearTimeout(summonGuardRef.current);
          summonGuardRef.current = window.setTimeout(() => {
            justSummonedRef.current = false;
            summonGuardRef.current = null;
          }, 300);
          surfaceRef.current?.summon();
        });
        // The shell asks for a dismiss animation instead of hiding directly
        // (hotkey toggle, window blur, tray actions).
        const offDismiss = await listen("launcher:dismiss", () => {
          justSummonedRef.current = false;
          surfaceRef.current?.dismiss();
        });
        const offHidden = await listen("launcher:hidden", () => {
          if (justSummonedRef.current) {
            // Stale hide from a dismissed generation; the launcher is visible again.
            justSummonedRef.current = false;
            return;
          }
          surfaceRef.current?.clear();
        });
        if (disposed) {
          offSummoned();
          offDismiss();
          offHidden();
        } else {
          unlisteners.push(offSummoned, offDismiss, offHidden);
        }
      } catch {
        /* Event plumbing unavailable: the launcher still works on first open. */
      }
    })();
    return () => {
      disposed = true;
      if (summonGuardRef.current != null) window.clearTimeout(summonGuardRef.current);
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  const onGeometry = useCallback((height: number, preview: boolean) => {
    if (!isTauriRuntime()) return;
    void api.setLauncherGeometry(height, preview);
  }, []);

  return (
    <LauncherSurface
      ref={surfaceRef}
      search={api.search}
      /* Use the bridge directly: App.openResult hides the launcher even when the
         engine returns a confirmation challenge, which would hide this bar. The
         surface owns hide-after-execute for every action type. */
      openResult={api.openResult}
      confirmNativeAction={api.confirmNativeAction}
      hideLauncher={hideLauncher}
      showMain={showMain}
      status={status}
      density={settings.density}
      onGeometry={onGeometry}
    />
  );
}
