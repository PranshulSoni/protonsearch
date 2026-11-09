import type { SearchResult } from "../bridge/types";
import { ResultIcon } from "./ResultIcon";
import { useLauncherPreview, type PreviewBody } from "./previewClient";
import type { MenuAction, MenuModel } from "./ResultActions";

/**
 * On-demand preview pane (Ctrl+Space). Default off. Rendering consumes the
 * PreviewPayload from previewClient — the Gate-2 preview IPC only replaces the
 * provider there; this panel's request/selection plumbing stays unchanged.
 */

function Body({ body }: { body: PreviewBody }) {
  switch (body.kind) {
    case "snippet":
      return (
        <blockquote className="preview-snippet">
          {body.line != null ? (
            <span className="preview-snippet-line">Line {body.line}</span>
          ) : null}
          <p>{body.text}</p>
        </blockquote>
      );
    case "text":
      return (
        <div className="preview-text">
          <span className="preview-label">{body.label}</span>
          <p>{body.text}</p>
        </div>
      );
    case "url":
      return (
        <div className="preview-text">
          <span className="preview-label">URL</span>
          <p className="preview-url">{body.text}</p>
        </div>
      );
    case "summary":
      return (
        <div className="preview-text">
          <span className="preview-label">{body.label}</span>
          <p>{body.text}</p>
        </div>
      );
    case "note":
    default:
      return <p className="preview-note">{body.text}</p>;
  }
}

export interface ResultPreviewProps {
  result: SearchResult | null;
  model: MenuModel | null;
  onRun: (action: MenuAction) => void;
}

export function ResultPreview({ result, model, onRun }: ResultPreviewProps) {
  const preview = useLauncherPreview(result, result !== null);

  if (!result) {
    return (
      <aside className="preview" aria-label="Preview" data-testid="preview-panel">
        <div className="preview-scroll" tabIndex={0} aria-label="Preview details">
          <p className="preview-note">Select a result to preview it.</p>
        </div>
      </aside>
    );
  }

  const actions = model?.actions ?? [];
  const payload = preview.payload;

  return (
    <aside className="preview" aria-label="Preview" data-testid="preview-panel">
      <div className="preview-scroll" tabIndex={0} aria-label={"Preview of " + result.title}>
        <div className="preview-head">
          <ResultIcon result={result} />
          <div className="preview-heading">
            <div className="preview-title">{payload?.title ?? result.title}</div>
            <div className="preview-kind">
              {payload?.kindLabel ?? result.kind.replace(/_/g, " ")}
              {result.meta?.sourceLabel ? " · " + result.meta.sourceLabel : ""}
            </div>
          </div>
        </div>
        <div className="preview-path" title={payload?.path ?? result.path ?? result.subtitle}>
          {payload?.path ?? result.subtitle}
        </div>
        {preview.status === "loading" ? (
          <p className="preview-note" role="status">
            Loading preview…
          </p>
        ) : preview.status === "unavailable" || !payload ? (
          <p className="preview-note">Preview unavailable.</p>
        ) : (
          <>
            <Body body={payload.body} />
            {payload.facts.length > 0 ? (
              <dl className="preview-facts">
                {payload.facts.map((fact) => (
                  <div className="preview-fact" key={fact.label}>
                    <dt>{fact.label}</dt>
                    <dd title={fact.title}>{fact.value}</dd>
                  </div>
                ))}
              </dl>
            ) : null}
          </>
        )}
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
