import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { api } from "../bridge/api";
import type { IslandCardId, SearchResult, SearchScope } from "../bridge/types";
import {
  AgentsCard,
  BrowserCard,
  ClipboardCard,
  GitCard,
  SearchCard,
  TodoCard,
  UnsupportedCard,
} from "./IslandCards";
import {
  ControlsCard,
  MediaCard,
} from "./NativeCards";
import { useIslandControls } from "./islandNativeCards";
import { IslandPageDots } from "./IslandPageDots";
import {
  useIslandBootstrap,
  useIslandSearch,
  type ClipTab,
} from "./islandData";
import {
  islandCardDims,
  islandCardLabel,
  islandWindowWidth,
  visibleIslandCards,
  type IslandVariant,
} from "./islandDims";
import { markTyping, recordFrames } from "../lib/perfProbe";
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

type Phase = "idle" | "out" | "in";

// Must match --island-dur-open / --island-dur-close in island.css.
const OPEN_MS = 420;
const CLOSE_MS = 150;
// Scroll delta that switches a card, and the silence that ends a scroll gesture.
const WHEEL_STEP = 48;
const WHEEL_GESTURE_GAP_MS = 160;

function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export default function IslandView() {
  const bootstrap = useIslandBootstrap();
  const settings = bootstrap.settings?.launcher?.island ?? null;
  const cards = useMemo(() => visibleIslandCards(settings), [settings]);
  // task-58: every summon is the compact oval search pill.
  const [activeCard, setActiveCard] = useState<IslandCardId>("search");
  const [compact, setCompact] = useState(true);
  const [variant, setVariant] = useState<IslandVariant>("default");
  const [clipTab, setClipTab] = useState<ClipTab>("all");
  const [phase, setPhase] = useState<Phase>("idle");
  const [visibility, setVisibility] = useState<"opening" | "shown" | "closing" | "hidden">("shown");
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<SearchScope>("all");
  const renders = (window as unknown as { __islandRenders?: number });
  renders.__islandRenders = (renders.__islandRenders ?? 0) + 1;
  // Prefetch all card data as soon as the island webview has bootstrap settings.
  // Card slots are already mounted, so the first switch is a render-only change
  // (sources-content perf finding: first switches paid mount + async fetch).
  const controls = useIslandControls(bootstrap.ready, activeCard === "media" && visibility !== "hidden");


  const inputRef = useRef<HTMLInputElement>(null);
  const initialisedRef = useRef(false);
  const tokenRef = useRef(0);
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

  useEffect(() => {
    if (activeCard === "clipboard") {
      setClipboardRevision((r) => r + 1);
    }
  }, [activeCard]);

  const searchState = useIslandSearch(
    scope,
    query,
    bootstrap.ready && activeCard === "search",
  );
  // Warm every card's data in parallel while the compact pill is shown.
  const clipboardState = useIslandSearch("clipboard", "", bootstrap.ready, clipboardRevision);
  const browserState = useIslandSearch("browser", "", bootstrap.ready);
  const gitState = useIslandSearch("git", "", bootstrap.ready);

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

  const dims = islandCardDims({
    card: activeCard,
    variant,
    stageWidth,
    compact,
    searchMatches: searchState.results.length,
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

  const surfaceDims = dims;
  // The native window keeps one width (the widest card); only its height follows
  // the card. Changing the width also re-centres the window, and that move lands
  // a frame apart from the resize: the whole island visibly jumped sideways at the
  // start of a card switch and again when the size settled afterwards.
  const windowWidth = islandWindowWidth(stageWidth);
  const lastGeometryKeyRef = useRef("");
  const geometryHeightRef = useRef(surfaceDims.height);
  const geometrySettleTimerRef = useRef<number | null>(null);
  const reportGeometry = useCallback((height: number) => {
    const key = windowWidth + "x" + Math.ceil(height);
    if (lastGeometryKeyRef.current === key) return;
    lastGeometryKeyRef.current = key;
    const w = (window as unknown as { __islandGeometryCalls?: number });
    w.__islandGeometryCalls = (w.__islandGeometryCalls ?? 0) + 1;
    void api.setIslandGeometry(windowWidth, Math.ceil(height)).catch(() => undefined);
  }, [windowWidth]);
  // Grow before the morph and shrink after it, so the surface is never clipped.
  const scheduleGeometry = useCallback((height: number, duration = 550) => {
    if (geometrySettleTimerRef.current != null) window.clearTimeout(geometrySettleTimerRef.current);
    if (prefersReducedMotion()) {
      geometryHeightRef.current = height;
      reportGeometry(height);
      geometrySettleTimerRef.current = null;
      return;
    }
    const reserved = Math.max(geometryHeightRef.current, height);
    geometryHeightRef.current = reserved;
    reportGeometry(reserved);
    geometrySettleTimerRef.current = window.setTimeout(() => {
      geometryHeightRef.current = height;
      reportGeometry(height);
      geometrySettleTimerRef.current = null;
    }, duration);
  }, [reportGeometry]);
  useLayoutEffect(() => {
    scheduleGeometry(surfaceDims.height);
  }, [scheduleGeometry, surfaceDims.height]);

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
        tokenRef.current += 1;
        const currentDims = islandCardDims({
          card,
          variant: nextVariant,
          stageWidth,
          compact: nextCompact,
          searchMatches: searchState.results.length,
        });
        scheduleGeometry(currentDims.height);
        if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
        switchTimerRef.current = null;
        pendingSwitchRef.current = null;
        setPhase("idle");
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
      const token = ++tokenRef.current;
      const targetDims = islandCardDims({
        card,
        variant: nextVariant,
        stageWidth,
        compact: nextCompact,
        searchMatches: searchState.results.length,
      });
      scheduleGeometry(targetDims.height, prefersReducedMotion() ? 0 : 670);
      recordFrames(`island-switch:${activeCard}>${card}`, 800);
      if (prefersReducedMotion()) {
        setActiveCard(card);
        setCompact(nextCompact);
        setVariant(nextVariant);
        setPhase("idle");
        return;
      }
      setPhase("out");
      window.setTimeout(() => {
        if (token !== tokenRef.current) return;
        setActiveCard(card);
        setCompact(nextCompact);
        setVariant(nextVariant);
        setPhase("in");
        window.setTimeout(() => {
          if (token === tokenRef.current) setPhase("idle");
        }, 250);
      }, 120);
    },
    [activeCard, cards, compact, query, scheduleGeometry, searchState.results.length, stageWidth, variant],
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
    tokenRef.current += 1;
    if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
    switchTimerRef.current = null;
    pendingSwitchRef.current = null;
    setPhase("idle");
    if (variant !== "default") {
      const target = islandCardDims({
        card: activeCard,
        variant: "default",
        stageWidth,
        compact,
        searchMatches: searchState.results.length,
      });
      scheduleGeometry(target.height);
      setVariant("default");
      return;
    }
    if (!compact) {
      const target = islandCardDims({ card: "search", stageWidth, compact: true, searchMatches: searchState.results.length });
      scheduleGeometry(target.height);
      // Expanded card -> compact search (predictable Esc chain), then hide.
      setActiveCard("search");
      setCompact(true);
      setVariant("default");
      setQuery("");
      setScope("all");
      return;
    }
    scheduleGeometry(dims.height, 0);
    requestHide();
    }, [activeCard, compact, dims.height, requestHide, scheduleGeometry, searchState.results.length, stageWidth, variant]);

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
          tokenRef.current += 1;
          if (switchTimerRef.current != null) window.clearTimeout(switchTimerRef.current);
          switchTimerRef.current = null;
          pendingSwitchRef.current = null;
          const target = islandCardDims({ card: "search", stageWidth, compact: true, searchMatches: 0 });
          scheduleGeometry(target.height, 0);
          recordFrames("island-open", 450);
          if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
          if (openTimerRef.current != null) window.clearTimeout(openTimerRef.current);
          setActiveCard("search");
          setCompact(true);
          setQuery("");
          setScope("all");
          setVariant("default");
          setPhase("idle");
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
  }, [scheduleGeometry, stageWidth]);

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
      const scrollable = (event.target as Element | null)?.closest(".island-scrollable, .island-results, .island-todo-list, .island-agent-list, .island-choose-list");
      if (scrollable instanceof HTMLElement) {
        // When the pointer is anywhere inside a scrollable content area, keep scrolling
        // completely contained inside the list and never let it switch cards accidentally.
        return;
      }
      // One card per scroll gesture. A touchpad swipe and its inertia tail (or a
      // flick of a free-spinning wheel) arrive as dozens of wheel events; switching
      // each time their sum crossed the threshold walked three or four cards.
      const wheel = wheelRef.current;
      const now = performance.now();
      const size = Math.abs(event.deltaY);
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
      wheel.accum += event.deltaY;
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
    pointerStartRef.current = { x: event.clientX, y: event.clientY };
  }, []);

  const onPointerUp = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      const start = pointerStartRef.current;
      pointerStartRef.current = null;
      if (!start) return;
      const dx = event.clientX - start.x;
      const dy = event.clientY - start.y;
      if (Math.abs(dx) > 48 && Math.abs(dx) > Math.abs(dy)) {
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
            total={searchState.total}
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

  const cardSlots = cards.map((cardId) => (
    <div
      key={cardId}
      className="island-card-slot"
      style={{ display: cardId === activeCard ? "flex" : "none" }}
      aria-hidden={cardId !== activeCard}
    >
      {renderCard(cardId)}
    </div>
  ));

  const showDots = settings?.showDots !== false;
  const dots = showDots ? (
    <IslandPageDots
      cards={cards}
      active={activeCard}
      compact={compact}
      onSelect={(next) => switchCard(next)}
    />
  ) : null;

  return (
    <div className="island-host" data-anchor={settings?.placement?.anchor ?? "top_center"}>
      <div
        className={[
          "island-surface",
          compact ? "island-surface--compact" : "island-surface--expanded",
          phase === "out" ? "island-surface--out" : "",
          phase === "in" ? "island-surface--in" : "",
          visibility === "opening" ? "island-surface--opening" : "",
          visibility === "closing" ? "island-surface--closing" : "",
          visibility === "hidden" ? "island-surface--hidden" : "",
          activeCard === "media" ? "island-surface--media" : "",
          stageWidth <= 380
            ? "island-surface--narrow"
            : stageWidth <= 520
              ? "island-surface--mid"
              : "",
        ]
          .filter(Boolean)
          .join(" ")}
        style={{
          width: surfaceDims.width + "px",
          height: surfaceDims.height + "px",
          borderRadius: surfaceDims.radius + "px",
        }}
        role="region"
        aria-label="ProtonSearch island"
        tabIndex={-1}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
        onClick={onSurfaceClick}
      >
        <div className={"island-inner" + (compact ? " island-inner--compact" : " island-scrollable")}>
          {cardSlots}
        </div>
        {dots}
      </div>
    </div>
  );
}
