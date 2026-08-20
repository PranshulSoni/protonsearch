# Linux distribution test matrix

Only an executed test receives `PASS`. `MAPPED` means the shared backend and
dependency mapping are implemented but the required clean desktop test has not
run. `BLOCKED` records unavailable test infrastructure rather than hiding it.

| Feature | Arch + Hyprland | Ubuntu + GNOME | Mint + Cinnamon | Debian + GNOME/KDE | Fedora + GNOME |
| --- | --- | --- | --- | --- | --- |
| Install | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Launch/service | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Alt+Space | PASS* | MAPPED | MAPPED | MAPPED | MAPPED |
| Search/files/folders | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Settings | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Dependencies | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Light mode | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Dark mode | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Tray | PASS* | MAPPED | MAPPED | MAPPED | MAPPED |
| Hermes detection | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Real Agent prompt | PASS (CLI path) | BLOCKED | BLOCKED | BLOCKED | BLOCKED |
| Agent History | MAPPED | MAPPED | MAPPED | MAPPED | MAPPED |
| Restart behavior | PASS | MAPPED | MAPPED | MAPPED | MAPPED |
| Idle CPU/memory | PASS | BLOCKED | BLOCKED | BLOCKED | BLOCKED |

`*` Arch evidence is from the current developer desktop and is not a claim
about every Arch desktop environment. The clean-install and package artifact
details are recorded in `docs/PRODUCTION_RELEASE_CHECKLIST.md`.

## Required per-distribution run

For each primary distribution, use a clean VM or real installation and record:

1. The exact image/version and desktop/display-server combination.
2. The public install command and all optional-dependency prompts.
3. `doctor` output with secrets removed.
4. Launcher, settings, theme, tray, search, clipboard, and restart results.
5. A real ProtonSearch → Hermes → Agent response and Agent History reopen.
6. Cold/warm startup, search latency, idle CPU, memory, and task counts.

If a VM or desktop session is unavailable, leave the row `BLOCKED` and report
the infrastructure gap instead of upgrading it to PASS.
