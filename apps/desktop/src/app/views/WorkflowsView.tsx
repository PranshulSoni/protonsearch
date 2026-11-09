// Workflows route: documented native commands, the optional on-demand Pi agent
// workflow panel, and read-only quicklinks/snippets reporting.

import { useApp } from "../../App";
import { SECTION_EYEBROWS } from "../sections";
import { Icon, Panel, RouteHeading, type IconName } from "../ui";
import PiAgentPanel from "./WorkflowsAgentPanel";

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
  const { settings } = useApp();
  const legacyMigrated = settings.legacyMigrated;

  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.workflows}>Workflows</RouteHeading>
      <p className="page-lead">
        Native commands respond instantly; the optional Pi agent runs only when you start a workflow.
      </p>

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

      <PiAgentPanel />

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