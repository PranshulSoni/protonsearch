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

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const offSummoned = await listen("launcher:summoned", () => {
          surfaceRef.current?.summon();
        });
        const offHidden = await listen("launcher:hidden", () => {
          surfaceRef.current?.clear();
        });
        if (disposed) {
          offSummoned();
          offHidden();
        } else {
          unlisteners.push(offSummoned, offHidden);
        }
      } catch {
        /* Event plumbing unavailable: the launcher still works on first open. */
      }
    })();
    return () => {
      disposed = true;
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
