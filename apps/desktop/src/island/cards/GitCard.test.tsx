import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { SearchResult } from "../../bridge/types";
import { api } from "../../bridge/api";
import { GitCard } from "./GitCard";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const commit = {
  id: "git:commit:1",
  kind: "git_commit",
  title: "Fix search ranking",
  subtitle: "Ada - 1a2b3c4d - C:\\dev\\protonsearch",
  path: "C:\\dev\\protonsearch",
  meta: { modified: Date.now() - 120_000 },
  score: 0,
} as SearchResult;

const repo = {
  id: "git:repo:c:\\dev\\protonsearch",
  kind: "git_repository",
  title: "protonsearch",
  subtitle: "Repository - argon",
  path: "C:\\dev\\protonsearch",
  score: 0,
} as SearchResult;

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

afterEach(async () => {
  await act(async () => root?.unmount());
  host?.remove();
  root = null;
  host = null;
});

async function mount(ui: ReactNode): Promise<HTMLDivElement> {
  const el = document.createElement("div");
  document.body.appendChild(el);
  host = el;
  root = createRoot(el);
  const r = root;
  await act(async () => r.render(ui));
  return el;
}

const keyOn = (el: HTMLElement, key: string) =>
  act(async () => el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true })));

describe("GitCard", () => {
  it("shows the repo and branch from the results with the commit rows", async () => {
    const el = await mount(<GitCard results={[commit, repo]} pending={false} error={null} onOpen={vi.fn()} />);
    expect(el.textContent).toContain("protonsearch");
    expect(el.textContent).toContain("argon");
    expect(el.textContent).toContain("Fix search ranking");
    expect(el.textContent).toContain("Ada · 1a2b3c4d");
    expect(el.textContent).not.toContain("C:\dev");
    expect(el.textContent).toContain("2 min ago");
  });

  it("opens the highlighted row on Enter", async () => {
    const onOpen = vi.fn();
    const older = { ...commit, id: "git:commit:2", title: "Older commit" };
    const el = await mount(<GitCard results={[commit, older, repo]} pending={false} error={null} onOpen={onOpen} />);
    const list = el.querySelector<HTMLElement>('[aria-label="Git activity"]')!;
    await keyOn(list, "ArrowDown");
    await keyOn(list, "Enter");
    expect(onOpen).toHaveBeenCalledWith(older);
  });

  it("lists every branch and loads the picked one's commits", async () => {
    const onBranch = { ...commit, id: "git:commit:9", title: "Work on dev" };
    const load = vi.spyOn(api, "gitBranchCommits").mockResolvedValue([onBranch]);
    const withBranches = { ...repo, meta: { branches: ["argon", "dev"] } } as SearchResult;
    const el = await mount(<GitCard results={[commit, withBranches]} pending={false} error={null} onOpen={vi.fn()} />);
    await act(async () => el.querySelector<HTMLButtonElement>('[aria-label="Branch: argon"]')!.click());
    const dev = [...el.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "dev")!;
    await act(async () => dev.click());
    expect(load).toHaveBeenCalledWith("C:\\dev\\protonsearch", "dev");
    expect(el.textContent).toContain("Work on dev");
    expect(el.textContent).not.toContain("Fix search ranking");
  });

  it("opens a row on click", async () => {
    const onOpen = vi.fn();
    const el = await mount(<GitCard results={[commit, repo]} pending={false} error={null} onOpen={onOpen} />);
    const row = [...el.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.includes("Fix search ranking"))!;
    await act(async () => row.click());
    expect(onOpen).toHaveBeenCalledWith(commit);
  });

  it("says when the source is unavailable", async () => {
    const el = await mount(<GitCard results={[]} pending={false} error="index offline" onOpen={vi.fn()} />);
    expect(el.textContent).toContain("This source is unavailable.");
    expect(el.textContent).toContain("index offline");
  });

  it("says when there is no Git activity yet", async () => {
    const el = await mount(<GitCard results={[]} pending={false} error={null} onOpen={vi.fn()} />);
    expect(el.textContent).toContain("No Git activity yet.");
  });
});
