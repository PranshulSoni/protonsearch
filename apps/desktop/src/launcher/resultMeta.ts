import type { SearchResult } from "../bridge/types";
import { formatRecency } from "./launcherFormat";

/** Same trailing meta rule for every launcher/island result row. */
export function resultMetaText(result: SearchResult): string {
  const meta = result.meta;
  if (!meta) return "";
  // Clipboard category labels live in the section header, not on every row; the
  // trailing slot shows recency instead.
  if (result.kind === "clipboard" && meta.label && meta.modified) {
    return formatRecency(meta.modified);
  }
  if (meta.label) return meta.label;
  if (meta.modified) return formatRecency(meta.modified);
  if (meta.itemCount != null) return meta.itemCount + " items";
  return "";
}
