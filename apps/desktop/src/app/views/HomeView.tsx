// Home route: greeting, launcher button, indexed-file count and per-source switches.
// Real EngineStatus and Settings data only.

import { useEffect, useState } from "react";
import { useApp } from "../../App";
import { api } from "../../bridge/api";
import type { EngineStatus } from "../../bridge/types";
import {
  describeSource,
  greetingForTimestamp,
  hasIndexCount,
  indexCountText,
  relativeTime,
  sourceStatusText,
  type SourceInfo,
} from "../describe";
import { displayHotkey } from "../hotkey";
import type { Section } from "../sections";
import { SECTION_EYEBROWS } from "../sections";
import { sourceIconName } from "../sourceIcons";
import { Icon, RouteHeading, StateNotice, Switch } from "../ui";

/** What the indexer is doing right now, or null while the first count is pending. */
function indexNote(status: EngineStatus): string | null {
  if (!hasIndexCount(status)) return null;
  switch (status.state) {
    case "indexing":
      return "Indexing in the background";
    case "paused":
      return "Indexing is paused";
    case "error":
      return "Indexing stopped";
    default:
      return status.lastUpdated ? "Updated " + relativeTime(status.lastUpdated) : null;
  }
}

export default function HomeView({
  onNavigate,
}: {
  onNavigate: (section: Section, sourceId?: string) => void;
}) {
  const { settings, status, updateSettings, refreshStatus } = useApp();
  const [osName, setOsName] = useState<string | null>(null);
  const [failedSource, setFailedSource] = useState<SourceInfo | null>(null);

  useEffect(() => {
    let alive = true;
    api
      .osUserName()
      .then((name) => {
        if (alive) setOsName(name);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);

  const greeting = greetingForTimestamp() + (osName ? ", " + osName : "");
  const shortcut = displayHotkey(settings.hotkey);
  const sources = status?.sources ?? [];
  const note = status ? indexNote(status) : null;

  const openIsland = () => {
    api.showIsland().catch(() => undefined);
  };

  const toggleSource = async (info: SourceInfo) => {
    if (!info.privacyKey) return;
    const key = info.privacyKey;
    setFailedSource(null);
    try {
      await updateSettings({ privacy: { ...settings.privacy, [key]: !settings.privacy[key] } });
    } catch {
      setFailedSource(info);
    }
  };

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.home}>{greeting}</RouteHeading>

      <div className="home-launch">
        <button type="button" className="btn btn-primary" onClick={openIsland}>
          Open launcher
          {shortcut ? <span className="kbd">{shortcut}</span> : null}
        </button>
      </div>

      <div className="home-stat">
        <p className="home-stat-label">Indexed files</p>
        {status ? (
          <>
            <p className="index-stat">{indexCountText(status)}</p>
            {note ? <p className="home-stat-note">{note}</p> : null}
          </>
        ) : (
          <>
            <p className="index-stat">—</p>
            <p className="home-stat-note">
              Couldn&apos;t read the index status.{" "}
              <button type="button" className="linkbtn" onClick={() => void refreshStatus()}>
                Try again
              </button>
            </p>
          </>
        )}
      </div>

      <section className="home-sources" aria-labelledby="home-sources">
        <div className="section-head">
          <h2 id="home-sources">Sources</h2>
          <button type="button" className="linkbtn" onClick={() => onNavigate("indexes")}>
            Manage all sources
          </button>
        </div>
        {failedSource ? (
          <StateNotice
            tone="warn"
            title="Couldn't save that change."
            body="Your previous setting is still active."
            actionLabel="Try again"
            onAction={() => void toggleSource(failedSource)}
          />
        ) : null}
        {sources.length === 0 ? (
          <p className="home-empty">The local engine has not reported any sources yet.</p>
        ) : (
          <ul className="home-source-grid">
            {sources.map((source) => {
              const info = describeSource(source, settings);
              return (
                <li className="home-source" key={source.id}>
                  <span className="home-source-icon" aria-hidden="true">
                    <Icon name={sourceIconName(source.id)} size={16} />
                  </span>
                  <div className="home-source-body">
                    <p className="home-source-title">{info.title}</p>
                    <p className="home-source-desc">{info.description}</p>
                    <p className="home-source-meta">{sourceStatusText(source, info.enabled)}</p>
                  </div>
                  {info.privacyKey ? (
                    <Switch
                      checked={info.enabled}
                      label={info.title + " enabled"}
                      onChange={() => void toggleSource(info)}
                    />
                  ) : (
                    <span className="badge">Always on</span>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </section>
    </div>
  );
}
