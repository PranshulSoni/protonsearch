# Proton 2.0 planning pack

Prepared 20 October 2026 (Asia/Kolkata). Branch: `2.0.0`.
Baseline application commit: `98d52ce` on `1.1.1`.

**Revised visual direction:** read [Glass dock design plan](glass-dock-design.md).
It supersedes the green concept's appearance and uniform tiles. The old HTML illustration
is retained as a historical concept until the revised design is reviewed and visualized.

Open [index.html](index.html) directly in a browser. All assets are local; no build,
CDN, account, package installation or application execution is required.
For an HTTP preview from the repository root:

```powershell
python -m http.server 8765 --bind 127.0.0.1 --directory docs/2.0.0
```

The browser site is a planning artifact with sample data. It does not call Windows
APIs, index files, run agents, hide the taskbar or persist a dock configuration.

## Read in this order

1. [Overview and interactive concept](index.html)
2. [Product and interaction requirements](product.html)
3. [System architecture](architecture.html)
4. [Tech stack and renderer decision](stack.html)
5. [Windows integration and recovery](windows.html)
6. [Widget SDK and profiles](widgets.html)
7. [Agent runtime and capability policy](agents.html)
8. [Roadmap and release gates](roadmap.html)
9. [Repository audit and data migration](migration.html)
10. [Primary sources and installed tooling](sources.html)

The canonical written specification is [specification.md](specification.md).
The actionable task sequence is [implementation-plan.md](implementation-plan.md).
The tooling pin is [tooling-lock.json](tooling-lock.json).

## Main decisions

- Product: customizable Windows dock; Search is one core experience.
- Core: retain Rust, windows-rs, SQLite FTS5 and existing indexing behavior.
- UI: Slint first candidate, identical Tauri 2 + React/TypeScript spike before lock.
- Native Windows taskbar: coexist first, optional reversible hiding only after gates.
- Widgets: shared lifecycle/configuration and constrained host-rendered views.
- Agents: separate scheduler, per-run capabilities, explicit execution action.
- Data: preview identity separate from v1; consistent backup and reversible import.
- Ecosystem: first-party widgets, declarative packages, sandboxed logic later.

These are recommendations, not user-approved implementation decisions.
Performance numbers are targets; no 2.0 runtime has been built or benchmarked.
Unresolved platform questions have experiments/fallbacks rather than invented answers.

## Installed skill

`.agents/skills/rust-skills` is vendored from
`leonardomso/rust-skills` at
`fd2a861ab0406a4ac536a55274d14ea6fd1ca9c9`.
Upstream MIT license retained. Available to Codex on the next turn.
Imported optional checks/examples are tooling assets and were not run.
No new MCP server, application dependency or production configuration was installed.

## Scope and repository rules

Application source, production package versions, installer, updater and existing README
remain unchanged. This branch contains the planning portal and pinned skill.
The repository requires commit/push after edits and prohibits graphify and
background launcher execution. The app was not launched or built during planning.
