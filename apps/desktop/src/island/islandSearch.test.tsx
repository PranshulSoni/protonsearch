import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "../bridge/api";
import type { SearchResponse, SearchResult, SearchScope, SourceKind } from "../bridge/types";
import { groupResultsInRankOrder } from "../lib/results";
import { SearchCard } from "./IslandCards";
import { STALE_RESULTS_MS, useIslandSearch, type ScopedSearchState } from "./islandData";
import { expandedSearchHeight } from "./islandDims";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function row(id: string, kind: SourceKind, title = id): SearchResult {
  return {
    id,
    kind,
    title,
    subtitle: "",
    path: null,
    icon: { type: "monogram" },
    score: 0,
    matchRanges: [],
    meta: {},
    action: { type: "none" },
    actions: [],
  } as unknown as SearchResult;
}

function response(query: string, results: SearchResult[]): SearchResponse {
  return {
    protocol: 1,
    requestId: 1,
    query,
    total: results.length,
    results,
    tookMs: 1,
    timings: [],
    status: { state: "ready", indexedFiles: 0, contentDocs: 0 },
  } as unknown as SearchResponse;
}

let root: Root | null = null;
let container: HTMLDivElement | null = null;

afterEach(async () => {
  await act(async () => root?.unmount());
  container?.remove();
  root = null;
  vi.restoreAllMocks();
  vi.useRealTimers();
});

async function mount(node: React.ReactNode) {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => root!.render(node));
}

describe("rank-order grouping", () => {
  it("keeps ranked order and starts a label wherever the type changes", () => {
    const groups = groupResultsInRankOrder([
      row("app:exact", "application"),
      row("file:exact", "file"),
      row("file:stem", "file"),
      row("app:weak", "application"),
      row("folder:prefix", "folder"),
      row("file:beyond-view", "file"),
    ], 5);
    expect(groups.map((group) => [group.label, group.items.map((item) => item.result.id)])).toEqual([
      ["APPLICATIONS", ["app:exact"]],
      ["FILES", ["file:exact", "file:stem"]],
      ["APPLICATIONS", ["app:weak"]],
      ["FOLDERS", ["folder:prefix"]],
    ]);
    // Content and OCR rows share one DOCUMENTS label.
    expect(groupResultsInRankOrder([row("c", "content_match"), row("o", "ocr_text")]).length).toBe(1);
  });

  it("grows the island by one line per extra label", () => {
    expect(expandedSearchHeight(5)).toBe(expandedSearchHeight(5, 1));
    expect(expandedSearchHeight(2, 3) - expandedSearchHeight(2, 1)).toBe(48);
    expect(expandedSearchHeight(9, 1)).toBe(expandedSearchHeight(5, 1));
  });
});

describe("island search card", () => {
  function card(results: SearchResult[], onOpen: (result: SearchResult) => void) {
    return (
      <SearchCard
        compact={false}
        query="budget"
        onQueryChange={() => undefined}
        scope="all"
        onScope={() => undefined}
        results={results}
        pending={false}
        error={null}
        onOpen={onOpen}
        onFocus={() => undefined}
        inputRef={{ current: null }}
        placeholder="Search"
      />
    );
  }

  it("shows rows in ranked order, not grouped by type", async () => {
    await mount(card([
      row("app:1", "application", "Budget"),
      row("file:1", "file", "budget.xlsx"),
      row("app:2", "application", "Budgeteer"),
    ], () => undefined));
    const titles = [...container!.querySelectorAll(".island-result-row .filename")].map((el) => el.getAttribute("title"));
    expect(titles).toEqual(["Budget", "budget.xlsx", "Budgeteer"]);
  });

  it("puts the selection back on the top hit for every new result set", async () => {
    const opened: string[] = [];
    const first = [row("a", "file"), row("b", "file"), row("c", "file")];
    await mount(card(first, (result) => opened.push(result.id)));
    const input = container!.querySelector("input")!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    });
    expect(container!.querySelector(".island-result-row.is-selected")?.getAttribute("data-result-id")).toBe("c");
    // Same length, different rows (the next keystroke's answer).
    await act(async () => root!.render(card([row("x", "file"), row("y", "file"), row("z", "file")], (result) => opened.push(result.id))));
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    });
    expect(opened).toEqual(["x"]);
  });
});

describe("useIslandSearch", () => {
  let latest: ScopedSearchState | null = null;
  function Probe({ scope, query }: { scope: SearchScope; query: string }) {
    latest = useIslandSearch(scope, query, true);
    return null;
  }

  it("never lets an older answer overwrite a newer query", async () => {
    const pending: Array<(value: SearchResponse) => void> = [];
    vi.spyOn(api, "search").mockImplementation(
      () => new Promise<SearchResponse>((resolve) => pending.push(resolve)),
    );
    await mount(<Probe scope="all" query="bu" />);
    await act(async () => root!.render(<Probe scope="all" query="bud" />));
    // The newer request answers first, then the older one.
    await act(async () => pending[1](response("bud", [row("new", "file")])));
    await act(async () => pending[0](response("bu", [row("old", "file")])));
    expect(latest!.results.map((r) => r.id)).toEqual(["new"]);
    expect(latest!.pending).toBe(false);
  });

  it("swaps rows in place on a filter switch and never leaves stale rows standing", async () => {
    vi.useFakeTimers();
    const pending: Array<(value: SearchResponse) => void> = [];
    vi.spyOn(api, "search").mockImplementation(
      () => new Promise<SearchResponse>((resolve) => pending.push(resolve)),
    );
    await mount(<Probe scope="all" query="report" />);
    await act(async () => pending[0](response("report", [row("all-1", "application"), row("all-2", "file")])));

    // Fast answer: the old rows bridge the gap (no empty flash), then swap.
    await act(async () => root!.render(<Probe scope="files" query="report" />));
    expect(latest!.results.map((r) => r.id)).toEqual(["all-1", "all-2"]);
    await act(async () => pending[1](response("report", [row("file-1", "file")])));
    expect(latest!.results.map((r) => r.id)).toEqual(["file-1"]);

    // Slow answer: after STALE_RESULTS_MS the old rows give way to loading.
    await act(async () => root!.render(<Probe scope="folders" query="report" />));
    await act(async () => {
      vi.advanceTimersByTime(STALE_RESULTS_MS + 10);
    });
    expect(latest!.results).toEqual([]);
    expect(latest!.pending).toBe(true);
    await act(async () => pending[2](response("report", [row("folder-1", "folder")])));
    expect(latest!.results.map((r) => r.id)).toEqual(["folder-1"]);
  });

  it("does not show the empty query's rows when typing starts", async () => {
    const pending: Array<(value: SearchResponse) => void> = [];
    vi.spyOn(api, "search").mockImplementation(
      () => new Promise<SearchResponse>((resolve) => pending.push(resolve)),
    );
    await mount(<Probe scope="all" query="" />);
    await act(async () => pending[0](response("", [row("recent-1", "recent")])));
    await act(async () => root!.render(<Probe scope="all" query="c" />));
    expect(latest!.results).toEqual([]);
    expect(latest!.pending).toBe(true);
  });
});
