import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// RR-01/RR-02 regression guard (docs/security/PI_AGENT_SECURITY.md §7).
const source = readFileSync("src/app/views/AgentsSection.tsx", "utf8");

describe("Pi agent security copy and write gate", () => {
  it("never claims the workspace confines the agent", () => {
    expect(source).not.toContain("Files outside it are not exposed");
    expect(source).not.toContain("inside the workspace");
    expect(source).not.toContain("sandbox is");
    expect(source).toContain("not a sandbox");
  });

  it("states Windows permissions, no approvals and provider egress", () => {
    expect(source).toContain("Pi runs with your Windows permissions");
    expect(source).toContain("Read tools can open any file you can read");
    expect(source).toContain("There is no per-tool approval");
    expect(source).toContain("sent to the configured");
  });

  it("keeps write mode disabled in alpha and never persists allowWrites", () => {
    expect(source).toContain("File edits and shell access are disabled in this alpha.");
    expect(source).not.toContain("allowWrites");
    expect(source).not.toContain("writesConsent");
    expect(source).not.toContain("pendingConfirm");
    expect(source).not.toContain("Allow and run");
  });

  it("offers the default-agent switch only for installed agents", () => {
    expect(source).toContain("disabled={!found || isDefault}");
  });
});
