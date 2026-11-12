import type { TextRange } from "../bridge/types";

function normalize(ranges: TextRange[] | undefined, length: number): TextRange[] {
  if (!ranges || ranges.length === 0) return [];
  const sorted = [...ranges]
    .map((r) => ({ start: Math.max(0, Math.min(r.start, length)), end: Math.max(0, Math.min(r.end, length)) }))
    .filter((r) => r.end > r.start)
    .sort((a, b) => a.start - b.start);
  const merged: TextRange[] = [];
  for (const range of sorted) {
    const last = merged[merged.length - 1];
    if (last && range.start <= last.end) {
      last.end = Math.max(last.end, range.end);
    } else {
      merged.push({ ...range });
    }
  }
  return merged;
}

/** Renders a result title with quiet, accessible match highlights. */
export function Highlight({ text, ranges }: { text: string; ranges?: TextRange[] }) {
  const clean = normalize(ranges, text.length);
  if (clean.length === 0) return <>{text}</>;
  const parts: React.ReactNode[] = [];
  let cursor = 0;
  clean.forEach((range, index) => {
    if (range.start > cursor) parts.push(text.slice(cursor, range.start));
    parts.push(
      <mark key={index} className="match">
        {text.slice(range.start, range.end)}
      </mark>,
    );
    cursor = range.end;
  });
  if (cursor < text.length) parts.push(text.slice(cursor));
  return <>{parts}</>;
}
