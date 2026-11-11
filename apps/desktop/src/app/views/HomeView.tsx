// Home route: greeting, launcher entry, recent activity, index coverage and
// source management. Real EngineStatus/activity data only; honest empty states.

import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../App";
import type { ActivityItem } from "../../bridge/types";
import { formatNumber } from "../../lib/format";
import {
  describeSource,
  greetingLine,
  hasIndexCount,
  indexCounts,
  indexHealth,
  indexCountText,
  recentAccessed,
  relativeTime,
  sourceStatusText,
  summarizeWarnings,
} from "../describe";
import { useOpenLauncher } from "../hooks";
import { displayHotkey } from "../hotkey";
import type { Section } from "../sections";
import { SECTION_EYEBROWS } from "../sections";
import { sourceIconName } from "../sourceIcons";
import { Dot, EmptyState, Icon, RouteHeading, StateNotice } from "../ui";

export default function HomeView({
  onNavigate,
}: {
  onNavigate: (section: Section, sourceId?: string) => void;
}) {
  const { settings, status, getActivity, refreshStatus } = useApp();
  const openLauncher = useOpenLauncher();
  const [recent, setRecent] = useState<ActivityItem[] | null>(null);
  const [activityError, setActivityError] = useState(false);

  const loadActivity = useCallback(async () => {
    try {
      const items = await getActivity({ limit: 40 });
      setRecent(items);
      setActivityError(false);
    } catch {
      setRecent([]);
      setActivityError(true);
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
  const warningSummary = summarizeWarnings(status?.warnings ?? []);

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
        <Icon name="search" size={16} className="launcher-entry-glyph" />
        <span className="launcher-entry-text">Find a file, open an app, or run an action…</span>
        <Icon name="arrowUpRight" size={16} className="launcher-entry-arrow" />
      </button>

      {status && (status.state === "paused" || status.state === "error") ? (
        <StateNotice
          tone={status.state === "error" ? "error" : "warn"}
          role={status.state === "error" ? "alert" : "status"}
          title={
            status.state === "error"
              ? "Indexing stopped."
              : "Indexing is paused."
          }
          body="New files aren't being added. Existing search still works."
          actionLabel="Open Search & Indexing"
          onAction={() => onNavigate("indexes")}
        />
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
              title="Nothing opened recently."
              body="Files and apps you open from the launcher will appear here."
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
                    <Icon name={item.kind === "launch" ? "grid" : "folder"} size={16} />
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
            <StateNotice
              title="Couldn't load activity."
              body="Your history stays on this PC."
              actionLabel="Try again"
              onAction={() => void loadActivity()}
            />
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
            <StateNotice
              title="Couldn't read the index status."
              body="The local search engine didn't respond. Your settings are unaffected."
              actionLabel="Try again"
              onAction={() => void refreshStatus()}
            />
          ) : !hasIndexCount(status) ? (
            <>
              <p className="index-stat">{indexCountText(status)}</p>
              <p className="index-stat-sub">The local index is warming up.</p>
              <p className="index-state-note">
                Indexed counts appear as soon as the engine reports them.
              </p>
            </>
          ) : counts.total === 0 ? (
            <EmptyState
              title="Nothing indexed yet."
              body="Add a folder to start building the index. It stays on this PC."
              action={
                <button type="button" className="btn" onClick={() => onNavigate("indexes")}>
                  Add location
                </button>
              }
            />
          ) : (
            <>
              <p className="index-stat">{indexCountText(status)}</p>
              <p className="index-stat-sub">items indexed on this PC</p>
              <p className="index-state-note">
                {status.state === "indexing"
                  ? "Indexing in the background"
                  : health.tone === "error"
                    ? "Existing results stay searchable"
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
                      <Icon name={sourceIconName(source.id)} size={16} />
                    </span>
                    <span className="source-main">
                      <span className="source-title">{info.title}</span>
                      <span className="source-meta">
                        {sourceStatusText(source, info.enabled)}
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