import type { SearchResult } from "../bridge/types";
import { formatBytes, timeAgo } from "../lib/format";
import { ResultIcon } from "./ResultIcon";
import type { MenuAction, MenuModel } from "./ResultActions";

/**
 * On-demand preview pane (Ctrl+Space). Default off; no always-on preview.
 * Honest about what the frozen IPC contract can supply: snippet/line for
 * content + OCR matches, otherwise metadata only. Binding: SCREENS §2.6.
 */

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

function fullDate(timestamp: number | null | undefined): string | undefined {
  if (!timestamp) return undefined;
  return new Date(timestamp).toLocaleString();
}

function previewBody(result: SearchResult) {
  const meta = result.meta;
  if (meta?.snippet) {
    return (
      <blockquote className="preview-snippet">
        {meta.line != null ? <span className="preview-snippet-line">Line {meta.line}</span> : null}
        <p>{meta.snippet}</p>
      </blockquote>
    );
  }
  switch (result.kind) {
    case "clipboard":
    case "snippet":
      return (
        <div className="preview-text">
          <span className="preview-label">Text</span>
          <p>{result.title}</p>
        </div>
      );
    case "browser_history":
    case "bookmark":
      return (
        <div className="preview-text">
          <span className="preview-label">URL</span>
          <p className="preview-url">{result.path ?? result.subtitle}</p>
        </div>
      );
    case "application":
    case "action":
    case "command":
    case "calculator":
    case "window":
    case "setting":
    case "agent":
      return (
        <div className="preview-text">
          <span className="preview-label">{kindLabel(result)}</span>
          <p>{result.subtitle || result.title}</p>
        </div>
      );
    case "content_match":
    case "ocr_text":
    case "document":
    case "file":
    case "recent":
    case "folder":
    case "git_commit":
    case "git_repository":
    case "git_todo":
      return <p className="preview-note">Preview not available — press Enter to open.</p>;
    default:
      return <p className="preview-note">No preview for this item.</p>;
  }
}

export interface ResultPreviewProps {
  result: SearchResult | null;
  model: MenuModel | null;
  onRun: (action: MenuAction) => void;
}

export function ResultPreview({ result, model, onRun }: ResultPreviewProps) {
  if (!result) {
    return (
      <aside className="preview" aria-label="Preview" data-testid="preview-panel">
        <div className="preview-scroll" tabIndex={0} aria-label="Preview details">
          <p className="preview-note">Select a result to preview it.</p>
        </div>
      </aside>
    );
  }

  const meta = result.meta;
  const size = formatBytes(meta?.size);
  const modified = timeAgo(meta?.modified);
  const facts: { label: string; value: string; title?: string }[] = [];
  if (size) facts.push({ label: "Size", value: size });
  if (modified) facts.push({ label: "Modified", value: modified, title: fullDate(meta?.modified) });
  if (meta?.itemCount != null) {
    facts.push({ label: "Contents", value: meta.itemCount.toLocaleString() + " items" });
  }
  if (meta?.line != null) facts.push({ label: "Line", value: String(meta.line) });

  const actions = model?.actions ?? [];

  return (
    <aside className="preview" aria-label="Preview" data-testid="preview-panel">
      <div className="preview-scroll" tabIndex={0} aria-label={"Preview of " + result.title}>
        <div className="preview-head">
          <ResultIcon result={result} />
          <div className="preview-heading">
            <div className="preview-title">{result.title}</div>
            <div className="preview-kind">
              {kindLabel(result)}
              {meta?.sourceLabel ? " · " + meta.sourceLabel : ""}
            </div>
          </div>
        </div>
        {result.path ? (
          <div className="preview-path" title={result.path}>
            {result.path}
          </div>
        ) : (
          <div className="preview-path">{result.subtitle}</div>
        )}
        {previewBody(result)}
        {facts.length > 0 ? (
          <dl className="preview-facts">
            {facts.map((fact) => (
              <div className="preview-fact" key={fact.label}>
                <dt>{fact.label}</dt>
                <dd title={fact.title}>{fact.value}</dd>
              </div>
            ))}
          </dl>
        ) : null}
      </div>
      {actions.length > 0 ? (
        <div className="preview-actions">
          {actions.map((action) => (
            <button key={action.id} type="button" className="preview-action" onClick={() => onRun(action)}>
              {action.label}
            </button>
          ))}
        </div>
      ) : null}
    </aside>
  );
}
