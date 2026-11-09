# ProtonSearch v2 — v10 Gate 0 Audit

Owner: L1-E (rel-recon). Date: 2026-11-10. Binding sources: docs/reference/v10-handoff/* (master
prompt, V9 Dynamic Island HTML, settings placement concept, V9 audit, README). No code changes were
made for this audit. This document records environment, repository/legal state, feature inventory,
data/backup locations, P0 reproduction plans, V9 reference inventory, benchmark baseline, honest
blockers and Gate-0 status.
**Revision 2026-11-10b:** records the L0-ratified staffing structure (L0 + 5 L1 + 3 L2-scope = 9
members; 8-teammate platform cap; no L3 workers) and the L0-created Gate-0 backup. The G0-E
documentation package was accepted by L0 on 2026-11-10; full Gate 0 remains in progress until the
G0-A/B/C/D evidence lands.

## 1. Environment (verified on this host)

| Item | Value | How verified |
|---|---|---|
| OS | Microsoft Windows 11 Home Single Language, 10.0.26300, 64-bit | Get-CimInstance Win32_OperatingSystem |
| Boot / session | last boot 2026-11-09 02:59 local; interactive desktop session | Win32_OperatingSystem / session state |
| CPU | AMD Ryzen AI 9 465 w/ Radeon 880M, 10 cores / 20 logical | Win32_Processor |
| RAM | 31.2 GB | Win32_ComputerSystem |
| Disk | C: 352,403,767,296 bytes free at audit time | Get-PSDrive C |
| GPU | AMD Radeon(TM) 880M Graphics, driver 32.0.22042.11003 | Win32_VideoController |
| Display (physical) | single built-in panel SDC ATNA60HR07-0, 2880x1800, 120 Hz | WmiMonitorID active=True; Win32_VideoController |
| Effective scaling | 150% (AppliedDPI 144; logical desktop 1920x1200 for 2880x1800) | WindowMetrics + WinForms Screen |
| Display config | ONE active monitor (\.\DISPLAY1 primary). Dual/mixed-DPI/disconnect NOT AVAILABLE on this host | Screen.AllScreens; WmiMonitorID |
| WebView2 | 157.0.4341.42 | EdgeUpdate clients registry; msedgewebview2 processes present |
| Rust / Cargo | rustc 1.99.0, cargo 1.99.0 | rustc/cargo --version (recorded in prior gate evidence) |
| Node | v26.7.0 | node --version |
| Python | 3.14.7 (MSYS2); project venvs as documented in docs/agent | python --version |
| Antivirus / Defender state | NOT MEASURED | not instrumented at Gate 0 |
| AC/DC / battery state | NOT MEASURED (no battery telemetry captured) | not instrumented at Gate 0 |
| Accessibility test tooling (Narrator/High Contrast) | NOT CONFIGURED for automated runs | not instrumented at Gate 0 |

Implication: 100/125/150/200% scaling can be exercised by changing Windows scaling, but the host is a
single 2880x1800 120 Hz display. Claims that require a second monitor, mixed DPI, disconnect/reconnect
or AC/DC battery state are BLOCKED at Gate 0 and must be recorded as gaps - not simulated.

## 2. Repository, history and legal state

- Branch: `v2-phoenix`. HEAD: `0e4c9ec` (docs(reference): import v10 handoff package).
- Rollback baseline tag: `pre-v10-handoff` = `7fe878e23f99206b2766e0d2b9d09789b9c2aaf0`. The working
  tree was clean at audit time (git status count 0).
- Remote: origin https://github.com/PranshulSoni/protonsearch (public GitHub). No pushes performed.
- Licence: MIT License, Copyright (c) 2026 Pranshul Soni (LICENSE at repo root). The v2 workspace
  declares MIT (Cargo.toml [workspace.package]).
- Prior phase evidence retained: tag v1.1.1-v2-baseline (legacy baseline), v2 G0-G7 evidence in
  docs/audit/**, docs/implementation/**, docs/performance/**, docs/security/**, docs/release/**.
- v10 handoff package imported under docs/reference/v10-handoff/ (master prompt 55,714 B; V9 HTML
  47,347 B; settings concept HTML 18,924 B; V9 audit 3,845 B; README 2,158 B; logo PNG 7,125 B;
  two screenshots 90,803/93,101 B).
- Hygiene note: the repo root contains gitignored scratch/data directories from prior phases
  (ps-migration-fixture-* x22, .devdata, .devdata-live, .devdata-mig, .devdata-mig2, .bench,
  ocr-tmp, dist-installer, target). They are not tracked; they should be cleaned or archived before
  a release package is produced. Do not delete blindly - .devdata* and fixtures may contain evidence;
  L0 decision required.

## 3. Existing feature inventory (current tree, honest summary)

Implemented and evidenced in the v2 phase (see docs/implementation/ACCEPTANCE_REPORT.md,
FEATURE_PARITY_MATRIX.md, docs/performance/BENCHMARKS.md):

- Rust workspace crates: ps-ipc (frozen IPC v1 + agent events), ps-core (engine facade, query
  coordinator/ranker, settings, activity, migration), ps-index (compact in-memory filename index +
  scan/persist/matching), ps-content (FTS5 content index, extractors, legacy import), ps-sources
  (apps/recent/clipboard/browser/git/quicklinks/calculator/actions), ps-windows (Win32 shell/icons/
  clipboard/foreground window/monitors/DPI), ps-ocr (Windows.Media.Ocr pipeline), ps-agent (Pi runtime
  hardened, read-only default), ps-bench (dev benchmark CLI).
- Tauri 2 shell (apps/desktop/src-tauri): search, open_result, native confirm handshake, settings,
  activity, onboarding, index command, launcher window control, agent status/start/cancel/approve/stop,
  tray, single instance, global shortcut, exit cleanup. Compiles (cargo check -p protonsearch-v2 exit 0
  at prior verification).
- React/TS UI: launcher surface with grouped results/preview panel/keyboard model; desktop app with
  Home/Search&Indexes/Activity/Workflows/Preferences; onboarding; theme/density/motion; Pi agent panel
  (read-only default with per-run write consent UI; server-side write-consent gate still open).
- Data/tests: legacy DB forensics, read-only migration with WAL/SHM hashing, Pi security hardening
  (71 ps-agent tests at task-16), v2 IPC/engine tests, benchmark harness/results, CI skeleton,
  unsigned NSIS installer artifact built in the v2 phase.
- Current v2 feature statuses: see FEATURE_PARITY_MATRIX.md (rows 1-39) and Appendix C (v10 rows).

Explicit v10 gaps (not implemented at Gate 0): Dynamic Island and its eight cards; Normal Launcher
All/Files/Folders/Apps filter row + required quick destinations; graphical island placement editor;
card manager; media/output-device/Quick Settings cards; browser/git/notifications/agents island cards;
real preview pipeline repair (P0-1); clipboard image repair (P0-2); server-side agent write consent;
signed updater; v10 Windows E2E suite.

## 4. User data inventory and backup plan

| Location | Size / facts | Handling |
|---|---|---|
| %APPDATA%\protonsearch\file_index.db (legacy v1) | 149,372,928 B; SHA-256 58249F011A045E74F02582F50722C80699E28BD6EDBE99073F567D386F361809 | open READ-ONLY only; never mutate; include in pre-upgrade backup |
| %APPDATA%\protonsearch\file_index.db-wal | 23,788,912 B (un-checkpointed) | back up WAL + SHM together; never copy .db alone |
| %APPDATA%\protonsearch\file_index.db-shm | 32,768 B | back up with WAL |
| %APPDATA%\protonsearch\ (logs/layout/notes) | indexer.log 1,063,069 B; git_indexer.log 27,073 B; protondock-layout.json 1,602 B + .bak; empty notes/; stale 0-byte index.db | back up settings/layout; logs are diagnostic only |
| %APPDATA%\protonsearch\v2\ | currently only cache2 at audit time (v2 data dir) | back up entire v2 dir before migration/update |
| Repo evidence/data dirs | .devdata 30.8 MB / 603 files; .devdata-live 553.6 MB / 408; .devdata-mig 324.5 MB / 184; .devdata-mig2 303.6 MB / 209; .bench 185 B/1 file + state/golden/migration dirs (migration 1,717,958,060 B/3,659 files) | do not delete without L0; archive only if not needed for reproduction |
| Rollback tag | pre-v10-handoff = 7fe878e | source rollback reference; not a data backup |
| Installer artifacts | dist-installer (unsigned NSIS + exe + SHA256SUMS + BUILD_INFO from v2 phase) | keep as historical artifact; v10 artifacts must be rebuilt and re-hashed |

### 4b. Gate-0 backup (created by L0, 2026-11-10 02:05, verified read-only)

Path: %USERPROFILE%\ProtonSearch-Backups\pre-v10-20261110-0205
(observed: C:\Users\Pranshul\ProtonSearch-Backups\pre-v10-20261110-0205)

| Item | Size | Verification |
|---|---:|---|
| repo-pre-v10.bundle | 79,717,794 B | SHA-256 124B911FBBA92560D2242FDCE00F331BE9269884B81214CB647563996885639A; git bundle verify exit 0: bundle contains 37 refs, records a complete history, is okay; HEAD = 0e4c9ec249ef547d3fa9582878ea62aa81bb4220 |
| HEAD.txt | 86 B | SHA-256 7FE1A8D9C09595C862B981E5334BEF76272F6DA7409B3210F30DF976B545B3B7; content 0e4c9ec249ef547d3fa9582878ea62aa81bb4220 |
| legacy-db.sha256 | 134 B | SHA-256 CF863FA6C9FB9D47FEEFAE238550D45416856A32799A1A74C0AF60B25571413E; content 58249F011A045E74F02582F50722C80699E28BD6EDBE99073F567D386F361809 (matches the live legacy DB hash) |
| appdata-protonsearch/ | 174,287,955 B / 8 files | contains file_index.db 149,372,928 B, file_index.db-wal 23,788,912 B, file_index.db-shm 32,768 B, indexer.log, git_indexer.log, protondock-layout.json + .bak, stale 0-byte index.db, empty notes/ and v2/ |

Restore/rollback procedure: restore source from the bundle (git clone or git fetch <bundle>), check
out HEAD 0e4c9ec; restore appdata-protonsearch/ into %APPDATA%\protonsearch only while the app is not
running; verify file_index.db SHA-256 against legacy-db.sha256 after restore. Keep the
pre-v10-handoff tag as an additional source reference. No user data was modified by this audit
(read-only commands only). Backup refresh is mandatory before any schema/migration change or G2 data
work; an external-drive copy remains a nice-to-have (not created).

## 5. P0 bug reproduction plan (status: in progress)

Full detail and acceptance criteria: docs/implementation/BUG_LEDGER.md.

- BUG-P0-01 (previews do not show): owner G0-A/task-29 (engine-core); plan = controlled preview corpus
  (TXT/code/MD/HTML/JSON/CSV/PDF text/scanned PDF/DOCX/images/OCR/Unicode/0-byte/large/locked/encrypted),
  capture IPC SearchResult preview fields, preview request/response, UI state/screenshots, console
  errors, stale/cancellation behavior, compare engine snippets to displayed preview; both launcher
  modes (island blocked until G5 and must be marked blocked, not skipped). No reproduction result is
  recorded in this audit yet; G0-A is still running.
- BUG-P0-02 (clipboard images missing): owner G0-D/task-30 (sources-content); plan = real copy tests
  from Notepad/VS Code/Paint/Snipping Tool/browser image/HTML/File Explorer; capture clipboard formats,
  listener/decode result, DB row/image_path/hash, UI row/thumbnail, restart persistence and paste-back;
  dedupe one event across formats; locked/empty/rapid/restart/corrupt cases. Code audit note: v2 already
  contains CF_DIB decode/PNG encode and image recording, so G0-D must find the broken link rather than
  assume absent code. No reproduction result yet; G0-D is still running.
- Environment for both: Windows 11 build 26300, single 2880x1800 @120 Hz, effective 150% DPI, WebView2
  157.0.4341.42; record debug/release build hash per run.

## 6. V9 reference inventory (source of truth for G5/G6)

- protonsearch_dynamic_island_cards_v9_polished.html (47,347 B) - authoritative island visual/motion
  reference. Eight logical cards and default order: search, media, clipboard, agent, controls
  (Quick Settings), browser, git, bell (Notifications). V9 audit confirms: hidden-by-default capsule;
  Alt+Space summon; eight clickable pagination dots with accessible names/active state; wheel/swipe
  switching; spring-like width/height transitions; split Wi-Fi/Bluetooth buttons (toggle vs chevron
  drill-down); fixed clipboard code sizing/wrapping; media alignment/output device; agent list
  completeness; responsive 1150/430/335 px; versioned card-render token; reduced-motion support;
  dark graphite/navy visual language; no radial wheel or navigation rail.
- protonsearch_island_settings_placement_concept.html (18,924 B) - placement/card-management reference:
  monitor preview with selectable display tabs, V9 island ghost drag, 9 anchor presets, X/Y offsets,
  monitor strategy (primary/cursor/foreground/specific), keep-inside-work-area and auto-flip toggles,
  Apply/Revert/Test actions; Cards & order tab with ordered 8-card list, enable/pin/default/up-down/
  drag, live pagination-dot preview, reset-to-V9; Behavior tab with island enabled, return-to-search,
  reduced-motion, primary-launcher mode. UI-only simulation - native placement/persistence is unbuilt.
- V9 audit also lists integration requirements: replace sample data with Rust/Tauri APIs, Windows
  media-session/device/clipboard services, optional agent backend, real global hotkey, and never use
  browser-only interaction tests as proof of native behavior.
- Also supplied: original_p_logo_reference.png (7,125 B), settings_placement_preview.png (90,803 B),
  settings_cards_1240.png (93,101 B). The V9 HTML is the visual authority; earlier references cannot
  override it.
- Design extraction (task-28, L1-B) is in progress; this audit only inventories the files and the
  verified V9 behaviors. No pixel/motion fidelity claim is made at Gate 0.

## 7. Baseline benchmark status (link: docs/performance/BENCHMARKS.md)

- Filename engine: MET targets on committed a28042d - 100k p95 1.047 ms, 1M p95 7.228 ms (quiet);
  repeat ranges and a contended 1M outlier (28.4 ms) are published, not hidden.
- Content retrieval: v2 ps-content p95 43.0 ms at HEAD 8692997 (target 30 ms NOT MET); legacy v1 FTS5
  baseline 6.07 ms p95 (baseline-for-migration, not a v2 number). Top performance gap.
- OCR: Windows.Media.Ocr extraction p50 3.6 ms/p95 7.3 ms on the available corpus; CJK accuracy fails
  because no CJK OCR language pack is installed (language-pack limitation, not proven engine limit).
  OCR FTS search is not integrated/measured yet.
- UI/launcher latency: only an exploratory n=4 debug-build input->visible sample (3.9-7.8 ms) exists;
  hotkey->visible, release-build p95, focus timing, FPS/frame pacing and idle CPU are UNMEASURED.
- v10 frame-pacing and both-launcher targets: unmeasured (plan in docs/performance/BENCHMARK_PLAN.md).
- Competitor comparisons: Kensaku/Everything observations are read-only and scope-mismatched; no
  cross-tool latency table is published (BENCHMARKS section 9). Never use star count/repo size as
  performance evidence.

## 8. Honest blockers and gaps

1. Single physical monitor only: dual-monitor, mixed-DPI, disconnect/reconnect placement tests and
   some cross-feature pairs are BLOCKED; must be marked gaps, not simulated.
2. No code-signing certificate: v10 installer/update cannot be signed/verified on this host unless the
   user supplies credentials. Unsigned artifacts must be labeled as such; S-02 signed-update gate stays
   open.
3. No non-Pi agent/provider credentials: live model/agent runs are UNVERIFIED; tests use the offline
   fake-Pi harness. Provider auth/store behavior beyond docs is unverified.
4. No CJK OCR language pack: CJK OCR accuracy cannot be claimed; must be retested on a machine with
   the pack or reported as a gap.
5. No island implementation yet: all island claims (V9 fidelity, dots, cards, motion) are NOT STARTED;
   browser/HTML simulation is not native evidence.
6. No clean VM / second machine: installer upgrade/uninstall/fault-injection and cross-hardware
   performance claims are limited to this host.
7. No automated accessibility harness (Narrator/High Contrast/screen readers) configured yet.
8. Antivirus/Defender, AC/DC battery state, high-refresh 60 Hz comparison and multi-language UI were
   not measured at Gate 0.
9. P0 reproductions are in progress (tasks 29/30) - no confirmed root cause or reproduction evidence
   is claimed in this audit yet.
10. Previous program debt carried forward: insecure legacy updater (S-02), leaked legacy API key (S-01),
    Pi write-consent server-side gate (RR-02), content p95 miss, unsigned installer, single-monitor
    scope. These are release gates, not forgotten items.

## 9. Gate-0 exit checklist

| Criterion | Status |
|---|---|
| Handoff package read and inventoried | DONE (files + sizes + V9 behavior inventory) |
| Environment recorded (OS/build/display/DPI/refresh/WebView2/hardware) | DONE (single-monitor gap explicit) |
| Repository/legal/licence/tag rollback state | DONE (tag pre-v10-handoff, MIT, clean tree) |
| Feature inventory summary | DONE (current v2 statuses + explicit v10 gaps) |
| Data locations + backup plan | DONE - L0 backup at %USERPROFILE%\ProtonSearch-Backups\pre-v10-20261110-0205 verified (bundle complete history, HEAD + legacy DB hash match); refresh before any migration/schema change |
| P0-1/P0-2 reproduction plans | DONE; reproductions IN PROGRESS (tasks 29/30), no fabricated results |
| V9 reference inventory | DONE (files + verified behaviors); pixel/motion extraction task-28 in progress |
| Baseline benchmark status | DONE (existing evidence indexed; v10 new metrics planned) |
| Program docs/matrices | DELIVERED by L1-E: WORK_BREAKDOWN, FEATURE_TEST_MATRIX, CROSS_FEATURE_PAIRWISE_MATRIX, BUG_LEDGER, RELEASE_GATES, BENCHMARK_PLAN, FEATURE_PARITY_MATRIX v10 rows, this audit |
| Honest blocker list | DONE (section 8) |

**Gate-0 status: the G0-E documentation/audit package was accepted by L0 on 2026-11-10. Full Gate 0
remains IN PROGRESS until the P0 reproductions (G0-A/G0-D), V9 extraction (G0-B) and benchmark
plan (G0-C) evidence lands and L0 accepts the consolidated Gate 0.**

## 10. Evidence sources

- docs/reference/v10-handoff/* (master prompt, V9 HTML, settings concept HTML, V9 audit, README,
  screenshots/logo).
- docs/implementation/{WORK_BREAKDOWN,FEATURE_TEST_MATRIX,CROSS_FEATURE_PAIRWISE_MATRIX,BUG_LEDGER,
  RELEASE_GATES}.md; docs/performance/BENCHMARK_PLAN.md; docs/performance/BENCHMARKS.md + results/.
- docs/audit/GATE0_BASELINE.md (legacy DB row counts/SHA-256/WAL facts); docs/implementation/
  ACCEPTANCE_REPORT.md; docs/implementation/GATE_LOG.md; docs/security/**; docs/release/**.
- Commands used: git status/log/tag/remote; Get-CimInstance Win32_* / WmiMonitorID; WinForms Screen;
  EdgeUpdate registry for WebView2; file listing/hashes (read-only); prior benchmark/test artifacts;
  Gate-0 backup verification: Get-ChildItem/Get-FileHash on the backup tree and git bundle verify
  (exit 0, complete history).
