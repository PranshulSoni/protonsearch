// Preferences → Dynamic Island (docs/design/ISLAND_SETTINGS_SPEC.md). Real
// monitors come from list_monitors; placement/card/behavior changes persist via
// the typed launcher.island settings. The frame mirrors the concept while using
// the app's real data and quiet-state rules.

import { useCallback, useEffect, useRef, useState } from "react";
import { useApp } from "../../App";
import { Logo } from "../../components/Logo";
import { api } from "../../bridge/api";
import type {
  IslandCardId,
  IslandSettings,
  LauncherMode,
  MonitorInfo,
} from "../../bridge/types";
import {
  ANCHORS,
  CARD_META,
  chooseMonitor,
  clampOffset,
  dpiLabel,
  monitorAspect,
  moveCard,
  normalizeIslandSettings,
  offsetsFromPercent,
  percentFromOffsets,
  resetIslandToV9,
  resolveDefaultCard,
  setCardEnabled,
} from "../islandLogic";
import {
  Icon,
  SavedStamp,
  StateNotice,
  Switch,
  useSavedStamp,
} from "../ui";

type Tab = "cards" | "behavior";
type PrefTab = "general" | "launcher" | "island";

const TABS: { id: Tab; label: string }[] = [
  { id: "cards", label: "Cards & order" },
  { id: "behavior", label: "Behavior" },
];

const MONITOR_POLICIES = [
  { value: "primary", label: "Primary monitor" },
  { value: "cursor", label: "Monitor under cursor" },
  { value: "active", label: "Active window's monitor" },
  { value: "specific", label: "Selected specific monitor" },
];

export default function IslandSettingsView({
  onNavigate,
  initialTab = "cards",
  embedded = false,
}: {
  onNavigate: (tab: PrefTab) => void;
  initialTab?: Tab;
  embedded?: boolean;
}) {
  const { settings, updateSettings } = useApp();
  const { savedKey, markSaved } = useSavedStamp();
  const committed = normalizeIslandSettings(settings.launcher.island);
  const [tab, setTab] = useState<Tab>(initialTab);
  const [draft, setDraft] = useState<IslandSettings>(committed);
  const [primaryMode, setPrimaryMode] = useState<LauncherMode>(settings.launcher.primaryMode);
  const [monitors, setMonitors] = useState<MonitorInfo[] | null>(null);
  const [monitorError, setMonitorError] = useState(false);
  const [selectedMonitorId, setSelectedMonitorId] = useState<string | null>(
    draft.placement.monitor,
  );
  const [monitorPolicy, setMonitorPolicy] = useState(
    draft.placement.monitor ? "specific" : "primary",
  );
  const [notice, setNotice] = useState<string | null>(null);
  const [saveError, setSaveError] = useState(false);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const testingTimer = useRef<number | null>(null);
  const dragRef = useRef(false);
  const dragIndex = useRef<number | null>(null);
  const [dragging, setDragging] = useState<number | null>(null);

  const loadMonitors = useCallback(async () => {
    setMonitorError(false);
    try {
      setMonitors(await api.listMonitors());
    } catch {
      setMonitors([]);
      setMonitorError(true);
    }
  }, []);

  useEffect(() => {
    void loadMonitors();
  }, [loadMonitors]);

  useEffect(() => {
    return () => {
      dragRef.current = false;
      if (testingTimer.current) window.clearTimeout(testingTimer.current);
      void api.hideIsland().catch(() => undefined);
    };
  }, []);

  const monitorChoice = chooseMonitor(monitors ?? [], selectedMonitorId ?? draft.placement.monitor);
  const activeMonitor = monitorChoice.monitor;
  const builtIn = monitors?.find((monitor) => monitor.primary) ?? monitors?.[0] ?? null;
  const external = monitors?.find((monitor) => monitor.id !== builtIn?.id) ?? null;
  const mode: LauncherMode = primaryMode;

  const updatePlacement = (patch: Partial<IslandSettings["placement"]>) => {
    setDraft((current) => ({
      ...current,
      placement: { ...current.placement, ...patch },
    }));
  };

  const selectMonitor = (monitor: MonitorInfo) => {
    setSelectedMonitorId(monitor.id);
    setMonitorPolicy("specific");
    updatePlacement({ monitor: monitor.id });
  };

  const selectBuiltInOrExternal = (which: "built-in" | "external") => {
    const target = which === "built-in" ? builtIn : external;
    if (!target) return;
    setSelectedMonitorId(target.id);
    updatePlacement({ monitor: target.id });
  };

  const ghostPercent = activeMonitor
    ? percentFromOffsets(
        draft.placement.anchor,
        draft.placement.offsetX,
        draft.placement.offsetY,
        activeMonitor,
      )
    : { x: 50, y: 12 };

  const setOffsets = (offsetX: number, offsetY: number) => {
    updatePlacement({ offsetX: clampOffset(offsetX), offsetY: clampOffset(offsetY) });
  };

  const pointerToOffsets = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!activeMonitor) return;
    const parent = event.currentTarget.parentElement;
    if (!parent) return;
    const rect = parent.getBoundingClientRect();
    const xPercent = ((event.clientX - rect.left) / Math.max(1, rect.width)) * 100;
    const yPercent = ((event.clientY - rect.top) / Math.max(1, rect.height)) * 100;
    const next = offsetsFromPercent(
      draft.placement.anchor,
      xPercent,
      yPercent,
      activeMonitor,
    );
    setOffsets(next.offsetX, next.offsetY);
  };

  const onGhostPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    dragRef.current = true;
    event.currentTarget.setPointerCapture(event.pointerId);
    pointerToOffsets(event);
  };

  const onGhostPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current) return;
    pointerToOffsets(event);
  };

  const onGhostPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    dragRef.current = false;
    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      /* pointer already released */
    }
  };

  const onGhostKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!activeMonitor) return;
    const stepX = Math.max(1, Math.round(activeMonitor.workWidth * 0.01));
    const stepY = Math.max(1, Math.round(activeMonitor.workHeight * 0.01));
    const deltas: Record<string, [number, number]> = {
      ArrowLeft: [-stepX, 0],
      ArrowRight: [stepX, 0],
      ArrowUp: [0, -stepY],
      ArrowDown: [0, stepY],
    };
    const delta = deltas[event.key];
    if (!delta) return;
    event.preventDefault();
    setOffsets(draft.placement.offsetX + delta[0], draft.placement.offsetY + delta[1]);
  };

  const apply = async () => {
    if (saving) return;
    setSaving(true);
    setSaveError(false);
    const normalized = normalizeIslandSettings(draft);
    const adjusted =
      normalized.placement.offsetX !== draft.placement.offsetX ||
      normalized.placement.offsetY !== draft.placement.offsetY;
    setDraft(normalized);
    try {
      const next = await updateSettings({
        launcher: { ...settings.launcher, primaryMode: mode, island: normalized },
      });
      setDraft(normalizeIslandSettings(next.launcher.island));
      setPrimaryMode(next.launcher.primaryMode);
      markSaved("island");
      setNotice(
        adjusted
          ? "Position adjusted to stay inside the work area."
          : (tab as string) === "placement"
            ? "Placement saved for the selected display."
            : "Settings saved.",
      );
    } catch {
      setSaveError(true);
    } finally {
      setSaving(false);
    }
  };

  const revert = () => {
    const restored = normalizeIslandSettings(settings.launcher.island);
    setDraft(restored);
    setPrimaryMode(settings.launcher.primaryMode);
    setSelectedMonitorId(restored.placement.monitor);
    setMonitorPolicy(restored.placement.monitor ? "specific" : "primary");
    setNotice(null);
    setSaveError(false);
    markSaved("island");
  };

  const stopTest = async () => {
    if (testingTimer.current) window.clearTimeout(testingTimer.current);
    testingTimer.current = null;
    setTesting(false);
    await api.hideIsland().catch(() => undefined);
  };

  const testPlacement = async () => {
    if (testing) {
      await stopTest();
      return;
    }
    setNotice(null);
    setTesting(true);
    try {
      await api.setIslandGeometry(154, 30).catch(() => undefined);
      await api.showIsland();
      testingTimer.current = window.setTimeout(() => {
        setTesting(false);
        testingTimer.current = null;
        void api.hideIsland().catch(() => undefined);
      }, 5000);
    } catch {
      setTesting(false);
      setNotice("The island couldn't be shown. Your saved placement is unchanged.");
    }
  };

  const updateIsland = (patch: Partial<IslandSettings>) => {
    setDraft((current) => ({ ...current, ...patch }));
  };

  const move = (from: number, to: number, id: IslandCardId) => {
    const cards = moveCard(draft.cards, from, to);
    if (cards === draft.cards) return;
    updateIsland({ cards, defaultCard: resolveDefaultCard(cards, draft.defaultCard) });
    window.requestAnimationFrame(() => {
      document.querySelector<HTMLElement>('[data-move-card="' + id + '"]')?.focus();
    });
  };

  const toggleCard = (id: IslandCardId, enabled: boolean) => {
    const cards = setCardEnabled(draft.cards, id, enabled);
    updateIsland({ cards, defaultCard: resolveDefaultCard(cards, draft.defaultCard) });
  };

  const setDefault = (id: IslandCardId) => {
    updateIsland({ defaultCard: resolveDefaultCard(draft.cards, id) });
  };

  const resetV9 = () => {
    setDraft((current) => resetIslandToV9(current));
    setNotice("Card order reset to V9. Apply changes to keep it.");
  };

  const selectedDefault = resolveDefaultCard(draft.cards, draft.defaultCard);
  const enabled = draft.cards.filter((card) => card.enabled);

  const footer = (
    <div className="island-footer">
      <span className="island-toast" role="status">
        {saveError
          ? "Couldn't save that change. Your previous placement is still active."
          : notice ?? ""}
      </span>
      <div className="island-footer-actions">
        <button type="button" className="btn" onClick={revert} disabled={saving}>
          Revert
        </button>
        <button type="button" className="btn" onClick={() => void testPlacement()} disabled={saving || !activeMonitor}>
          {testing ? "Hide preview" : "Test placement"}
        </button>
        <button type="button" className="btn btn-primary" onClick={() => void apply()} disabled={saving}>
          {saving ? "Saving…" : "Apply changes"}
        </button>
      </div>
    </div>
  );

  return (
    <section
      className={embedded ? "island-embedded" : "island-frame"}
      aria-labelledby={embedded ? undefined : "island-settings-title"}
    >
      {!embedded ? (
      <aside className="island-sidebar">
        <div className="island-brand">
          <Logo size={26} />
          <span>ProtonSearch</span>
        </div>
        <div className="island-nav-group">
          <p className="island-nav-label">Workspace</p>
          <button type="button" className="island-nav" onClick={() => onNavigate("general")}>
            <Icon name="settings" size={16} /> Preferences
          </button>
        </div>
        <div className="island-nav-group">
          <p className="island-nav-label">Preferences</p>
          <button type="button" className="island-nav" onClick={() => onNavigate("general")}>
            <Icon name="lock" size={16} /> Privacy
          </button>
          <button type="button" className="island-nav" onClick={() => onNavigate("launcher")}>
            <Icon name="play" size={16} /> Normal Launcher
          </button>
          <button type="button" className="island-nav is-active" aria-current="page">
            <Icon name="grid" size={16} /> Dynamic Island
          </button>
        </div>
        <p className="island-sidefoot">
          These settings are stored with your local preferences. Window placement and shortcuts are
          applied by the desktop shell.
        </p>
      </aside>
      ) : null}

      <div className="island-main">
        {!embedded ? (
        <header className="island-head">
          <p className="island-eyebrow">PREFERENCES / LAUNCHER</p>
          <h1 id="island-settings-title">Make it yours.</h1>
          <p className="island-lead">
            Choose where the island lives, how it behaves and which cards you see. The actual island
            keeps its V9 appearance.
          </p>
        </header>
        ) : null}

        {!embedded ? (
        <div className="island-tabs" role="tablist" aria-label="Dynamic Island settings">
          {TABS.map((entry) => (
            <button
              key={entry.id}
              type="button"
              role="tab"
              aria-selected={tab === entry.id}
              className={"island-tab" + (tab === entry.id ? " is-active" : "")}
              onClick={() => setTab(entry.id)}
            >
              {entry.label}
            </button>
          ))}
        </div>
        ) : null}

        {saveError ? (
          <StateNotice
            tone="error"
            title="Couldn't save that change."
            body="Your previous placement is still active."
            actionLabel="Try again"
            onAction={() => void apply()}
          />
        ) : null}

        {(tab as string) === "placement" ? (
          <div className="island-tab-panel" role="tabpanel" aria-label="Placement">
            <div className="island-section-head">
              <h2>Position on screen</h2>
              <span>Drag the island to reposition</span>
            </div>

            <div className="island-monitor-area">
              <div className="island-monitor-top">
                <span className="island-monitor-title">DISPLAY PREVIEW</span>
                <span className="island-seg" role="group" aria-label="Display">
                  <button
                    type="button"
                    className={
                      "island-seg-btn" +
                      (activeMonitor?.id === builtIn?.id ? " is-active" : "")
                    }
                    aria-pressed={activeMonitor?.id === builtIn?.id}
                    onClick={() => selectBuiltInOrExternal("built-in")}
                  >
                    Built-in
                  </button>
                  <button
                    type="button"
                    className={
                      "island-seg-btn" +
                      (activeMonitor && activeMonitor.id === external?.id ? " is-active" : "")
                    }
                    aria-pressed={Boolean(activeMonitor && activeMonitor.id === external?.id)}
                    disabled={!external}
                    onClick={() => selectBuiltInOrExternal("external")}
                  >
                    External
                  </button>
                </span>
              </div>

              {monitorError ? (
                <StateNotice
                  title="Couldn't read your displays."
                  body="Nothing is changed. Search still works."
                  actionLabel="Try again"
                  onAction={() => void loadMonitors()}
                />
              ) : monitors === null ? (
                <p className="island-muted" aria-busy="true">Reading your displays…</p>
              ) : monitors.length === 0 ? (
                <StateNotice
                  title="Couldn't read your displays."
                  body="No monitors were returned. Nothing is changed."
                  actionLabel="Try again"
                  onAction={() => void loadMonitors()}
                />
              ) : activeMonitor ? (
                <>
                  <div
                    className="island-screen"
                    style={{ aspectRatio: String(monitorAspect(activeMonitor)) }}
                  >
                    <span className="island-screen-ellipse" aria-hidden="true" />
                    <div className="island-taskbar" aria-hidden="true">
                      <span /><span /><span /><span />
                    </div>
                    <div
                      className="island-ghost"
                      role="slider"
                      tabIndex={0}
                      aria-label="Island position, drag or use arrow keys"
                      aria-valuemin={0}
                      aria-valuemax={100}
                      aria-valuetext={
                        "X " + draft.placement.offsetX + " DIP, Y " + draft.placement.offsetY + " DIP"
                      }
                      style={{
                        left: ghostPercent.x + "%",
                        top: ghostPercent.y + "%",
                      }}
                      onPointerDown={onGhostPointerDown}
                      onPointerMove={onGhostPointerMove}
                      onPointerUp={onGhostPointerUp}
                      onPointerCancel={onGhostPointerUp}
                      onKeyDown={onGhostKeyDown}
                    >
                      <span className="island-ghost-mark" aria-hidden="true">P</span>
                      <span className="island-ghost-label">Search anything…</span>
                      <span className="island-ghost-dot" aria-hidden="true" />
                    </div>
                  </div>
                  <p className="island-screen-hint">
                    {activeMonitor.name} · {dpiLabel(activeMonitor)} · Preview only. Position is
                    constrained inside the selected monitor.
                  </p>
                </>
              ) : null}
            </div>

            <div className="island-placement-grid">
              <div>
                <p className="island-form-label">Anchor preset</p>
                <div className="island-anchors" role="group" aria-label="Anchor preset">
                  {ANCHORS.map((anchor) => (
                    <button
                      key={anchor.id}
                      type="button"
                      className={
                        "island-anchor" + (draft.placement.anchor === anchor.id ? " is-active" : "")
                      }
                      aria-pressed={draft.placement.anchor === anchor.id}
                      aria-label={anchor.label + " anchor"}
                      title={anchor.label}
                      onClick={() => updatePlacement({ anchor: anchor.id })}
                    >
                      <span className="island-anchor-bar" aria-hidden="true" />
                    </button>
                  ))}
                </div>

                <p className="island-form-label">Monitor behavior</p>
                <select
                  className="island-select"
                  value={monitorPolicy}
                  onChange={(event) => {
                    const next = event.target.value;
                    setMonitorPolicy(next);
                    if (next === "specific") {
                      if (activeMonitor) {
                        setSelectedMonitorId(activeMonitor.id);
                        updatePlacement({ monitor: activeMonitor.id });
                      }
                    } else {
                      updatePlacement({ monitor: null });
                    }
                  }}
                >
                  {MONITOR_POLICIES.map((policy) => (
                    <option key={policy.value} value={policy.value}>{policy.label}</option>
                  ))}
                </select>
                {monitors && monitors.length > 2 ? (
                  <select
                    className="island-select"
                    aria-label="Specific monitor"
                    value={activeMonitor?.id ?? ""}
                    onChange={(event) => {
                      const target = monitors.find((monitor) => monitor.id === event.target.value);
                      if (target) selectMonitor(target);
                    }}
                  >
                    {monitors.map((monitor) => (
                      <option key={monitor.id} value={monitor.id}>{monitor.name}</option>
                    ))}
                  </select>
                ) : null}
              </div>

              <div>
                <div className="island-offsets">
                  <label className="island-field">
                    <span className="island-form-label">Horizontal offset</span>
                    <input
                      className="input"
                      type="number"
                      min={-500}
                      max={500}
                      step={1}
                      value={draft.placement.offsetX}
                      onChange={(event) =>
                        setOffsets(clampOffset(Number(event.target.value)), draft.placement.offsetY)
                      }
                    />
                  </label>
                  <label className="island-field">
                    <span className="island-form-label">Vertical offset</span>
                    <input
                      className="input"
                      type="number"
                      min={-500}
                      max={500}
                      step={1}
                      value={draft.placement.offsetY}
                      onChange={(event) =>
                        setOffsets(draft.placement.offsetX, clampOffset(Number(event.target.value)))
                      }
                    />
                  </label>
                </div>

                <div className="island-switch-list">
                  <div className="island-switch-row">
                    <div>
                      <p className="island-switch-title">Keep inside work area</p>
                      <p className="island-switch-sub">Avoid taskbar and monitor edges</p>
                    </div>
                    <Switch
                      checked={draft.placement.keepInWorkArea}
                      label="Keep inside work area"
                      onChange={(next) => updatePlacement({ keepInWorkArea: next })}
                    />
                  </div>
                  <div className="island-switch-row">
                    <div>
                      <p className="island-switch-title">Auto-flip expanded cards</p>
                      <p className="island-switch-sub">Prevent tall cards from clipping</p>
                    </div>
                    <Switch
                      checked={draft.placement.autoFlipExpanded}
                      label="Auto-flip expanded cards"
                      onChange={(next) => updatePlacement({ autoFlipExpanded: next })}
                    />
                  </div>
                </div>
              </div>
            </div>

            {monitorChoice.lost ? (
              <StateNotice
                tone="warn"
                title="Saved display isn't connected."
                body="The preview shows the primary display. Your saved display preference is kept."
                actionLabel="Refresh displays"
                onAction={() => void loadMonitors()}
              />
            ) : null}
            {testing ? (
              <p className="island-testing" role="status">
                Test overlay is showing. It hides automatically after a few seconds.
              </p>
            ) : null}
          </div>
        ) : null}

        {tab === "cards" ? (
          <div className="island-tab-panel" role="tabpanel" aria-label="Cards and order">
            <div className="island-section-head">
              <h2>Card order</h2>
              <button type="button" className="btn" onClick={resetV9}>Reset to V9</button>
            </div>
            <p className="island-help">
              Move cards up or down, enable only those you use, and pick the default. Dots, wheel
              switching and swipe order follow this list.
            </p>

            <ul className="island-card-list">
              {draft.cards.map((card, index) => {
                const meta = CARD_META[card.id];
                const disabled = !card.enabled;
                return (
                  <li
                    key={card.id}
                    className={"island-card-row" + (disabled ? " is-disabled" : "")}
                    draggable
                    onDragStart={(event) => {
                      event.dataTransfer.effectAllowed = "move";
                      event.dataTransfer.setData("text/plain", String(index));
                      dragIndex.current = index;
                      setDragging(index);
                    }}
                    onDragOver={(event) => event.preventDefault()}
                    onDrop={(event) => {
                      event.preventDefault();
                      const from = dragIndex.current;
                      dragIndex.current = null;
                      setDragging(null);
                      if (from !== null && from !== index) move(from, index, draft.cards[index].id);
                    }}
                    onDragEnd={() => {
                      dragIndex.current = null;
                      setDragging(null);
                    }}
                  >
                    <span className="island-reorder">
                      <button
                        type="button"
                        data-move-card={card.id}
                        aria-label={"Move " + meta.title + " up"}
                        disabled={index === 0}
                        onClick={() => move(index, index - 1, card.id)}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        aria-label={"Move " + meta.title + " down"}
                        disabled={index === draft.cards.length - 1}
                        onClick={() => move(index, index + 1, card.id)}
                      >
                        ↓
                      </button>
                    </span>
                    <span className="island-card-icon" aria-hidden="true">
                      <Icon name={meta.icon} size={15} />
                    </span>
                    <span className="island-card-main">
                      <span className="island-card-title">{meta.title}</span>
                      <span className="island-card-desc">{meta.description}</span>
                    </span>
                    {selectedDefault === card.id ? (
                      <span className="island-default-tag">DEFAULT</span>
                    ) : null}
                    <button
                      type="button"
                      className="btn island-set-default"
                      disabled={!card.enabled || selectedDefault === card.id}
                      onClick={() => setDefault(card.id)}
                    >
                      Set default
                    </button>
                    <input
                      type="checkbox"
                      className="island-card-check"
                      checked={card.enabled}
                      disabled={card.id === "search"}
                      aria-label={"Enable " + meta.title}
                      onChange={(event) => toggleCard(card.id, event.target.checked)}
                    />
                  </li>
                );
              })}
            </ul>

            <div className="island-dot-panel">
              <div className="island-dot-island">
                <span className="island-dot-default">Default · {CARD_META[selectedDefault].title}</span>
                <span className="island-dots" aria-label={enabled.length + " enabled cards"}>
                  {enabled.map((card) => (
                    <span
                      key={card.id}
                      className={"island-dot" + (card.id === selectedDefault ? " is-active" : "")}
                    />
                  ))}
                </span>
              </div>
              <p className="island-help">
                Active card indicators change immediately when cards are reordered or hidden.
              </p>
            </div>

            <div className="island-switch-list">
              <div className="island-switch-row">
                <div>
                  <p className="island-switch-title">Show page dots</p>
                  <p className="island-switch-sub">One dot per enabled card</p>
                </div>
                <Switch
                  checked={draft.showDots}
                  label="Show page dots"
                  onChange={(next) => updateIsland({ showDots: next })}
                />
              </div>
            </div>
            {dragging !== null ? <p className="island-help">Drop to reorder this card.</p> : null}
          </div>
        ) : null}

        {tab === "behavior" ? (
          <div className="island-tab-panel" role="tabpanel" aria-label="Behavior">
            <div className="island-section-head">
              <h2>Behavior</h2>
            </div>
            <div className="island-switch-list">
              <div className="island-switch-row">
                <div>
                  <p className="island-switch-title">Dynamic Island enabled</p>
                  <p className="island-switch-sub">Keep the alternate launcher available</p>
                </div>
                <Switch
                  checked={draft.enabled}
                  label="Dynamic Island enabled"
                  onChange={(next) => updateIsland({ enabled: next })}
                />
              </div>
              <div className="island-switch-row">
                <div>
                  <p className="island-switch-title">Return to Search on next opening</p>
                  <p className="island-switch-sub">Otherwise remember the last active card</p>
                </div>
                <Switch
                  checked={draft.returnToSearch}
                  label="Return to Search on next opening"
                  onChange={(next) => updateIsland({ returnToSearch: next })}
                />
              </div>
              <div className="island-switch-row">
                <div>
                  <p className="island-switch-title">Hide when focus is lost</p>
                  <p className="island-switch-sub">Dismiss the island when another window takes focus</p>
                </div>
                <Switch
                  checked={draft.hideOnFocusLoss}
                  label="Hide when focus is lost"
                  onChange={(next) => updateIsland({ hideOnFocusLoss: next })}
                />
              </div>
              <div className="island-switch-row">
                <div>
                  <p className="island-switch-title">Skip disabled cards</p>
                  <p className="island-switch-sub">Wheel and swipe move through enabled cards only</p>
                </div>
                <Switch
                  checked={draft.skipDisabled}
                  label="Skip disabled cards"
                  onChange={(next) => updateIsland({ skipDisabled: next })}
                />
              </div>
              <div className="island-switch-row">
                <div>
                  <p className="island-switch-title">Follow Windows reduced motion</p>
                  <p className="island-switch-sub">Preserve the V9 animation for standard motion</p>
                </div>
                <Switch
                  checked={settings.motion === "system"}
                  label="Follow Windows reduced motion"
                  onChange={(next) => {
                    void updateSettings({ motion: next ? "system" : "full" }).catch(() =>
                      setSaveError(true),
                    );
                  }}
                />
              </div>
            </div>

            <div className="island-section-head island-section-head--stacked">
              <h2>Primary launcher when pressing Alt + Space</h2>
              <span>Also asked during first-run setup</span>
            </div>
            <div className="island-choice-grid">
              {(
                [
                  {
                    value: "dynamic_island" as LauncherMode,
                    title: "Dynamic Island",
                    body: "The compact island at the top of the screen, with cards and dots.",
                  },
                  {
                    value: "normal" as LauncherMode,
                    title: "Normal Launcher",
                    body: "The familiar centred launcher window with the full result list.",
                  },
                ]
              ).map((option) => (
                <label
                  key={option.value}
                  className={"island-choice" + (mode === option.value ? " is-selected" : "")}
                >
                  <input
                    type="radio"
                    name="primary-launcher"
                    value={option.value}
                    checked={mode === option.value}
                    onChange={() => setPrimaryMode(option.value)}
                  />
                  <span>
                    <span className="island-choice-title">{option.title}</span>
                    <span className="island-choice-body">{option.body}</span>
                  </span>
                </label>
              ))}
            </div>
            <p className="island-help">
              Settings here are visual. The shell stores your choice, applies the shortcut and
              positions the real window.
            </p>
          </div>
        ) : null}

        {saveError ? null : null}
        {footer}
        <SavedStamp show={savedKey === "island"} />
      </div>
    </section>
  );
}