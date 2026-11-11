// Desktop-app navigation taxonomy. Kept separate from the shell so views and
// the sidebar can share section ids without importing each other.

export type Section = "home" | "indexes" | "workflows" | "todo" | "preferences" | "activity";

export const SECTION_TITLES: Record<Section, string> = {
  home: "Home",
  indexes: "Search & Indexing",
  workflows: "Workflows",
  todo: "To-do",
  preferences: "Settings",
  activity: "Activity",
};

export const SECTION_EYEBROWS: Record<Section, string> = {
  home: "Your workspace / Home",
  indexes: "Your workspace / Search & Indexing",
  workflows: "Your workspace / Workflows",
  todo: "Your workspace / To-do",
  preferences: "Your workspace / Settings",
  activity: "Your workspace / Activity",
};
