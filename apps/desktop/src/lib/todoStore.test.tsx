import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { getStoredTodos, useTodos } from "./todoStore";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("../bridge/api", () => ({ isTauriRuntime: () => true }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("shared tasks", () => {
  it("starts empty without inserting sample tasks", () => {
    localStorage.clear();
    expect(getStoredTodos()).toEqual([]);
    expect(localStorage.getItem("protonsearch_todos_v2")).toBeNull();
  });

  it("imports existing tasks before loading the shared database", async () => {
    const legacy = { id: "old", title: "Keep me", completed: false, createdAt: 10 };
    localStorage.setItem("protonsearch_todos_v2", JSON.stringify([legacy]));
    mocks.listen.mockResolvedValue(() => undefined);
    mocks.invoke.mockImplementation(async (command) => command === "todos_list" ? [legacy] : undefined);
    let state: ReturnType<typeof useTodos> | undefined;
    function Probe() { state = useTodos(); return null; }
    const root = createRoot(document.createElement("div"));
    await act(async () => { root.render(<Probe />); });
    expect(mocks.invoke.mock.calls.map((call) => call[0])).toEqual(["todos_import", "todos_list"]);
    expect(state?.todos).toEqual([legacy]);
    expect(JSON.parse(localStorage.getItem("protonsearch_todos_v2")!)).toEqual([legacy]);
    await act(async () => { root.unmount(); });
    localStorage.clear();
  });
});
