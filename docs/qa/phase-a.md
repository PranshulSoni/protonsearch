# Phase A integration checkpoint (IC-A) - 2026-11-09

Status: **automated gates PASS; native checks BLOCKED**. Phase A is not accepted and
Phase B must not start until the native gate is either observed or explicitly waived
by the user. Nothing in this file is native-verified.

## Tree under test

* Branch `dock`, HEAD `69b407fb871f6ee90ca380bb5545f2e6ccbe1cd4` (unchanged before and after).
* The tree was **not clean** at the start: 9 tracked paths are modified/deleted by another
  lane (legacy ProtonSearch: `protonsearch/Cargo.toml`, `build.rs`, `indexer.rs`, `main.rs`,
  `taskbar_recovery.rs`; plus `.gitignore`, `docs/qa/STATUS.md`, `preview-tahoe-glass.html`,
  deleted `protondock-preview.html`) and 3 tracked `graphify-out/` files were already modified.
  Gates therefore certify **this dirty tree**, not a clean checkout. Gates 1-9 changed no
  tracked file (verified by before/after `git status`); `dist/` is gitignored. IC step 7
  (`graphify update .`) then rewrote the three already-dirty `graphify-out/` files; HEAD did
  not move and no new path appeared, but the tree is no longer byte-identical to the gate
  snapshot. `graphify-out/` is never staged.
* Environment: Windows 11, Node v26.7.0, npm 11.19.0, cargo 1.99.0, git 2.55.0.
* Before gates: `protondock.exe` and `protonsearch.exe` were killed.

## Automated gates (all commands run from the repo root)

| # | Command | Exit | Result |
|---|---|---|---|
| 1 | `node scripts/html-pin.mjs` | 0 | `dock-editable.html drift: yes (0d8c471d...)`; last line `html-pin: ok` |
| 2 | `npx tsc -b` | 0 | no diagnostics |
| 3 | `npm test -- --reporter=dot` | 0 | **23 test files, 224 tests passed** (11.00 s first run; 8.11 s on the audit re-run - duration is load-dependent) |
| 4 | `npm run widgets:validate` | 0 | prints `Validated 29 widget IDs ...` (shallow schema check; see F6) |
| 5 | `npm run build` | 0 | Vite 7.3.7, 61 modules; `dist/assets/index-DJprqrGc.js` 341.75 kB |
| 6 | `cargo test --manifest-path src-tauri/Cargo.toml -- --skip local_calendar::tests::` | 0 | 4 binaries: **108 passed, 0 failed, 0 ignored** (84 + 3 + 8 + 13; the 6 filtered are the `local_calendar::tests` module in the main binary) |
| 7 | `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::tests::` | 101 | **1 passed, 5 failed, 84 filtered** - known fixture failures, see below |
| 8 | `cargo test --manifest-path protonsearch/Cargo.toml --lib weather` | 0 | **12 passed, 0 failed** (40 filtered) |
| 9 | `npx tauri build --debug --no-bundle` | 0 | `src-tauri/target/debug/protondock.exe`, 31,498,752 bytes; SHA-256 is build-instance-specific (preserved copy `22A0843C...`, rebuild `0712C3FB...`) |

Notes:

* Gate 1 prints `drift: yes` by design; the pinned copy is the authority and the drift line
  never fails the run (task A1). The worktree `dock-editable.html` hash is `0d8c471d`.
* Gate 6 reported exit 0 on a verbatim re-run (`CARGO_EXIT=0`). The first invocation showed a
  pwsh wrapper exit 1 while all four test binaries were already green; no failed test or
  compile error was ever observed. The 5 `ProtonSearch` warnings are pre-existing dead-code
  and naming warnings.
* Gate 7 failure site is `src-tauri/src/local_calendar.rs:504:40`, `Fixture` `create_dir`
  `Os { code: 5, kind: PermissionDenied }`. `docs/qa/STATUS.md:698-699,809-810` still says
  "ten ... failures at `local_calendar.rs:465`"; that is stale (the plan only requires
  recording the count). Current module has 6 matching tests: 1 passes, 5 fail. Counts are environmental (temp-dir creation
  denied), not a code regression.
* Gate 9 copies the artifact to `%TEMP%\protondock-ic\protondock.exe` (same size). The copy was
  **not launched**, because launching without Computer Use cannot be observed and would be an
  unverified claim. The SHA-256 is **not a reproducibility claim**: two builds of the same
  sources differ in 24 bytes: the PE `TimeDateStamp` (`0x100`), three debug-directory timestamps
  (`0x1A20304`, `0x1A20320`, `0x1A2033C`) and the 16-byte CodeView RSDS GUID (`0x1A21078`).
  Size is stable (31,498,752 B); SHA-256 changes. The preserved temp copy matches the first
  build (`22A0843C...`); the audit rebuild produced `0712C3FB...`.
* **Artifact side effect (expected).** The values above were captured at build time
  (2026-11-09 03:17 IST). Gate re-runs that execute `cargo test` relink the binary target, so
  `src-tauri/target/debug/protondock.exe` was later replaced (31,357,952 B, SHA-256
  `D8027276...`). This is normal cargo behaviour, not a gate failure. The retained artifact is
  the `%TEMP%\protondock-ic\protondock.exe` copy (`22A0843C...`); Gate 9 is evidence, not a
  reproducibility gate.

## Wiring checks (static, from plan IC-A)

* `src-tauri/src/main.rs:755` - `_ => return providers::snapshot(&provider_id),` (registry fallthrough).
* `src-tauri/src/widget_data.rs:339` - action delegates to `crate::providers::action(action)` after
  the native lock is dropped.
* `src-tauri/src/main.rs:458,467` - settings load/save go through `layout_store`;
  `main.rs:635` (weather) and `main.rs:655,665,682` (calendar select/snapshot/forget) read the
  layout through `layout_store::load` closures.
* A recursive `Select-String` for `protondock-layout` (this host has no `grep` on PATH) prints
  only the two constants in `src-tauri/src/layout_store.rs:4-5` (A13 exit condition).
* `src-tauri/src/main.rs:1077,1083` - hotkey registration records success/failure and has no `?`
  on `register`, so a taken Alt+Space no longer aborts startup.
* `cargo test --test ipc_manifest` is part of gate 6: 3 passed (manifest/handler parity).
* `git status --short -uall -- src src-tauri` lists only generated
  `src-tauri/gen/` and `src-tauri/permissions/autogenerated/`; no untracked source files.

## IC step 7 - knowledge graph refresh

`graphify update .` -> exit 0. Output: `Rebuilt: 3089 nodes, 6110 edges, 187 communities`;
it backed up the curated graph to `2026-11-09/` and reported 16 code files that yielded no
symbols (Cargo.toml, `protonsearch/src/ui.rs`, install scripts, `DockSurface.test.tsx`, ...).
That count was printed to the session log only; no persisted CLI log exists, so it is
informational and not part of the gate.
The three tracked `graphify-out/` files were already dirty and are **never staged**.

## Native checks - BLOCKED, not verified

Blocker (exact): the bundled `computer-use` skill requires the `orca` CLI, and
`Get-Command orca` returns nothing on this host. Without it the app cannot be inspected,
so the launch step was skipped rather than claimed.

| Check | What must be observed | Status |
|---|---|---|
| N-A1 jitter (bar 1) | 60 s idle, 10 hover cycles, 5 Daily-Focus switches: rail edges never move | **unverified - needs computer-use (`orca` missing)** |
| N-A2 icons (bar 2) | Notepad + one small-icon app beside the Windows taskbar icon, rest and hover peak | **unverified - needs computer-use** |
| N-A3 hot on blur | Hover rail, Alt+Tab away: shelf visible within 300 ms | **unverified - needs computer-use** |
| N-A4 ACL | Widget card, Customize and Spotlight open from the dock | **unverified - needs computer-use** |
| N-A5 layout safety | Switch profile, relaunch: restored; `.bak` exists after save | **unverified - needs computer-use + launch** |
| N-A6 hotkey | Only if another program holds Alt+Space: dock still starts | **unverified - needs computer-use** |

Unverified gates are recorded as blocked, not passed. No fabricated metrics are used anywhere
in this page.

## Findings outside this checkpoint's scope (not fixed)

* **F1 - untracked build inputs.** `scripts/widget-create.mjs`, `scripts/widgets-validate.mjs`,
  `scripts/tahoe-preview.test.mjs`, `scripts/patch-icon-sharpness.cjs` and
  `.github/workflows/widgets.yml` are untracked, and `package.json` wires
  `npm run widgets:validate` to one of them. A clean checkout cannot run gate 4. A0 tracked
  source files but not these scripts; recommend one follow-up task to track them.
* **F2 - uncommitted evidence/docs.** `docs/qa/dock-defects-2026-11-07.md`, many `docs/qa/*`
  files, `docs/HANDOFF.md`, `docs/widgets/*` and `docs/superpowers/plans/2026-11-05-*.md` are
  untracked, so the plan's own evidence references do not survive a clean checkout.
* **F3 - stale known-failure record.** `docs/qa/STATUS.md:698-699,809-810` says "ten
  ... failures at `local_calendar.rs:465`"; the plans only require recording the count. The
  current count is 5 of 6 tests at `local_calendar.rs:504:40`. Update when the native gate is
  revisited.
* **F4 - pre-existing graph modifications.** `graphify-out/GRAPH_REPORT.md`, `graph.html`,
  `graph.json` were already modified before this checkpoint and were rewritten by IC step 7
  (`graphify update .`, exit 0: 3089 nodes / 6110 edges / 187 communities).
  `GRAPH_REPORT.md` alone shows 885 changed lines (684 insertions / 201 deletions); the
  three-file total is 689 insertions / 206 deletions. Intentionally not staged.
* **F5 - preview deletion.** `protondock-preview.html` is deleted in the worktree but not
  committed; left untouched because it belongs to another lane.
* **F6 - shallow widget validation.** `npm run widgets:validate` seeds 28 IDs from
  `src/design/reference-widgets.json` and asserts full manifest/contract/fixture shape for
  only the single built-in directory (`src/widgets/builtins/storage-health`); it also asserts
  the catalog smoke test exists. Printing `29 widget IDs` therefore is **schema coverage, not
  per-widget contract coverage**. Revisit when more built-ins land; do not cite 29 as coverage.

## Verdict

* **Automated IC-A: PASS** for the tree at `69b407f` (gate 7 is the documented known-count;
  gate 8's warnings are pre-existing; IC step 7 refreshed the graph; gate 9's hash is
  build-instance-specific, not a reproducibility gate).
* **Forensic audit:** three rounds; final verdict **CLEAN** (round 3). Rounds 1-2
  found documentation precision defects only (hash labelling, widget-coverage wording, graph
  numbers, stale-count attribution); each was corrected and independently re-verified. No
  fabricated metric was found. Audit log: `.agents/teamwork/PROJECT.md`.
* **Native IC-A: BLOCKED** by the missing `orca` CLI. Phase A is code-complete but **not
  accepted**. Phase B stays unexpanded until the user either installs Computer Use/`orca`,
  verifies N-A1..N-A6 manually, or explicitly waives the native gate. The waiver must be
  written here before Phase B starts.
