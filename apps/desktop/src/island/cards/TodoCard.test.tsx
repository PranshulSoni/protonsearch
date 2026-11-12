import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { getStoredTodos } from "../../lib/todoStore";
import { TodoCard } from "./TodoCard";
import { todayIso } from "./todoRules";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const STORE_KEY = "protonsearch_todos_v2";

let root: Root | null = null;
let host: HTMLDivElement | null = null;

beforeAll(() => {
  vi.stubGlobal("ResizeObserver", class {
    observe() {}
    unobserve() {}
    disconnect() {}
  });
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({ matches: false, media: query, addEventListener() {}, removeEventListener() {} }),
  });
});

beforeEach(() => {
  localStorage.clear();
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

afterEach(async () => {
  await act(async () => root?.unmount());
  host?.remove();
  root = null;
  host = null;
  vi.restoreAllMocks();
});

async function mount(ui: ReactNode): Promise<HTMLDivElement> {
  const el = document.createElement("div");
  document.body.appendChild(el);
  host = el;
  root = createRoot(el);
  const r = root;
  await act(async () => r.render(ui));
  await flush();
  return el;
}

const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

function setInput(input: HTMLInputElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function seed(items: object[]) {
  localStorage.setItem(STORE_KEY, JSON.stringify(items));
}

describe("TodoCard", () => {
  it("adds a task from the add row and reads quick tags", async () => {
    const el = await mount(<TodoCard />);
    const input = el.querySelector<HTMLInputElement>('input[aria-label="New task"]')!;
    await act(async () => setInput(input, "Ship the card !high @today"));
    await act(async () => el.querySelector("form")!.requestSubmit());
    await flush();
    const [saved] = getStoredTodos();
    expect(saved.title).toBe("Ship the card");
    expect(saved.priority).toBe("high");
    expect(saved.dueDate?.startsWith(todayIso())).toBe(true);
    expect(el.textContent).toContain("Ship the card");
  });

  it("completes a task and moves it between the Active and Done filters", async () => {
    seed([
      { id: "a", title: "Write notes", completed: false, createdAt: 1 },
      { id: "b", title: "File taxes", completed: true, createdAt: 2, completedAt: 3 },
    ]);
    const el = await mount(<TodoCard />);
    expect(el.textContent).toContain("Write notes");
    expect(el.textContent).not.toContain("File taxes");

    await act(async () => el.querySelector<HTMLElement>('[role="checkbox"][aria-label="Write notes"]')!.click());
    await flush();
    expect(getStoredTodos().find((item) => item.id === "a")?.completed).toBe(true);
    expect(el.textContent).not.toContain("Write notes");

    const done = [...el.querySelectorAll<HTMLElement>('[role="radio"]')].find((tab) => tab.textContent === "Done")!;
    await act(async () => done.click());
    expect(el.textContent).toContain("Write notes");
  });

  it("deletes the highlighted task from the keyboard after confirming", async () => {
    seed([{ id: "a", title: "Write notes", completed: false, createdAt: 1 }]);
    const el = await mount(<TodoCard />);
    const list = el.querySelector<HTMLElement>('[aria-label="Task list"]')!;
    await act(async () => list.dispatchEvent(new KeyboardEvent("keydown", { key: "Delete", bubbles: true })));
    await flush();
    expect(window.confirm).toHaveBeenCalled();
    expect(getStoredTodos()).toEqual([]);
  });

  it("opens a due date & time input from the add row and saves it with the new task", async () => {
    const el = await mount(<TodoCard />);
    const dueToggle = () => el.querySelector<HTMLButtonElement>('button[title="Set due date & time"]')!;
    expect(el.querySelector('input[aria-label="Due date and time"]')).toBeNull();

    await act(async () => dueToggle().click());
    const due = el.querySelector<HTMLInputElement>('input[aria-label="Due date and time"]')!;
    expect(due).not.toBeNull();
    expect(document.activeElement).toBe(due);

    await act(async () => setInput(due, "2099-01-15T09:30"));
    expect(dueToggle().textContent).toBe("01-15 09:30");

    const draft = el.querySelector<HTMLInputElement>('input[aria-label="New task"]')!;
    await act(async () => setInput(draft, "Pay rent"));
    await act(async () => el.querySelector("form")!.requestSubmit());
    await flush();
    const [saved] = getStoredTodos();
    expect(saved.title).toBe("Pay rent");
    expect(saved.dueDate).toBe("2099-01-15T09:30");
  });
});
