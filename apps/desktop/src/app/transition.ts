// Window-open / onboarding→app transition helpers shared by the app shell and
// the onboarding view. No bridge calls; purely local browser state.

export const ONBOARDING_CROSSFADE_FLAG = "__psOnboardingCrossfade";

function holder(): Record<string, unknown> {
  return window as unknown as Record<string, unknown>;
}

/** True when the OS or app setting asks for reduced motion. */
export function prefersReducedMotion(): boolean {
  if (typeof window === "undefined") return true;
  const byOs =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const bySetting = document.documentElement.dataset.motion === "reduced";
  return Boolean(byOs || bySetting);
}

/**
 * Mark that the next DesktopApp mount is the new half of a View Transition
 * crossfade; the shell then skips its own mount animation to avoid a double
 * fade. The mark is consumed once.
 */
export function beginOnboardingCrossfade(): void {
  holder()[ONBOARDING_CROSSFADE_FLAG] = true;
}

export function endOnboardingCrossfade(): void {
  holder()[ONBOARDING_CROSSFADE_FLAG] = false;
}

export function takeOnboardingCrossfade(): boolean {
  const active = Boolean(holder()[ONBOARDING_CROSSFADE_FLAG]);
  holder()[ONBOARDING_CROSSFADE_FLAG] = false;
  return active;
}

export function supportsViewTransition(): boolean {
  return typeof document !== "undefined" && typeof document.startViewTransition === "function";
}

/** Wait one paint so React has committed the new route before the snapshot. */
export function nextPaint(): Promise<void> {
  return new Promise((resolve) => {
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame(() => resolve());
    } else {
      setTimeout(resolve, 0);
    }
  });
}
