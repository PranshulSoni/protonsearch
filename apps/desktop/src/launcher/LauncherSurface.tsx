import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type RefObject,
  type WheelEvent as ReactWheelEvent,
} from "react";
import type {
  ActionSpec,
  Density,
  EngineStatus,
  NativeCommand,
  OpenOutcome,
  SearchRequest,
  SearchResponse,
  SearchResult,
  SearchScope,
} from "../bridge/types";
import { Kbd } from "../components/Kbd";
import { Logo } from "../components/Logo";
import { isTauriRuntime } from "../bridge/api";
import { groupResults } from "../lib/results";
import { formatNumber } from "../lib/format";
import {
  SCOPE_CHIPS,
  clipboardSectionDomId,
  flattenClipboardSections,
  groupClipboardSections,
  guardScopeResults,
  prefixScope,
  scopeIndex,
  scopeLabel as scopeLabelText,
  stripScopePrefix,
} from "./launcherScopes";
import {
  QUICK_DESTINATIONS_HEIGHT,
  QUICK_DESTINATIONS_HEIGHT_COMPACT,
  SEARCH_SYNTAX_HINT,
  buildQuickDestinations,
  quickDestinationDomId,
} from "./launcherQuickDestinations";
import { QuickDestinationOptions } from "./QuickDestinations";
import { SharedResultRow } from "./SharedResultRow";
import { resultMetaText } from "./resultMeta";
import { ResultPreview } from "./ResultPreview";
import { ResultActions, buildMenuActions, type MenuAction, type MenuModel } from "./ResultActions";
import {
  EMPTY_SELECTION,
  MAX_RENDERED_ROWS,
  PAGE_STEP,
  SEARCH_LIMIT,
  clampedIndex,
  commitSelection,
  computeLauncherMetrics,
  countMatches,
  descriptionDomId,
  getWorkAreaSize,
  groupLabelDomId,
  metaDomId,
  optionDomId,
  scrollTopForIndex,
  wrappedIndex,
  type LauncherMetrics,
  type SelectionState,
  type WorkAreaSize,
} from "./launcherLayout";
import "./launcher.css";

/**
 * Quick launcher surface. Binding specs:
 * - docs/design/SCREENS.md §2 (structure, states, copy, footer, preview)
 * - docs/design/KEYBOARD_AND_A11Y.md §1, §2, §4, §7 (focus, keys, stale rules, ARIA)
 *
 * Rendering is bounded (<= 50 rows) with fixed row heights; no measuring in
 * handlers, no animated layout, selection is a colour change only.
 */

export interface LauncherSurfaceProps {
  search: (request: SearchRequest) => Promise<SearchResponse>;
  openResult: (action: ActionSpec) => Promise<OpenOutcome>;
  hideLauncher: () => void;
  showMain: () => void;
  status: EngineStatus | null;
  /** Settings.privacy.clipboardHistory; hides the category row and explains off. */
  clipboardHistoryEnabled?: boolean;
  /** Server-side confirmation for destructive native commands (V2-04). */
  confirmNativeAction?: (command: NativeCommand) => Promise<OpenOutcome>;
  density?: Density;
  /** Overridable for tests; defaults to the screen work area. */
  workArea?: WorkAreaSize;
  onGeometry?: (height: number, previewOpen: boolean) => void;
}

export interface LauncherSurfaceHandle {
  /** Reset to an empty query, focus the input, select the first row, animate in. */
  summon: () => void;
  /** Play the exit animation, then ask the shell to hide the window. */
  dismiss: () => void;
  /** Clear all launcher state (window hidden). */
  clear: () => void;
}

/** Exit motion contract (task-1): 110 ms close; the 150 ms enter runs in CSS. */
const CLOSE_MS = 110;

function nextFrame(callback: () => void): void {
  if (typeof window.requestAnimationFrame === "function") {
    window.requestAnimationFrame(callback);
    return;
  }
  window.setTimeout(callback, 16);
}

function prefersReducedMotion(): boolean {
  if (typeof document !== "undefined" && document.documentElement.dataset.motion === "reduced") {
    return true;
  }
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

const LISTBOX_ID = "ps-results";

interface CommittedResponse {
  epoch: number;
  query: string;
  results: SearchResult[];
}

type FooterTone = "positive" | "warning" | "neutral" | "negative";

interface FooterStatus {
  word: string;
  tone: FooterTone;
  detail?: string;
}

interface FooterHint {
  keyLabel: string;
  label: string;
  /** Lowest-priority hint; hidden by CSS when the surface is very narrow. */
  optional?: boolean;
}

/** Polite live-region throttle: at most one count announcement per 300 ms. */
function useThrottledLiveText(text: string, delayMs = 300): string {
  const [liveText, setLiveText] = useState(text);
  const latestRef = useRef(text);
  const lastUpdateRef = useRef(0);
  const timerRef = useRef<number | null>(null);
  latestRef.current = text;
  useEffect(() => {
    if (liveText === text) return;
    const now = Date.now();
    const elapsed = now - lastUpdateRef.current;
    if (elapsed >= delayMs) {
      lastUpdateRef.current = now;
      setLiveText(text);
      return;
    }
    if (timerRef.current != null) return;
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      lastUpdateRef.current = Date.now();
      setLiveText(latestRef.current);
    }, delayMs - elapsed);
  }, [text, liveText, delayMs]);
  useEffect(
    () => () => {
      if (timerRef.current != null) window.clearTimeout(timerRef.current);
    },
    [],
  );
  return liveText;
}

function hidesLauncher(action: ActionSpec): boolean {
  return action.type !== "none" && action.type !== "copy_text" && action.type !== "copy_image";
}

function countTextFor(
  pending: boolean,
  response: SearchResponse | null,
  searchError: string | null,
  query: string,
  overrideTotal?: number,
): string {
  if (searchError) return "Search unavailable";
  if (pending) return "Searching…";
  if (!response) return "";
  // An empty query with no recents is the quiet placeholder state, not "0 matches".
  if (overrideTotal === undefined && response.total === 0 && query.trim().length === 0) return "";
  return countMatches(overrideTotal ?? response.total);
}

/**
 * Truthful index item count: the larger of the search-response counter and
 * EngineStatus.filenameEntries. Never renders "0 items" while either source has
 * entries (the engine can report 0 in a SearchStatus before its counter is set).
 */
function itemCountFor(
  status: EngineStatus | null,
  response: SearchResponse | null,
): number | undefined {
  const searchStatus = response?.status;
  const fromSearch =
    searchStatus != null &&
    (searchStatus.type === "ready" || searchStatus.type === "indexing")
      ? searchStatus.indexedFiles
      : 0;
  const fromEngine = status?.filenameEntries ?? 0;
  const count = Math.max(
    Number.isFinite(fromSearch) ? fromSearch : 0,
    Number.isFinite(fromEngine) ? fromEngine : 0,
  );
  return count > 0 ? count : undefined;
}

function deriveFooterStatus(
  status: EngineStatus | null,
  response: SearchResponse | null,
  searchError: string | null,
): FooterStatus {
  if (searchError) return { word: "Index unavailable", tone: "negative", detail: searchError };
  // The search response is the freshest engine status available to the launcher
  // window (it does not poll EngineStatus). Prefer it so the footer never keeps
  // saying "Indexing…" after the engine has reported ready.
  if (response?.status.type === "degraded") {
    return { word: "Index unavailable", tone: "negative", detail: response.status.reason };
  }
  if (response?.status.type === "ready") {
    return { word: "Index ready", tone: "positive" };
  }
  if (response?.status.type === "indexing") {
    return { word: "Indexing…", tone: "warning" };
  }
  switch (status?.state) {
    case "ready":
      return { word: "Index ready", tone: "positive" };
    case "indexing":
      return { word: "Indexing…", tone: "warning" };
    case "paused":
      return { word: "Index paused", tone: "neutral" };
    case "error":
      return {
        word: "Index unavailable",
        tone: "negative",
        detail: status.warnings.join("\n") || undefined,
      };
    case "starting":
    default:
      return { word: "Starting…", tone: "neutral" };
  }
}

function deriveHints(resultsCount: number, query: string, previewOpen: boolean): FooterHint[] {
  if (previewOpen) {
    return [
      { keyLabel: "Ctrl+Space", label: "Close preview", optional: true },
      { keyLabel: "Esc", label: "Close" },
    ];
  }
  if (resultsCount === 0 && query.length > 0) {
    return [{ keyLabel: "Esc", label: "Clear" }];
  }
  // At most three hints, in priority order. "Navigate" is the first to go when
  // the surface is too narrow for the row (launcher.css, <= 620 px).
  return [
    { keyLabel: "↑ ↓", label: "Navigate", optional: true },
    { keyLabel: "Enter", label: "Open" },
    { keyLabel: "Esc", label: query.length > 0 ? "Clear" : "Close" },
  ];
}

const LauncherSurface = forwardRef<LauncherSurfaceHandle, LauncherSurfaceProps>(
  function LauncherSurface(props, ref) {
    const { status, density, onGeometry, clipboardHistoryEnabled = true } = props;
    const workArea = props.workArea ?? getWorkAreaSize();

    const [query, setQuery] = useState("");
    const [response, setResponse] = useState<SearchResponse | null>(null);
    const [pending, setPending] = useState(false);
    const [searchError, setSearchError] = useState<string | null>(null);
    const [selection, setSelection] = useState<SelectionState>(EMPTY_SELECTION);
    const [previewOpen, setPreviewOpen] = useState(false);
    const [menuOpen, setMenuOpen] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);
    const [announcement, setAnnouncement] = useState("");
    const [listScrollTop, setListScrollTop] = useState(0);
    const [scope, setScope] = useState<SearchScope>("all");
    const [quickIndex, setQuickIndex] = useState(0);
    const [motionPhase, setMotionPhase] = useState<"closed" | "entering" | "visible" | "closing">(
      () => (isTauriRuntime() ? "closed" : "visible"),
    );
    const [pendingConfirmation, setPendingConfirmation] = useState<{
      command: NativeCommand;
      message: string;
    } | null>(null);

    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);
    const footerRef = useRef<HTMLButtonElement>(null);
    const menuRef = useRef<HTMLDivElement | null>(null);
    const confirmCancelRef = useRef<HTMLButtonElement>(null);
    const chipRefs = useRef<(HTMLButtonElement | null)[]>([]);

    const requestIdRef = useRef(0);
    const epochRef = useRef(0);
    const epochQueryRef = useRef("");
    const epochScopeRef = useRef<SearchScope>("all");
    const queryRef = useRef("");
    const scopeRef = useRef<SearchScope>("all");
    const motionPhaseRef = useRef<"closed" | "entering" | "visible" | "closing">(
      isTauriRuntime() ? "closed" : "visible",
    );
    const motionTimerRef = useRef<number | null>(null);
    const composingRef = useRef(false);
    const selectionRef = useRef<SelectionState>(EMPTY_SELECTION);
    const resultsRef = useRef<SearchResult[]>([]);
    const committedRef = useRef<CommittedResponse | null>(null);
    const previewOpenRef = useRef(false);
    const bootedRef = useRef(false);
    const noticeTimerRef = useRef<number | null>(null);
    const lastGeometryRef = useRef<{ height: number; previewOpen: boolean } | null>(null);

    previewOpenRef.current = previewOpen;
    scopeRef.current = scope;
    motionPhaseRef.current = motionPhase;

    const clearMotionTimer = useCallback(() => {
      if (motionTimerRef.current != null) {
        window.clearTimeout(motionTimerRef.current);
        motionTimerRef.current = null;
      }
    }, []);

    const rawResults = useMemo(
      () => (response ? response.results.slice(0, MAX_RENDERED_ROWS) : []),
      [response],
    );
    // A typed prefix is the only indicator for the additional sources; the
    // visible chip stays on the four filters (v10 Normal Launcher).
    const prefixedScope = prefixScope(query);
    const activeScope = prefixedScope ?? scope;
    // Strict FILES/FOLDERS guard until ps-core task-19 lands (alpha debt).
    const results = useMemo(
      () => guardScopeResults(rawResults, activeScope),
      [rawResults, activeScope],
    );
    const groups = useMemo(
      () => (activeScope === "clipboard" ? [] : groupResults(results)),
      [results, activeScope],
    );
    // CLIPBOARD results render as clean category sections (task-25); keyboard
    // order is the flattened visual order of those sections.
    const clipboardSections = useMemo(
      () => (activeScope === "clipboard" ? groupClipboardSections(results) : []),
      [results, activeScope],
    );
    const displaySections = useMemo(() => {
      if (activeScope === "clipboard") {
        return clipboardSections.map((section) => ({
          key: "clipboard:" + section.id,
          label: section.label,
          labelDomId: clipboardSectionDomId(section.id),
          items: section.items,
          subgroup: true,
        }));
      }
      return groups.map((group) => ({
        key: group.kind as string,
        label: group.label,
        labelDomId: groupLabelDomId(group.kind),
        items: group.items.map((item) => item.result),
        subgroup: false,
      }));
    }, [activeScope, clipboardSections, groups]);
    const orderedResults = useMemo(
      () => displaySections.flatMap((section) => section.items),
      [displaySections],
    );
    resultsRef.current = orderedResults;
    const visualIndexById = useMemo(() => {
      const map = new Map<string, number>();
      orderedResults.forEach((item, index) => map.set(item.id, index));
      return map;
    }, [orderedResults]);
    const groupRowCounts = useMemo(
      () => displaySections.map((section) => section.items.length),
      [displaySections],
    );

    const activeIndex = useMemo(() => {
      if (orderedResults.length === 0) return -1;
      if (selection.resultId) {
        const found = orderedResults.findIndex((result) => result.id === selection.resultId);
        if (found >= 0) return found;
      }
      return clampedIndex(selection.index, orderedResults.length);
    }, [orderedResults, selection]);

    const selectedResult = activeIndex >= 0 ? orderedResults[activeIndex] : null;

    const quickDestinations = useMemo(() => buildQuickDestinations(status), [status]);
    const quickActive =
      query.length === 0 &&
      activeScope === "all" &&
      results.length === 0 &&
      (response?.total ?? 0) === 0 &&
      quickDestinations.length > 0 &&
      searchError === null &&
      !(pending && response === null);
    const metrics = useMemo<LauncherMetrics>(
      () =>
        computeLauncherMetrics({
          groupRowCounts,
          density,
          previewOpen,
          confirmOpen: pendingConfirmation !== null,
          extraContentHeight: quickActive
            ? density === "compact"
              ? QUICK_DESTINATIONS_HEIGHT_COMPACT
              : QUICK_DESTINATIONS_HEIGHT
            : 0,
          workArea,
        }),
      [
        groupRowCounts,
        density,
        previewOpen,
        pendingConfirmation,
        quickActive,
        workArea,
      ],
    );

    // Fixed-height display entries for scroll-into-view, including clipboard
    // sections and empty-state quick destinations (no rows).
    const flatEntries = useMemo(() => {
      const entries: Array<{
        type: "group" | "row";
        key: string;
        label?: string;
        result?: SearchResult;
        index?: number;
        top: number;
        height: number;
      }> = [];
      let top = 10;
      let visualIndex = 0;
      for (const section of displaySections) {
        entries.push({
          type: "group",
          key: "group:" + section.key,
          label: section.label,
          top,
          height: metrics.labelHeight,
        });
        top += metrics.labelHeight;
        for (const item of section.items) {
          entries.push({
            type: "row",
            key: "row:" + item.id,
            result: item,
            index: visualIndex,
            top,
            height: metrics.rowHeight,
          });
          visualIndex += 1;
          top += metrics.rowHeight;
        }
      }
      return { entries, contentHeight: top - 10 };
    }, [displaySections, metrics.rowHeight, metrics.labelHeight]);

    const menuModel: MenuModel | null = useMemo(
      () => (selectedResult ? buildMenuActions(selectedResult) : null),
      [selectedResult],
    );

    const announce = useCallback((text: string) => {
      setAnnouncement(text);
    }, []);

    const showNotice = useCallback((text: string, autoClear = true) => {
      if (noticeTimerRef.current != null) {
        window.clearTimeout(noticeTimerRef.current);
        noticeTimerRef.current = null;
      }
      setNotice(text);
      if (autoClear) {
        noticeTimerRef.current = window.setTimeout(() => {
          noticeTimerRef.current = null;
          setNotice(null);
        }, 2000);
      }
    }, []);

    useEffect(
      () => () => {
        if (noticeTimerRef.current != null) window.clearTimeout(noticeTimerRef.current);
      },
      [],
    );

    const focusInput = useCallback(() => {
      const input = inputRef.current;
      if (!input) return;
      input.focus();
      const end = input.value.length;
      try {
        input.setSelectionRange(end, end);
      } catch {
        /* not all input types support selection ranges */
      }
    }, []);

    const commitResponse = useCallback((next: SearchResponse, epoch: number) => {
      const nextResults = next.results.slice(0, MAX_RENDERED_ROWS);
      // Selection is tracked against the visible rows; the committed guard keeps
      // the full engine set so a category change never invalidates the epoch.
      const orderedScope = prefixScope(next.query) ?? scopeRef.current;
      const guarded = guardScopeResults(nextResults, orderedScope);
      const ordered =
        orderedScope === "clipboard"
          ? flattenClipboardSections(groupClipboardSections(guarded))
          : groupResults(guarded).flatMap((group) =>
              group.items.map((item) => item.result),
            );
      const nextSelection = commitSelection(selectionRef.current, ordered, epoch);
      selectionRef.current = nextSelection;
      setSelection(nextSelection);
      resultsRef.current = ordered;
      committedRef.current = { epoch, query: next.query, results: nextResults };
      setResponse(next);
      setMenuOpen(false);
    }, []);

    const runSearch = useCallback(
      async (nextQuery: string, nextScope: SearchScope = scopeRef.current) => {
        const requestId = ++requestIdRef.current;
        const epoch =
          nextQuery === epochQueryRef.current && nextScope === epochScopeRef.current
            ? epochRef.current
            : ++epochRef.current;
        epochQueryRef.current = nextQuery;
        epochScopeRef.current = nextScope;
        queryRef.current = nextQuery;
        scopeRef.current = nextScope;
        setPending(true);
        setSearchError(null);
        setNotice(null);
        setPendingConfirmation(null);
        try {
          const next = await props.search({
            requestId,
            query: nextQuery,
            scope: nextScope,
            limit: SEARCH_LIMIT,
          });
          // Drop stale responses: only the newest request may render.
          if (requestId !== requestIdRef.current) return;
          commitResponse(next, epoch);
        } catch (error) {
          if (requestId !== requestIdRef.current) return;
          committedRef.current = null;
          resultsRef.current = [];
          selectionRef.current = EMPTY_SELECTION;
          setSelection(EMPTY_SELECTION);
          setResponse(null);
          setMenuOpen(false);
          setSearchError(error instanceof Error ? error.message : String(error));
        } finally {
          if (requestId === requestIdRef.current) setPending(false);
        }
      },
      [props.search, commitResponse],
    );

    const resetState = useCallback((nextQuery: string) => {
      requestIdRef.current += 1; // invalidate any in-flight response
      queryRef.current = nextQuery;
      epochQueryRef.current = nextQuery;
      epochScopeRef.current = scopeRef.current;
      epochRef.current += 1;
      committedRef.current = null;
      resultsRef.current = [];
      selectionRef.current = EMPTY_SELECTION;
      setSelection(EMPTY_SELECTION);
      setQuery(nextQuery);
      setResponse(null);
      setPending(false);
      setSearchError(null);
      setNotice(null);
      setMenuOpen(false);
      setPendingConfirmation(null);
    }, []);

    const setMotion = useCallback((phase: "closed" | "entering" | "visible" | "closing") => {
      motionPhaseRef.current = phase;
      setMotionPhase(phase);
    }, []);

    const startEnterMotion = useCallback(() => {
      clearMotionTimer();
      if (prefersReducedMotion()) {
        setMotion("visible");
        return;
      }
      setMotion("entering");
      // Two frames: the entering style paints first, then the visible style
      // transitions in. No layout work happens in this window.
      nextFrame(() => {
        nextFrame(() => setMotion("visible"));
      });
    }, [clearMotionTimer, setMotion]);

    const summon = useCallback(() => {
      resetState("");
      setPreviewOpen(false);
      focusInput();
      startEnterMotion();
      void runSearch("", scopeRef.current);
    }, [resetState, focusInput, runSearch, startEnterMotion]);

    const dismiss = useCallback(() => {
      if (motionPhaseRef.current === "closing") return;
      clearMotionTimer();
      if (prefersReducedMotion()) {
        setMotion("closed");
        props.hideLauncher();
        return;
      }
      setMotion("closing");
      motionTimerRef.current = window.setTimeout(() => {
        motionTimerRef.current = null;
        setMotion("closed");
        props.hideLauncher();
      }, CLOSE_MS + 10);
    }, [clearMotionTimer, props.hideLauncher, setMotion]);

    const clear = useCallback(() => {
      clearMotionTimer();
      setMotion("closed");
      resetState("");
      setPreviewOpen(false);
    }, [clearMotionTimer, resetState, setMotion]);

    useImperativeHandle(ref, () => ({ summon, dismiss, clear }), [summon, dismiss, clear]);

    useEffect(() => {
      if (bootedRef.current) return;
      bootedRef.current = true;
      focusInput();
      void runSearch("", scopeRef.current);
    }, [focusInput, runSearch]);

    useEffect(
      () => () => {
        clearMotionTimer();
      },
      [clearMotionTimer],
    );

    // Request window geometry only when the computed height/preview changes.
    useEffect(() => {
      const last = lastGeometryRef.current;
      if (last && last.height === metrics.totalHeight && last.previewOpen === previewOpen) {
        return;
      }
      lastGeometryRef.current = { height: metrics.totalHeight, previewOpen };
      onGeometry?.(metrics.totalHeight, previewOpen);
    }, [metrics.totalHeight, previewOpen, onGeometry]);

    // Keep the selected row visible with pure arithmetic (no measuring).
    useLayoutEffect(() => {
      if (activeIndex < 0) return;
      const list = listRef.current;
      if (!list) return;
      const target = scrollTopForIndex(
        flatEntries.entries,
        activeIndex,
        metrics.listViewport,
        list.scrollTop,
      );
      if (target != null) list.scrollTop = target;
    }, [activeIndex, flatEntries, metrics.listViewport]);

    const currentIndex = useCallback(() => {
      const current = selectionRef.current;
      if (!current.resultId) return -1;
      return resultsRef.current.findIndex((result) => result.id === current.resultId);
    }, []);

    const selectIndex = useCallback(
      (index: number) => {
        const list = resultsRef.current;
        if (list.length === 0) return;
        const next = clampedIndex(index, list.length);
        const state: SelectionState = {
          resultId: list[next].id,
          index: next,
          epoch: epochRef.current,
        };
        selectionRef.current = state;
        setSelection(state);
      },
      [],
    );

    const move = useCallback(
      (delta: number) => {
        const list = resultsRef.current;
        if (list.length === 0) return;
        const from = currentIndex();
        if (from < 0) {
          selectIndex(delta > 0 ? 0 : list.length - 1);
          return;
        }
        selectIndex(wrappedIndex(from, delta, list.length));
      },
      [currentIndex, selectIndex],
    );

    const showConfirmation = useCallback((command: NativeCommand, message: string) => {
      setMenuOpen(false);
      setPendingConfirmation({ command, message });
    }, []);

    const cancelConfirmation = useCallback(() => {
      setPendingConfirmation(null);
      announce("Action cancelled");
      focusInput();
    }, [announce, focusInput]);

    const runOpenAction = useCallback(
      async (action: ActionSpec, successNotice?: string) => {
        if (action.type === "none") return;
        // Native commands answer with a confirmation challenge instead of
        // executing destructively; keep the window visible until confirmed.
        const deferHide = action.type === "run_native";
        if (!deferHide && hidesLauncher(action)) props.hideLauncher();
        try {
          const outcome = await props.openResult(action);
          if (
            !outcome.ok &&
            action.type === "run_native" &&
            outcome.message.startsWith("Confirmation required")
          ) {
            showConfirmation(action.command, outcome.message);
            return;
          }
          if (!outcome.ok) {
            showNotice(outcome.message || "The action could not be completed.", false);
            return;
          }
          if (deferHide) props.hideLauncher();
          if (action.type === "copy_text") showNotice(successNotice ?? "Copied");
        } catch (error) {
          showNotice(
            "Couldn't run action: " + (error instanceof Error ? error.message : String(error)),
            false,
          );
        }
      },
      [props.hideLauncher, props.openResult, showConfirmation, showNotice],
    );

    const confirmPendingAction = useCallback(async () => {
      const pending = pendingConfirmation;
      if (!pending) return;
      setPendingConfirmation(null);
      const confirm = props.confirmNativeAction;
      if (!confirm) {
        showNotice("Confirmation is not available in this build.", false);
        focusInput();
        return;
      }
      try {
        const outcome = await confirm(pending.command);
        if (outcome.ok) {
          props.hideLauncher();
        } else {
          showNotice(outcome.message || "The action could not be completed.", false);
          focusInput();
        }
      } catch (error) {
        showNotice(
          "Couldn't confirm action: " + (error instanceof Error ? error.message : String(error)),
          false,
        );
        focusInput();
      }
    }, [
      pendingConfirmation,
      props.confirmNativeAction,
      props.hideLauncher,
      showNotice,
      focusInput,
    ]);

    useEffect(() => {
      if (pendingConfirmation) confirmCancelRef.current?.focus();
    }, [pendingConfirmation]);

    const commitIsCurrent = useCallback(() => {
      const committed = committedRef.current;
      return (
        committed !== null &&
        committed.epoch === epochRef.current &&
        committed.query === queryRef.current
      );
    }, []);

    const openResultFor = useCallback(
      (result: SearchResult) => {
        if (!commitIsCurrent()) {
          announce("Result changed — choose again.");
          return;
        }
        const current = committedRef.current?.results.find((item) => item.id === result.id);
        if (!current) {
          announce("Result changed — choose again.");
          return;
        }
        void runOpenAction(current.action);
      },
      [announce, commitIsCurrent, runOpenAction],
    );

    const openActive = useCallback(() => {
      // Enter is a no-op with no rows or in the degraded/error state
      // (KEYBOARD_AND_A11Y §2) — never a surprise fallback action.
      if (searchError !== null || resultsRef.current.length === 0) return;
      const selected = selectionRef.current.resultId;
      if (!selected || !commitIsCurrent()) {
        announce("Result changed — choose again.");
        return;
      }
      const current = committedRef.current?.results.find((item) => item.id === selected);
      if (!current) {
        announce("Result changed — choose again.");
        return;
      }
      void runOpenAction(current.action);
    }, [announce, commitIsCurrent, runOpenAction, searchError]);

    const openMenu = useCallback(() => {
      const selected = selectionRef.current.resultId;
      if (!selected || !commitIsCurrent()) return;
      const current = committedRef.current?.results.find((item) => item.id === selected);
      if (!current) return;
      if (buildMenuActions(current).actions.length === 0) return;
      setMenuOpen(true);
    }, [commitIsCurrent]);

    const closeMenu = useCallback(() => {
      setMenuOpen(false);
      focusInput();
    }, [focusInput]);

    const runMenuAction = useCallback(
      (item: MenuAction) => {
        setMenuOpen(false);
        focusInput();
        void runOpenAction(item.action);
      },
      [focusInput, runOpenAction],
    );

    const copySelected = useCallback(() => {
      if (!commitIsCurrent()) {
        announce("Result changed — choose again.");
        return;
      }
      const selected = selectionRef.current.resultId;
      const result = committedRef.current?.results.find((item) => item.id === selected);
      if (!result) return;
      const copyActions = (result.actions ?? []).filter(
        (item) => !item.destructive && item.action.type === "copy_text",
      );
      const declared =
        copyActions.find((item) => /path/i.test(item.id + " " + item.label)) ?? copyActions[0];
      let action: ActionSpec | null = declared ? declared.action : null;
      if (!action) {
        if (result.kind === "browser_history" || result.kind === "bookmark") {
          const url = result.path ?? result.subtitle;
          if (url) action = { type: "copy_text", text: url };
        } else if (result.path) {
          action = { type: "copy_text", text: result.path };
        } else if (result.kind === "clipboard") {
          action = { type: "copy_text", text: result.title };
        }
      }
      if (!action) return;
      void runOpenAction(action, "Path copied");
    }, [announce, commitIsCurrent, runOpenAction]);

    const applyScope = useCallback(
      (nextScope: SearchScope, focusTarget: "input" | "chip" = "input") => {
        if (nextScope === scopeRef.current) {
          if (focusTarget === "input") focusInput();
          return;
        }
        scopeRef.current = nextScope;
        setScope(nextScope);
        const chip = SCOPE_CHIPS.find((candidate) => candidate.scope === nextScope);
        if (chip) announce(chip.label + " filter");
        // Choosing a chip clears a leading engine scope prefix from the query.
        const cleanedQuery = stripScopePrefix(queryRef.current);
        if (cleanedQuery !== queryRef.current) {
          setQuery(cleanedQuery);
        }
        // Reset selection immediately; the committed response selects row 0 for
        // the new query epoch (scope changes bump the epoch in runSearch).
        selectionRef.current = EMPTY_SELECTION;
        setSelection(EMPTY_SELECTION);
        void runSearch(cleanedQuery, nextScope);
        if (focusTarget === "input") focusInput();
      },
      [announce, focusInput, runSearch],
    );

    const onChipKeyDown = useCallback(
      (index: number) => (event: ReactKeyboardEvent<HTMLButtonElement>) => {
        const last = SCOPE_CHIPS.length - 1;
        let next: number | null = null;
        switch (event.key) {
          case "ArrowRight":
          case "ArrowDown":
            next = index === last ? 0 : index + 1;
            break;
          case "ArrowLeft":
          case "ArrowUp":
            next = index === 0 ? last : index - 1;
            break;
          case "Home":
            next = 0;
            break;
          case "End":
            next = last;
            break;
          default:
            return;
        }
        event.preventDefault();
        const chip = SCOPE_CHIPS[next];
        applyScope(chip.scope, "chip");
        nextFrame(() => chipRefs.current[next]?.focus());
      },
      [applyScope],
    );

    const onChipClick = useCallback(
      (nextScope: SearchScope) => {
        applyScope(nextScope, "input");
      },
      [applyScope],
    );

    // Pointer/trackpad wheel scrolls the strip horizontally; no drag (SCREENS §2.11).
    const onStripWheel = useCallback((event: ReactWheelEvent<HTMLDivElement>) => {
      const strip = event.currentTarget;
      if (strip.scrollWidth <= strip.clientWidth) return;
      strip.scrollLeft += event.deltaY + event.deltaX;
    }, []);

    useEffect(() => {
      const chip = chipRefs.current[scopeIndex(scope)];
      if (chip && typeof chip.scrollIntoView === "function") {
        chip.scrollIntoView({ block: "nearest", inline: "nearest" });
      }
    }, [scope]);

    useEffect(() => {
      if (!quickActive) return;
      setQuickIndex((current) =>
        Math.min(current, Math.max(0, quickDestinations.length - 1)),
      );
    }, [quickActive, quickDestinations.length]);

    const activateQuickDestination = useCallback(
      (index: number) => {
        const destination = quickDestinations[index];
        if (!destination) return;
        if (destination.action.type === "main") {
          props.showMain();
          dismiss();
          return;
        }
        const prefix = destination.action.prefix;
        setQuery(prefix);
        focusInput();
        void runSearch(prefix, scopeRef.current);
      },
      [dismiss, focusInput, props.showMain, quickDestinations, runSearch],
    );

    const togglePreview = useCallback(() => {
      if (!selectionRef.current.resultId) return;
      const next = !previewOpenRef.current;
      previewOpenRef.current = next;
      setPreviewOpen(next);
      announce(next ? "Preview opened" : "Preview closed");
    }, [announce]);

    const onInputKeyDown = useCallback(
      (event: ReactKeyboardEvent<HTMLInputElement>) => {
        if (event.nativeEvent.isComposing || composingRef.current) return;
        const count = resultsRef.current.length;
        const key = event.key === "Spacebar" ? " " : event.key;
        if (event.ctrlKey && !event.altKey && !event.shiftKey && /^[1-9]$/.test(key)) {
          const index = Number(key) - 1;
          if (index < SCOPE_CHIPS.length) {
            event.preventDefault();
            applyScope(SCOPE_CHIPS[index].scope, "input");
            return;
          }
        }
        if (quickActive) {
          if (key === "ArrowDown") {
            event.preventDefault();
            setQuickIndex((current) => (current + 1) % quickDestinations.length);
            return;
          }
          if (key === "ArrowUp") {
            event.preventDefault();
            setQuickIndex(
              (current) =>
                (current - 1 + quickDestinations.length) % quickDestinations.length,
            );
            return;
          }
          if (key === "Enter" && !event.ctrlKey && !event.shiftKey) {
            event.preventDefault();
            activateQuickDestination(quickIndex);
            return;
          }
        }
        switch (key) {
          case "ArrowDown":
            event.preventDefault();
            move(1);
            break;
          case "ArrowUp":
            event.preventDefault();
            move(-1);
            break;
          case "PageDown":
            event.preventDefault();
            selectIndex(clampedIndex(currentIndex() + PAGE_STEP, count));
            break;
          case "PageUp":
            event.preventDefault();
            selectIndex(clampedIndex(currentIndex() - PAGE_STEP, count));
            break;
          case "Home":
            if (event.ctrlKey) {
              event.preventDefault();
              selectIndex(0);
            }
            break;
          case "End":
            if (event.ctrlKey) {
              event.preventDefault();
              selectIndex(count - 1);
            }
            break;
          case "Enter":
            if (event.ctrlKey) {
              event.preventDefault();
              openMenu();
            } else if (!event.shiftKey) {
              event.preventDefault();
              openActive();
            }
            break;
          case "Tab":
            if (event.shiftKey) {
              // Wrap backwards: last stop is the footer button.
              event.preventDefault();
              footerRef.current?.focus();
            }
            break;
          case " ":
            if (event.ctrlKey) {
              event.preventDefault();
              togglePreview();
            }
            break;
          case "ContextMenu":
            event.preventDefault();
            openMenu();
            break;
          case "F10":
            if (event.shiftKey) {
              event.preventDefault();
              openMenu();
            }
            break;
          case "c":
          case "C": {
            if (!event.ctrlKey || event.altKey) break;
            const input = inputRef.current;
            const hasSelection =
              input != null &&
              input.selectionStart != null &&
              input.selectionEnd != null &&
              input.selectionStart !== input.selectionEnd;
            if (hasSelection) break; // native copy wins
            event.preventDefault();
            copySelected();
            break;
          }
          default:
            break;
        }
      },
      [
        activateQuickDestination,
        applyScope,
        copySelected,
        currentIndex,
        move,
        openActive,
        openMenu,
        quickActive,
        quickDestinations.length,
        quickIndex,
        selectIndex,
        togglePreview,
      ],
    );

    const onRootKeyDown = useCallback(
      (event: ReactKeyboardEvent<HTMLDivElement>) => {
        if (event.key !== "Escape") return;
        event.preventDefault();
        if (pendingConfirmation) {
          cancelConfirmation();
          return;
        }
        if (menuOpen) {
          closeMenu();
          return;
        }
        if (previewOpen) {
          setPreviewOpen(false);
          announce("Preview closed");
          focusInput();
          return;
        }
        // Esc chain: filter reset -> query clear -> hide.
        // Confirmation/menu/preview handled above.
        if (scopeRef.current !== "all") {
          applyScope("all", "input");
          return;
        }
        if (queryRef.current.length > 0) {
          resetState("");
          focusInput();
          void runSearch("", scopeRef.current);
          return;
        }
        dismiss();
      },
      [
        announce,
        applyScope,
        cancelConfirmation,
        closeMenu,
        dismiss,
        focusInput,
        menuOpen,
        pendingConfirmation,
        previewOpen,
        resetState,
        runSearch,
      ],
    );

    const onFooterKeyDown = useCallback((event: ReactKeyboardEvent<HTMLButtonElement>) => {
      if (event.key === "Tab" && !event.shiftKey) {
        // Wrap forward from the last tab stop back to the input.
        event.preventDefault();
        inputRef.current?.focus();
      }
    }, []);

    const onListScroll = useCallback((event: React.UIEvent<HTMLDivElement>) => {
      setListScrollTop(event.currentTarget.scrollTop);
    }, []);

    const onRootMouseDown = useCallback(
      (event: ReactMouseEvent<HTMLDivElement>) => {
        if (!menuOpen) return;
        const menu = menuRef.current;
        if (menu && event.target instanceof Node && menu.contains(event.target)) return;
        setMenuOpen(false);
      },
      [menuOpen],
    );

    const onInputChange = useCallback(
      (event: React.ChangeEvent<HTMLInputElement>) => {
        const value = event.target.value;
        setQuery(value);
        void runSearch(value);
      },
      [runSearch],
    );

    const selectAndOpen = useCallback(
      (index: number, result: SearchResult) => {
        selectIndex(index);
        openResultFor(result);
      },
      [openResultFor, selectIndex],
    );

    // Anchor the context menu to the selected row's edge, using the same fixed
    // row offsets as the keyboard scroll math (no DOM measuring in handlers).
    const menuAnchorStyle = useMemo<CSSProperties | undefined>(() => {
      if (!menuOpen || activeIndex < 0) return undefined;
      const entry = flatEntries.entries.find((candidate) => candidate.index === activeIndex);
      if (!entry) return undefined;
      const rowTop =
        1 + metrics.searchlineHeight + metrics.filterStripHeight + entry.top - listScrollTop;
      const rowBottom = rowTop + metrics.rowHeight;
      if (rowTop > metrics.totalHeight / 2) {
        return { bottom: Math.max(8, metrics.totalHeight - rowBottom - 8) };
      }
      return { top: Math.max(8, rowTop + 8) };
    }, [menuOpen, activeIndex, flatEntries, listScrollTop, metrics]);

    const footerStatus = deriveFooterStatus(status, response, searchError);
    const scopedGuardActive = activeScope === "files" || activeScope === "folders";
    const countOverride = scopedGuardActive ? results.length : undefined;
    const liveCount = useThrottledLiveText(
      countTextFor(pending, response, searchError, query, countOverride),
    );
    const hints = deriveHints(results.length, query, previewOpen);
    const itemCount = itemCountFor(status, response);
    const countText = countTextFor(pending, response, searchError, query, countOverride);
    const showSkeleton = pending && response === null && !searchError;
    const queryText = query.trim();
    const effectiveQueryText = stripScopePrefix(query).trim();
    const activeScopeText = scopeLabelText(activeScope);
    let emptyStrong = "";
    let emptyHint = "";
    if (quickActive) {
      emptyStrong = "";
      emptyHint = "";
    } else if (activeScope === "clipboard") {
      if (!clipboardHistoryEnabled) {
        emptyStrong = "Clipboard history is off.";
        emptyHint = "Turn it on in ProtonSearch → Preferences to keep copied text on this PC.";
      } else if (effectiveQueryText.length > 0) {
        emptyStrong = "No matches in Clipboard.";
        emptyHint = "Try different words, or press Esc to clear the filter.";
      } else {
        emptyStrong = "No clipboard history yet.";
        emptyHint =
          "Copied text and images will appear here from now on. Only copies made while ProtonSearch is running are captured.";
      }
    } else if (activeScope !== "all") {
      emptyStrong = "No matches in " + activeScopeText + ".";
      emptyHint = "Try another category, or press Esc to clear the filter.";
    } else if (queryText.length > 0) {
      emptyStrong = "No matches for “" + queryText + "”.";
      emptyHint = "Try a different spelling, or a prefix such as file:, history:, or code:.";
    } else {
      emptyStrong = "Start typing to search files, apps, and actions.";
      emptyHint = "Try a file name, an app, or a command like “lock”.";
    }
    const previewMode = metrics.previewMode;
    const rootClassName = [
      "launcher",
      motionPhase === "entering" ? "launcher--entering" : "",
      motionPhase === "closing" ? "launcher--closing" : "",
      previewOpen ? "launcher--preview" : "",
      previewOpen ? "launcher--preview-" + previewMode : "",
    ]
      .filter(Boolean)
      .join(" ");

    return (
      <div
        className={rootClassName}
        role="dialog"
        aria-modal="true"
        aria-label="ProtonSearch"
        aria-keyshortcuts="ArrowUp ArrowDown Enter Escape Control+Space"
        onKeyDown={onRootKeyDown}
        onMouseDown={onRootMouseDown}
        data-phase={motionPhase}
        data-testid="launcher"
      >
        <div className="searchline">
          <Logo size={29} />
          <input
            ref={inputRef}
            className="search-input"
            value={query}
            onChange={onInputChange}
            onKeyDown={onInputKeyDown}
            onCompositionStart={() => {
              composingRef.current = true;
            }}
            onCompositionEnd={() => {
              composingRef.current = false;
            }}
            placeholder="Search files, apps, and actions…"
            role="combobox"
            aria-expanded={results.length > 0 || quickActive}
            aria-controls={LISTBOX_ID}
            aria-autocomplete="list"
            aria-activedescendant={
              quickActive
                ? quickDestinationDomId(
                    quickDestinations[Math.min(quickIndex, quickDestinations.length - 1)].id,
                  )
                : activeIndex >= 0
                  ? optionDomId(orderedResults[activeIndex].id)
                  : undefined
            }
            aria-label="Search files, apps, and actions"
            autoComplete="off"
            autoCorrect="off"
            spellCheck={false}
            data-testid="launcher-input"
          />
          <span className="count" data-testid="launcher-count" aria-hidden="true">
            {countText}
          </span>
          <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">
            {liveCount}
          </span>
          <span className="sr-only" role="status" aria-live="polite">
            {announcement}
          </span>
        </div>

        <div
          className="filter-strip"
          role="radiogroup"
          aria-label="Filter results"
          data-testid="filter-strip"
          onWheel={onStripWheel}
        >
          {SCOPE_CHIPS.map((chip, index) => {
            const active = scope === chip.scope;
            return (
              <button
                key={chip.id}
                type="button"
                role="radio"
                aria-checked={active}
                aria-keyshortcuts={chip.shortcut.replace("Ctrl+", "Control+")}
                tabIndex={active ? 0 : -1}
                title={chip.label + " (" + chip.shortcut + ")"}
                className={"filter-chip" + (active ? " filter-chip--active" : "")}
                data-testid="filter-chip"
                data-scope={chip.scope}
                ref={(element) => {
                  chipRefs.current[index] = element;
                }}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onChipClick(chip.scope)}
                onKeyDown={onChipKeyDown(index)}
              >
                {chip.label}
              </button>
            );
          })}
        </div>

        <div className="launcher-body">
          <div className="results" ref={listRef} onScroll={onListScroll}>
            {quickActive ? (
              <div className="quick-heading" role="presentation">
                QUICK DESTINATIONS
              </div>
            ) : null}
            <div
              id={LISTBOX_ID}
              role="listbox"
              aria-label={quickActive ? "Quick destinations" : "Results"}
              data-testid={quickActive ? "quick-destinations" : "results-listbox"}
              aria-busy={pending || undefined}
            >
              {quickActive ? (
                <QuickDestinationOptions
                  destinations={quickDestinations}
                  selectedIndex={quickIndex}
                  onSelect={setQuickIndex}
                  onActivate={activateQuickDestination}
                />
              ) : displaySections.map((section) => (
                <div
                  key={section.key}
                  className={"result-group" + (section.subgroup ? " result-subgroup" : "")}
                  role="group"
                  aria-labelledby={section.labelDomId}
                  data-testid="result-group"
                  data-subgroup={section.subgroup ? "true" : undefined}
                >
                  <div
                    className={section.subgroup ? "result-subgroup-label" : "group-label"}
                    id={section.labelDomId}
                    role="presentation"
                  >
                    {section.label}
                  </div>
                  {section.items.map((result) => {
                    const index = visualIndexById.get(result.id) ?? 0;
                    const isSelected = index === activeIndex;
                    const meta = resultMetaText(result);
                    return (
                      <SharedResultRow
                        key={result.id}
                        result={result}
                        selected={isSelected}
                        optionId={optionDomId(result.id)}
                        descriptionId={descriptionDomId(result.id)}
                        metaId={metaDomId(result.id)}
                        ariaLabel={result.title}
                        ariaDescribedBy={[
                          descriptionDomId(result.id),
                          meta ? metaDomId(result.id) : null,
                        ]
                          .filter(Boolean)
                          .join(" ")}
                        testId="result-row"
                        className="result"
                        onMouseDown={(event) => event.preventDefault()}
                        onClick={() => selectAndOpen(index, result)}
                        onContextMenu={(event) => {
                          event.preventDefault();
                          selectIndex(index);
                          openMenu();
                        }}
                      />
                    );
                  })}
                </div>
              ))}
            </div>

            {quickActive ? (
              <p className="quick-syntax">{SEARCH_SYNTAX_HINT}</p>
            ) : null}

            {showSkeleton ? (
              <div className="launcher-skeleton" aria-hidden="true">
                {[0, 1, 2].map((row) => (
                  <div className="launcher-skeleton-row" key={row}>
                    <span className="launcher-skeleton-icon" />
                    <span className="launcher-skeleton-lines">
                      <span className="launcher-skeleton-title" />
                      <span className="launcher-skeleton-path" />
                    </span>
                  </div>
                ))}
              </div>
            ) : null}

            {!showSkeleton && searchError ? (
              <div className="launcher-state launcher-state--error" role="alert" data-testid="error-state">
                <strong>Search is unavailable right now.</strong>
                <span>{searchError}</span>
                <button
                  type="button"
                  className="inline-retry"
                  onClick={() => void runSearch(queryRef.current)}
                >
                  Retry
                </button>
              </div>
            ) : null}

            {!showSkeleton && !searchError && results.length === 0 && !quickActive ? (
              <div className="launcher-state" role="status" data-testid="empty-state">
                <strong>{emptyStrong}</strong>
                <span>{emptyHint}</span>
              </div>
            ) : null}
          </div>

          {previewOpen ? (
            <ResultPreview result={selectedResult} model={menuModel} onRun={runMenuAction} />
          ) : null}
        </div>

        {pendingConfirmation ? (
          <div
            className="launcher-confirm"
            role="alertdialog"
            aria-label="Confirm system action"
            aria-describedby="launcher-confirm-message"
            data-testid="confirm-bar"
          >
            <span className="launcher-confirm-text" id="launcher-confirm-message" title={pendingConfirmation.message}>
              {pendingConfirmation.message}
            </span>
            <div className="launcher-confirm-actions">
              <button
                ref={confirmCancelRef}
                type="button"
                className="confirm-button confirm-cancel"
                onClick={cancelConfirmation}
              >
                Cancel
              </button>
              <button
                type="button"
                className="confirm-button confirm-run"
                onClick={() => void confirmPendingAction()}
              >
                Run action
              </button>
            </div>
          </div>
        ) : null}

        <div className="footer" data-testid="launcher-footer">
          <div className="leftfoot">
            <span
              className={"status-dot status-dot--" + footerStatus.tone}
              aria-hidden="true"
              data-testid="status-dot"
            />
            {notice ? (
              <span className="footer-status" role="status" aria-live="polite">
                {notice}
              </span>
            ) : (
              <span
                className="footer-status"
                role="status"
                aria-live="polite"
                title={footerStatus.detail}
              >
                {footerStatus.word}
              </span>
            )}
            {itemCount != null ? (
              <>
                <span className="foot-sep foot-sep--before-count" aria-hidden="true">
                  ·
                </span>
                <span className="footer-count">{formatNumber(itemCount)} items</span>
              </>
            ) : null}
            <span className="foot-sep" aria-hidden="true">
              ·
            </span>
            <button
              ref={footerRef}
              type="button"
              className="footlink"
              onClick={props.showMain}
              onKeyDown={onFooterKeyDown}
            >
              Open ProtonSearch
            </button>
          </div>
          <div className="rightfoot" aria-label="Keyboard shortcuts">
            {hints.map((hint) => (
              <span
                className={"hint" + (hint.optional ? " hint--optional" : "")}
                key={hint.keyLabel + hint.label}
                data-testid="footer-hint"
              >
                <span aria-hidden="true">
                  <Kbd>{hint.keyLabel}</Kbd>
                </span>{" "}
                {hint.label}
              </span>
            ))}
          </div>
        </div>

        {menuOpen && selectedResult && menuModel ? (
          <ResultActions
            result={selectedResult}
            model={menuModel}
            menuRef={menuRef as RefObject<HTMLDivElement | null>}
            style={menuAnchorStyle}
            onRun={runMenuAction}
            onClose={closeMenu}
          />
        ) : null}
      </div>
    );
  },
);

export default LauncherSurface;
