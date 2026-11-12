import type { IconName } from "./ui";

/** Map engine source ids to real outline icons (never glyph placeholders). */
export function sourceIconName(id: string): IconName {
  switch (id) {
    case "apps":
      return "grid";
    case "browser":
      return "browser";
    case "clipboard":
      return "clipboard";
    case "recent":
      return "activity";
    case "git":
      return "git";
    default:
      return "folder";
  }
}
