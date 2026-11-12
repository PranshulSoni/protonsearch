import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { EmptyState, StateNotice } from "./ui";

// Vitest runs with cwd = apps/desktop; read the stylesheet source for selector regressions.
const uiCss = readFileSync("src/app/ui.css", "utf8");

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let container: HTMLDivElement;

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
});

afterEach(() => {
  container.remove();
});

async function render(ui: React.ReactElement) {
  const root = createRoot(container);
  await act(async () => {
    root.render(ui);
  });
  const html = container.innerHTML;
  await act(async () => {
    root.unmount();
  });
  return html;
}

describe("quiet state variants", () => {
  it("emits canonical state-- modifier classes for every tone", async () => {
    for (const tone of ["neutral", "warn", "error", "offline", "empty"] as const) {
      const html = await render(
        <StateNotice tone={tone} title="Couldn't load activity." body="Your history stays on this PC." />,
      );
      expect(html).toContain("state--" + tone);
      expect(html).toContain("state-icon");
    }
  });

  it("renders EmptyState with the shared state--empty anatomy (20 px icon, title, body)", async () => {
    const html = await render(
      <EmptyState title="No exclusions." body="Everything in your locations is indexed." />,
    );
    expect(html).toContain("state state--empty");
    expect(html).toContain("state-title");
    expect(html).toContain("state-body");
    expect(html).toContain("state-icon");
    expect(html).not.toContain("empty-block");
  });

  it("targets severity icon colours with the state-- modifiers (REVIEW_2 F3)", () => {
    expect(uiCss).toContain(".state--warn .state-icon");
    expect(uiCss).toContain(".state--error .state-icon");
    expect(uiCss).not.toContain(".state-warn .state-icon");
    expect(uiCss).not.toContain(".state-error .state-icon");
  });

  it("styles the offline and empty variants", () => {
    expect(uiCss).toContain(".state--offline");
    expect(uiCss).toContain(".state--empty");
    expect(uiCss).toContain(".state--offline .state-icon");
    expect(uiCss).toContain(".state--empty .state-icon");
  });
});