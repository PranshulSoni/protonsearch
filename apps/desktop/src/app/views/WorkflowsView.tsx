// Workflows route: what can run (native commands, documented not faked), the
// optional Python agent runtime (honest not-configured state), permissions
// policy, and read-only quicklinks/snippets reporting.

import { useState } from "react";
import { useApp } from "../../App";
import { describeAgent } from "../describe";
import { SECTION_EYEBROWS } from "../sections";
import {
  Dot,
  Icon,
  Panel,
  RouteHeading,
  SavedStamp,
  StateNotice,
  Switch,
  useSavedStamp,
  type IconName,
} from "../ui";

interface CommandDoc {
  label: string;
  description: string;
  icon: IconName;
  requiresConfirmation: boolean;
}

interface CommandGroup {
  id: string;
  title: string;
  rows: CommandDoc[];
}

const COMMAND_GROUPS: CommandGroup[] = [
  {
    id: "system",
    title: "System actions",
    rows: [
      {
        label: "Lock workstation",
        description: "Locks Windows immediately, keeping this session signed in.",
        icon: "lock",
        requiresConfirmation: false,
      },
      {
        label: "Sleep",
        description: "Puts this PC to sleep. Unsaved work in other apps may be lost.",
        icon: "moon",
        requiresConfirmation: true,
      },
      {
        label: "Restart",
        description: "Restarts Windows; other apps will close.",
        icon: "restart",
        requiresConfirmation: true,
      },
      {
        label: "Shut down",
        description: "Shuts down Windows; other apps will close.",
        icon: "power",
        requiresConfirmation: true,
      },
      {
        label: "Empty recycle bin",
        description: "Permanently deletes everything in the Recycle Bin.",
        icon: "trash",
        requiresConfirmation: true,
      },
    ],
  },
  {
    id: "tools",
    title: "Windows tools",
    rows: [
      {
        label: "Open Windows Settings",
        description: "Opens the Windows Settings app.",
        icon: "settings",
        requiresConfirmation: false,
      },
      {
        label: "Open Task Manager",
        description: "Opens Task Manager for the current session.",
        icon: "grid",
        requiresConfirmation: false,
      },
      {
        label: "Open Terminal",
        description: "Opens the default terminal in your home folder.",
        icon: "terminal",
        requiresConfirmation: false,
      },
      {
        label: "Toggle Windows dark mode",
        description: "Flips the Windows app theme between light and dark.",
        icon: "moon",
        requiresConfirmation: false,
      },
    ],
  },
  {
    id: "indexing",
    title: "Index control",
    rows: [
      {
        label: "Pause indexing",
        description: "Stops background indexing without deleting the index.",
        icon: "pause",
        requiresConfirmation: false,
      },
      {
        label: "Resume indexing",
        description: "Continues background indexing from where it stopped.",
        icon: "play",
        requiresConfirmation: false,
      },
    ],
  },
];

export default function WorkflowsView() {
  const { settings, status, updateSettings } = useApp();
  const { savedKey, markSaved } = useSavedStamp();
  const [actionError, setActionError] = useState<{
    title: string;
    body: string;
    retry?: () => void;
  } | null>(null);
  const agent = describeAgent(status?.agent, settings);
  const provider = settings.agent.provider ?? "No provider configured";
  const model = settings.agent.model ?? "No model configured";
  const legacyMigrated = settings.legacyMigrated;

  const toggleAgent = async (next: boolean) => {
    setActionError(null);
    try {
      await updateSettings({ agent: { ...settings.agent, enabled: next } });
      markSaved("agent");
    } catch {
      setActionError({
        title: "Couldn't save that change.",
        body: "Your previous setting is still active.",
        retry: () => void toggleAgent(next),
      });
    }
  };

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.workflows}>Workflows</RouteHeading>
      <p className="page-lead">
        Native commands respond instantly. The Python agent sidecar is optional and starts only when
        needed.
      </p>

      {actionError ? (
        <StateNotice
          role="alert"
          title={actionError.title}
          body={actionError.body}
          actionLabel={actionError.retry ? "Try again" : undefined}
          onAction={actionError.retry}
        />
      ) : null}

      <Panel
        id="workflows-native"
        title="Native commands"
        hint="These run from the launcher. Destructive commands require an explicit confirmation there before anything happens."
      >
        {COMMAND_GROUPS.map((group) => (
          <div className="command-group" key={group.id}>
            <h3 className="command-group-title">{group.title}</h3>
            <ul className="command-list">
              {group.rows.map((row) => (
                <li className="command-row" key={row.label}>
                  <span className="command-icon" aria-hidden="true">
                    <Icon name={row.icon} size={16} />
                  </span>
                  <span className="command-main">
                    <span className="command-title">{row.label}</span>
                    <span className="command-desc">{row.description}</span>
                  </span>
                  {row.requiresConfirmation ? (
                    <span className="badge badge-warn">Confirmation required</span>
                  ) : null}
                </li>
              ))}
            </ul>
          </div>
        ))}
        <p className="field-help">
          This page documents the command surface; command execution is verified in the launcher.
        </p>
      </Panel>

      <Panel
        id="workflows-agent"
        title="Agent runtime"
        hint="Agent workflows are optional. The runtime starts only when you run one, then exits when idle."
        actions={<SavedStamp show={savedKey === "agent"} />}
      >
        <div className="config-row">
          <div className="config-main">
            <div className="config-title">
              <Dot tone={agent.tone} />
              {agent.stateLabel}
            </div>
            {agent.tone !== "off" ? <p className="config-desc">{agent.detail}</p> : null}
          </div>
          <Switch
            checked={settings.agent.enabled}
            label="Enable agent workflows"
            onChange={(next) => void toggleAgent(next)}
          />
        </div>
        {agent.tone === "off" ? (
          <StateNotice
            className="state-inline"
            title="Agent workflows are off."
            body="ProtonSearch works fully without them."
          />
        ) : null}
        <dl className="metric-list">
          <div className="metric-row">
            <dt>Runtime</dt>
            <dd>{agent.runtime}</dd>
          </div>
          <div className="metric-row">
            <dt>Provider</dt>
            <dd>{provider}</dd>
          </div>
          <div className="metric-row">
            <dt>Model</dt>
            <dd>{model}</dd>
          </div>
        </dl>
        {settings.agent.enabled && !agent.installed ? (
          <StateNotice
            tone="warn"
            title="Agent workflows aren't set up."
            body="Your preference is saved, but no runtime is installed in this build. Nothing was downloaded or started."
          />
        ) : null}
        {agent.tone === "error" ? (
          <StateNotice
            tone="error"
            role="alert"
            title="Agent workflows are unavailable."
            body="Search still works. The optional runtime didn't start."
          />
        ) : null}
        <p className="field-help">
          No models are downloaded automatically. Installing and configuring the runtime is not
          available in this build.
        </p>
      </Panel>

      <Panel
        id="workflows-permissions"
        title="Agent permissions"
        hint="Policy the optional runtime must follow when it ships. No agent runtime is installed in this build."
      >
        <dl className="metric-list">
          <div className="metric-row">
            <dt>Files &amp; folders</dt>
            <dd>Read only</dd>
          </div>
          <div className="metric-row">
            <dt>Clipboard</dt>
            <dd>Ask every time</dd>
          </div>
          <div className="metric-row">
            <dt>Shell commands</dt>
            <dd>Ask every time</dd>
          </div>
          <div className="metric-row">
            <dt>Network</dt>
            <dd>Ask every time</dd>
          </div>
        </dl>
        <p className="field-help">
          Writes, deletes, shell commands, and network access always ask first. Nothing leaves this
          PC unless you configure a provider and approve an action.
        </p>
      </Panel>

      <Panel
        id="workflows-quicklinks"
        title="Quicklinks &amp; snippets"
        hint="Part of your local library, not a separate editor."
      >
        <p className="muted">
          Quicklinks and snippets surface as launcher results once the library is available to the
          new index. This build has no bridge command to list or edit them, so no editor is shown
          rather than a dead control.
        </p>
        <p className="field-help">
          Migration status:{" "}
          {legacyMigrated
            ? "the engine reports the legacy library as migrated."
            : "not reported by the engine yet. Old data is opened read-only and never modified."}
        </p>
      </Panel>
    </div>
  );
}