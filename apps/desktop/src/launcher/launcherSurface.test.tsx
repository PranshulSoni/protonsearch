import { act, createRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
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
  hideLauncher: ReturnType<typeof vi.fn>;
  showMain: ReturnType<typeof vi.fn>;
  onGeometry: ReturnType<typeof vi.fn>;
  ref: ReturnType<typeof createRef<LauncherSurfaceHandle>>;
}

const disposers: (() => void)[] = [];

afterEach(() => {
  for (const dispose of disposers.splice(0)) dispose();
  document.body.innerHTML = "";
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

function setup(options?: { workArea?: WorkAreaSize; status?: EngineStatus }): Harness {
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
        hideLauncher={hideLauncher}
        showMain={showMain}
        status={options?.status ?? READY}
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
    expect(harness.container.querySelectorAll(".skeleton-row").length).toBe(3);
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

  it("shows the exact empty-input and 0-match copy", async () => {
    const harness = setup();
    await resolveSearch(harness, 0, []);
    const empty = harness.container.querySelector('[data-testid="empty-state"]');
    expect(empty?.textContent).toContain("Start typing to search files, apps, and actions.");
    expect(empty?.textContent).toContain(
      "Try a file name, an app, or a command like “lock”.",
    );

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
    expect(harness.onGeometry).toHaveBeenLastCalledWith(280, true);

    await press(input, " ", { ctrlKey: true });
    expect(harness.container.querySelector('[data-testid="preview-panel"]')).toBeNull();
    expect(harness.onGeometry).toHaveBeenLastCalledWith(280, false);
  });

  it("grows the window for a bottom drawer on narrow work areas", async () => {
    const harness = setup({ workArea: { width: 1024, height: 768 } });
    await resolveSearch(harness, 0, [previewResult]);
    await press(inputOf(harness), " ", { ctrlKey: true });
    expect(harness.onGeometry).toHaveBeenLastCalledWith(496, true);
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

  it("reports truthful index states in the footer", async () => {
    const indexing = setup({ status: { ...READY, state: "indexing" } });
    await resolveSearch(indexing, 0, THREE);
    expect(footerText(indexing)).toContain("Indexing…");
    expect(
      indexing.container.querySelector('[data-testid="status-dot"]')?.className,
    ).toContain("status-dot--warning");

    const paused = setup({ status: { ...READY, state: "paused" } });
    await resolveSearch(paused, 0, THREE);
    expect(footerText(paused)).toContain("Index paused");
    expect(
      paused.container.querySelector('[data-testid="status-dot"]')?.className,
    ).toContain("status-dot--neutral");

    const broken = setup({ status: { ...READY, state: "error", warnings: ["Drive offline"] } });
    await resolveSearch(broken, 0, THREE);
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
    // Only the error-state Retry button may live outside the footer.
    expect(outsideFooter.every((button) => button.classList.contains("inline-retry"))).toBe(true);
  });
});
