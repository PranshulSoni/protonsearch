// Preferences → Setup. Honest summary of first-run choices with links to the
// real settings that own them. The shell has no reset-onboarding command in this
// build, so no dead "Run setup again" control is rendered.

import { useApp } from "../../App";
import { formatNumber } from "../../lib/format";
import type { Section } from "../sections";
import { CARD_META } from "../islandLogic";
import { Icon } from "../ui";

export default function OnboardingSettings({
  onSelectCard,
  onNavigateRoute,
}: {
  onSelectCard: (card: "general" | "cards" | "behavior" | "onboarding") => void;
  onNavigateRoute: (section: Section) => void;
}) {
  const { settings } = useApp();
  const roots = settings.indexing.roots.filter((root) => root.enabled);
  const complete = settings.onboardingComplete;
  const launcher = "Dynamic Island";

  return (
    <div className="settings-grid settings-grid--single">
      <section className="settings-section" aria-labelledby="prefs-setup">
        <div className="settings-label">
          <h2 id="prefs-setup">First-run setup</h2>
        </div>
        <div className="panel">
          <dl className="metric-list">
            <div className="metric-row">
              <dt>Status</dt>
              <dd>{complete ? "Complete" : "Not finished"}</dd>
            </div>
            <div className="metric-row">
              <dt>Primary launcher</dt>
              <dd>{launcher}</dd>
            </div>
            <div className="metric-row">
              <dt>Shortcut</dt>
              <dd>{settings.hotkey.replace(/\+/g, " + ")}</dd>
            </div>
            <div className="metric-row">
              <dt>Included locations</dt>
              <dd>{formatNumber(roots.length)}</dd>
            </div>
            <div className="metric-row">
              <dt>Agent workflows</dt>
              <dd>{settings.agent.enabled ? "Enabled" : "Off"}</dd>
            </div>
          </dl>
          <p className="field-help">
            First-run setup runs once. These choices stay editable here and in Search &amp;
            indexes; this build has no separate “run setup again” command.
          </p>
          <div className="section-actions">
            <button type="button" className="btn" onClick={() => onSelectCard("cards")}>
              <Icon name="clipboard" size={14} /> Cards &amp; order
            </button>
            <button type="button" className="btn" onClick={() => onSelectCard("behavior")}>
              <Icon name="workflows" size={14} /> Open behavior
            </button>
            <button type="button" className="btn" onClick={() => onNavigateRoute("indexes")}>
              <Icon name="folder" size={14} /> Search &amp; indexes
            </button>
            <button type="button" className="btn" onClick={() => onSelectCard("general")}>
              <Icon name="settings" size={14} /> General settings
            </button>
          </div>
        </div>
      </section>
      <section className="settings-section" aria-labelledby="prefs-setup-cards">
        <div className="settings-label">
          <h2 id="prefs-setup-cards">Default island card</h2>
        </div>
        <div className="panel">
          <p className="pref-title">{CARD_META[settings.launcher.island.defaultCard].title}</p>
          <p className="pref-desc">
            Change card order, visibility and the default in Cards &amp; order.
          </p>
          <div className="section-actions">
            <button type="button" className="btn" onClick={() => onSelectCard("cards")}>
              <Icon name="clipboard" size={14} /> Cards &amp; order
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
