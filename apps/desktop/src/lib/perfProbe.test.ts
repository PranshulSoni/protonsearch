import { describe, expect, it } from "vitest";
import { formatFrameMark, frameStats } from "./perfProbe";

describe("perfProbe", () => {
  it("summarises frame intervals against the median", () => {
    const stats = frameStats([16, 16, 16, 16, 16, 16, 16, 16, 16, 40, 60]);
    expect(stats).toEqual({ n: 11, p50: 16, p95: 60, p99: 60, max: 60, drop: 2, long: 1 });
  });

  it("writes one mark line with no quotes or backslashes", () => {
    const line = formatFrameMark("island-switch:search>media", frameStats([16, 20, 60]));
    expect(line).toBe("island-switch:search>media|n=3|p50=20.0|p95=60.0|p99=60.0|max=60.0|drop=1|long=1");
    expect(line).not.toMatch(/["\\]/);
  });
});
