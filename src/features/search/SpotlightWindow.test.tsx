// @vitest-environment jsdom
import {
  act,
  fireEvent,
  render,
  screen,
  cleanup,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { SpotlightWindow } from "./SpotlightWindow";
const { invoke, listeners } = vi.hoisted(() => ({ invoke: vi.fn(), listeners: new Map<string, () => void>() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke, isTauri: () => true }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockImplementation((name: string, callback: () => void) => {
    listeners.set(name, callback);
    return Promise.resolve(() => listeners.delete(name));
  }),
}));
beforeEach(() => {
  vi.useFakeTimers();
  invoke.mockReset();
});
afterEach(() => {
  cleanup();
  listeners.clear();
  vi.useRealTimers();
});
const row = { id: "result", title: "Current result", detail: "local", type: "file", actions: ["open"] };
const calcRow = {
  id: "calc",
  title: "2 + 2 = 4",
  detail: "Calculator > Press Enter to copy  4",
  type: "CALC",
  actions: ["open"],
};
function searchQueries() {
  return invoke.mock.calls
    .filter(([command]) => command === "search")
    .map(([, args]) => (args as { query: string }).query);
}
function lastSearchQuery() {
  const queries = searchQueries();
  return queries[queries.length - 1];
}
function mockSearch(results: unknown[]) {
  invoke.mockImplementation((command: string, args?: { requestId?: number }) => Promise.resolve(
    command === "search"
      ? { requestId: args?.requestId ?? 1, results, elapsedMs: 1 }
      : undefined,
  ));
}
async function showResults(results: unknown[] = [row]) {
  mockSearch(results);
  render(<SpotlightWindow />);
  await act(async () => { vi.advanceTimersByTime(100); });
}
test("discards stale query responses and executes keyboard selection", async () => {
  const pending: Array<(value: unknown) => void> = [];
  invoke.mockImplementation((command: string) =>
    command === "search"
      ? new Promise((resolve) => pending.push(resolve))
      : Promise.resolve(),
  );
  render(<SpotlightWindow />);
  await act(async () => {
    vi.advanceTimersByTime(100);
  });
  fireEvent.change(screen.getByRole("combobox"), {
    target: { value: "latest" },
  });
  await act(async () => {
    vi.advanceTimersByTime(100);
  });
  await act(async () =>
    pending[1]({
      requestId: 2,
      results: [
        {
          id: "new",
          title: "Latest result",
          detail: "local",
          type: "file",
          actions: [],
        },
      ],
      elapsedMs: 2,
    }),
  );
  await act(async () =>
    pending[0]({
      requestId: 1,
      results: [
        {
          id: "old",
          title: "Old result",
          detail: "local",
          type: "file",
          actions: [],
        },
      ],
      elapsedMs: 1,
    }),
  );
  expect(screen.queryByText("Old result")).toBeNull();
  expect(screen.getByText("Latest result")).toBeTruthy();
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  await act(async () => {});
  expect(invoke).toHaveBeenCalledWith("execute_search_result", {
    resultId: "new",
  });
  expect(invoke).not.toHaveBeenCalledWith("hide_search");
});
test("Escape dismisses the native search window", async () => {
  invoke.mockResolvedValue({ requestId: 1, results: [], elapsedMs: 0 });
  render(<SpotlightWindow />);
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });
  expect(invoke).toHaveBeenCalledWith("hide_search");
});
test("Enter on a focused scope button does not execute the input selection", async () => {
  await showResults();
  fireEvent.keyDown(screen.getByRole("button", { name: "Apps" }), {
    key: "Enter",
  });
  expect(
    invoke.mock.calls.filter(
      ([command]) => command === "execute_search_result",
    ),
  ).toHaveLength(0);
});
test("Tab cycles search scopes without rewriting the query", async () => {
  mockSearch([]);
  render(<SpotlightWindow />);
  const input = screen.getByRole("combobox");
  fireEvent.keyDown(input, { key: "Tab" });
  expect(screen.getByRole("button", { name: "Apps" }).getAttribute("aria-pressed")).toBe("true");
  fireEvent.keyDown(input, { key: "Tab", shiftKey: true });
  expect(screen.getByRole("button", { name: "All" }).getAttribute("aria-pressed")).toBe("true");
});
test("routes scope queries through Rust prefixes without double-prefixing", async () => {
  mockSearch([]);
  render(<SpotlightWindow />);
  const input = screen.getByRole("combobox");
  await act(async () => { vi.advanceTimersByTime(100); });
  fireEvent.change(input, { target: { value: "chrome" } });
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(lastSearchQuery()).toBe("chrome");
  fireEvent.click(screen.getByRole("button", { name: "Apps" }));
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(lastSearchQuery()).toBe("app:chrome");
  fireEvent.click(screen.getByRole("button", { name: "Files" }));
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(lastSearchQuery()).toBe("file:chrome");
  fireEvent.click(screen.getByRole("button", { name: "Folders" }));
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(lastSearchQuery()).toBe("folder:chrome");
  fireEvent.click(screen.getByRole("button", { name: "OCR" }));
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(lastSearchQuery()).toBe("ocr:chrome");
  fireEvent.change(input, { target: { value: "file:report" } });
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(screen.getByRole("button", { name: "Files" }).getAttribute("aria-pressed")).toBe("true");
  expect(lastSearchQuery()).toBe("file:report");
  fireEvent.click(screen.getByRole("button", { name: "All" }));
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(lastSearchQuery()).toBe("file:report");
  fireEvent.change(input, { target: { value: "" } });
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(screen.getByRole("button", { name: "All" }).getAttribute("aria-pressed")).toBe("true");
  expect(lastSearchQuery()).toBe("");
});
test("a changed query cannot execute the previous results during debounce", async () => {
  await showResults();
  fireEvent.change(screen.getByRole("combobox"), { target: { value: "different" } });
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  expect(screen.queryByText("Current result")).toBeNull();
  expect(invoke.mock.calls.filter(([command]) => command === "execute_search_result")).toHaveLength(0);
});
test("composition Enter does not execute a result", async () => {
  await showResults();
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter", isComposing: true });
  expect(invoke.mock.calls.filter(([command]) => command === "execute_search_result")).toHaveLength(0);
});
test("repeated Enter cannot dispatch duplicate actions while execution is pending", async () => {
  await showResults();
  invoke.mockImplementation(() => new Promise(() => {}));
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter", repeat: true });
  expect(invoke.mock.calls.filter(([command]) => command === "execute_search_result")).toHaveLength(1);
});
test("empty queries show a quiet type-to-search state", async () => {
  mockSearch([]);
  render(<SpotlightWindow />);
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(screen.getByText("Type to search")).toBeTruthy();
  expect(screen.getByRole("status").textContent).toBe("Type to search");
});
test("completed queries announce no matching results", async () => {
  mockSearch([]);
  render(<SpotlightWindow />);
  fireEvent.change(screen.getByRole("combobox"), { target: { value: "missing" } });
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(screen.getByText("No matching results")).toBeTruthy();
  expect(screen.getByRole("status").textContent).toBe("No matching results");
});
test("reports aria-busy while a search is pending", async () => {
  const pending: Array<(value: unknown) => void> = [];
  invoke.mockImplementation((command: string) => command === "search"
    ? new Promise((resolve) => pending.push(resolve))
    : Promise.resolve());
  render(<SpotlightWindow />);
  expect(screen.getByRole("listbox").getAttribute("aria-busy")).toBe("true");
  await act(async () => { vi.advanceTimersByTime(100); pending[0]({ requestId: 1, results: [], elapsedMs: 1 }); });
  expect(screen.getByRole("listbox").getAttribute("aria-busy")).toBe("false");
});
test("a CALC result renders a parsed hero and is excluded from the row list", async () => {
  await showResults([calcRow, row]);
  expect(screen.getByText("2 + 2")).toBeTruthy();
  expect(screen.getByText("4")).toBeTruthy();
  expect(screen.getByText("Enter to copy")).toBeTruthy();
  expect(screen.queryByText("2 + 2 = 4")).toBeNull();
  expect(screen.getAllByRole("option")).toHaveLength(2);
  expect(screen.getByRole("option", { selected: true }).id).toBe("calc-hero");
  expect(invoke).toHaveBeenCalledWith("fit_search", { height: 236 });
});
test("clicking the CALC hero uses the authoritative execute path", async () => {
  await showResults([calcRow, row]);
  fireEvent.click(document.getElementById("calc-hero") as HTMLElement);
  await act(async () => {});
  expect(invoke).toHaveBeenCalledWith("execute_search_result", { resultId: "calc" });
  expect(invoke).not.toHaveBeenCalledWith("execute_search_result", { resultId: "result" });
});
test("Enter executes the CALC hero and ArrowDown moves to the first row", async () => {
  await showResults([calcRow, row]);
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  await act(async () => {});
  expect(invoke).toHaveBeenCalledWith("execute_search_result", { resultId: "calc" });
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "ArrowDown" });
  expect(screen.getByRole("option", { selected: true }).id).toBe("result-0");
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  await act(async () => {});
  expect(invoke).toHaveBeenCalledWith("execute_search_result", { resultId: "result" });
});
test("parses the calculation at the last separator", async () => {
  await showResults([{ ...calcRow, title: "round(1.5) = 2 = 2" }, row]);
  expect(screen.getByText("round(1.5) = 2")).toBeTruthy();
  expect(screen.getByText("2")).toBeTruthy();
});
test("result icons are inline SVGs chosen from the result kind", async () => {
  await showResults([
    { id: "tsx", title: "Sidebar-Bridge.tsx", detail: "src", type: "FILE", actions: [] },
    { id: "cpp", title: "main.cpp", detail: "src", type: "FILE", actions: [] },
    { id: "rust", title: "weather.rs", detail: "src", type: "CODE", actions: [] },
    { id: "folder", title: "ProtonDock Sources", detail: "C:\\Projects", type: "FOLDER", actions: [] },
    { id: "terminal", title: "Windows Terminal", detail: "System Shell", type: "app", actions: [] },
    { id: "vscode", title: "Visual Studio Code", detail: "Applications", type: "app", actions: [] },
    { id: "spotify", title: "Spotify", detail: "Audio Player", type: "app", actions: [] },
  ]);
  const kinds = Array.from(document.querySelectorAll("[data-icon]")).map((node) => node.getAttribute("data-icon"));
  expect(kinds).toEqual(["tsx", "cpp", "rust", "folder", "terminal", "vscode", "spotify"]);
  expect(screen.queryByText("FILE")).toBeNull();
  expect(document.querySelectorAll(".ps-result-badge")).toHaveLength(0);
});
test("a failed action reports its error and permits retry without dismissing", async () => {
  await showResults();
  invoke.mockRejectedValueOnce(new Error("Result expired; search again"));
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  await act(async () => {});
  expect(screen.getByRole("alert").textContent).toContain("Result expired");
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  await act(async () => {});
  expect(invoke.mock.calls.filter(([command]) => command === "execute_search_result")).toHaveLength(2);
  expect(invoke).not.toHaveBeenCalledWith("hide_search");
});
test("failed dismissal reports a native error", async () => {
  invoke.mockRejectedValue(new Error("Search window unavailable"));
  render(<SpotlightWindow />);
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });
  await act(async () => {});
  expect(screen.getByRole("alert").textContent).toContain("Search window unavailable");
});
test("search opens compact and expands to a bounded height for results", async () => {
  mockSearch(Array.from({ length: 30 }, (_, i) => ({ ...row, id: String(i) })));
  render(<SpotlightWindow />);
  expect(invoke).toHaveBeenCalledWith("fit_search", { height: 174 });
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(invoke).toHaveBeenCalledWith("fit_search", { height: 500 });
});
test("search restores persisted theme and follows cross-window settings updates", async () => {
  let theme = "light";
  invoke.mockImplementation((command: string) => Promise.resolve(command === "load_dock_settings" ? { theme } : undefined));
  render(<SpotlightWindow />);
  await act(async () => {});
  expect(document.body.dataset.theme).toBe("light");
  theme = "glass";
  await act(async () => { listeners.get("settings-updated")?.(); });
  expect(document.body.dataset.theme).toBe("glass");
});
test("theme read failure retains safe dark default and reports the error", async () => {
  invoke.mockImplementation((command: string) => command === "load_dock_settings"
    ? Promise.reject(new Error("disk unavailable")) : Promise.resolve());
  render(<SpotlightWindow />);
  await act(async () => {});
  expect(document.body.dataset.theme).toBe("dark");
  expect(screen.getByText("Theme unavailable").getAttribute("title")).toContain("disk unavailable");
});
test("theme read failure does not hide or prevent executing valid search results", async () => {
  invoke.mockImplementation((command: string) => command === "load_dock_settings"
    ? Promise.reject(new Error("disk unavailable"))
    : Promise.resolve(command === "search" ? { requestId: 1, results: [row], elapsedMs: 1 } : undefined));
  render(<SpotlightWindow />);
  await act(async () => { vi.advanceTimersByTime(100); });
  expect(screen.getByText("Current result")).toBeTruthy();
  expect(screen.getByText("Theme unavailable")).toBeTruthy();
  fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });
  await act(async () => {});
  expect(invoke).toHaveBeenCalledWith("execute_search_result", { resultId: "result" });
});
test("an older settings read cannot overwrite a newer theme event", async () => {
  const pending: Array<(value: unknown) => void> = [];
  invoke.mockImplementation((command: string) => command === "load_dock_settings"
    ? new Promise((resolve) => pending.push(resolve)) : Promise.resolve());
  render(<SpotlightWindow />);
  await act(async () => {});
  await act(async () => { listeners.get("settings-updated")?.(); });
  await act(async () => { pending[1]({ theme: "glass" }); });
  await act(async () => { pending[0]({ theme: "light" }); });
  expect(document.body.dataset.theme).toBe("glass");
});
