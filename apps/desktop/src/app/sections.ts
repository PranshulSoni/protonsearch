// Desktop-app navigation taxonomy. Kept separate from the shell so views and
// the sidebar can share section ids without importing each other.

export type Section = "home" | "indexes" | "activity" | "workflows" | "preferences";

export const SECTION_TITLES: Record<Section, string> = {
  home: "Home",
  indexes: "Search & indexes",
  activity: "Activity",
  workflows: "Workflows",
  preferences: "Preferences",
};

export const SECTION_EYEBROWS: Record<Section, string> = {
  home: "Your workspace / Home",
  indexes: "Your workspace / Search & indexes",
  activity: "Your workspace / Activity",
  workflows: "Your workspace / Workflows",
  preferences: "Your workspace / Preferences",
};
