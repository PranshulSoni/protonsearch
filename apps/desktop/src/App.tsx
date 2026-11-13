import {
  createContext,
  lazy,
  Suspense,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { api, isTauriRuntime } from "./bridge/api";
import type {
  ActionSpec,
  ActivityItem,
  ActivityQuery,
  EngineStatus,
  IndexCommand,
  OnboardingInput,
  OnboardingState,
  OpenOutcome,
  Settings,
  SettingsPatch,
} from "./bridge/types";
import LauncherView from "./launcher/LauncherView";
import DesktopApp from "./app/DesktopApp";
import OnboardingView from "./onboarding/OnboardingView";
import { Logo } from "./components/Logo";

// Lazy: the island's Tailwind preflight CSS loads only in the island window.
const IslandView = lazy(() => import("./island/IslandView"));

export type ViewKind = "launcher" | "main" | "island";

export interface AppContextValue {
  view: ViewKind;
  settings: Settings;
  status: EngineStatus | null;
  onboarding: OnboardingState | null;
  isMock: boolean;
  updateSettings: (patch: SettingsPatch) => Promise<Settings>;
  completeOnboarding: (input: OnboardingInput) => Promise<Settings>;
  openResult: (action: ActionSpec) => Promise<OpenOutcome>;
  refreshStatus: () => Promise<void>;
  getActivity: (query?: ActivityQuery) => Promise<ActivityItem[]>;
  indexCommand: (command: IndexCommand) => Promise<void>;
  showMain: () => void;
  showLauncher: () => void;
  hideLauncher: () => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) throw new Error("useApp must be used inside AppContext.Provider");
  return value;
}

function resolveTheme(settings: Settings): "light" | "dark" {
  if (settings.theme !== "system") return settings.theme;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyChrome(settings: Settings) {
  const root = document.documentElement;
  root.dataset.theme = resolveTheme(settings);
  root.dataset.density = settings.density;
  root.dataset.motion =
    settings.motion === "reduced" ||
    (settings.motion === "system" && window.matchMedia("(prefers-reduced-motion: reduce)").matches)
      ? "reduced"
      : "full";
}

function BootScreen() {
  return (
    <div className="boot-screen" role="status" aria-live="polite">
      <Logo size={34} />
      <span>Starting ProtonSearch…</span>
    </div>
  );
}

/// Technical details are for support/diagnostics: strip absolute user paths and
/// cap the length so no secrets or full directory layout leaks onto the surface.
function sanitizeBootDetail(message: string): string {
  return message
    .replace(/[A-Za-z]:\\Users\\[^\\\s]+(?:\\[^\s:;,"')]+)*/g, "<user path>")
    .replace(/\\\\(?:\?\\)?[^\s:;,"')]+/g, "<path>")
    .slice(0, 600);
}

function FatalScreen({ message, onRetry }: { message: string; onRetry: () => void }) {
  // Quiet presentation: the user sees what happened and what to do next; the
  // technical detail lives in a native disclosure for support/diagnostics.
  return (
    <div className="boot-screen boot-screen-error" role="alert">
      <Logo size={34} />
      <strong>ProtonSearch couldn't start</strong>
      <span className="boot-detail">
        Your files and settings on this PC were not touched. Try again — if it keeps happening, open
        ProtonSearch's data folder and check that it is writable.
      </span>
      <details className="boot-details">
        <summary>Technical details</summary>
        <code>{sanitizeBootDetail(message)}</code>
      </details>
      <button className="btn" onClick={onRetry}>
        Try again
      </button>
    </div>
  );
}

export default function App({ view }: { view: ViewKind }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [status, setStatus] = useState<EngineStatus | null>(null);
  const [onboarding, setOnboarding] = useState<OnboardingState | null>(null);
  const [error, setError] = useState<string | null>(null);
  const bootedRef = useRef(false);

  const bootstrap = useCallback(async () => {
    setError(null);
    try {
      const [nextSettings, nextStatus] = await Promise.all([api.getSettings(), api.getStatus()]);
      setSettings(nextSettings);
      setStatus(nextStatus);
      if (view === "main") {
        try {
          setOnboarding(await api.getOnboarding());
        } catch {
          /* onboarding info is non-fatal */
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [view]);

  useEffect(() => {
    if (bootedRef.current) return;
    bootedRef.current = true;
    void bootstrap();
  }, [bootstrap]);

  useEffect(() => {
    if (!settings) return;
    applyChrome(settings);
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyChrome(settings);
    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [settings]);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await api.getStatus());
    } catch {
      /* keep last known status */
    }
  }, []);

  // Status polling only in the desktop app, and slowly: indexing progress is
  // the only thing that moves. The launcher never polls (it reacts to search).
  useEffect(() => {
    if (view !== "main") return;
    const id = window.setInterval(() => void refreshStatus(), 4000);
    return () => window.clearInterval(id);
  }, [view, refreshStatus]);

  const updateSettings = useCallback(async (patch: SettingsPatch) => {
    const next = await api.updateSettings(patch);
    setSettings(next);
    // An engine change, pause or new root moves the index state; show it now, not on the next poll.
    void refreshStatus();
    return next;
  }, [refreshStatus]);

  const completeOnboarding = useCallback(async (input: OnboardingInput) => {
    const next = await api.completeOnboarding(input);
    setSettings(next);
    setOnboarding((prev) => (prev ? { ...prev, complete: true, defaults: next } : prev));
    void refreshStatus();
    return next;
  }, [refreshStatus]);

  const openResult = useCallback(async (action: ActionSpec) => {
    const outcome = await api.openResult(action);
    // Only dismiss on a completed action: destructive commands first return a
    // confirmation challenge (ok:false) and the launcher must stay open.
    if (view === "launcher" && outcome.ok) {
      void api.hideLauncher();
    }
    return outcome;
  }, [view]);

  const getActivity = useCallback((query?: ActivityQuery) => api.getActivity(query), []);

  const indexCommand = useCallback(
    async (command: IndexCommand) => {
      await api.indexCommand(command);
      void refreshStatus();
    },
    [refreshStatus],
  );

  const showMain = useCallback(() => {
    if (isTauriRuntime()) void api.showMain();
  }, []);

  const showLauncher = useCallback(() => {
    if (isTauriRuntime()) void api.showLauncher();
  }, []);

  const hideLauncher = useCallback(() => {
    if (isTauriRuntime()) void api.hideLauncher();
  }, []);

  const value = useMemo<AppContextValue | null>(() => {
    if (!settings) return null;
    return {
      view,
      settings,
      status,
      onboarding,
      isMock: !isTauriRuntime(),
      updateSettings,
      completeOnboarding,
      openResult,
      refreshStatus,
      getActivity,
      indexCommand,
      showMain,
      showLauncher,
      hideLauncher,
    };
  }, [
    view,
    settings,
    status,
    onboarding,
    updateSettings,
    completeOnboarding,
    openResult,
    refreshStatus,
    getActivity,
    indexCommand,
    showMain,
    showLauncher,
    hideLauncher,
  ]);

  if (error) {
    return (
      <div className={view === "launcher" ? "launcher-shell" : "main-shell"}>
        <FatalScreen message={error} onRetry={() => void bootstrap()} />
      </div>
    );
  }

  if (!value) {
    if (view === "launcher" || view === "island") return null;
    return <BootScreen />;
  }

  const showOnboarding = view === "main" && onboarding !== null && !onboarding.complete;

  return (
    <AppContext.Provider value={value}>
      {view === "launcher" ? (
        <LauncherView />
      ) : view === "island" ? (
        <Suspense fallback={null}>
          <IslandView />
        </Suspense>
      ) : showOnboarding ? (
        <OnboardingView />
      ) : (
        <DesktopApp />
      )}
    </AppContext.Provider>
  );
}

/** Utility wrapper so views can render children without prop-drilling from App. */
export function Shell({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={className}>{children}</div>;
}