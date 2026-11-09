import { useEffect, useState } from "react";
import type { SearchResult } from "../bridge/types";
import { formatBytes } from "../lib/format";
import { formatFullDate, formatRecency } from "./launcherFormat";

/**
 * Preview client seam for the Normal Launcher. The panel only consumes the
 * PreviewPayload below; when the Gate-2 preview IPC lands, replace the default
 * provider (or call setPreviewProvider) and the panel wiring is unchanged.
 *
 * Today the provider exposes only what SearchResult already carries (metadata +
 * snippet). It never invents preview text: unsupported bodies are an explicit
 * "note" payload.
 */

export interface PreviewFact {
  label: string;
  value: string;
  title?: string;
}

export type PreviewBody =
  | { kind: "snippet"; text: string; line: number | null }
  | { kind: "text"; text: string; label: string }
  | { kind: "url"; text: string }
  | { kind: "summary"; text: string; label: string }
  | { kind: "note"; text: string };

export interface PreviewPayload {
  resultId: string;
  title: string;
  kindLabel: string;
  path: string | null;
  body: PreviewBody;
  facts: PreviewFact[];
}

export interface PreviewProvider {
  load(result: SearchResult, signal: AbortSignal): Promise<PreviewPayload | null>;
}

const KIND_LABELS: Record<string, string> = {
  application: "Application",
  file: "File",
  folder: "Folder",
  document: "Document",
  content_match: "Match in document",
  ocr_text: "Match in screenshot text",
  clipboard: "Clipboard",
  browser_history: "Browser history",
  bookmark: "Bookmark",
  recent: "Recent item",
  action: "Action",
  command: "Command",
  calculator: "Calculator",
  quicklink: "Workflow",
  snippet: "Snippet",
  git_commit: "Git commit",
  git_repository: "Git repository",
  git_todo: "Git TODO",
  setting: "Setting",
  window: "Window",
  agent: "Agent",
  system: "System",
};

function kindLabel(result: SearchResult): string {
  return KIND_LABELS[result.kind] ?? result.kind;
}

function bodyFor(result: SearchResult): PreviewBody {
  const meta = result.meta;
  if (meta?.snippet) {
    return { kind: "snippet", text: meta.snippet, line: meta.line ?? null };
  }
  switch (result.kind) {
    case "clipboard":
    case "snippet":
      return { kind: "text", label: "Text", text: result.title };
    case "browser_history":
    case "bookmark":
      return { kind: "url", text: result.path ?? result.subtitle };
    case "application":
    case "action":
    case "command":
    case "calculator":
    case "window":
    case "setting":
    case "agent":
      return { kind: "summary", label: kindLabel(result), text: result.subtitle || result.title };
    case "content_match":
    case "ocr_text":
    case "document":
    case "file":
    case "recent":
    case "folder":
    case "git_commit":
    case "git_repository":
    case "git_todo":
      return { kind: "note", text: "Preview not available — press Enter to open." };
    default:
      return { kind: "note", text: "No preview for this item." };
  }
}

export function buildMetadataPreview(result: SearchResult): PreviewPayload {
  const meta = result.meta;
  const facts: PreviewFact[] = [];
  const size = formatBytes(meta?.size);
  const modified = formatRecency(meta?.modified);
  if (size) facts.push({ label: "Size", value: size });
  if (modified) {
    facts.push({ label: "Modified", value: modified, title: formatFullDate(meta?.modified) });
  }
  if (meta?.itemCount != null) {
    facts.push({ label: "Contents", value: meta.itemCount.toLocaleString() + " items" });
  }
  if (meta?.line != null) facts.push({ label: "Line", value: String(meta.line) });
  return {
    resultId: result.id,
    title: result.title,
    kindLabel: kindLabel(result),
    path: result.path ?? null,
    body: bodyFor(result),
    facts,
  };
}

const metadataProvider: PreviewProvider = {
  async load(result) {
    return buildMetadataPreview(result);
  },
};

let activeProvider: PreviewProvider = metadataProvider;

/** Gate-2 wiring point: swap this provider when the preview IPC lands. */
export function setPreviewProvider(provider: PreviewProvider | null): void {
  activeProvider = provider ?? metadataProvider;
}

export interface LauncherPreviewState {
  status: "idle" | "loading" | "ready" | "unavailable";
  payload: PreviewPayload | null;
}

/** Request/selection plumbing: cancels stale requests when the selection moves. */
export function useLauncherPreview(
  result: SearchResult | null,
  enabled: boolean,
): LauncherPreviewState {
  const [state, setState] = useState<LauncherPreviewState>({
    status: "idle",
    payload: null,
  });

  useEffect(() => {
    if (!enabled || !result) {
      setState({ status: "idle", payload: null });
      return;
    }
    const controller = new AbortController();
    let cancelled = false;
    setState({ status: "loading", payload: null });
    activeProvider
      .load(result, controller.signal)
      .then((payload) => {
        if (cancelled || controller.signal.aborted) return;
        setState(
          payload
            ? { status: "ready", payload }
            : { status: "unavailable", payload: null },
        );
      })
      .catch(() => {
        if (cancelled || controller.signal.aborted) return;
        setState({ status: "unavailable", payload: null });
      });
    return () => {
      cancelled = true;
      controller.abort();
    };
  }, [enabled, result]);

  return state;
}
