export const themes = ["light", "dark", "glass"] as const;
export type Theme = (typeof themes)[number];

export function isTheme(value: unknown): value is Theme {
  return themes.some((theme) => value === theme);
}

export function savedTheme(value: unknown): Theme {
  if (value && typeof value === "object" && "theme" in value && isTheme(value.theme)) return value.theme;
  return "dark";
}

export function applyTheme(theme: Theme) {
  document.body.dataset.theme = theme;
  document.documentElement.style.colorScheme = theme === "dark" ? "dark" : "light";
}
