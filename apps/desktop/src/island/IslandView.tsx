import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import "./island.css";

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
  const controls = useIslandControls();
  const settings = bootstrap.settings?.launcher?.island ?? null;
  const cards = useMemo(() => visibleIslandCards(settings), [settings]);
  const defaultCard: IslandCardId = useMemo(() => {
    const configured = settings?.defaultCard;
    if (configured && cards.includes(configured)) return configured;
    return cards[0] ?? "search";
  }, [cards, settings?.defaultCard]);

  const [activeCard, setActiveCard] = useState<IslandCardId>(defaultCard);
  const [compact, setCompact] = useState(defaultCard === "search");
  const [variant, setVariant] = useState<IslandVariant>("default");
  const [clipTab, setClipTab] = useState<ClipTab>("all");
  const [phase, setPhase] = useState<Phase>("idle");
  const [visibility, setVisibility] = useState<"opening" | "shown" | "closing" | "hidden">("shown");
  const [query, setQuery] = useState("");
  const [scope, setScope] = useState<SearchScope>("all");


  const inputRef = useRef<HTMLInputElement>(null);
  const initialisedRef = useRef(false);
  const tokenRef = useRef(0);
  const lastSwitchRef = useRef(0);
  const closeTimerRef = useRef<number | null>(null);
  const wheelAccumRef = useRef(0);
  const wheelAtRef = useRef(0);
  const pointerStartRef = useRef<{ x: number; y: number } | null>(null);

  const searchExpanded = activeCard === "search" && !compact;
  const searchState = useIslandSearch("all", query, searchExpanded);
  const clipboardState = useIslandSearch("clipboard", "", activeCard === "clipboard");
  const browserState = useIslandSearch("browser", "", activeCard === "browser");
  const gitState = useIslandSearch("git", "", activeCard === "git");

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
  const dimsRef = useRef(dims);
  dimsRef.current = dims;

  // Reset to the configured default when the shell summons the island.
  useEffect(() => {
    if (initialisedRef.current || !bootstrap.ready) return;
    initialisedRef.current = true;
    setActiveCard(defaultCard);
    setCompact(defaultCard === "search");
  }, [bootstrap.ready, defaultCard]);

  // Surface bbox contract: report the TARGET surface size once per settled
  // dims change. A ResizeObserver during the 550 ms CSS morph caused a native
  // window-resize storm; the window adds the 112 px shadow pad itself.
  useEffect(() => {
    const timer = window.setTimeout(() => {
      void api.setIslandGeometry(dims.width, dims.height).catch(() => undefined);
    }, 80);
    return () => window.clearTimeout(timer);
  }, [dims.width, dims.height]);

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
    setVisibility("closing");
    if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
    closeTimerRef.current = window.setTimeout(finish, 120);
  }, [visibility]);

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
        }, 250);
      }, 140);
    },
    [activeCard, cards, compact, variant],
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
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        if (disposed) return;
        unlisteners.push(
          await listen("island:shown", () => {
            if (closeTimerRef.current != null) window.clearTimeout(closeTimerRef.current);
            setActiveCard(defaultCard);
            setCompact(defaultCard === "search");
            setVariant("default");
            setPhase("idle");
            setVisibility("opening");
            window.setTimeout(() => setVisibility("shown"), 180);
            bootstrap.refresh();
            window.setTimeout(() => inputRef.current?.focus(), 190);
          }),
        );
        unlisteners.push(
          await listen("island:dismiss", () => {
            requestHide();
          }),
        );
      } catch {
        /* browser preview: no Tauri event bridge */
      }
    })();
    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [bootstrap, defaultCard, requestHide]);

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

  const card = (() => {
    switch (activeCard) {
      case "search":
        return (
          <SearchCard
            compact={compact}
            query={query}
            onQueryChange={(value) => {
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
            onFocus={() => {
              if (compact) setCompact(false);
            }}
            inputRef={inputRef}
            placeholder={
              stageWidth <= 380
                ? "Search…"
                : "Search files, folders, apps and actions…"
            }
          />
        );
      case "clipboard": {
        // Scope search can return non-clipboard rows in mock/preview builds;
        // the card is strictly clipboard history.
        const clipboardRows = clipboardState.results.filter(
          (result) => result.kind === "clipboard",
        );
        return (
          <ClipboardCard
            results={clipboardRows}
            pending={clipboardState.pending}
            error={clipboardState.error}
            tab={clipTab}
            onTab={setClipTab}
            onOpen={openResult}
          />
        );
      }
      case "agent":
        return (
          <AgentsCard
            status={bootstrap.agentStatus}
            tasks={bootstrap.agentTasks}
            onPause={(taskId) => {
              void api.agentPause(taskId).catch(() => undefined);
              bootstrap.refresh();
            }}
            onResume={(taskId) => {
              void api.agentResume(taskId).catch(() => undefined);
              bootstrap.refresh();
            }}
          />
        );
      case "browser":
        return (
          <BrowserCard
            results={browserState.results}
            pending={browserState.pending}
            error={browserState.error}
            onOpen={openResult}
          />
        );
      case "git":
        return (
          <GitCard
            results={gitState.results}
            pending={gitState.pending}
            error={gitState.error}
            onOpen={openResult}
          />
        );
      case "media":
        return (
          <MediaCard
            status={controls.status}
            pending={controls.pending}
            error={controls.error}
            refresh={controls.refresh}
          />
        );
      case "controls":
        return (
          <ControlsCard
            status={controls.status}
            pending={controls.pending}
            error={controls.error}
            refresh={controls.refresh}
          />
        );
      case "bell":
        return (
          <NotificationsCard
            status={controls.status}
            pending={controls.pending}
            error={controls.error}
            refresh={controls.refresh}
          />
        );
      default:
        return (
          <UnsupportedCard
            card={activeCard}
            title={islandCardLabel(activeCard)}
            subtitle="Not wired yet"
            reason="This card has no native data adapter yet."
          />
        );
    }
  })();

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
    <div className="island-host">
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
        style={{ width: dims.width + "px", height: dims.height + "px", borderRadius: dims.radius + "px" }}
        role="region"
        aria-label="ProtonSearch island"
        tabIndex={-1}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
      >
        <div className={"island-inner" + (compact ? " island-inner--compact" : " island-scrollable")}>
          {card}
        </div>
        {dots}
      </div>
    </div>
  );
}
