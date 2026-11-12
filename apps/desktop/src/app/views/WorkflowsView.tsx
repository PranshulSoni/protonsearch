// Agentic workflows page: the agent runtimes (AgentsSection) and the native
// commands. A destructive command asks for a second press here, using the same
// server-side confirmation the launcher uses.

import { useState } from "react";
import { api } from "../../bridge/api";
import type { NativeCommand } from "../../bridge/types";
import { SECTION_EYEBROWS } from "../sections";
import { Icon, Panel, RouteHeading, type IconName } from "../ui";
import AgentsSection from "./AgentsSection";

interface CommandDoc {
  command: NativeCommand;
  label: string;
  description: string;
  icon: IconName;
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
        command: "lock_workstation",
        label: "Lock workstation",
        description: "Locks Windows immediately, keeping this session signed in.",
        icon: "lock",
      },
      {
        command: "sleep",
        label: "Sleep",
        description: "Puts this PC to sleep. Unsaved work in other apps may be lost.",
        icon: "moon",
      },
      { command: "restart", label: "Restart", description: "Restarts Windows; other apps will close.", icon: "restart" },
      { command: "shutdown", label: "Shut down", description: "Shuts down Windows; other apps will close.", icon: "power" },
      {
        command: "empty_recycle_bin",
        label: "Empty recycle bin",
        description: "Permanently deletes everything in the Recycle Bin.",
        icon: "trash",
      },
    ],
  },
  {
    id: "tools",
    title: "Windows tools",
    rows: [
      {
        command: "open_settings",
        label: "Open Windows Settings",
        description: "Opens the Windows Settings app.",
        icon: "settings",
      },
      {
        command: "open_task_manager",
        label: "Open Task Manager",
        description: "Opens Task Manager for the current session.",
        icon: "grid",
      },
      {
        command: "open_terminal",
        label: "Open Terminal",
        description: "Opens the default terminal in your home folder.",
        icon: "terminal",
      },
      {
        command: "toggle_dark_mode",
        label: "Toggle Windows dark mode",
        description: "Flips the Windows app theme between light and dark.",
        icon: "moon",
      },
    ],
  },
  {
    id: "indexing",
    title: "Index control",
    rows: [
      {
        command: "pause_indexing",
        label: "Pause indexing",
        description: "Stops background indexing without deleting the index.",
        icon: "pause",
      },
      {
        command: "resume_indexing",
        label: "Resume indexing",
        description: "Continues background indexing from where it stopped.",
        icon: "play",
      },
    ],
  },
];

type RowPhase = "idle" | "busy" | "confirm";

function CommandRow({ doc }: { doc: CommandDoc }) {
  const [phase, setPhase] = useState<RowPhase>("idle");
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setPhase("busy");
    setError(null);
    try {
      const outcome = await api.openResult({ type: "run_native", command: doc.command });
      // Destructive commands answer with a challenge first; the second press confirms it.
      if (!outcome.ok && outcome.message.startsWith("Confirmation required")) {
        setPhase("confirm");
        return;
      }
      setPhase("idle");
      if (!outcome.ok) setError(outcome.message || "The command could not run.");
    } catch (reason) {
      setPhase("idle");
      setError(String(reason));
    }
  };

  const confirm = async () => {
    setPhase("busy");
    setError(null);
    try {
      const outcome = await api.confirmNativeAction(doc.command);
      setPhase("idle");
      if (!outcome.ok) setError(outcome.message || "The command could not run.");
    } catch (reason) {
      setPhase("idle");
      setError(String(reason));
    }
  };

  return (
    <li className="command-row">
      <span className="command-icon" aria-hidden="true">
        <Icon name={doc.icon} size={16} />
      </span>
      <span className="command-main">
        <span className="command-title">{doc.label}</span>
        <span className="command-desc">{doc.description}</span>
      </span>
      <span className="command-action">
        {phase === "confirm" ? (
          <>
            <button type="button" className="btn btn-danger" aria-label={`Confirm ${doc.label}`} onClick={() => void confirm()}>
              Confirm
            </button>
            <button type="button" className="btn" onClick={() => setPhase("idle")}>
              Cancel
            </button>
          </>
        ) : (
          <button type="button" className="btn" disabled={phase === "busy"} onClick={() => void run()}>
            Run
          </button>
        )}
      </span>
      {error ? (
        <p className="command-error" role="alert">
          {error}
        </p>
      ) : null}
    </li>
  );
}

export default function WorkflowsView() {
  return (
    <div className="view">
      <RouteHeading eyebrow={SECTION_EYEBROWS.workflows}>Agentic workflows</RouteHeading>

      <AgentsSection />

      <Panel id="workflows-native" title="Native commands">
        {COMMAND_GROUPS.map((group) => (
          <div className="command-group" key={group.id}>
            <h3 className="command-group-title">{group.title}</h3>
            <ul className="command-list">
              {group.rows.map((row) => (
                <CommandRow key={row.command} doc={row} />
              ))}
            </ul>
          </div>
        ))}
      </Panel>
    </div>
  );
}
