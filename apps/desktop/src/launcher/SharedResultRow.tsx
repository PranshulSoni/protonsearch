import type { KeyboardEvent, MouseEvent } from "react";
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

export function SharedResultRow({
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
