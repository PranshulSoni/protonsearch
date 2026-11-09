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
  const { status, settings, openResult, hideLauncher, showMain } = useApp();
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
      openResult={openResult}
      hideLauncher={hideLauncher}
      showMain={showMain}
      status={status}
      density={settings.density}
      onGeometry={onGeometry}
    />
  );
}
