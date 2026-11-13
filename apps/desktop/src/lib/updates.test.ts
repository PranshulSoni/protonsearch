import { describe, expect, it } from "vitest";
import { compareVersions } from "./updates";

describe("compareVersions", () => {
  it("orders release tags against the running version", () => {
    expect(compareVersions("v2.1.0", "2.0.9")).toBeGreaterThan(0);
    expect(compareVersions("1.10.0", "v1.9.0")).toBeGreaterThan(0);
    expect(compareVersions("v2.0.0", "2.0.0")).toBe(0);
    expect(compareVersions("v1.2", "1.2.0")).toBe(0);
    expect(compareVersions("v1.4.0", "2.0.0-alpha.1")).toBeLessThan(0);
  });

  it("ranks a release above its prereleases", () => {
    expect(compareVersions("v2.0.0", "2.0.0-alpha.1")).toBeGreaterThan(0);
    expect(compareVersions("2.0.0-alpha.1", "2.0.0")).toBeLessThan(0);
    expect(compareVersions("2.0.0-alpha.10", "2.0.0-alpha.2")).toBeGreaterThan(0);
    expect(compareVersions("2.0.0-beta", "2.0.0-alpha.3")).toBeGreaterThan(0);
  });
});
