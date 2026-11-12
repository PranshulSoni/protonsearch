import { describe, expect, it, vi } from "vitest";
import {
  beginOnboardingCrossfade,
  endOnboardingCrossfade,
  nextPaint,
  prefersReducedMotion,
  supportsViewTransition,
  takeOnboardingCrossfade,
} from "./transition";

describe("window/onboarding transition helpers", () => {
  it("consumes the onboarding crossfade mark exactly once", () => {
    expect(takeOnboardingCrossfade()).toBe(false);
    beginOnboardingCrossfade();
    expect(takeOnboardingCrossfade()).toBe(true);
    expect(takeOnboardingCrossfade()).toBe(false);
    beginOnboardingCrossfade();
    endOnboardingCrossfade();
    expect(takeOnboardingCrossfade()).toBe(false);
  });

  it("detects the reduced-motion setting and OS preference", () => {
    document.documentElement.dataset.motion = "full";
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn(() => ({ matches: true })),
    });
    expect(prefersReducedMotion()).toBe(true);

    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn(() => ({ matches: false })),
    });
    expect(prefersReducedMotion()).toBe(false);
    document.documentElement.dataset.motion = "reduced";
    expect(prefersReducedMotion()).toBe(true);
    delete document.documentElement.dataset.motion;
  });

  it("reports View Transition support and resolves nextPaint", async () => {
    expect(supportsViewTransition()).toBe(false);
    await expect(nextPaint()).resolves.toBeUndefined();
  });
});
