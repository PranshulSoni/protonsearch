import { memo, type KeyboardEvent, type MouseEvent } from "react";
import type { SearchResult } from "../bridge/types";
import { Highlight } from "../components/Highlight";
import { resultMetaText } from "./resultMeta";
import { ResultIcon } from "./ResultIcon";

/**
 * Shared result row used by both the Normal Launcher and the Dynamic Island.
 * Keep DOM/ARIA/order identical so both presentations show the same engine
 * results, icons, groups and keyboard semantics.
 */
export interface SharedResultRowProps {
  result: SearchResult;
  selected: boolean;
  optionId: string;
  descriptionId: string;
  metaId: string;
  className: string;
  ariaLabel?: string;
  ariaDescribedBy?: string;
  testId?: string;
  onMouseDown?: (event: MouseEvent<HTMLDivElement>) => void;
  onMouseEnter?: () => void;
  onClick: () => void;
  onContextMenu?: (event: MouseEvent<HTMLDivElement>) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLDivElement>) => void;
}

function SharedResultRowBase({
  result,
  selected,
  optionId,
  descriptionId,
  metaId,
  className,
  ariaLabel,
  ariaDescribedBy,
  testId,
  onMouseDown,
  onMouseEnter,
  onClick,
  onContextMenu,
  onKeyDown,
}: SharedResultRowProps) {
  const meta = resultMetaText(result);
  const description = [result.path ?? result.subtitle, meta].filter(Boolean).join(" · ");
  return (
    <div
      id={optionId}
      role="option"
      aria-selected={selected}
      aria-label={ariaLabel}
      aria-describedby={ariaDescribedBy}
      data-testid={testId}
      data-result-id={result.id}
      className={className + (selected ? " is-selected" : "")}
      onMouseDown={onMouseDown ?? ((event) => event.preventDefault())}
      onMouseEnter={onMouseEnter}
      onClick={onClick}
      onContextMenu={onContextMenu}
      onKeyDown={onKeyDown}
    >
      <ResultIcon result={result} />
      <span className="result-body">
        <span className="filename" title={result.title}>
          <Highlight text={result.title} ranges={result.matchRanges} />
        </span>
        <span className="filepath" id={descriptionId} title={result.path ?? result.subtitle}>
          {result.subtitle}
        </span>
      </span>
      {meta ? (
        <span className="result-meta" id={metaId}>
          {meta}
        </span>
      ) : (
        <span className="sr-only" id={metaId}>
          {description}
        </span>
      )}
    </div>
  );
}

function rangesEqual(
  a?: Array<{ start: number; end: number }>,
  b?: Array<{ start: number; end: number }>,
): boolean {
  if (a === b) return true;
  if (!a || !b || a.length !== b.length) return false;
  return a.every((range, index) => range.start === b[index].start && range.end === b[index].end);
}

function areEqual(prev: SharedResultRowProps, next: SharedResultRowProps): boolean {
  if (
    prev.selected !== next.selected ||
    prev.optionId !== next.optionId ||
    prev.descriptionId !== next.descriptionId ||
    prev.metaId !== next.metaId ||
    prev.className !== next.className ||
    prev.ariaLabel !== next.ariaLabel ||
    prev.ariaDescribedBy !== next.ariaDescribedBy ||
    prev.testId !== next.testId
  ) {
    return false;
  }
  const a = prev.result;
  const b = next.result;
  if (
    a.id !== b.id ||
    a.title !== b.title ||
    a.subtitle !== b.subtitle ||
    a.path !== b.path ||
    a.kind !== b.kind ||
    a.icon.type !== b.icon.type ||
    a.icon.type === "file_type" && b.icon.type === "file_type" && a.icon.ext !== b.icon.ext ||
    a.icon.type === "app_icon_data" && b.icon.type === "app_icon_data" && a.icon.data !== b.icon.data ||
    a.icon.type === "app_icon" && b.icon.type === "app_icon" && a.icon.path !== b.icon.path
  ) {
    return false;
  }
  if (!rangesEqual(a.matchRanges, b.matchRanges)) return false;
  const am = a.meta;
  const bm = b.meta;
  return (
    am?.label === bm?.label &&
    am?.size === bm?.size &&
    am?.modified === bm?.modified &&
    am?.itemCount === bm?.itemCount &&
    am?.line === bm?.line &&
    am?.sourceLabel === bm?.sourceLabel
  );
}

/** Memoized so typing only re-renders changed rows, not the whole 30-row list. */
export const SharedResultRow = memo(SharedResultRowBase, areEqual);
