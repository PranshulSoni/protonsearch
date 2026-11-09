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
} from "../bridge/types";
import { Highlight } from "../components/Highlight";
import { Kbd } from "../components/Kbd";
import { Logo } from "../components/Logo";
import { groupResults } from "../lib/results";
import { formatBytes, formatNumber } from "../lib/format";
import { formatRecency } from "./launcherFormat";
import { ResultIcon } from "./ResultIcon";
import { ResultPreview } from "./ResultPreview";
import { ResultActions, buildMenuActions, type MenuAction, type MenuModel } from "./ResultActions";
import {
  EMPTY_SELECTION,
  MAX_RENDERED_ROWS,
  PAGE_STEP,
  SEARCH_LIMIT,
  buildFlatEntries,
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
  /** Server-side confirmation for destructive native commands (V2-04). */
  confirmNativeAction?: (command: NativeCommand) => Promise<OpenOutcome>;
  density?: Density;
  /** Overridable for tests; defaults to the screen work area. */
  workArea?: WorkAreaSize;
  onGeometry?: (height: number, previewOpen: boolean) => void;
}

export interface LauncherSurfaceHandle {
  /** Reset to an empty query, focus the input and select the first row. */
  summon: () => void;
  /** Clear all launcher state (window hidden). */
  clear: () => void;
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
  return action.type !== "none" && action.type !== "copy_text";
}

function countTextFor(
  pending: boolean,
  response: SearchResponse | null,
  searchError: string | null,
  query: string,
): string {
  if (searchError) return "Search unavailable";
  if (pending) return "Searching…";
  if (!response) return "";
  // An empty query with no recents is the quiet placeholder state, not "0 matches".
  if (response.total === 0 && query.trim().length === 0) return "";
  return countMatches(response.total);
}

/** Truthful index item count: search response first, EngineStatus as fallback. */
function itemCountFor(
  status: EngineStatus | null,
  response: SearchResponse | null,
): number | undefined {
  const searchStatus = response?.status;
  const fromSearch =
    searchStatus != null &&
    (searchStatus.type === "ready" || searchStatus.type === "indexing")
      ? searchStatus.indexedFiles
      : undefined;
  return fromSearch ?? status?.filenameEntries;
}

function metaText(result: SearchResult): string {
  const meta = result.meta;
  if (!meta) return "";
  if (meta.label) return meta.label;
  if (meta.modified) return formatRecency(meta.modified);
  if (meta.line != null) return "Line " + meta.line;
  if (meta.itemCount != null) return meta.itemCount.toLocaleString() + " items";
  if (meta.size != null) return formatBytes(meta.size);
  return "";
}

function deriveFooterStatus(
  status: EngineStatus | null,
  response: SearchResponse | null,
  searchError: string | null,
): FooterStatus {
  if (searchError) return { word: "Index unavailable", tone: "negative", detail: searchError };
  if (response?.status.type === "degraded") {
    return { word: "Index unavailable", tone: "negative", detail: response.status.reason };
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
    const { status, density, onGeometry } = props;
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
    const [pendingConfirmation, setPendingConfirmation] = useState<{
      command: NativeCommand;
      message: string;
    } | null>(null);

    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);
    const footerRef = useRef<HTMLButtonElement>(null);
    const menuRef = useRef<HTMLDivElement | null>(null);
    const confirmCancelRef = useRef<HTMLButtonElement>(null);

    const requestIdRef = useRef(0);
    const epochRef = useRef(0);
    const epochQueryRef = useRef("");
    const queryRef = useRef("");
    const composingRef = useRef(false);
    const selectionRef = useRef<SelectionState>(EMPTY_SELECTION);
    const resultsRef = useRef<SearchResult[]>([]);
    const committedRef = useRef<CommittedResponse | null>(null);
    const previewOpenRef = useRef(false);
    const bootedRef = useRef(false);
    const noticeTimerRef = useRef<number | null>(null);
    const lastGeometryRef = useRef<{ height: number; previewOpen: boolean } | null>(null);

    previewOpenRef.current = previewOpen;

    const results = useMemo(
      () => (response ? response.results.slice(0, MAX_RENDERED_ROWS) : []),
      [response],
    );
    const groups = useMemo(() => groupResults(results), [results]);
    // Keyboard order is the visual grouped order, so arrow keys move between
    // rendered neighbours instead of jumping across group sections.
    const orderedResults = useMemo(
      () => groups.flatMap((group) => group.items.map((item) => item.result)),
      [groups],
    );
    resultsRef.current = orderedResults;
    const visualIndexById = useMemo(() => {
      const map = new Map<string, number>();
      orderedResults.forEach((item, index) => map.set(item.id, index));
      return map;
    }, [orderedResults]);
    const groupRowCounts = useMemo(() => groups.map((group) => group.items.length), [groups]);

    const activeIndex = useMemo(() => {
      if (orderedResults.length === 0) return -1;
      if (selection.resultId) {
        const found = orderedResults.findIndex((result) => result.id === selection.resultId);
        if (found >= 0) return found;
      }
      return clampedIndex(selection.index, orderedResults.length);
    }, [orderedResults, selection]);

    const selectedResult = activeIndex >= 0 ? orderedResults[activeIndex] : null;

    const metrics = useMemo<LauncherMetrics>(
      () =>
        computeLauncherMetrics({
          groupRowCounts,
          density,
          previewOpen,
          confirmOpen: pendingConfirmation !== null,
          workArea,
        }),
      [groupRowCounts, density, previewOpen, pendingConfirmation, workArea],
    );

    const flatEntries = useMemo(
      () => buildFlatEntries(groups, metrics.rowHeight, metrics.labelHeight),
      [groups, metrics.rowHeight, metrics.labelHeight],
    );

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
      const ordered = groupResults(nextResults).flatMap((group) =>
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
      async (nextQuery: string) => {
        const requestId = ++requestIdRef.current;
        const epoch =
          nextQuery === epochQueryRef.current ? epochRef.current : ++epochRef.current;
        epochQueryRef.current = nextQuery;
        queryRef.current = nextQuery;
        setPending(true);
        setSearchError(null);
        setNotice(null);
        setPendingConfirmation(null);
        try {
          const next = await props.search({ requestId, query: nextQuery, limit: SEARCH_LIMIT });
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

    const summon = useCallback(() => {
      resetState("");
      setPreviewOpen(false);
      focusInput();
      void runSearch("");
    }, [resetState, focusInput, runSearch]);

    const clear = useCallback(() => {
      resetState("");
      setPreviewOpen(false);
    }, [resetState]);

    useImperativeHandle(ref, () => ({ summon, clear }), [summon, clear]);

    useEffect(() => {
      if (bootedRef.current) return;
      bootedRef.current = true;
      focusInput();
      void runSearch("");
    }, [focusInput, runSearch]);

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
      [copySelected, currentIndex, move, openActive, openMenu, selectIndex, togglePreview],
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
        if (queryRef.current.length > 0) {
          resetState("");
          focusInput();
          void runSearch("");
          return;
        }
        props.hideLauncher();
      },
      [
        announce,
        cancelConfirmation,
        closeMenu,
        focusInput,
        menuOpen,
        pendingConfirmation,
        previewOpen,
        props.hideLauncher,
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
        1 + metrics.searchlineHeight + entry.top - listScrollTop;
      const rowBottom = rowTop + metrics.rowHeight;
      if (rowTop > metrics.totalHeight / 2) {
        return { bottom: Math.max(8, metrics.totalHeight - rowBottom - 8) };
      }
      return { top: Math.max(8, rowTop + 8) };
    }, [menuOpen, activeIndex, flatEntries, listScrollTop, metrics]);

    const footerStatus = deriveFooterStatus(status, response, searchError);
    const liveCount = useThrottledLiveText(countTextFor(pending, response, searchError, query));
    const hints = deriveHints(results.length, query, previewOpen);
    const itemCount = itemCountFor(status, response);
    const countText = countTextFor(pending, response, searchError, query);
    const showSkeleton = pending && response === null && !searchError;
    const queryText = query.trim();
    const previewMode = metrics.previewMode;
    const rootClassName = [
      "launcher",
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
            aria-expanded={results.length > 0}
            aria-controls={LISTBOX_ID}
            aria-autocomplete="list"
            aria-activedescendant={
              activeIndex >= 0 ? optionDomId(orderedResults[activeIndex].id) : undefined
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

        <div className="launcher-body">
          <div className="results" ref={listRef} onScroll={onListScroll}>
            <div
              id={LISTBOX_ID}
              role="listbox"
              aria-label="Results"
              aria-busy={pending || undefined}
            >
              {groups.map((group) => (
                <div
                  key={group.kind}
                  className="result-group"
                  role="group"
                  aria-labelledby={groupLabelDomId(group.kind)}
                  data-testid="result-group"
                >
                  <div
                    className="group-label"
                    id={groupLabelDomId(group.kind)}
                    role="presentation"
                  >
                    {group.label}
                  </div>
                  {group.items.map(({ result }) => {
                    const index = visualIndexById.get(result.id) ?? 0;
                    const isSelected = index === activeIndex;
                    const meta = metaText(result);
                    const description = [result.path ?? result.subtitle, meta].filter(Boolean).join(" · ");
                    return (
                      <div
                        key={result.id}
                        id={optionDomId(result.id)}
                        role="option"
                        aria-selected={isSelected}
                        aria-label={result.title}
                        aria-describedby={
                          [
                            descriptionDomId(result.id),
                            meta ? metaDomId(result.id) : null,
                          ]
                            .filter(Boolean)
                            .join(" ")
                        }
                        className="result"
                        data-testid="result-row"
                        data-result-id={result.id}
                        onMouseDown={(event) => event.preventDefault()}
                        onClick={() => selectAndOpen(index, result)}
                        onContextMenu={(event) => {
                          event.preventDefault();
                          selectIndex(index);
                          openMenu();
                        }}
                      >
                        <ResultIcon result={result} />
                        <span className="result-body">
                          <span className="filename" title={result.title}>
                            <Highlight text={result.title} ranges={result.matchRanges} />
                          </span>
                          <span className="filepath" id={descriptionDomId(result.id)} title={result.path ?? result.subtitle}>
                            {result.subtitle}
                          </span>
                        </span>
                        {meta ? (
                          <span className="result-meta" id={metaDomId(result.id)}>
                            {meta}
                          </span>
                        ) : (
                          <span className="sr-only" id={metaDomId(result.id)}>
                            {description}
                          </span>
                        )}
                      </div>
                    );
                  })}
                </div>
              ))}
            </div>

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

            {!showSkeleton && !searchError && results.length === 0 ? (
              <div className="launcher-state" role="status" data-testid="empty-state">
                {queryText.length > 0 ? (
                  <>
                    <strong>No matches for “{queryText}”.</strong>
                    <span>Try a different spelling, or a prefix such as file:, history:, or code:.</span>
                  </>
                ) : (
                  <>
                    <strong>Start typing to search files, apps, and actions.</strong>
                    <span>Try a file name, an app, or a command like “lock”.</span>
                  </>
                )}
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
