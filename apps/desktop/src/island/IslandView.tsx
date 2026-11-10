import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../bridge/api";
import type { IslandCardId, SearchResult, SearchScope } from "../bridge/types";
import {
  AgentsCard,
  BrowserCard,
  ClipboardCard,
  GitCard,
  SearchCard,
  UnsupportedCard,
} from "./IslandCards";
import {
  ControlsCard,
  MediaCard,
  NotificationsCard,
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
const MemoNotificationsCard = memo(NotificationsCard);

/**
 * V9 Dynamic Island surface. The native window adds 96 px transparent shadow
 * padding around this surface; geometry reported here is the surface only.
 * Binding: docs/design/ISLAND_V9_SPEC.md.
 */

type Phase = "idle" | "out" | "in";

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
  const controls = useIslandControls(bootstrap.ready);


  const inputRef = useRef<HTMLInputElement>(null);
  const initialisedRef = useRef(false);
  const tokenRef = useRef(0);
  const lastSwitchRef = useRef(0);
  const closeTimerRef = useRef<number | null>(null);
  const wheelAccumRef = useRef(0);
  const wheelAtRef = useRef(0);
  const pointerStartRef = useRef<{ x: number; y: number } | null>(null);
  const typedAtRef = useRef(0);
  const hideRef = useRef<() => void>(() => undefined);
  const refreshRef = useRef<() => void>(() => undefined);

  const searchState = useIslandSearch(
    "all",
    query,
    bootstrap.ready && activeCard === "search",
  );
  // Warm every card's data in parallel while the compact pill is shown.
  const clipboardState = useIslandSearch("clipboard", "", bootstrap.ready);
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

  // task-58 geometry contract: the expanded search card has a fixed 460 px
  // surface height so typing never resizes the native window. The window is
  // resized ONCE per card/mode change (never per frame or per keystroke).
  // A fixed expanded-search surface height removes native resizes while typing;
  // the card switch path pre-resizes ONCE before the out phase.
  const surfaceDims = activeCard === "search" && !compact
    ? { width: dims.width, height: 397, radius: dims.radius }
    : dims;
  const geometryKey = surfaceDims.width + "x" + surfaceDims.height;
  const lastGeometryKeyRef = useRef("");
  const reportGeometry = useCallback((width: number, height: number) => {
    const key = Math.ceil(width) + "x" + Math.ceil(height);
    if (lastGeometryKeyRef.current === key) return;
    lastGeometryKeyRef.current = key;
    const w = (window as unknown as { __islandGeometryCalls?: number });
    w.__islandGeometryCalls = (w.__islandGeometryCalls ?? 0) + 1;
    void api.setIslandGeometry(Math.ceil(width), Math.ceil(height)).catch(() => undefined);
  }, []);
  useEffect(() => {
    reportGeometry(surfaceDims.width, surfaceDims.height);
  }, [geometryKey, reportGeometry, surfaceDims.width, surfaceDims.height]);

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
    closeTimerRef.current = window.setTimeout(finish, 120);
  }, [visibility]);

  hideRef.current = requestHide;
  refreshRef.current = bootstrap.refresh;

  const switchCard = useCallback(
    (card: IslandCardId, nextVariant: IslandVariant = "default") => {
      if (!cards.includes(card)) return;
      if (card === activeCard && nextVariant === variant) {
        if (compact) setCompact(false);
        return;
      }
      const now = Date.now();
      if (now - lastSwitchRef.current < 350) return;
      lastSwitchRef.current = now;
      const token = ++tokenRef.current;
      const targetDims = islandCardDims({
        card,
        variant: nextVariant,
        stageWidth,
        compact: false,
      });
      reportGeometry(targetDims.width, card === "search" ? 397 : targetDims.height);
      recordFrames(`island-switch:${activeCard}>${card}`, 800);
      if (prefersReducedMotion()) {
        setActiveCard(card);
        setCompact(false);
        setVariant(nextVariant);
        setPhase("idle");
        return;
      }
      setPhase("out");
      window.setTimeout(() => {
        if (token !== tokenRef.current) return;
        setActiveCard(card);
        setCompact(false);
        setVariant(nextVariant);
        setPhase("in");
        window.setTimeout(() => {
          if (token === tokenRef.current) setPhase("idle");
        }, 180);
      }, 120);
    },
    [activeCard, cards, compact, reportGeometry, stageWidth, variant],
  );

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
    if (variant !== "default") {
      setVariant("default");
      return;
    }
    if (!compact) {
      // Expanded card -> compact search (predictable Esc chain), then hide.
      setActiveCard("search");
      setCompact(true);
      setVariant("default");
      setQuery("");
      setScope("all");
      return;
    }
    requestHide();
  }, [compact, requestHide, variant]);

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
          recordFrames("island-open", 450);
          if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
          setActiveCard("search");
          setCompact(true);
          setQuery("");
          setScope("all");
          setVariant("default");
          setPhase("idle");
          setVisibility("opening");
          window.setTimeout(() => setVisibility("shown"), 180);
          refreshRef.current();
          window.setTimeout(() => inputRef.current?.focus(), 190);
        });
        addListener(offShown);
        const offDismiss = await listen("island:dismiss", () => {
          hideRef.current();
        });
        addListener(offDismiss);
      } catch {
        /* browser preview: no Tauri event bridge */
      }
    })();
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
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
      const scrollable = (event.target as Element | null)?.closest(".island-scrollable");
      if (scrollable instanceof HTMLElement) {
        const canScroll = scrollable.scrollHeight > scrollable.clientHeight;
        if (canScroll) {
          if (event.deltaY > 0 && scrollable.scrollTop + scrollable.clientHeight < scrollable.scrollHeight) return;
          if (event.deltaY < 0 && scrollable.scrollTop > 0) return;
        }
      }
      const now = Date.now();
      if (now - wheelAtRef.current > 500) wheelAccumRef.current = 0;
      wheelAtRef.current = now;
      wheelAccumRef.current += event.deltaY;
      if (Math.abs(wheelAccumRef.current) < 48) return;
      const direction = wheelAccumRef.current > 0 ? 1 : -1;
      wheelAccumRef.current = 0;
      switchRelative(direction);
    },
    [switchRelative],
  );

  const onSurfaceClick = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (!compact) return;
    const target = event.target;
    if (target instanceof Element && target.closest("button")) return;
    setCompact(false);
  }, [compact]);

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
              if (compact) setCompact(false);
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
            placeholder=""
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
          <MemoNotificationsCard
            status={controls.status}
            pending={controls.pending}
            error={controls.error}
            refresh={controls.refresh}
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