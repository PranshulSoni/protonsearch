import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { api } from "../bridge/api";
import type { IslandCardId, SearchResult, SearchScope } from "../bridge/types";
import { SearchCard, UnsupportedCard } from "./IslandCards";
import { AgentsCard } from "./cards/AgentsCard";
import { BrowserCard } from "./cards/BrowserCard";
import { ClipboardCard } from "./cards/ClipboardCard";
import { ControlsCard } from "./cards/ControlsCard";
import { GitCard } from "./cards/GitCard";
import { MediaCard } from "./cards/MediaCard";
import { TodoCard } from "./cards/TodoCard";
import { useIslandControls } from "./islandNativeCards";
import { GooeyNav } from "./ui/GooeyNav";
import {
  useIslandBootstrap,
  useIslandSearch,
  type ClipTab,
} from "./islandData";
import {
  islandCardDims,
  MAX_ISLAND_HEIGHT,
  islandCardLabel,
  ISLAND_VISIBLE_ROWS,
  islandWindowWidth,
  visibleIslandCards,
  type IslandVariant,
} from "./islandDims";
import { markTyping, recordFrames } from "../lib/perfProbe";
import { groupResultsInRankOrder } from "../lib/results";
import { lookFrom, readIslandLook, useResolvedTheme, writeIslandLook } from "../lib/islandLook";
import "./island-tw.css";
import "./island.css";

// Memoized card bodies: typing in Search must not re-render hidden card DOM
// (engine payload perf: clipboard thumbs/app icon data URLs are stable).
const MemoClipboardCard = memo(ClipboardCard);
const MemoAgentsCard = memo(AgentsCard);
const MemoBrowserCard = memo(BrowserCard);
const MemoGitCard = memo(GitCard);
const MemoMediaCard = memo(MediaCard);
const MemoControlsCard = memo(ControlsCard);
const MemoTodoCard = memo(TodoCard);

/**
 * V9 Dynamic Island surface. The native window adds 96 px transparent shadow
 * padding around this surface; geometry reported here is the surface only.
 * Binding: docs/design/ISLAND_V9_SPEC.md.
 */

// Must match --island-dur-open / --island-dur-close in island.css.
const OPEN_MS = 420;
const CLOSE_MS = 150;
// Longest the morph runs. The native window grows before it and shrinks after.
const MORPH_SETTLE_MS = 600;
// Card swap: the old card fades up and out, the new one fades in from below.
const CARD_OUT_S = 0.14;
const CARD_IN_S = 0.22;
// Scroll delta that switches a card, and the silence that ends a scroll gesture.
const WHEEL_STEP = 48;
const WHEEL_GESTURE_GAP_MS = 160;

function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Morph origin: the anchor edge of the surface, so it grows away from where it is pinned. */
export default function IslandView() {
  const bootstrap = useIslandBootstrap();
  const settings = bootstrap.settings?.launcher?.island ?? null;
  const cards = useMemo(() => visibleIslandCards(settings), [settings]);
  // Until the bootstrap settings arrive, the look cached by the settings page
  // paints the first frame, so the surface never flashes the default style.
  const [cachedLook] = useState(readIslandLook);
  const look = bootstrap.settings ? lookFrom(bootstrap.settings.launcher.island, bootstrap.settings.theme) : cachedLook;
  const resolvedTheme = useResolvedTheme(look.theme);
  useEffect(() => {
    if (bootstrap.settings) {
      writeIslandLook(lookFrom(bootstrap.settings.launcher.island, bootstrap.settings.theme));
    }
  }, [bootstrap.settings]);
  const reduced = useReducedMotion();
  // task-58: every summon is the compact oval search pill.
  const [activeCard, setActiveCard] = useState<IslandCardId>("search");
  const [compact, setCompact] = useState(true);
  const [variant, setVariant] = useState<IslandVariant>("default");
  const [clipTab, setClipTab] = useState<ClipTab>("all");
  const [visibility, setVisibility] = useState<"opening" | "shown" | "closing" | "hidden">("shown");
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<SearchScope>("all");
  const renders = (window as unknown as { __islandRenders?: number });
  renders.__islandRenders = (renders.__islandRenders ?? 0) + 1;
  // Prefetch all card data as soon as the island webview has bootstrap settings,
  // so a card switch is a render-only change (sources-content perf finding).
  const controls = useIslandControls(bootstrap.ready, activeCard === "media" && visibility !== "hidden");


  const inputRef = useRef<HTMLInputElement>(null);
  const initialisedRef = useRef(false);
  const lastSwitchRef = useRef(0);
  const closeTimerRef = useRef<number | null>(null);
  const openTimerRef = useRef<number | null>(null);
  const switchTimerRef = useRef<number | null>(null);
  const pendingSwitchRef = useRef<[IslandCardId, IslandVariant] | null>(null);
  const switchCardRef = useRef<(card: IslandCardId, nextVariant?: IslandVariant) => void>(() => undefined);
  const wheelRef = useRef({ accum: 0, at: 0, spent: false, spentAt: 0, peak: 0, floor: 0 });
  const pointerStartRef = useRef<{ x: number; y: number } | null>(null);
  const typedAtRef = useRef(0);
  const hideRef = useRef<() => void>(() => undefined);

  const [clipboardRevision, setClipboardRevision] = useState(0);
  const [browserRevision, setBrowserRevision] = useState(0);
  const [gitRevision, setGitRevision] = useState(0);

  useEffect(() => {
    if (activeCard === "clipboard") {
      setClipboardRevision((r) => r + 1);
    } else if (activeCard === "browser") {
      setBrowserRevision((r) => r + 1);
    } else if (activeCard === "git") {
      setGitRevision((r) => r + 1);
    }
  }, [activeCard]);

  const searchState = useIslandSearch(
    scope,
    query,
    bootstrap.ready && activeCard === "search",
  );
  // Warm every card's data in parallel while the compact pill is shown.
  const clipboardState = useIslandSearch("clipboard", "", bootstrap.ready, clipboardRevision);
  const browserState = useIslandSearch("browser", "", bootstrap.ready, browserRevision);
  const gitState = useIslandSearch("git", "", bootstrap.ready, gitRevision);

  const stageParam = Number(new URLSearchParams(window.location.search).get("stage") ?? "0");
  const monitor = useMemo(() => {
    const id = settings?.placement?.monitor;
    return (
      bootstrap.monitors.find((candidate) => candidate.id === id) ??
      bootstrap.monitors.find((candidate) => candidate.primary) ??
      bootstrap.monitors[0] ??
      null
    );
  }, [bootstrap.monitors, settings?.placement?.monitor]);
  const stageWidth =
    stageParam ||
    monitor?.workWidth ||
    (typeof screen !== "undefined" ? screen.availWidth : 0) ||
    1150;

  // Group labels among the shown rows: each one adds a line to the height.
  const searchLabels = useMemo(
    () => groupResultsInRankOrder(searchState.results, ISLAND_VISIBLE_ROWS).length,
    [searchState.results],
  );
  const dims = islandCardDims({
    card: activeCard,
    variant,
    stageWidth,
    compact,
    searchMatches: searchState.results.length,
    searchLabels,
  });
  // task-58: every summon starts as the compact oval search pill, regardless of
  // the configured default card. Expanding begins on a user click or typing.
  useEffect(() => {
    if (initialisedRef.current || !bootstrap.ready) return;
    initialisedRef.current = true;
    setActiveCard("search");
    setCompact(true);
    setQuery("");
    setScope("all");
  }, [bootstrap.ready]);

  // Every card but search sizes to its content (measured below), up to the max height;
  // beyond that the card's own list scrolls.
  const surfaceRef = useRef<HTMLDivElement>(null);
  const [natural, setNatural] = useState<{ card: IslandCardId; height: number } | null>(null);
  const fitted =
    activeCard !== "search" && natural?.card === activeCard
      ? Math.max(140, Math.min(MAX_ISLAND_HEIGHT, natural.height + 33)) // 33 = dots footer + borders
      : null;
  const surfaceDims = fitted == null ? dims : { ...dims, height: fitted };
  useLayoutEffect(() => {
    const slot = surfaceRef.current?.querySelector<HTMLElement>(`.island-card-slot[data-card="${activeCard}"]`);
    if (!slot || activeCard === "search") return;
    const measure = () => {
      const viewports = Array.from(slot.querySelectorAll<HTMLElement>("[data-radix-scroll-area-viewport]"));
      const scrollTops = viewports.map((viewport) => viewport.scrollTop);
      slot.setAttribute("data-measuring", "");
      const height = slot.offsetHeight;
      slot.removeAttribute("data-measuring");
      // The measuring CSS makes the viewports auto-height, which clamps scrollTop to 0.
      viewports.forEach((viewport, index) => {
        viewport.scrollTop = scrollTops[index];
      });
      // 0 means no layout engine (tests): keep the card's default size.
      if (height) setNatural((prev) => (prev?.card === activeCard && prev.height === height ? prev : { card: activeCard, height }));
    };
    measure();
    const observer = new MutationObserver(measure);
    observer.observe(slot, { childList: true, subtree: true, characterData: true });
    return () => observer.disconnect();
  }, [activeCard]);
  // The native window keeps one width (the widest card); only its height follows
  // the card. Changing the width also re-centres the window, and that move lands
  // a frame apart from the resize: the whole island visibly jumped sideways at the
  // start of a card switch and again when the size settled afterwards.
  const windowWidth = islandWindowWidth(stageWidth);
  const lastGeometryKeyRef = useRef("");
  const geometryHeightRef = useRef(surfaceDims.height);
  const geometrySettleTimerRef = useRef<number | null>(null);
  // Sends the native window size. Cards never call setIslandGeometry themselves.
  const fitWindow = useCallback((height: number) => {
    const key = windowWidth + "x" + Math.ceil(height);
    if (lastGeometryKeyRef.current === key) return;
    lastGeometryKeyRef.current = key;
    const w = (window as unknown as { __islandGeometryCalls?: number });
    w.__islandGeometryCalls = (w.__islandGeometryCalls ?? 0) + 1;
    void api.setIslandGeometry(windowWidth, Math.ceil(height)).catch(() => undefined);
  }, [windowWidth]);
  // Grow before the morph and shrink after it, so the surface is never clipped.
  const armSettle = useCallback((height: number, duration = MORPH_SETTLE_MS) => {
    if (geometrySettleTimerRef.current != null) window.clearTimeout(geometrySettleTimerRef.current);
    if (prefersReducedMotion()) {
      geometryHeightRef.current = height;
      fitWindow(height);
      geometrySettleTimerRef.current = null;
      return;
    }
    const reserved = Math.max(geometryHeightRef.current, height);
    geometryHeightRef.current = reserved;
    fitWindow(reserved);
    geometrySettleTimerRef.current = window.setTimeout(() => {
      geometryHeightRef.current = height;
      fitWindow(height);
      geometrySettleTimerRef.current = null;
    }, duration);
  }, [fitWindow]);
  useLayoutEffect(() => {
    armSettle(surfaceDims.height);
  }, [armSettle, surfaceDims.height]);

  useEffect(() => {
    if (searchState.pending || typedAtRef.current === 0) return;
    const ms = performance.now() - typedAtRef.current;
    typedAtRef.current = 0;
    markTyping(ms);
  }, [searchState.pending, searchState.results]);

  useEffect(() => {
    if (compact) return;
    const timer = window.setTimeout(() => inputRef.current?.focus(), 190);
    return () => window.clearTimeout(timer);
  }, [compact, activeCard]);

  const requestHide = useCallback(() => {
    if (visibility === "closing" || visibility === "hidden") return;
    const finish = () => {
      setVisibility("hidden");
      void api.hideIsland().catch(() => undefined);
    };
    if (prefersReducedMotion()) {
      finish();
      return;
    }
    recordFrames("island-close", 250);
    setVisibility("closing");
    if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
    closeTimerRef.current = window.setTimeout(finish, CLOSE_MS);
  }, [visibility]);

  hideRef.current = requestHide;

  const switchCard = useCallback(
    (card: IslandCardId, nextVariant: IslandVariant = "default") => {
      if (!cards.includes(card)) return;
      const nextCompact = card === "search" && !query.trim();
      if (card === activeCard && nextVariant === variant) {
        const currentDims = islandCardDims({
          card,
          variant: nextVariant,
          stageWidth,
          compact: nextCompact,
          searchMatches: searchState.results.length,
          searchLabels,
        });
        armSettle(currentDims.height);
        if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
        switchTimerRef.current = null;
        pendingSwitchRef.current = null;
        setCompact(nextCompact);
        return;
      }
      const now = Date.now();
      if (now - lastSwitchRef.current < 350) {
        pendingSwitchRef.current = [card, nextVariant];
        if (switchTimerRef.current == null) {
          switchTimerRef.current = window.setTimeout(() => {
            switchTimerRef.current = null;
            const pending = pendingSwitchRef.current;
            pendingSwitchRef.current = null;
            if (pending) switchCardRef.current(...pending);
          }, 350 - (now - lastSwitchRef.current));
        }
        return;
      }
      lastSwitchRef.current = now;
      const targetDims = islandCardDims({
        card,
        variant: nextVariant,
        stageWidth,
        compact: nextCompact,
        searchMatches: searchState.results.length,
        searchLabels,
      });
      armSettle(targetDims.height);
      recordFrames(`island-switch:${activeCard}>${card}`, 800);
      setActiveCard(card);
      setCompact(nextCompact);
      setVariant(nextVariant);
    },
    [activeCard, armSettle, cards, query, searchLabels, searchState.results.length, stageWidth, variant],
  );
  switchCardRef.current = switchCard;

  const switchRelative = useCallback(
    (delta: number) => {
      if (cards.length === 0) return;
      const index = cards.indexOf(activeCard);
      const next = cards[(index + delta + cards.length) % cards.length];
      switchCard(next);
    },
    [activeCard, cards, switchCard],
  );

  const backToCard = useCallback(() => {
    if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
    switchTimerRef.current = null;
    pendingSwitchRef.current = null;
    if (variant !== "default") {
      const target = islandCardDims({
        card: activeCard,
        variant: "default",
        stageWidth,
        compact,
        searchMatches: searchState.results.length,
        searchLabels,
      });
      armSettle(target.height);
      setVariant("default");
      return;
    }
    if (!compact) {
      const target = islandCardDims({ card: "search", stageWidth, compact: true, searchMatches: searchState.results.length });
      armSettle(target.height);
      // Expanded card -> compact search (predictable Esc chain), then hide.
      setActiveCard("search");
      setCompact(true);
      setVariant("default");
      setQuery("");
      setScope("all");
      return;
    }
    armSettle(dims.height, 0);
    requestHide();
  }, [activeCard, armSettle, compact, dims.height, requestHide, searchLabels, searchState.results.length, stageWidth, variant]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        backToCard();
        return;
      }
      if (event.ctrlKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
        event.preventDefault();
        switchRelative(event.key === "ArrowDown" ? 1 : -1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [backToCard, switchRelative]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const addListener = (unlisten: () => void) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    };
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        if (disposed) return;
        // Listen exactly once: re-subscribing on every render previously caused
        // duplicate island:shown handlers and duplicate island-open perf marks.
        const offShown = await listen("island:shown", () => {
          setClipboardRevision((r) => r + 1);
          if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
          switchTimerRef.current = null;
          pendingSwitchRef.current = null;
          const target = islandCardDims({ card: "search", stageWidth, compact: true, searchMatches: 0 });
          armSettle(target.height, 0);
          recordFrames("island-open", 450);
          if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
          if (openTimerRef.current != null) window.clearTimeout(openTimerRef.current);
          setActiveCard("search");
          setCompact(true);
          setQuery("");
          setScope("all");
          setVariant("default");
          setVisibility("opening");
          openTimerRef.current = window.setTimeout(
            () => setVisibility((current) => (current === "opening" ? "shown" : current)),
            OPEN_MS,
          );
          // Focus now, not after the animation: keys typed right after the hotkey must land.
          inputRef.current?.focus();
        });
        addListener(offShown);
        const offClip = await listen("clipboard:change", () => {
          setClipboardRevision((r) => r + 1);
        });
        addListener(offClip);
        const offDismiss = await listen("island:dismiss", () => {
          hideRef.current();
        });
        addListener(offDismiss);
        // Park the surface in its pre-pop state whenever the window is hidden, so
        // the next summon pops in instead of flashing the previous, fully drawn frame.
        const offHidden = await listen("island:hidden", () => setVisibility("hidden"));
        addListener(offHidden);
        // The window is prewarmed hidden. If it was summoned before these listeners
        // existed it is already on screen and must stay visible.
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        if (!(await getCurrentWindow().isVisible())) {
          setVisibility((current) => (current === "shown" ? "hidden" : current));
        }
      } catch {
        /* browser preview: no Tauri event bridge */
      }
    })();
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [armSettle, stageWidth]);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
      if (openTimerRef.current != null) window.clearTimeout(openTimerRef.current);
      if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
      if (geometrySettleTimerRef.current != null) window.clearTimeout(geometrySettleTimerRef.current);
    };
  }, []);

  const openResult = useCallback(
    async (result: SearchResult) => {
      try {
        await api.openResult(result.action);
      } catch {
        return;
      }
      requestHide();
    },
    [requestHide],
  );

  const onWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      // Only a two-finger left/right swipe switches cards; a vertical wheel just scrolls.
      if (Math.abs(event.deltaX) <= Math.abs(event.deltaY)) return;
      const wheel = wheelRef.current;
      const now = performance.now();
      const delta = event.deltaX;
      const size = Math.abs(delta);

      if (now - wheel.at > WHEEL_GESTURE_GAP_MS) {
        // Silence ended the previous gesture.
        wheel.spent = false;
        wheel.accum = 0;
      } else if (wheel.spent && now - wheel.spentAt > 250 && size >= 24 && size > wheel.floor * 3) {
        // Still streaming, but the deltas stopped decaying and jumped: a new swipe
        // started on top of the last one's inertia.
        wheel.spent = false;
        wheel.accum = 0;
      }
      wheel.at = now;
      if (wheel.spent) {
        // Track the decay since the last peak so a re-swipe stands out from it.
        if (size > wheel.peak) {
          wheel.peak = size;
          wheel.floor = size;
        } else {
          wheel.floor = Math.min(wheel.floor, size);
        }
        return;
      }
      wheel.accum += delta;
      if (Math.abs(wheel.accum) < WHEEL_STEP) return;
      const direction = wheel.accum > 0 ? 1 : -1;
      wheel.accum = 0;
      wheel.spent = true;
      wheel.spentAt = now;
      wheel.peak = size;
      wheel.floor = size;
      switchRelative(direction);
    },
    [switchRelative],
  );

  const onSurfaceClick = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (activeCard !== "search") return;
    const target = event.target;
    if (target instanceof Element && target.closest("button, .island-result-row, .island-filter")) return;
    inputRef.current?.focus();
  }, [activeCard]);

  const onPointerDown = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    // A drag that starts on a slider changes its value; it is not a swipe to another card.
    // A drag on a slider or inside a text field edits it; it is not a swipe to another card.
    const onControl = event.target instanceof Element && event.target.closest("input, textarea, [contenteditable]") !== null;
    pointerStartRef.current = onControl ? null : { x: event.clientX, y: event.clientY };
  }, []);

  const onPointerUp = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const start = pointerStartRef.current;
      pointerStartRef.current = null;
      if (!start) return;
      const dx = event.clientX - start.x;
      const dy = event.clientY - start.y;
      // Dragging across text to select it (to copy) must not switch cards.
      if (Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy) && !window.getSelection()?.toString()) {
        switchRelative(dx < 0 ? 1 : -1);
      }
    },
    [switchRelative],
  );

  const clipboardRows = useMemo(
    () => clipboardState.results.filter((result) => result.kind === "clipboard"),
    [clipboardState.results],
  );
  const pauseAgent = useCallback((taskId: string) => {
    void api.agentPause(taskId).catch(() => undefined);
    bootstrap.refresh();
  }, [bootstrap]);
  const resumeAgent = useCallback((taskId: string) => {
    void api.agentResume(taskId).catch(() => undefined);
    bootstrap.refresh();
  }, [bootstrap]);

  const renderCard = (cardId: IslandCardId) => {
    switch (cardId) {
      case "search":
        return (
          <SearchCard
            compact={compact}
            query={query}
            onQueryChange={(value) => {
              typedAtRef.current = performance.now();
              setQuery(value);
              setCompact(!value.trim());
            }}
            scope={scope}
            onScope={setScope}
            results={searchState.results}
            pending={searchState.pending}
            error={searchState.error}
            onOpen={openResult}
            onFocus={() => undefined}
            inputRef={inputRef}
            placeholder="Search files, folders, apps…"
          />
        );
      case "clipboard":
        return (
          <MemoClipboardCard
            results={clipboardRows}
            pending={clipboardState.pending}
            error={clipboardState.error}
            tab={clipTab}
            onTab={setClipTab}
            onOpen={openResult}
          />
        );
      case "agent":
        return (
          <MemoAgentsCard
            status={bootstrap.agentStatus}
            tasks={bootstrap.agentTasks}
            onPause={pauseAgent}
            onResume={resumeAgent}
            onChanged={bootstrap.refresh}
          />
        );
      case "browser":
        return (
          <MemoBrowserCard
            results={browserState.results}
            pending={browserState.pending}
            error={browserState.error}
            onOpen={openResult}
          />
        );
      case "git":
        return (
          <MemoGitCard
            results={gitState.results}
            pending={gitState.pending}
            error={gitState.error}
            onOpen={openResult}
          />
        );
      case "media":
        return (
          <MemoMediaCard
            status={controls.status}
            pending={controls.pending}
            error={controls.error}
            refresh={controls.refresh}
          />
        );
      case "controls":
        return (
          <MemoControlsCard
            status={controls.status}
            pending={controls.pending}
            error={controls.error}
            refresh={controls.refresh}
          />
        );
      case "bell":
        return (
          <MemoTodoCard
            onOpenMain={() => requestHide()}
          />
        );
      default:
        return (
          <UnsupportedCard
            card={cardId}
            title={islandCardLabel(cardId)}
            subtitle="Not wired yet"
            reason="This card has no native data adapter yet."
          />
        );
    }
  };

  const showDots = look.showDots;
  const anchor = settings?.placement?.anchor ?? "top_center";
  const dots = showDots ? (
    <GooeyNav
      cards={cards}
      active={activeCard}
      compact={compact}
      onSelect={(next) => switchCard(next)}
    />
  ) : null;

  return (
    <div className="island-host" data-anchor={anchor}>
      {/* Liquid glass refraction (used by the glass style's slider lens via backdrop-filter). */}
      <svg aria-hidden="true" width="0" height="0" style={{ position: "absolute" }}>
        <filter id="ps-liquid" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.03 0.05" numOctaves="2" seed="3" result="noise" />
          <feGaussianBlur in="noise" stdDeviation="1.2" result="map" />
          <feDisplacementMap in="SourceGraphic" in2="map" scale="9" xChannelSelector="R" yChannelSelector="G" />
        </filter>
      </svg>
      {/* Size and radius spring directly (not a layout/FLIP scale), so content never stretches mid-morph.
          A pill's radius is half its height; springing from 999 would hold the corners round until the end. */}
      <motion.div
        ref={surfaceRef}
        style={{ "--glass": look.liquidGlassLevel / 100 } as React.CSSProperties}
        initial={false}
        animate={{
          width: surfaceDims.width,
          height: surfaceDims.height,
          borderRadius: Math.min(surfaceDims.radius, surfaceDims.height / 2),
        }}
        transition={reduced ? { duration: 0 } : { type: "spring", duration: 0.5, bounce: 0.2 }}
        className={[
          "island-surface",
          compact ? "island-surface--compact" : "island-surface--expanded",
          look.liquidGlass ? "island-surface--glass" : "",
          resolvedTheme === "light" ? "island-surface--light" : "",
          visibility === "opening" ? "island-surface--opening" : "",
          visibility === "closing" ? "island-surface--closing" : "",
          visibility === "hidden" ? "island-surface--hidden" : "",
          stageWidth <= 380
            ? "island-surface--narrow"
            : stageWidth <= 520
              ? "island-surface--mid"
              : "",
        ]
          .filter(Boolean)
          .join(" ")}
        role="region"
        aria-label="ProtonSearch island"
        tabIndex={-1}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
        onClick={onSurfaceClick}
      >
        <div className={"island-inner" + (compact ? " island-inner--compact" : " island-scrollable")}>
          <AnimatePresence mode="popLayout" initial={false}>
            <motion.div
              key={activeCard}
              className="island-card-slot"
              data-card={activeCard}
              style={{ display: "flex" }}
              aria-hidden={false}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0, transition: { duration: reduced ? 0 : CARD_IN_S } }}
              exit={{ opacity: 0, y: -6, transition: { duration: reduced ? 0 : CARD_OUT_S } }}
            >
              {renderCard(activeCard)}
            </motion.div>
          </AnimatePresence>
        </div>
        {dots}
      </motion.div>
    </div>
  );
}
