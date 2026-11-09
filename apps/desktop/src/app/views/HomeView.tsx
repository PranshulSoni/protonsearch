// Home route: greeting, launcher entry, recent activity, index coverage and
// source management. Real EngineStatus/activity data only; honest empty states.

import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import type { ActivityItem } from "../../bridge/types";
import { formatNumber } from "../../lib/format";
import {
  describeSource,
  greetingLine,
  indexCounts,
  indexHealth,
  recentAccessed,
  relativeTime,
} from "../describe";
import { useOpenLauncher } from "../hooks";
import { displayHotkey } from "../hotkey";
import type { Section } from "../sections";
import { SECTION_EYEBROWS } from "../sections";
import { sourceIconName } from "../sourceIcons";
import { Callout, Dot, EmptyState, Icon, RouteHeading } from "../ui";

export default function HomeView({
  onNavigate,
}: {
  onNavigate: (section: Section, sourceId?: string) => void;
}) {
  const { settings, status, getActivity, refreshStatus } = useApp();
  const openLauncher = useOpenLauncher();
  const [recent, setRecent] = useState<ActivityItem[] | null>(null);
  const [activityError, setActivityError] = useState<string | null>(null);

  const loadActivity = useCallback(async () => {
    try {
      const items = await getActivity({ limit: 40 });
      setRecent(items);
      setActivityError(null);
    } catch (err) {
      setRecent([]);
      setActivityError(err instanceof Error ? err.message : String(err));
    }
  }, [getActivity]);

  useEffect(() => {
    void loadActivity();
  }, [loadActivity]);

  const health = indexHealth(status);
  const counts = indexCounts(status);
  const accessed = recent ? recentAccessed(recent) : [];
  const userName = settings.userName?.trim() || null;
  const sources = status?.sources ?? [];
  const shortcut = displayHotkey(settings.hotkey);

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.home}>{greetingLine(userName)}</RouteHeading>
      <p className="page-lead">Everything on your computer, within reach.</p>

      <button
        type="button"
        className="launcher-entry"
        onClick={openLauncher}
        title={shortcut ? "Open the launcher (" + shortcut + ")" : "Open the launcher"}
      >
        <Icon name="search" size={18} className="launcher-entry-glyph" />
        <span className="launcher-entry-text">Find a file, open an app, or run an action…</span>
        <Icon name="arrowUpRight" size={15} className="launcher-entry-arrow" />
      </button>

      {status && (status.state === "paused" || status.state === "error") ? (
        <Callout tone={status.state === "error" ? "danger" : "warn"} role="status">
          <span>{health.detail}</span>
          <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
            Open Search &amp; indexes
          </button>
        </Callout>
      ) : null}

      <div className="home-columns">
        <section className="home-section" aria-labelledby="home-recent">
          <div className="section-head">
            <h2 id="home-recent">Recently accessed</h2>
          </div>
          {recent === null ? (
            <p className="muted" aria-busy="true">
              Loading recent activity…
            </p>
          ) : accessed.length === 0 ? (
            <EmptyState
              title="No recent items yet."
              body="Files and apps you open from the launcher appear here. Nothing leaves this device."
              action={
                <button type="button" className="btn" onClick={openLauncher}>
                  Open the launcher
                </button>
              }
            />
          ) : (
            <ul className="recent-list">
              {accessed.map((item) => (
                <li className="recent-row" key={item.id}>
                  <span className="recent-icon" aria-hidden="true">
                    <Icon name={item.kind === "launch" ? "grid" : "folder"} size={15} />
                  </span>
                  <span className="recent-text">
                    <span className="recent-title">{item.title}</span>
                    <span className="recent-sub">
                      {item.detail ?? item.path ?? (item.kind === "launch" ? "Launched" : "Opened")}
                    </span>
                  </span>
                  <span className="recent-time">{relativeTime(item.timestamp)}</span>
                </li>
              ))}
            </ul>
          )}
          {activityError ? (
            <Callout tone="danger">
              {activityError}
              <button type="button" className="linkbtn" onClick={() => void loadActivity()}>
                Retry
              </button>
            </Callout>
          ) : null}
          <div className="section-actions">
            <button type="button" className="btn" onClick={() => onNavigate("indexes")}>
              Manage sources
            </button>
          </div>
        </section>

        <section className="home-section" aria-labelledby="home-index">
          <div className="section-head">
            <h2 id="home-index">Your index</h2>
            {status ? (
              <span className="index-state">
                <Dot tone={health.tone} />
                {health.label}
              </span>
            ) : null}
          </div>
          {status === null ? (
            <Callout tone="danger">
              Couldn&apos;t read the index status.
              <button type="button" className="linkbtn" onClick={() => void refreshStatus()}>
                Retry
              </button>
            </Callout>
          ) : counts.total === 0 ? (
            <EmptyState
              title="Nothing indexed yet."
              body="Add a location to start building the index; it stays on this PC."
              action={
                <button type="button" className="btn" onClick={() => onNavigate("indexes")}>
                  Set up indexing
                </button>
              }
            />
          ) : (
            <>
              <p className="index-stat">{formatNumber(counts.total)}</p>
              <p className="index-stat-sub">items indexed on this PC</p>
              <p className="index-state-note">
                {status.state === "indexing"
                  ? "Indexing in the background"
                  : health.tone === "error"
                    ? health.label
                    : "Updated " + relativeTime(status.lastUpdated)}
              </p>
              <dl className="metric-list">
                <div className="metric-row">
                  <dt>Documents</dt>
                  <dd>{formatNumber(counts.documents)}</dd>
                </div>
                <div className="metric-row">
                  <dt>Applications</dt>
                  <dd>{formatNumber(counts.applications)}</dd>
                </div>
                <div className="metric-row">
                  <dt>Images &amp; OCR</dt>
                  <dd>{formatNumber(counts.imagesOcr)}</dd>
                </div>
                <div className="metric-row">
                  <dt>Other files</dt>
                  <dd>{formatNumber(counts.otherFiles)}</dd>
                </div>
              </dl>
            </>
          )}
        </section>
      </div>

      <section className="home-section home-sources" aria-labelledby="home-sources">
        <div className="section-head">
          <h2 id="home-sources">Sources</h2>
          <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
            Manage all sources
          </button>
        </div>
        {sources.length === 0 ? (
          <p className="muted">The local engine has not reported any sources yet.</p>
        ) : (
          <ul className="source-list">
            {sources.map((source) => {
              const info = describeSource(source, settings);
              return (
                <li key={source.id}>
                  <button
                    type="button"
                    className="source-row"
                    onClick={() => onNavigate("indexes", source.id)}
                  >
                    <span className="source-icon" aria-hidden="true">
                      <Icon name={sourceIconName(source.id)} size={15} />
                    </span>
                    <span className="source-main">
                      <span className="source-title">{info.title}</span>
                      <span className="source-meta">
                        {info.enabled
                          ? formatNumber(source.itemCount) + " items · " + info.stateLabel
                          : "Off"}
                      </span>
                    </span>
                    <Dot tone={info.tone} />
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}