// Small app-level hooks that need bridge or browser context.

import { useApp } from "../App";

/**
 * Opens the quick launcher. In the browser mock there is no launcher window, so
 * we navigate the preview to the launcher route (?view=launcher) — the same
 * mock-only entry the shell already supports. The Tauri build uses the real
 * window command.
 */
export function useOpenLauncher(): () => void {
  const { isMock, showLauncher } = useApp();
  return () => {
    if (isMock && typeof window !== "undefined") {
      const url = new URL(window.location.href);
      url.searchParams.set("view", "launcher");
      window.location.assign(url.toString());
      return;
    }
    showLauncher();
  };
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}