// Activity route: local history with filters, privacy pause and honest empty
// states. Rows that resolve to a real path can be re-opened through the bridge.

import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import type { ActivityItem, ActivityKind } from "../../bridge/types";
import { ACTIVITY_KIND_LABEL, formatClock, groupActivityByDay } from "../describe";
import { errorMessage } from "../hooks";
import { SECTION_EYEBROWS } from "../sections";
import {
  Callout,
  EmptyState,
  FilterBar,
  Icon,
  RouteHeading,
  Switch,
  type IconName,
} from "../ui";

type ActivityFilter = "all" | "searches" | "launches";

const FILTERS: { value: ActivityFilter; label: string }[] = [
  { value: "all", label: "All" },
  { value: "searches", label: "Searches" },
  { value: "launches", label: "Launches" },
];

function kindIcon(kind: ActivityKind): IconName {
  switch (kind) {
    case "search":
      return "search";
    case "open":
      return "folder";
    case "launch":
      return "grid";
    case "workflow":
      return "workflows";
    case "error":
      return "alert";
    case "system":
      return "settings";
    default:
      return "activity";
  }
}

function matchesFilter(item: ActivityItem, filter: ActivityFilter): boolean {
  if (filter === "all") return true;
  if (filter === "searches") return item.kind === "search";
  return item.kind === "open" || item.kind === "launch";
}

function emptyFilterCopy(filter: ActivityFilter): string {
  if (filter === "searches") return "No searches in this view.";
  if (filter === "launches") return "No launches in this view.";
  return "No activity yet.";
}

export default function ActivityView() {
  const { settings, getActivity, updateSettings, openResult } = useApp();
  const [items, setItems] = useState<ActivityItem[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [filter, setFilter] = useState<ActivityFilter>("all");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setBusy(true);
    try {
      const next = await getActivity({ limit: 100 });
      setItems(next);
      setLoadError(null);
    } catch (err) {
      setItems([]);
      setLoadError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }, [getActivity]);

  useEffect(() => {
    void load();
  }, [load]);

  const paused = !settings.privacy.activityHistory;

  const setPaused = async (nextPaused: boolean) => {
    setActionError(null);
    try {
      await updateSettings({ privacy: { ...settings.privacy, activityHistory: !nextPaused } });
    } catch (err) {
      setActionError(errorMessage(err));
    }
  };

  const openItem = async (item: ActivityItem) => {
    const path = item.path;
    if (!path) return;
    setActionError(null);
    try {
      const outcome = await openResult({ type: "open_path", path });
      if (!outcome.ok) setActionError(outcome.message);
      else void load();
    } catch (err) {
      setActionError(errorMessage(err));
    }
  };

  const filtered = (items ?? []).filter((item) => matchesFilter(item, filter));
  const days = groupActivityByDay(filtered);
  const hasAny = (items ?? []).length > 0;

  const renderRow = (item: ActivityItem) => {
    const actionable = item.kind === "open" && Boolean(item.path);
    const content = (
      <>
        <span className="activity-icon" aria-hidden="true">
          <Icon name={kindIcon(item.kind)} size={15} />
        </span>
        <span className="activity-main">
          <span className="activity-title">{item.title}</span>
          <span className="activity-sub">
            {ACTIVITY_KIND_LABEL[item.kind]} · {formatClock(item.timestamp)}
            {item.detail ? " · " + item.detail : ""}
          </span>
        </span>
        {actionable ? (
          <Icon name="arrowUpRight" size={13} className="activity-arrow" />
        ) : (
          <span className="activity-kindtime">{formatClock(item.timestamp)}</span>
        )}
      </>
    );
    return actionable ? (
      <button
        type="button"
        className="activity-row is-actionable"
        onClick={() => void openItem(item)}
        aria-label={"Open " + item.title}
      >
        {content}
      </button>
    ) : (
      <div className="activity-row">{content}</div>
    );
  };

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.activity}>Your activity</RouteHeading>
      <p className="page-lead">A local history of recent searches, launches, and optional workflows.</p>

      {actionError ? (
        <Callout tone="danger">
          {actionError}
          <button type="button" className="linkbtn" onClick={() => setActionError(null)}>
            Dismiss
          </button>
        </Callout>
      ) : null}

      <section className="privacy-strip" aria-labelledby="activity-privacy">
        <div className="privacy-main">
          <h2 id="activity-privacy">Stored on this PC. Pause recording at any time.</h2>
          <p>
            Activity never leaves this device. Clearing history is not available in this build, so
            entries stay until the engine adds a clear command.
          </p>
        </div>
        <div className="privacy-control">
          <span className="privacy-control-label">Pause recording</span>
          <Switch
            checked={paused}
            label="Pause recording activity"
            onChange={(next) => void setPaused(next)}
          />
        </div>
      </section>

      {paused ? (
        <Callout tone="warn" role="status">
          Activity recording is paused. Existing entries remain readable.
          <button type="button" className="linkbtn" onClick={() => void setPaused(false)}>
            Resume
          </button>
        </Callout>
      ) : null}

      <div className="toolbar">
        <FilterBar label="Filter activity" value={filter} options={FILTERS} onChange={setFilter} />
        <button type="button" className="linkbtn" disabled={busy} onClick={() => void load()}>
          {busy ? "Refreshing…" : "Refresh"}
        </button>
      </div>

      {items === null ? (
        <div className="skeleton-list" aria-busy="true" aria-label="Loading activity">
          {[0, 1, 2, 3, 4].map((row) => (
            <div className="skeleton-row" key={row} />
          ))}
        </div>
      ) : loadError ? (
        <Callout tone="danger">
          {loadError}
          <button type="button" className="linkbtn" onClick={() => void load()}>
            Retry
          </button>
        </Callout>
      ) : !hasAny ? (
        <EmptyState
          title="No activity yet."
          body="Searches and launches appear here while recording is on."
        />
      ) : filtered.length === 0 ? (
        <EmptyState title={emptyFilterCopy(filter)} body="Try a different filter." />
      ) : (
        <div className="activity-groups">
          {days.map((day) => (
            <section key={day.key} className="activity-day" aria-labelledby={"activity-day-" + day.key}>
              <h3 className="day-label" id={"activity-day-" + day.key}>
                {day.label}
              </h3>
              <ul className="activity-list">
                {day.items.map((item) => (
                  <li key={item.id}>{renderRow(item)}</li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
