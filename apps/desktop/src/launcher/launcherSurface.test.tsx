import { act, createRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  ActionSpec,
  EngineStatus,
  OpenOutcome,
  SearchRequest,
  SearchResponse,
  SearchResult,
} from "../bridge/types";
import LauncherSurface, { type LauncherSurfaceHandle } from "./LauncherSurface";
import type { WorkAreaSize } from "./launcherLayout";
import { clearPreviewCache, primePreviewCache } from "./previewClient";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const READY: EngineStatus = {
  protocol: 1,
  version: "test",
  state: "ready",
  filenameEntries: 124318,
  contentDocuments: 14609,
  ocrDocuments: 0,
  roots: [],
  sources: [],
  lastUpdated: 0,
  warnings: [],
};

const WIDE: WorkAreaSize = { width: 1600, height: 900 };

interface DeferredResponse {
  request: SearchRequest;
  resolve: (response: SearchResponse) => void;
  reject: (error: unknown) => void;
}

interface Harness {
  container: HTMLDivElement;
  root: Root;
  pending: DeferredResponse[];
  search: ReturnType<typeof vi.fn>;
  openResult: ReturnType<typeof vi.fn>;
  confirmNativeAction: ReturnType<typeof vi.fn>;
  hideLauncher: ReturnType<typeof vi.fn>;
  showMain: ReturnType<typeof vi.fn>;
  onGeometry: ReturnType<typeof vi.fn>;
  ref: ReturnType<typeof createRef<LauncherSurfaceHandle>>;
}

const disposers: (() => void)[] = [];

afterEach(() => {
  for (const dispose of disposers.splice(0)) dispose();
  document.body.innerHTML = "";
  try {
    window.sessionStorage.clear();
  } catch {
    /* ignore */
  }
  document.documentElement.dataset.motion = "";
});

function result(overrides: Partial<SearchResult> & { id: string; title: string }): SearchResult {
  return {
    kind: "file",
    subtitle: "Documents / Projects",
    icon: { type: "file_type", ext: "txt" },
    score: 1,
    action: { type: "open_path", path: "C:\\docs\\" + overrides.title },
    ...overrides,
  };
}

function response(
  request: SearchRequest,
  results: SearchResult[],
  status?: SearchResponse["status"],
): SearchResponse {
  return {
    protocol: 1,
    requestId: request.requestId,
    query: request.query,
    results,
    total: results.length,
    tookMs: 1,
    timings: [],
    status: status ?? { type: "ready", indexedFiles: 124318, contentDocs: 14609 },
  };
}

function setup(options?: {
  workArea?: WorkAreaSize;
  status?: EngineStatus;
  clipboardHistoryEnabled?: boolean;
}): Harness {
  const container = document.createElement("div");
  document.body.appendChild(container);
  const root = createRoot(container);
  const pending: DeferredResponse[] = [];
  const search = vi.fn((request: SearchRequest) => {
    return new Promise<SearchResponse>((resolve, reject) => {
      pending.push({ request, resolve, reject });
    });
  });
  const openResult = vi.fn(
    async (_action: ActionSpec): Promise<OpenOutcome> => ({ ok: true, message: "ok" }),
  );
  const confirmNativeAction = vi.fn(async () => ({ ok: true, message: "executed" }));
  const hideLauncher = vi.fn();
  const showMain = vi.fn();
  const onGeometry = vi.fn();
  const ref = createRef<LauncherSurfaceHandle>();
  act(() => {
    root.render(
      <LauncherSurface
        ref={ref}
        search={search}
        openResult={openResult}
        confirmNativeAction={confirmNativeAction}
        hideLauncher={hideLauncher}
        showMain={showMain}
        status={options?.status ?? READY}
        clipboardHistoryEnabled={options?.clipboardHistoryEnabled ?? true}
        density="comfortable"
        workArea={options?.workArea ?? WIDE}
        onGeometry={onGeometry}
      />,
    );
  });
  const harness: Harness = {
    container,
    root,
    pending,
    search,
    openResult,
    confirmNativeAction,
    hideLauncher,
    showMain,
    onGeometry,
    ref,
  };
  disposers.push(() => {
    act(() => root.unmount());
    container.remove();
  });
  return harness;
}

async function resolveSearch(
  harness: Harness,
  index: number,
  results: SearchResult[],
  status?: SearchResponse["status"],
): Promise<void> {
  const entry = harness.pending[index];
  if (!entry) throw new Error("No pending search request at index " + index);
  await act(async () => {
    entry.resolve(response(entry.request, results, status));
    await Promise.resolve();
  });
}

async function rejectSearch(harness: Harness, index: number, error: unknown): Promise<void> {
  const entry = harness.pending[index];
  if (!entry) throw new Error("No pending search request at index " + index);
  await act(async () => {
    entry.reject(error);
    await Promise.resolve();
  });
}

function inputOf(harness: Harness): HTMLInputElement {
  const input = harness.container.querySelector<HTMLInputElement>('[data-testid="launcher-input"]');
  if (!input) throw new Error("launcher input missing");
  return input;
}

function setInputValue(harness: Harness, value: string): void {
  const input = inputOf(harness);
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  if (!setter) throw new Error("value setter missing");
  act(() => {
    setter.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function press(element: Element | null, key: string, init: KeyboardEventInit = {}): Promise<void> {
  if (!element) throw new Error("Cannot press " + key + " on a missing element");
  await act(async () => {
    element.dispatchEvent(
      new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...init }),
    );
    await Promise.resolve();
    await Promise.resolve();
  });
}

function selectedId(harness: Harness): string | null {
  const option = harness.container.querySelector('[role="option"][aria-selected="true"]');
  return option?.getAttribute("data-result-id") ?? null;
}

function optionIds(harness: Harness): string[] {
  return [...harness.container.querySelectorAll('[role="option"]')].map(
    (option) => option.getAttribute("data-result-id") ?? "",
  );
}

function hintTexts(harness: Harness): string[] {
  return [...harness.container.querySelectorAll('[data-testid="footer-hint"]')].map((hint) =>
    (hint.textContent ?? "").replace(/\s+/g, " ").trim(),
  );
}

function footerText(harness: Harness): string {
  return harness.container.querySelector('[data-testid="launcher-footer"]')?.textContent ?? "";
}

const THREE = [result({ id: "r1", title: "Alpha.txt" }), result({ id: "r2", title: "Bravo.txt" }), result({ id: "r3", title: "Charlie.txt" })];

describe("LauncherSurface structure and grouping", () => {
  it("renders a frameless dialog with grouped, bounded results and correct ARIA", async () => {
    const harness = setup();
    // First query in flight shows the flat 3-row skeleton (no shimmer).
    expect(harness.container.querySelectorAll(".launcher-skeleton-row").length).toBe(3);
    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe(
      "Searching…",
    );

    const doc = result({
      id: "file:doc",
      title: "Architecture notes.docx",
      kind: "document",
      icon: { type: "file_type", ext: "docx" },
    });
    const app = result({
      id: "app:vsc",
      title: "Visual Studio Code",
      kind: "application",
      icon: { type: "monogram" },
      action: { type: "launch_app", appId: "vscode" },
    });
    const folder = result({
      id: "folder:arch",
      title: "Architecture",
      kind: "folder",
      icon: { type: "file_type", ext: "folder" },
    });
    await resolveSearch(harness, 0, [doc, app, folder]);

    expect(harness.container.querySelector(".launcher-skeleton")).toBeNull();
    const labels = [...harness.container.querySelectorAll(".group-label")].map(
      (label) => label.textContent,
    );
    // lib/results.ts group order: APPLICATIONS before FOLDERS before DOCUMENTS.
    expect(labels).toEqual(["APPLICATIONS", "FOLDERS", "DOCUMENTS"]);

    const options = [...harness.container.querySelectorAll('[role="option"]')];
    expect(options.length).toBe(3);
    expect(options.map((option) => option.getAttribute("aria-label"))).toEqual([
      "Visual Studio Code",
      "Architecture",
      "Architecture notes.docx",
    ]);
    expect(options[0].getAttribute("aria-selected")).toBe("true");
    expect(options[0].getAttribute("aria-describedby")).toContain("ps-desc-app_vsc");

    const dialog = harness.container.querySelector('[role="dialog"]');
    expect(dialog?.getAttribute("aria-label")).toBe("ProtonSearch");
    expect(dialog?.getAttribute("aria-modal")).toBe("true");

    const input = inputOf(harness);
    expect(input.getAttribute("role")).toBe("combobox");
    expect(input.getAttribute("aria-expanded")).toBe("true");
    expect(input.getAttribute("aria-controls")).toBe("ps-results");
    expect(input.getAttribute("aria-activedescendant")).toBe(options[0].id);

    const listbox = harness.container.querySelector('[role="listbox"]');
    expect(listbox?.getAttribute("aria-label")).toBe("Results");
    expect(harness.container.querySelector('[role="group"]')?.getAttribute("role")).toBe("group");

    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe(
      "3 matches",
    );
    const liveRegion = harness.container.querySelector('[role="status"][aria-live="polite"]');
    expect(liveRegion).not.toBeNull();
    expect(liveRegion?.getAttribute("aria-atomic")).toBe("true");

    expect(footerText(harness)).toContain("Index ready");
    expect(footerText(harness)).toContain((124318).toLocaleString() + " items");
    expect(footerText(harness)).toContain("Open ProtonSearch");

    // Deterministic file-type badges + glyph fallbacks (no unicode icons).
    const docOption = harness.container.querySelector('[data-result-id="file:doc"]');
    expect(docOption?.querySelector(".fileicon-text")?.textContent).toBe("DOC");
    const folderOption = harness.container.querySelector('[data-result-id="folder:arch"]');
    expect(folderOption?.querySelector("svg.fileicon-glyph")).not.toBeNull();
    const appOption = harness.container.querySelector('[data-result-id="app:vsc"]');
    expect(appOption?.querySelector(".fileicon-text")?.textContent).toBe("VS");
  });

  it("groups multiple kinds into the fixed shared labels from lib/results.ts", async () => {
    const harness = setup();
    const mixed: SearchResult[] = [
      result({ id: "recent:1", title: "Recent thing", kind: "recent" }),
      result({ id: "content:1", title: "Content match", kind: "content_match" }),
      result({ id: "doc:1", title: "Document", kind: "document" }),
      result({ id: "web:1", title: "Visited page", kind: "browser_history" }),
      result({ id: "git:1", title: "Commit", kind: "git_commit" }),
      result({ id: "calc:1", title: "2 + 2 = 4", kind: "calculator" }),
    ];
    await resolveSearch(harness, 0, mixed);
    const labels = [...harness.container.querySelectorAll(".group-label")].map(
      (label) => label.textContent,
    );
    // RECENT · APPLICATIONS · FILES · FOLDERS · DOCUMENTS · CLIPBOARD · WEB · CODE · ACTIONS
    expect(labels).toEqual(["RECENT", "DOCUMENTS", "WEB", "CODE", "ACTIONS"]);
    const documentsGroup = [...harness.container.querySelectorAll('[data-testid="result-group"]')].find(
      (group) => group.textContent?.includes("DOCUMENTS"),
    );
    expect(documentsGroup?.querySelectorAll('[role="option"]').length).toBe(2);
    expect(harness.container.querySelectorAll('[role="option"]').length).toBe(6);
  });

  it("shows quick destinations for empty input and the precise 0-match copy", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, []);
    // Empty query with no recents is the quick-destinations state.
    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe("");
    const quick = harness.container.querySelector('[data-testid="quick-destinations"]');
    expect(quick).not.toBeNull();
    expect(quick?.textContent).toContain("Browser Bookmarks");
    expect(quick?.textContent).toContain("Clipboard History");
    expect(quick?.textContent).toContain("Agents");

    setInputValue(harness, "zzz");
    await resolveSearch(harness, 1, []);
    const emptyAfter = harness.container.querySelector('[data-testid="empty-state"]');
    expect(emptyAfter?.textContent).toContain("No matches for “zzz”.");
    expect(emptyAfter?.textContent).toContain(
      "Try a different spelling, or a prefix such as file:, history:, or code:.",
    );
    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe(
      "No matches",
    );
  });
});

describe("keyboard model", () => {
  it("wraps ↑/↓, clamps PageUp/PageDown and supports Ctrl+Home/End", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const input = inputOf(harness);
    expect(selectedId(harness)).toBe("r1");

    await press(input, "ArrowUp");
    expect(selectedId(harness)).toBe("r3");
    const activeOption = harness.container.querySelector('[role="option"][aria-selected="true"]');
    expect(input.getAttribute("aria-activedescendant")).toBe(activeOption?.id);
    await press(input, "ArrowDown");
    expect(selectedId(harness)).toBe("r1");
    await press(input, "ArrowDown");
    expect(selectedId(harness)).toBe("r2");
    await press(input, "PageDown");
    expect(selectedId(harness)).toBe("r3");
    await press(input, "Home");
    expect(selectedId(harness)).toBe("r3"); // native caret key, not list navigation
    await press(input, "Home", { ctrlKey: true });
    expect(selectedId(harness)).toBe("r1");
    await press(input, "End", { ctrlKey: true });
    expect(selectedId(harness)).toBe("r3");
    await press(input, "ArrowDown");
    expect(selectedId(harness)).toBe("r1");
  });

  it("cycles focus input → footer → input with Tab/Shift+Tab", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const input = inputOf(harness);
    expect(document.activeElement).toBe(input);

    await press(input, "Tab", { shiftKey: true });
    const footerButton = harness.container.querySelector(".footlink");
    expect(document.activeElement).toBe(footerButton);

    await press(footerButton, "Tab");
    expect(document.activeElement).toBe(input);
  });

  it("uses Esc chain: preview → clear query → hide", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    setInputValue(harness, "alpha");
    await resolveSearch(harness, 1, THREE);
    const input = inputOf(harness);
    expect(input.value).toBe("alpha");

    await press(input, " ", { ctrlKey: true });
    expect(harness.container.querySelector('[data-testid="preview-panel"]')).not.toBeNull();

    await press(input, "Escape");
    expect(harness.container.querySelector('[data-testid="preview-panel"]')).toBeNull();
    expect(document.activeElement).toBe(input);
    expect(input.value).toBe("alpha");
    expect(harness.hideLauncher).not.toHaveBeenCalled();

    await press(input, "Escape");
    expect(input.value).toBe("");
    expect(harness.pending.length).toBe(3); // cleared query re-searched
    expect(harness.hideLauncher).not.toHaveBeenCalled();

    await press(input, "Escape");
    // Empty query -> dismiss animation (110 ms) before the window hides.
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 150));
    });
    expect(harness.hideLauncher).toHaveBeenCalledTimes(1);
  });
});

describe("stale response protection", () => {
  it("drops out-of-order responses and never opens a row from a stale query", async () => {
    const harness = setup();
    const oldResult = result({ id: "old", title: "Old.txt" });
    const newResult = result({ id: "new", title: "New.txt" });
    const freshResult = result({ id: "fresh", title: "Fresh.txt" });

    setInputValue(harness, "ar");
    setInputValue(harness, "arc");
    expect(harness.pending.length).toBe(3);

    await resolveSearch(harness, 2, [newResult]);
    expect(optionIds(harness)).toEqual(["new"]);

    await resolveSearch(harness, 1, [oldResult]); // late stale response
    expect(optionIds(harness)).toEqual(["new"]);

    setInputValue(harness, "arch");
    expect(harness.pending.length).toBe(4);

    // New query in flight: the committed set belongs to "arc", so Enter is a no-op.
    await press(inputOf(harness), "Enter");
    expect(harness.openResult).not.toHaveBeenCalled();
    expect(harness.container.textContent).toContain("Result changed — choose again.");

    await resolveSearch(harness, 3, [freshResult]);
    await press(inputOf(harness), "Enter");
    expect(harness.openResult).toHaveBeenCalledWith(freshResult.action);
    expect(harness.hideLauncher).toHaveBeenCalled();
  });

  it("resets selection to row 0 when the query epoch changes", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    await press(inputOf(harness), "ArrowDown");
    expect(selectedId(harness)).toBe("r2");

    setInputValue(harness, "x");
    await resolveSearch(harness, 1, [
      result({ id: "r3", title: "Charlie.txt" }),
      result({ id: "r1", title: "Alpha.txt" }),
      result({ id: "r2", title: "Bravo.txt" }),
    ]);
    expect(selectedId(harness)).toBe("r3");
  });
});

describe("search errors and copy shortcuts", () => {
  it("stays usable and honest after a failed search, with Retry", async () => {
    const harness = setup();
    await rejectSearch(harness, 0, new Error("engine offline"));
    const error = harness.container.querySelector('[data-testid="error-state"]');
    expect(error?.textContent).toContain("Search is unavailable right now.");
    expect(error?.textContent).toContain("engine offline");
    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe(
      "Search unavailable",
    );
    expect(footerText(harness)).toContain("Index unavailable");
    expect(inputOf(harness).disabled).toBe(false);
    await press(inputOf(harness), "Enter");
    expect(harness.openResult).not.toHaveBeenCalled();

    const retry = harness.container.querySelector<HTMLButtonElement>(".inline-retry");
    await act(async () => {
      retry?.click();
      await Promise.resolve();
    });
    expect(harness.pending.length).toBe(2);
    expect(harness.pending[1].request.query).toBe("");
    await resolveSearch(harness, 1, [result({ id: "back", title: "Back.txt" })]);
    expect(optionIds(harness)).toEqual(["back"]);
    expect(harness.container.querySelector('[data-testid="error-state"]')).toBeNull();
  });

  it("copies the selected path with Ctrl+C and announces it", async () => {
    const harness = setup();
    const selected = result({ id: "copy", title: "Copy Me.txt", path: "C:\\docs\\copy-me.txt" });
    await resolveSearch(harness, 0, [selected]);
    const input = inputOf(harness);

    await press(input, "c", { ctrlKey: true });
    expect(harness.openResult).toHaveBeenCalledWith({
      type: "copy_text",
      text: "C:\\docs\\copy-me.txt",
    });
    expect(harness.hideLauncher).not.toHaveBeenCalled();
    expect(footerText(harness)).toContain("Path copied");

    // A native text selection keeps native copy behaviour.
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
    setter?.call(input, "abc");
    input.setSelectionRange(0, 2);
    const calls = harness.openResult.mock.calls.length;
    await press(input, "c", { ctrlKey: true });
    expect(harness.openResult.mock.calls.length).toBe(calls);
  });
});

describe("footer hints", () => {
  it("shows Navigate/Open/Close when empty and Clear once a query is typed", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    expect(hintTexts(harness)).toEqual(["↑ ↓ Navigate", "Enter Open", "Esc Close"]);

    setInputValue(harness, "a");
    expect(hintTexts(harness)).toEqual(["↑ ↓ Navigate", "Enter Open", "Esc Clear"]);
  });

  it("keeps at most three priority hints and marks Navigate as the narrow-surface one", async () => {
    const harness = setup({ workArea: { width: 800, height: 600 } });
    await resolveSearch(harness, 0, THREE);
    expect(hintTexts(harness)).toEqual(["↑ ↓ Navigate", "Enter Open", "Esc Close"]);
    expect(harness.container.querySelectorAll('[data-testid="footer-hint"]').length).toBe(3);
  });
});

describe("preview pane", () => {
  const previewResult = result({
    id: "content:1",
    title: "search-architecture.md",
    kind: "content_match",
    path: "C:\\docs\\search-architecture.md",
    meta: {
      label: "line 42",
      size: 184320,
      modified: Date.now() - 86400000,
      line: 42,
      snippet: "The filename arena keeps warm queries under a millisecond.",
    },
    actions: [
      {
        id: "reveal",
        label: "Reveal in Explorer",
        action: { type: "reveal_path", path: "C:\\docs\\search-architecture.md" },
      },
      {
        id: "copy",
        label: "Copy path",
        action: { type: "copy_text", text: "C:\\docs\\search-architecture.md" },
      },
    ],
  });

  const previewPayload = {
    resultId: previewResult.id,
    title: "search-architecture.md",
    subtitle: "Projects / ProtonSearch / docs",
    format: "markdown",
    text: null,
    snippet: "The filename arena keeps warm queries under a millisecond.",
    highlightRanges: [{ start: 4, end: 22 }],
    imageDataUrl: null,
    ocrText: null,
    fields: [
      { label: "Size", value: "180 KB" },
      { label: "Modified", value: "Yesterday" },
    ],
    status: "ok" as const,
    truncated: false,
    error: null,
    imageMime: null,
    unsupportedReason: null,
  };

  beforeEach(() => {
    clearPreviewCache();
    primePreviewCache(previewResult.id, previewPayload);
  });

  it("toggles with Ctrl+Space and reports geometry to the shell", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [previewResult, result({ id: "r2", title: "Other.txt" })]);
    const input = inputOf(harness);
    // FILES group sorts before IN DOCUMENTS; move to the content match.
    await press(input, "ArrowDown");

    await press(input, " ", { ctrlKey: true });
    const panel = harness.container.querySelector('[data-testid="preview-panel"]');
    expect(panel?.textContent).toContain("search-architecture.md");
    expect(panel?.textContent).toContain("C:\\docs\\search-architecture.md");
    expect(panel?.textContent).toContain("The filename arena keeps warm queries under a millisecond.");
    expect(panel?.textContent).toContain("180 KB");
    expect(hintTexts(harness)).toEqual(["Ctrl+Space Close preview", "Esc Close"]);
    // Geometry: search 72 + filter strip 40 + padding 23 + (2 labels 68 + 2 rows 116)
    // + footer 44 + border 2 = 365 -> snapped 366.
    expect(harness.onGeometry).toHaveBeenLastCalledWith(366, true);

    await press(input, " ", { ctrlKey: true });
    expect(harness.container.querySelector('[data-testid="preview-panel"]')).toBeNull();
    expect(harness.onGeometry).toHaveBeenLastCalledWith(366, false);
  });

  it("grows the window for a bottom drawer on narrow work areas", async () => {
    const harness = setup({ workArea: { width: 1024, height: 768 } });
    await resolveSearch(harness, 0, [previewResult]);
    await press(inputOf(harness), " ", { ctrlKey: true });
    // 72 + 40 + 23 + (34 + 58) + 44 + 2 = 273 -> floor 296; drawer +240 = 536.
    expect(harness.onGeometry).toHaveBeenLastCalledWith(536, true);
  });

  it("runs declared preview actions without hiding the window for copy actions", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [previewResult]);
    await press(inputOf(harness), " ", { ctrlKey: true });
    const buttons = [...harness.container.querySelectorAll(".preview-action")].map(
      (button) => button.textContent,
    );
    expect(buttons).toEqual(["Open", "Reveal in Explorer", "Copy path"]);

    const copy = [...harness.container.querySelectorAll<HTMLButtonElement>(".preview-action")].find(
      (button) => button.textContent === "Copy path",
    );
    await act(async () => {
      copy?.click();
      await Promise.resolve();
    });
    expect(harness.openResult).toHaveBeenCalledWith({
      type: "copy_text",
      text: "C:\\docs\\search-architecture.md",
    });
    expect(harness.hideLauncher).not.toHaveBeenCalled();
  });

  it("renders bounded images and OCR text from the real payload", async () => {
    const imageResult = result({
      id: "image:diagram",
      title: "diagram.png",
      kind: "file",
      icon: { type: "file_type", ext: "png" },
      action: { type: "open_path", path: "C:\\Pictures\\diagram.png" },
    });
    primePreviewCache(imageResult.id, {
      ...previewPayload,
      resultId: imageResult.id,
      title: "diagram.png",
      format: "image",
      text: null,
      snippet: null,
      highlightRanges: [],
      imageDataUrl:
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2n0kAAAAASUVORK5CYII=",
      ocrText: "Quarterly launch checklist",
      fields: [{ label: "Dimensions", value: "1280 × 720" }],
    });
    const harness = setup();
    await resolveSearch(harness, 0, [imageResult]);
    await press(inputOf(harness), " ", { ctrlKey: true });
    const panel = harness.container.querySelector('[data-testid="preview-panel"]');
    expect(panel?.querySelector(".preview-image")?.getAttribute("src")).toContain("data:image/png");
    expect(panel?.querySelector('[data-testid="preview-ocr"]')?.textContent).toContain(
      "Quarterly launch checklist",
    );
  });

  it("renders highlighted extracted text and truncation", async () => {
    const textResult = result({
      id: "file:long",
      title: "large.txt",
      kind: "file",
      icon: { type: "file_type", ext: "txt" },
      action: { type: "open_path", path: "C:\\docs\\large.txt" },
    });
    primePreviewCache(textResult.id, {
      ...previewPayload,
      resultId: textResult.id,
      title: "large.txt",
      format: "text",
      text: "The filename arena keeps warm queries under a millisecond.",
      snippet: null,
      highlightRanges: [{ start: 4, end: 22 }],
      truncated: true,
    });
    const harness = setup();
    await resolveSearch(harness, 0, [textResult]);
    await press(inputOf(harness), " ", { ctrlKey: true });
    const panel = harness.container.querySelector('[data-testid="preview-panel"]');
    expect(panel?.querySelector("mark.preview-highlight")?.textContent).toBe("filename arena kee");
    expect(panel?.querySelector('[data-testid="preview-truncated"]')).not.toBeNull();
  });

  it("shows unsupported and error fallbacks instead of a blank panel", async () => {
    const unsupportedResult = result({
      id: "file:blob",
      title: "blob.bin",
      kind: "file",
      icon: { type: "file_type", ext: "bin" },
      action: { type: "open_path", path: "C:\\docs\\blob.bin" },
    });
    primePreviewCache(unsupportedResult.id, {
      ...previewPayload,
      resultId: unsupportedResult.id,
      title: "blob.bin",
      format: "binary",
      text: null,
      snippet: null,
      highlightRanges: [],
      fields: [],
      status: "unsupported",
      unsupportedReason: "No preview for this item.",
    });
    const harness = setup();
    await resolveSearch(harness, 0, [unsupportedResult]);
    await press(inputOf(harness), " ", { ctrlKey: true });
    expect(
      harness.container.querySelector('[data-testid="preview-unsupported"]')?.textContent,
    ).toContain("No preview for this item.");

    const errorResult = result({
      id: "file:broken",
      title: "broken.pdf",
      kind: "file",
      icon: { type: "file_type", ext: "pdf" },
      action: { type: "open_path", path: "C:\\docs\\broken.pdf" },
    });
    primePreviewCache(errorResult.id, {
      ...previewPayload,
      resultId: errorResult.id,
      title: "broken.pdf",
      format: "pdf",
      text: null,
      snippet: null,
      highlightRanges: [],
      fields: [],
      status: "error",
      error: "Preview extraction failed.",
    });
    const errorHarness = setup();
    await resolveSearch(errorHarness, 0, [errorResult]);
    await press(inputOf(errorHarness), " ", { ctrlKey: true });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    expect(
      errorHarness.container.querySelector('[data-testid="preview-error"]')?.textContent,
    ).toContain("Preview is unavailable right now.");
  });
});

describe("destructive native command confirmation (V2-04)", () => {
  const shutdown = result({
    id: "action:shutdown",
    title: "Shut down",
    kind: "action",
    icon: { type: "kind", kind: "action" },
    action: { type: "run_native", command: "shutdown" },
  });

  async function openConfirmBar(harness: Harness): Promise<void> {
    harness.openResult.mockResolvedValueOnce({
      ok: false,
      message: "Confirmation required for Shutdown. Ask again to execute.",
    });
    await resolveSearch(harness, 0, [shutdown]);
    await press(inputOf(harness), "Enter");
  }

  it("shows the inline confirmation bar instead of hiding or executing", async () => {
    const harness = setup();
    await openConfirmBar(harness);
    const bar = harness.container.querySelector('[data-testid="confirm-bar"]');
    expect(bar).not.toBeNull();
    expect(bar?.textContent).toContain("Confirmation required for Shutdown");
    expect(bar?.getAttribute("role")).toBe("alertdialog");
    expect(harness.openResult).toHaveBeenCalledWith({ type: "run_native", command: "shutdown" });
    expect(harness.confirmNativeAction).not.toHaveBeenCalled();
    expect(harness.hideLauncher).not.toHaveBeenCalled();
    // Safe default focus is Cancel; the result selection is preserved.
    expect(document.activeElement?.textContent).toBe("Cancel");
    expect(selectedId(harness)).toBe("action:shutdown");
  });

  it("cancels with Esc, keeps the selection and returns focus to the input", async () => {
    const harness = setup();
    await openConfirmBar(harness);
    await press(document.activeElement, "Escape");
    expect(harness.container.querySelector('[data-testid="confirm-bar"]')).toBeNull();
    expect(harness.confirmNativeAction).not.toHaveBeenCalled();
    expect(harness.hideLauncher).not.toHaveBeenCalled();
    expect(selectedId(harness)).toBe("action:shutdown");
    expect(document.activeElement).toBe(inputOf(harness));
  });

  it("runs the action only after the explicit confirm button", async () => {
    const harness = setup();
    await openConfirmBar(harness);
    const runButton = harness.container.querySelector<HTMLButtonElement>(".confirm-run");
    await act(async () => {
      runButton?.click();
      await Promise.resolve();
    });
    expect(harness.confirmNativeAction).toHaveBeenCalledWith("shutdown");
    expect(harness.container.querySelector('[data-testid="confirm-bar"]')).toBeNull();
    expect(harness.hideLauncher).toHaveBeenCalledTimes(1);
  });

  it("keeps non-destructive Enter unchanged (no confirmation bar)", async () => {
    const harness = setup();
    const plain = result({ id: "plain", title: "Plain.txt", action: { type: "open_path", path: "C:\\docs\\plain.txt" } });
    await resolveSearch(harness, 0, [plain]);
    await press(inputOf(harness), "Enter");
    expect(harness.container.querySelector('[data-testid="confirm-bar"]')).toBeNull();
    expect(harness.openResult).toHaveBeenCalledWith({ type: "open_path", path: "C:\\docs\\plain.txt" });
    expect(harness.confirmNativeAction).not.toHaveBeenCalled();
    expect(harness.hideLauncher).toHaveBeenCalledTimes(1);
  });
});

describe("context menu", () => {
  const withActions = result({
    id: "menu:1",
    title: "Menu.txt",
    path: "C:\\docs\\menu.txt",
    actions: [
      {
        id: "reveal",
        label: "Reveal in Explorer",
        action: { type: "reveal_path", path: "C:\\docs\\menu.txt" },
      },
      {
        id: "copy",
        label: "Copy path",
        action: { type: "copy_text", text: "C:\\docs\\menu.txt" },
      },
      {
        id: "delete",
        label: "Delete",
        destructive: true,
        action: { type: "none" },
      },
    ],
  });

  it("opens on Shift+F10, lists only declared safe actions, and runs one", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [withActions]);
    const input = inputOf(harness);

    await press(input, "F10", { shiftKey: true });
    const menu = harness.container.querySelector('[data-testid="actions-menu"]');
    expect(menu).not.toBeNull();
    const labels = [...harness.container.querySelectorAll('[role="menuitem"]')].map(
      (item) => item.textContent,
    );
    expect(labels).toEqual(["Open", "Reveal in Explorer", "Copy path"]);
    expect(menu?.textContent).not.toContain("Delete");
    expect(document.activeElement).toBe(
      harness.container.querySelector('[role="menuitem"]'),
    );

    await press(document.activeElement, "ArrowDown");
    await press(document.activeElement, "Enter");
    expect(harness.openResult).toHaveBeenCalledWith({
      type: "reveal_path",
      path: "C:\\docs\\menu.txt",
    });
    expect(harness.hideLauncher).toHaveBeenCalled();
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).toBeNull();
  });

  it("closes with Esc and returns focus to the input; Tab also closes", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [withActions]);
    const input = inputOf(harness);

    await press(input, "Enter", { ctrlKey: true });
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).not.toBeNull();
    await press(document.activeElement, "Escape");
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).toBeNull();
    expect(document.activeElement).toBe(input);

    await press(input, "F10", { shiftKey: true });
    await press(document.activeElement, "Tab");
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).toBeNull();
    expect(document.activeElement).toBe(input);
  });

  it("opens from a row right-click and never from empty space", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [withActions]);
    const results = harness.container.querySelector(".results");
    await act(async () => {
      results?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await Promise.resolve();
    });
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).toBeNull();

    const option = harness.container.querySelector('[data-result-id="menu:1"]');
    await act(async () => {
      option?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await Promise.resolve();
    });
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).not.toBeNull();
  });
});

describe("window, density and bounded rendering", () => {
  it("renders at most 50 result rows for a large response", async () => {
    const harness = setup();
    const many = Array.from({ length: 80 }, (_, index) =>
      result({ id: "many-" + index, title: "Item " + index }),
    );
    await resolveSearch(harness, 0, many);
    expect(harness.container.querySelectorAll('[role="option"]').length).toBe(50);
    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe(
      "80 matches",
    );
  });

  it("applies 20 rapid arrow presses in order without queued lag", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const input = inputOf(harness);
    await act(async () => {
      for (let index = 0; index < 20; index += 1) {
        input.dispatchEvent(
          new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true, cancelable: true }),
        );
      }
      await Promise.resolve();
    });
    expect(selectedId(harness)).toBe("r3"); // (0 + 20) % 3 = index 2
  });

  it("resets to an empty, focused query when the window is summoned", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    setInputValue(harness, "abc");
    await resolveSearch(harness, 1, THREE);
    const input = inputOf(harness);
    await press(input, "ArrowDown");

    const before = harness.pending.length;
    await act(async () => {
      harness.ref.current?.summon();
      await Promise.resolve();
    });
    expect(harness.pending.length).toBe(before + 1);
    expect(harness.pending[before].request.query).toBe("");
    expect(input.value).toBe("");
    expect(document.activeElement).toBe(input);

    await resolveSearch(harness, before, THREE);
    expect(selectedId(harness)).toBe("r1");
  });

  it("opens the desktop app from the footer button", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const button = harness.container.querySelector<HTMLButtonElement>(".footlink");
    await act(async () => {
      button?.click();
      await Promise.resolve();
    });
    expect(harness.showMain).toHaveBeenCalledTimes(1);
  });

  it("falls back to the EngineStatus item count when the search response omits counts", async () => {
    const harness = setup({ status: { ...READY, filenameEntries: 777 } });
    await resolveSearch(harness, 0, THREE, {
      type: "ready",
      indexedFiles: undefined as unknown as number,
      contentDocs: 0,
    });
    expect(footerText(harness)).toContain((777).toLocaleString() + " items");
  });

  it("renders extension badges instead of generic glyphs for file results", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [
      result({ id: "md", title: "notes.md", kind: "document", icon: { type: "file_type", ext: "md" } }),
      result({ id: "js", title: "app.js", kind: "file", icon: { type: "file_type", ext: "js" } }),
    ]);
    // FILES group renders before DOCUMENTS; both use extension badges.
    const badges = [...harness.container.querySelectorAll('[role="option"] .fileicon-text')].map(
      (badge) => badge.textContent,
    );
    expect(badges).toEqual(["JS", "MD"]);
  });

  it("reports truthful index states in the footer", async () => {
    // The search response is fresher than the boot-time EngineStatus.
    const indexing = setup({ status: { ...READY, state: "ready" } });
    await resolveSearch(indexing, 0, THREE, {
      type: "indexing",
      indexedFiles: 42018,
      phase: "content",
    });
    expect(footerText(indexing)).toContain("Indexing…");
    expect(
      indexing.container.querySelector('[data-testid="status-dot"]')?.className,
    ).toContain("status-dot--warning");

    const readyAfterIndexing = setup({ status: { ...READY, state: "indexing" } });
    await resolveSearch(readyAfterIndexing, 0, THREE);
    expect(footerText(readyAfterIndexing)).toContain("Index ready");

    // Before the first response the boot snapshot is the only source.
    const paused = setup({ status: { ...READY, state: "paused" } });
    expect(footerText(paused)).toContain("Index paused");
    expect(
      paused.container.querySelector('[data-testid="status-dot"]')?.className,
    ).toContain("status-dot--neutral");

    const broken = setup({ status: { ...READY, state: "error", warnings: ["Drive offline"] } });
    expect(footerText(broken)).toContain("Index unavailable");
    expect(
      broken.container.querySelector('[data-testid="status-dot"]')?.className,
    ).toContain("status-dot--negative");
  });
});

describe("menu key and footer", () => {
  it("opens the context menu with the Windows Menu key", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    await press(inputOf(harness), "ContextMenu");
    expect(harness.container.querySelector('[data-testid="actions-menu"]')).not.toBeNull();
  });

  it("never renders an action-styled focus target outside the footer", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const outsideFooter = [...harness.container.querySelectorAll("button")].filter(
      (button) => !button.closest('[data-testid="launcher-footer"]'),
    );
    // Only the error Retry button, filter chips and confirm buttons may live outside
    // the footer; result rows are options, not stray buttons.
    expect(
      outsideFooter.every(
        (button) =>
          button.classList.contains("inline-retry") ||
          button.classList.contains("filter-chip") ||
          button.classList.contains("confirm-button"),
      ),
    ).toBe(true);
  });
});

describe("category filter strip", () => {
  const chip = (harness: Harness, label: string): HTMLButtonElement | null =>
    [...harness.container.querySelectorAll<HTMLButtonElement>('[data-testid="filter-chip"]')].find(
      (element) => element.textContent === label,
    ) ?? null;

  it("renders four chips, defaults to All and sends scope on the initial search", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const chips = [...harness.container.querySelectorAll('[data-testid="filter-chip"]')];
    expect(chips.length).toBe(4);
    expect(chips.map((element) => element.textContent)).toEqual([
      "ALL",
      "FILES",
      "FOLDERS",
      "APPS",
    ]);
    expect(chips[0].getAttribute("aria-checked")).toBe("true");
    expect(chips[3].getAttribute("aria-keyshortcuts")).toBe("Control+4");
    expect(
      harness.container.querySelector('[role="radiogroup"][aria-label="Filter results"]'),
    ).not.toBeNull();
    expect(harness.pending[0].request.scope).toBe("all");
  });

  it("clicking a chip searches with that scope, keeps input focus and resets selection", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    await press(inputOf(harness), "ArrowDown");
    expect(selectedId(harness)).toBe("r2");

    const files = chip(harness, "FILES");
    await act(async () => {
      files?.click();
      await Promise.resolve();
    });
    expect(harness.pending[1].request.scope).toBe("files");
    expect(harness.pending[1].request.requestId).toBeGreaterThan(harness.pending[0].request.requestId);
    expect(document.activeElement).toBe(inputOf(harness));

    await resolveSearch(harness, 1, [
      result({ id: "f1", title: "First file" }),
      result({ id: "f2", title: "Second file" }),
    ]);
    expect(selectedId(harness)).toBe("f1");
    expect(chip(harness, "FILES")?.getAttribute("aria-checked")).toBe("true");
    expect(chip(harness, "ALL")?.getAttribute("aria-checked")).toBe("false");
  });

  it("supports Ctrl+1..4 from the input without moving focus", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const input = inputOf(harness);

    await press(input, "4", { ctrlKey: true });
    expect(harness.pending[1].request.scope).toBe("apps");
    expect(document.activeElement).toBe(input);
    await resolveSearch(harness, 1, THREE);
    expect(chip(harness, "APPS")?.getAttribute("aria-checked")).toBe("true");

    // Ctrl+5 is unbound: no request and no text insertion.
    await press(input, "5", { ctrlKey: true });
    expect(harness.pending.length).toBe(2);

    await press(input, "1", { ctrlKey: true });
    expect(harness.pending[2].request.scope).toBe("all");
    expect(document.activeElement).toBe(input);
  });

  it("always starts on ALL: the filter choice is not persisted", async () => {
    window.sessionStorage.setItem("protonsearch.launcher.scope", "git");
    const harness = setup();
    expect(harness.pending[0].request.scope).toBe("all");
    expect(chip(harness, "ALL")?.getAttribute("aria-checked")).toBe("true");
    await resolveSearch(harness, 0, THREE);
    expect(chip(harness, "ALL")?.getAttribute("aria-checked")).toBe("true");
  });

  it("keeps the removed content/git scopes reachable only through query prefixes", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const labels = [...harness.container.querySelectorAll('[data-testid="filter-chip"]')].map(
      (element) => element.textContent,
    );
    expect(labels).not.toContain("DOCUMENTS");
    expect(labels).not.toContain("GIT");
    expect(harness.pending[0].request.query).toBe("");
  });

  it("keeps the strip horizontally scrollable without wrapping chips", async () => {
    const harness = setup({ workArea: { width: 560, height: 640 } });
    await resolveSearch(harness, 0, THREE);
    const strip = harness.container.querySelector<HTMLDivElement>('[data-testid="filter-strip"]');
    expect(strip).not.toBeNull();
    expect(strip?.getAttribute("role")).toBe("radiogroup");
    // Chips are flex-none buttons: the strip scrolls instead of wrapping.
    for (const element of harness.container.querySelectorAll('[data-testid="filter-chip"]')) {
      expect(element.className).toContain("filter-chip");
    }
  });
});

describe("item count truthfulness and dismiss motion", () => {
  it("never shows 0 items while EngineStatus has entries", async () => {
    const harness = setup({ status: { ...READY, filenameEntries: 777 } });
    await resolveSearch(harness, 0, THREE, {
      type: "ready",
      indexedFiles: 0,
      contentDocs: 0,
    });
    expect(footerText(harness)).toContain((777).toLocaleString() + " items");
    expect(footerText(harness)).not.toContain("0 items");
  });

  it("dismisses immediately under reduced motion and lets the shell hide the window", async () => {
    document.documentElement.dataset.motion = "reduced";
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    await act(async () => {
      harness.ref.current?.dismiss();
      await Promise.resolve();
    });
    expect(harness.hideLauncher).toHaveBeenCalledTimes(1);
    expect(
      harness.container.querySelector('[data-testid="launcher"]')?.getAttribute("data-phase"),
    ).toBe("closed");
  });
});

describe("real app icons (IconRef app_icon_data)", () => {
  const pngData =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

  it("renders app_icon_data as an img inside the fixed tile", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [
      result({
        id: "app:code",
        title: "Visual Studio Code",
        kind: "application",
        icon: { type: "app_icon_data", data: pngData },
        action: { type: "launch_app", appId: "code" },
      }),
    ]);
    const img = harness.container.querySelector<HTMLImageElement>('[role="option"] img');
    expect(img).not.toBeNull();
    expect(img?.getAttribute("src")).toBe(pngData);
    expect(img?.getAttribute("alt")).toBe("");
    expect(img?.getAttribute("data-icon-source")).toBe("data");
    expect(img?.getAttribute("width")).toBe("28");
  });

  it("falls back to the monogram badge for unsafe/non-image data URLs", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [
      result({
        id: "app:unsafe",
        title: "Visual Studio Code",
        kind: "application",
        icon: { type: "app_icon_data", data: "data:text/html;base64," + "A".repeat(80) },
        action: { type: "launch_app", appId: "code" },
      }),
    ]);
    expect(harness.container.querySelector('[role="option"] img')).toBeNull();
    const badge = harness.container.querySelector('[role="option"] .fileicon-text');
    expect(badge?.textContent).toBe("VS");
  });

  it("removes a broken image and shows the monogram fallback (no broken glyph)", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [
      result({
        id: "app:broken",
        title: "Notepad",
        kind: "application",
        icon: { type: "app_icon_data", data: pngData },
        action: { type: "launch_app", appId: "notepad" },
      }),
    ]);
    const img = harness.container.querySelector<HTMLImageElement>('[role="option"] img');
    expect(img).not.toBeNull();
    await act(async () => {
      img?.dispatchEvent(new Event("error", { bubbles: true }));
      await Promise.resolve();
    });
    expect(harness.container.querySelector('[role="option"] img')).toBeNull();
    expect(harness.container.querySelector('[role="option"] .fileicon-text')?.textContent).toBe("NO");
  });
});

describe("dismiss handshake cancellation", () => {
  it("cancels a pending close animation when the launcher is re-summoned", async () => {
    document.documentElement.dataset.motion = "full";
    const harness = setup();
    await resolveSearch(harness, 0, THREE);

    await act(async () => {
      harness.ref.current?.dismiss();
      await Promise.resolve();
    });
    expect(
      harness.container.querySelector('[data-testid="launcher"]')?.getAttribute("data-phase"),
    ).toBe("closing");

    await act(async () => {
      harness.ref.current?.summon();
      await Promise.resolve();
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 180));
    });
    expect(harness.hideLauncher).not.toHaveBeenCalled();
    const phase = harness.container
      .querySelector('[data-testid="launcher"]')
      ?.getAttribute("data-phase");
    expect(phase === "visible" || phase === "entering").toBe(true);
  });
});

describe("FILES vs FOLDERS strict rendering", () => {
  const fileRow = result({
    id: "file:report",
    title: "report.txt",
    kind: "file",
    subtitle: "Documents / Reports",
    icon: { type: "file_type", ext: "txt" },
  });
  const folderRow = result({
    id: "folder:projects",
    title: "Projects",
    kind: "folder",
    subtitle: "Documents / Projects",
    icon: { type: "file_type", ext: "folder" },
  });

  it("renders separate FILES and FOLDERS groups with the right row anatomy in ALL", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, [fileRow, folderRow]);
    const labels = [...harness.container.querySelectorAll(".group-label")].map(
      (label) => label.textContent,
    );
    expect(labels).toEqual(["FILES", "FOLDERS"]);
    const fileOption = harness.container.querySelector('[data-result-id="file:report"]');
    expect(fileOption?.querySelector(".fileicon-text")?.textContent).toBe("TXT");
    const folderOption = harness.container.querySelector('[data-result-id="folder:projects"]');
    expect(folderOption?.querySelector("svg.fileicon-glyph")).not.toBeNull();
    expect(folderOption?.getAttribute("aria-label")).toBe("Projects");
  });

  it("renders FILES-only and FOLDERS-only responses strictly", async () => {
    const filesOnly = setup();
    await resolveSearch(filesOnly, 0, [fileRow]);
    expect(
      [...filesOnly.container.querySelectorAll(".group-label")].map((label) => label.textContent),
    ).toEqual(["FILES"]);
    expect(filesOnly.container.querySelectorAll('[role="option"]').length).toBe(1);
    expect(
      [...filesOnly.container.querySelectorAll(".group-label")].map((label) => label.textContent),
    ).not.toContain("FOLDERS");

    const foldersOnly = setup();
    await resolveSearch(foldersOnly, 0, [folderRow]);
    expect(
      [...foldersOnly.container.querySelectorAll(".group-label")].map((label) => label.textContent),
    ).toEqual(["FOLDERS"]);
    expect(foldersOnly.container.querySelector('[role="option"] svg.fileicon-glyph')).not.toBeNull();
  });

  it("sends strict files/folders scopes and keeps single-kind rendering", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const filesChip = [...harness.container.querySelectorAll<HTMLButtonElement>('[data-testid="filter-chip"]')].find(
      (element) => element.textContent === "FILES",
    );
    await act(async () => {
      filesChip?.click();
      await Promise.resolve();
    });
    expect(harness.pending[1].request.scope).toBe("files");
    await resolveSearch(harness, 1, [fileRow]);
    expect(
      [...harness.container.querySelectorAll(".group-label")].map((label) => label.textContent),
    ).toEqual(["FILES"]);

    const foldersChip = [...harness.container.querySelectorAll<HTMLButtonElement>('[data-testid="filter-chip"]')].find(
      (element) => element.textContent === "FOLDERS",
    );
    await act(async () => {
      foldersChip?.click();
      await Promise.resolve();
    });
    expect(harness.pending[2].request.scope).toBe("folders");
    await resolveSearch(harness, 2, [folderRow]);
    expect(
      [...harness.container.querySelectorAll(".group-label")].map((label) => label.textContent),
    ).toEqual(["FOLDERS"]);
  });
});

describe("clipboard grouped sections", () => {
  const clip = (id: string, label: string | null, title: string): SearchResult =>
    result({
      id,
      title,
      kind: "clipboard",
      subtitle: "Copied just now",
      icon: { type: "monogram" },
      meta: label ? { label, modified: Date.now() - 60_000 } : { modified: Date.now() - 60_000 },
      action: { type: "copy_text", text: title },
    });
  const seeds = [
    clip("clip:link", "LINK", "https://example.com/docs"),
    clip("clip:email", "EMAIL", "hello@example.com"),
    clip("clip:path", "PATH", "C:\\Users\\Pranshul\\Documents"),
    clip("clip:code", "CODE", "const answer = 42;"),
    clip("clip:text", "TEXT", "remember the milk"),
    clip("clip:image", "IMAGE", "Screenshot.png"),
  ];

  const openClipboard = async (
    harness: Harness,
    rows: SearchResult[],
    query = "clipboard:",
  ): Promise<void> => {
    await resolveSearch(harness, 0, THREE);
    setInputValue(harness, query);
    await resolveSearch(harness, 1, rows);
  };
  const sectionLabels = (harness: Harness): (string | null)[] =>
    [...harness.container.querySelectorAll('[data-testid="result-group"]')].map(
      (group) =>
        group.querySelector(".result-subgroup-label, .group-label")?.textContent ?? null,
    );

  it("renders only non-empty category sections without a second chip row or row tags", async () => {
    const harness = setup();
    await openClipboard(harness, [seeds[0], seeds[2], seeds[3], seeds[4], seeds[5]]);
    expect(sectionLabels(harness)).toEqual(["LINKS", "PATHS", "CODE", "TEXT", "IMAGES"]);
    expect(harness.container.querySelector('[data-testid="clipboard-categories"]')).toBeNull();
    expect(harness.container.querySelectorAll(".result-tag").length).toBe(0);
    // No duplicate category words in the trailing meta either.
    const metas = [...harness.container.querySelectorAll(".result-meta")].map(
      (element) => element.textContent ?? "",
    );
    expect(metas).not.toContain("LINK");
    expect(harness.container.querySelectorAll('[role="option"]').length).toBe(5);
    expect(harness.container.querySelector('[data-testid="launcher-count"]')?.textContent).toBe(
      "5 matches",
    );
    const group = harness.container.querySelector('[data-testid="result-group"]');
    expect(group?.getAttribute("role")).toBe("group");
    expect(group?.getAttribute("aria-labelledby")).toBe("ps-clipboard-section-links");
    expect(group?.querySelector(".result-subgroup-label")?.textContent).toBe("LINKS");
    expect(group?.getAttribute("data-subgroup")).toBe("true");
  });

  it("traverses flattened rows in visual section order and Enter is stale-safe", async () => {
    const harness = setup();
    await openClipboard(harness, seeds);
    const input = inputOf(harness);
    expect(selectedId(harness)).toBe("clip:link");

    await press(input, "ArrowDown");
    expect(selectedId(harness)).toBe("clip:email");
    await press(input, "ArrowDown");
    await press(input, "ArrowDown");
    expect(selectedId(harness)).toBe("clip:code");
    await press(input, "End", { ctrlKey: true });
    expect(selectedId(harness)).toBe("clip:image");
    await press(input, "ArrowDown");
    expect(selectedId(harness)).toBe("clip:link"); // wraps across sections
    const active = harness.container.querySelector('[role="option"][aria-selected="true"]');
    expect(input.getAttribute("aria-activedescendant")).toBe(active?.id);
  });

  it("classifies unlabeled text into the same sections", async () => {
    const harness = setup();
    await openClipboard(harness, [
      clip("clip:url", null, "https://example.com/page"),
      clip("clip:mail", null, "person@example.com"),
      clip("clip:winpath", null, "C:\\Users\\Pranshul\\Documents"),
      clip("clip:plain", null, "ordinary copied words"),
    ]);
    expect(sectionLabels(harness)).toEqual(["LINKS", "EMAILS", "PATHS", "TEXT"]);
  });

  it("shows the honest empty clipboard copy when there are no items", async () => {
    const harness = setup();
    await openClipboard(harness, []);
    const empty = harness.container.querySelector('[data-testid="empty-state"]');
    expect(empty?.textContent).toContain("No clipboard history yet.");
    expect(empty?.textContent).toContain(
      "Only copies made while ProtonSearch is running are captured.",
    );
  });

  it("uses the query empty copy when a clipboard query has no matches", async () => {
    const harness = setup();
    await openClipboard(harness, [], "clipboard: zzz nothing");
    const empty = harness.container.querySelector('[data-testid="empty-state"]');
    expect(empty?.textContent).toContain("No matches in Clipboard.");
    expect(empty?.textContent).toContain("Try different words, or press Esc to clear the filter.");
  });

  it("explains clipboard history being off", async () => {
    const harness = setup({ clipboardHistoryEnabled: false });
    await resolveSearch(harness, 0, THREE);
    setInputValue(harness, "clipboard:");
    await resolveSearch(harness, 1, []);
    const empty = harness.container.querySelector('[data-testid="empty-state"]');
    expect(empty?.textContent).toContain("Clipboard history is off.");
    expect(empty?.textContent).toContain("Turn it on in ProtonSearch → Preferences");
  });
});

describe("FILES guard and prefix clearing (alpha debt)", () => {
  it("filters folder rows out of a FILES response until the engine fix lands", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const filesChip = [...harness.container.querySelectorAll<HTMLButtonElement>('[data-testid="filter-chip"]')].find(
      (element) => element.textContent === "FILES",
    );
    await act(async () => {
      filesChip?.click();
      await Promise.resolve();
    });
    expect(harness.pending[1].request.scope).toBe("files");
    await resolveSearch(harness, 1, [
      result({ id: "file:kept", title: "kept.txt", kind: "file" }),
      result({ id: "folder:leaked", title: "Leaked folder", kind: "folder", icon: { type: "file_type", ext: "folder" } }),
    ]);
    expect(harness.container.querySelectorAll('[role="option"]').length).toBe(1);
    expect(harness.container.querySelector('[data-result-id="folder:leaked"]')).toBeNull();
    expect(harness.container.querySelector('[data-result-id="file:kept"]')).not.toBeNull();
  });

  it("clears a typed scope prefix when a chip is chosen", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    setInputValue(harness, "file: architecture");
    await resolveSearch(harness, 1, THREE);
    const foldersChip = [...harness.container.querySelectorAll<HTMLButtonElement>('[data-testid="filter-chip"]')].find(
      (element) => element.textContent === "FOLDERS",
    );
    await act(async () => {
      foldersChip?.click();
      await Promise.resolve();
    });
    expect(harness.pending[2].request.query).toBe("architecture");
    expect(harness.pending[2].request.scope).toBe("folders");
    expect(inputOf(harness).value).toBe("architecture");
  });
});

describe("scoped empty copy", () => {
  it("uses the scoped filter empty copy", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, THREE);
    const appsChip = [...harness.container.querySelectorAll<HTMLButtonElement>('[data-testid="filter-chip"]')].find(
      (element) => element.textContent === "APPS",
    );
    await act(async () => {
      appsChip?.click();
      await Promise.resolve();
    });
    await resolveSearch(harness, 1, []);
    const empty = harness.container.querySelector('[data-testid="empty-state"]');
    expect(empty?.textContent).toContain("No matches in APPS.");
    expect(empty?.textContent).toContain("Try another category, or press Esc to clear the filter.");
  });
});

describe("v10 quick destinations", () => {
  const destination = (harness: Harness, id: string): HTMLButtonElement | null =>
    harness.container.querySelector<HTMLButtonElement>(
      '[data-testid="quick-destination"][data-destination-id="' + id + '"]',
    );

  it("shows real quick destinations in the empty state without fake counts", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, []);
    const quick = harness.container.querySelector('[data-testid="quick-destinations"]');
    expect(quick).not.toBeNull();
    const labels = [...harness.container.querySelectorAll('[data-testid="quick-destination"]')].map(
      (element) => element.querySelector(".quick-destination-label")?.textContent,
    );
    expect(labels).toEqual([
      "Browser Bookmarks",
      "Browser History",
      "Clipboard History",
      "Agents",
      "Git history",
      "Agent History",
    ]);
    // READY fixture has no sources -> honest off state, never a fabricated count.
    expect(quick?.textContent).toContain("Off — enable in Preferences");
    expect(harness.container.querySelector(".quick-syntax")?.textContent).toContain(
      "bookmarks:",
    );
  });

  it("navigates destinations with arrows and activates with Enter", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, []);
    const input = inputOf(harness);
    expect(input.getAttribute("aria-activedescendant")).toBe("ps-quick-bookmarks");
    await press(input, "ArrowDown");
    expect(input.getAttribute("aria-activedescendant")).toBe("ps-quick-history");
    await press(input, "ArrowDown");
    await press(input, "ArrowDown");
    expect(input.getAttribute("aria-activedescendant")).toBe("ps-quick-agents");
    await press(input, "Enter");
    expect(harness.showMain).toHaveBeenCalled();
  });

  it("activates a search destination by writing its prefix", async () => {
    const harness = setup({
      status: {
        ...READY,
        sources: [
          { id: "browser", label: "Browser", enabled: true, itemCount: 42, state: "ready" },
        ],
      },
    });
    await resolveSearch(harness, 0, []);
    const history = destination(harness, "history");
    await act(async () => {
      history?.click();
      await Promise.resolve();
    });
    expect(inputOf(harness).value).toBe("history:");
    expect(harness.pending[1].request.query).toBe("history:");
  });

  it("shows real source counts from EngineStatus when available", async () => {
    const harness = setup({
      status: {
        ...READY,
        sources: [
          { id: "browser", label: "Browser", enabled: true, itemCount: 4523, state: "ready" },
          { id: "clipboard", label: "Clipboard", enabled: true, itemCount: 12, state: "ready" },
          { id: "git", label: "Git", enabled: true, itemCount: 15, state: "ready" },
        ],
      },
    });
    await resolveSearch(harness, 0, []);
    const quick = harness.container.querySelector('[data-testid="quick-destinations"]');
    expect(quick?.textContent).toContain((4523).toLocaleString() + " items");
    expect(quick?.textContent).toContain("12 items");
    expect(quick?.textContent).toContain("15 items");
  });
});
