// Island look (liquid glass, glass amount, page dots, app theme) cached in
// localStorage. The island webview paints its first frame before the bootstrap
// settings arrive, so the cached look keeps that frame in the saved style. The
// settings page and the island share one origin and read and write one entry.

import { useEffect, useState } from "react";
import type { IslandSettings, ThemeMode } from "../bridge/types";

export interface IslandLook {
  liquidGlass: boolean;
  liquidGlassLevel: number;
  showDots: boolean;
  theme: ThemeMode;
}

const STORAGE_KEY = "protonsearch.islandLook";
const DARK_QUERY = "(prefers-color-scheme: dark)";
const THEMES: ThemeMode[] = ["system", "light", "dark"];
const DEFAULT_LOOK: IslandLook = {
  liquidGlass: false,
  liquidGlassLevel: 60,
  showDots: true,
  theme: "system",
};

export function lookFrom(island: IslandSettings, theme: ThemeMode): IslandLook {
  return {
    liquidGlass: island.liquidGlass === true,
    liquidGlassLevel: island.liquidGlassLevel ?? DEFAULT_LOOK.liquidGlassLevel,
    showDots: island.showDots !== false,
    theme,
  };
}

export function readIslandLook(): IslandLook {
  try {
    const parsed = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? "null") as Partial<IslandLook> | null;
    if (!parsed) return DEFAULT_LOOK;
    return {
      liquidGlass: parsed.liquidGlass === true,
      liquidGlassLevel:
        typeof parsed.liquidGlassLevel === "number" ? parsed.liquidGlassLevel : DEFAULT_LOOK.liquidGlassLevel,
      showDots: parsed.showDots !== false,
      theme: THEMES.includes(parsed.theme as ThemeMode) ? (parsed.theme as ThemeMode) : "system",
    };
  } catch {
    return DEFAULT_LOOK;
  }
}

export function writeIslandLook(look: IslandLook): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(look));
  } catch {
    /* storage unavailable: the first paint falls back to the default look */
  }
}

/** Light or dark for the app theme. System follows the OS setting. */
export function useResolvedTheme(theme: ThemeMode): "light" | "dark" {
  const [systemDark, setSystemDark] = useState(() => window.matchMedia(DARK_QUERY).matches);
  useEffect(() => {
    const media = window.matchMedia(DARK_QUERY);
    const onChange = () => setSystemDark(media.matches);
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, []);
  if (theme === "light" || theme === "dark") return theme;
  return systemDark ? "dark" : "light";
}
