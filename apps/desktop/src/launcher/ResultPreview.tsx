import type { ReactNode } from "react";
import type { PreviewPayload, PreviewRange, SearchResult } from "../bridge/types";
import { ResultIcon } from "./ResultIcon";
import { useLauncherPreview } from "./previewClient";
import type { MenuAction, MenuModel } from "./ResultActions";

/**
 * Preview pane (Ctrl+Space). Rendering consumes the engine PreviewPayload via
 * previewClient: loading skeleton, extracted text with real highlight ranges,
 * bounded image, OCR text, metadata, and explicit unsupported/error fallbacks.
 * Never a blank panel.
 */

const IMAGE_DATA_URL = /^data:image\/(png|jpe?g|webp|gif|avif|bmp);base64,/i;

function HighlightedText({ text, ranges }: { text: string; ranges: PreviewRange[] }) {
  if (!ranges || ranges.length === 0) return <>{text}</>;
  const ordered = [...ranges]
    .map((range) => ({
      start: Math.max(0, Math.min(text.length, range.start)),
      end: Math.max(0, Math.min(text.length, range.end)),
    }))
    .filter((range) => range.end > range.start)
    .sort((a, b) => a.start - b.start);
  if (ordered.length === 0) return <>{text}</>;
  const parts: ReactNode[] = [];
  let cursor = 0;
  ordered.forEach((range, index) => {
    if (range.start > cursor) parts.push(text.slice(cursor, range.start));
    parts.push(
      <mark className="preview-highlight" key={"hl-" + index}>
        {text.slice(range.start, range.end)}
      </mark>,
    );
    cursor = Math.max(cursor, range.end);
  });
  if (cursor < text.length) parts.push(text.slice(cursor));
  return <>{parts}</>;
}

function Skeleton() {
  return (
    <div className="preview-skeleton" data-testid="preview-loading" aria-hidden="true">
      <span className="preview-skeleton-line" />
      <span className="preview-skeleton-line preview-skeleton-line--short" />
      <span className="preview-skeleton-line" />
    </div>
  );
}

function Body({ payload }: { payload: PreviewPayload }) {
  const hasImage = payload.imageDataUrl != null && IMAGE_DATA_URL.test(payload.imageDataUrl);
  const text = payload.text ?? payload.snippet;
  return (
    <>
      {hasImage ? (
        <figure className="preview-image-wrap">
          <img
            className="preview-image"
            src={payload.imageDataUrl ?? undefined}
            alt={"Preview of " + payload.title}
            loading="lazy"
          />
        </figure>
      ) : null}
      {payload.status === "unsupported" ? (
        <p className="preview-note" data-testid="preview-unsupported">
          {payload.unsupportedReason || "No preview for this item."}
        </p>
      ) : null}
      {text ? (
        <div className="preview-text-body" data-testid="preview-text">
          <span className="preview-label">{payload.format ? payload.format + " preview" : "Preview"}</span>
          <pre>
            <HighlightedText text={text} ranges={payload.highlightRanges} />
          </pre>
        </div>
      ) : null}
      {payload.ocrText ? (
        <div className="preview-ocr" data-testid="preview-ocr">
          <span className="preview-label">Text found in image</span>
          <pre>{payload.ocrText}</pre>
        </div>
      ) : null}
      {payload.truncated ? (
        <p className="preview-truncated" data-testid="preview-truncated">
          Showing the first {payload.text?.length ?? 0} characters. Press Enter to open the full item.
        </p>
      ) : null}
      {!hasImage && !text && !payload.ocrText && payload.status === "ok" ? (
        <p className="preview-note">No preview body for this item.</p>
      ) : null}
      {payload.fields.length > 0 ? (
        <dl className="preview-facts" data-testid="preview-fields">
          {payload.fields.map((field) => (
            <div className="preview-fact" key={field.label}>
              <dt>{field.label}</dt>
              <dd title={field.value}>{field.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}
    </>
  );
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
              {payload?.format ? payload.format.toUpperCase() : result.kind.replace(/_/g, " ")}
              {result.meta?.sourceLabel ? " · " + result.meta.sourceLabel : ""}
              {payload?.truncated ? " · truncated" : ""}
            </div>
          </div>
        </div>
        <div className="preview-path" title={result.path ?? payload?.subtitle ?? result.subtitle}>
          {result.path ?? payload?.subtitle ?? result.subtitle}
        </div>
        {preview.status === "loading" ? (
          <Skeleton />
        ) : preview.status === "error" ? (
          <div className="preview-error" role="alert" data-testid="preview-error">
            <p className="preview-note">Preview is unavailable right now.</p>
            <p className="preview-error-detail">{preview.error ?? payload?.error}</p>
            <button type="button" className="inline-retry" onClick={preview.reload}>
              Retry
            </button>
          </div>
        ) : preview.status === "unsupported" ? (
          <Body payload={payload ?? { ...emptyPayload(result.id) }} />
        ) : payload ? (
          <Body payload={payload} />
        ) : (
          <p className="preview-note">No preview for this item.</p>
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

function emptyPayload(resultId: string): PreviewPayload {
  return {
    resultId,
    title: "",
    subtitle: "",
    format: "",
    text: null,
    snippet: null,
    highlightRanges: [],
    imageDataUrl: null,
    ocrText: null,
    fields: [],
    status: "unsupported",
    truncated: false,
    error: null,
    imageMime: null,
    unsupportedReason: "No preview for this item.",
  };
}
