import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "../../bridge/api";
import { PiSetup } from "./PiSetup";

vi.mock("../../App", () => ({ useApp: () => ({ settings: { agent: {} }, updateSettings: vi.fn() }) }));
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root;
let container: HTMLDivElement;
const changed = vi.fn();
async function mount() {
  vi.spyOn(api, "piInstallStatus").mockResolvedValue({ state: "not_installed", path: null, version: null,
    nodeOk: true, authenticationConfigured: false, installationMethod: null, gitBashAvailable: false, detail: "Pi is missing" });
  container = document.createElement("div");
  root = createRoot(container);
  await act(async () => { root.render(<PiSetup onChanged={changed} />); });
}
async function click(label: string) {
  const button = [...container.querySelectorAll("button")].find((button) => button.textContent === label);
  expect(button).toBeTruthy();
  await act(async () => { button!.click(); });
}
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  vi.restoreAllMocks();
});
describe("Pi setup", () => {
  it("requires a reviewed plan and explicit confirmation before installation", async () => {
    const install = vi.spyOn(api, "piInstall").mockResolvedValue();
    vi.spyOn(api, "piInstallPlan").mockResolvedValue({ version: "1.5.2", unpackedBytes: 24000000,
      integrity: "sha512-test", license: "MIT", destination: "C:\\app\\pi", sourceUrl: "https://pi.dev/docs/latest/quickstart",
      dependenciesSizeKnown: false });
    await mount();
    expect(install).not.toHaveBeenCalled();
    await click("Review Pi installation");
    expect(install).not.toHaveBeenCalled();
    expect(container.textContent).toContain("Additional dependency downloads vary");
    await click("Confirm installation");
    expect(install).toHaveBeenCalledExactlyOnceWith("1.5.2", true);
  });
  it("shows installation errors and retains the plan for retry", async () => {
    vi.spyOn(api, "piInstallPlan").mockRejectedValue(new Error("Registry unavailable"));
    await mount();
    await click("Review Pi installation");
    expect(container.querySelector('[role="alert"]')?.textContent).toContain("Registry unavailable");
    expect([...container.querySelectorAll("button")].find((button) => button.textContent === "Review Pi installation")?.disabled).toBe(false);
  });
});
