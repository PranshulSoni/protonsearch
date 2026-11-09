# ProtonSearch v2 — v10 Program Work Breakdown, Ownership & Progress Ledger

**Program phase:** v10 handoff (Project Phoenix, Phase 2). **Started:** 2026-11-10.
**Binding sources:** docs/reference/v10-handoff/PROTONSEARCH_V2_FINAL_MASTER_IMPLEMENTATION_PROMPT.md
(master prompt), protonsearch_dynamic_island_cards_v9_polished.html (V9 island, binding),
protonsearch_island_settings_placement_concept.html (placement/card settings reference),
PROTONSEARCH_ISLAND_V9_UI_AUDIT.md, README_START_HERE.md.
**Baseline:** tag `pre-v10-handoff` (7fe878e), branch `v2-phoenix`, HEAD `0e4c9ec`, clean tree at Gate 0.
**Owner of this file:** L0 (Lead). Maintained by L0; L1-E supplies Gate-0 evidence and matrices.

## 0. Operating rules

1. Evidence over claims: a task is DONE only when the owner and an independent reviewer re-run the
   verification and L0 accepts the evidence (commands, test output, screenshots, benchmark JSON).
   "The agent said done" is not acceptance.
2. One writer per path. Shared contracts (crates/ps-ipc, apps/desktop/src/bridge,
   apps/desktop/src/styles/tokens.css, apps/desktop/src-tauri shell entry points) are L0-owned.
   Code changes in crates/** belong to the named L1/L2 owners; docs/implementation/**,
   docs/security/**, docs/release/**, docs/performance/** are L1-E-owned at Gate 0.
3. Legacy `protonsearch/` stays read-only reference. New work is re-implementation.
4. No fabricated evidence. Any number in docs/performance must come from a command on this machine
   with environment recorded; unavailable environments are listed as gaps, not simulated.
5. No pushes without user authorisation. Local conventional commits only. No GUI app left running as
   an unbounded daemon; launch, verify, kill.
6. Do not mark a gate complete because the plan exists. Gate completion needs the evidence table
   below to be satisfied and accepted by L0.

## 1. Ratified staffing structure (L0 decision, 2026-11-10)

The runtime platform allows up to **8 teammates**; teammates cannot spawn further teammates
(delegation depth for L1/L2 is zero), so **L3 workers cannot be created**. L0 ratified the v10
structure on 2026-11-10 as **L0 + 5 L1 leads + 3 L2-scope specialists = 9 members total**
(8 teammates + L0):

| Level | Seat | Owner | v10 responsibility |
|---|---|---|---|
| L0 | Program Director / Final Integrator | lead | architecture, shared contracts, reviews, integration, installer, final acceptance |
| L1-A | Rust Core / Windows Platform | engine-core | ps-core/ps-index/ps-ipc, query planner, filename index, preview backend interfaces, IPC |
| L1-B | UI / Motion / Product Design | design-spec | V9 fidelity extraction, design system, screenshots, motion/a11y/DPI specs, review veto |
| L1-C | Search Quality / Performance | perf-bench | datasets, latency/frame-pacing harness, relevance/OCR benchmarks, claim veto |
| L1-D | Agent Runtime / Extensibility | agent-sidecar | Pi runtime (already hardened), agent protocol, approvals, history, MCP/tool policy |
| L1-E | Reliability / Security / Integration / Release | rel-recon | Gate-0 audit, matrices, security, migration, CI, installer/update, release evidence |
| L2-A | Sources & native adapters (A2/A3/A4/A5) + content/OCR (C2/C3) | sources-content (reports to L1-A) | filesystem/clipboard text+bitmap/preview extractors/browser/git/OCR/native device adapters |
| L2-B1 | V9 island + Normal Launcher implementation | ui-launcher (reports to L1-B) | island renderer/cards, normal launcher, filters, launcher keyboard/preview integration |
| L2-B2 | Desktop settings/onboarding implementation | ui-app (reports to L1-B) | desktop app, onboarding, placement editor, card manager, settings sections |

**Ratification:** L0 confirmed this roster on 2026-11-10. The platform limit applies to
*teammates* (8), so the roster is **L0 + 8 = 9 members**: 5 L1 leads and 3 L2-scope specialists
(sources-content under L1-A; ui-launcher and ui-app under L1-B). An earlier Gate-0 draft flagged
this as an anomaly; that is corrected here - this is the ratified structure. No L3 workers are
spawnable; L3 tasks are executed directly by the owning L1/L2 as sequential passes.

**Consequence for planning:** work that the master prompt assigns to L3 (narrow temporary workers,
e.g. one CF_DIBV5 clipboard test, one PDF snippet fix, one V9 card audit) is absorbed as scoped
sequential passes inside L1/L2 tasks. The work breakdown below therefore names every deliverable
with a single owner and an independent reviewer rather than pretending a larger org exists.

## 2. Gates (master prompt section 14)

| Gate | Deliverable | Required evidence | Owner(s) | Status (Gate 0 snapshot) |
|---|---|---|---|---|
| G0 | Audit and baseline | forensic audit, P0 reproductions, feature/parity/status matrices, benchmark baseline, V9 reference inventory, data backup + rollback tag | L0 + L1-E (+G0-A/B/C/D tasks) | **IN PROGRESS** — this file/matrices/audit delivered; P0 reproductions in progress (task-29, task-30); V9 extraction in progress (task-28); benchmark plan in progress (task-31) |
| G1 | Architecture/contracts/hierarchy | ADRs, Rust crate boundaries, typed IPC, settings schema + migration, ownership/reviewer map, test strategy | L0, L1-A, L1-E | TODO (inherits v2 frozen IPC v1 and ADR set; v10 additions: island placement/card settings schema, preview payloads, device adapter capabilities) |
| G2 | Repair P0 bugs early | real preview pipeline for required formats; real clipboard bitmap capture/persistence/re-paste; Windows E2E evidence (Notepad/VS Code/Paint/Snipping Tool, PDF, DOCX) | L1-A (preview), L2-A (clipboard), reviewers L1-E + L0 | TODO — blocked on G0 reproductions |
| G3 | Rust search & retrieval | filename + content + OCR + sources wired; before/after benchmarks; relevance/regression fixtures; preview extraction integrated | L1-A, L1-C, L2-A | PARTIAL (v2 engine exists; content p95 misses 30 ms target; OCR FTS integration pending — see BENCHMARKS) |
| G4 | Normal Launcher production integration | centered multi-monitor window, All/Files/Folders/Apps filters, quick destinations, previews, keyboard/actions, real screenshots + E2E | L1-B + L2-B, review L1-E/L0 | PARTIAL (v2 launcher exists without v10 filter/quick-destination/parity acceptance) |
| G5 | V9 Dynamic Island implementation | all eight cards, V9 motion/geometry fidelity, dots, scroll/swipe, native data, no clipping, screenshot/motion audit | L1-B + L2-B, review L0 | **NOT STARTED** (no island code in tree at Gate 0) |
| G6 | Full desktop settings + onboarding | home, settings, permissions, graphical placement editor, card manager, persisted per-monitor placement, onboarding primary-launcher choice, migration | L1-B + L2-B + L0 | PARTIAL (v2 settings/onboarding exist; placement editor + card manager not started) |
| G7 | Optional agents & advanced integrations | Pi sidecar wired, provider config, permissions/approvals, agents/subagents, honest controls, logs/history; capability-gated native devices | L1-D, L1-A, L2-A | PARTIAL (Pi backend hardened + shell wired; write-consent server-side gate open; agent UI/Workflows review pending) |
| G8 | Cross-feature verification | full FEATURE_TEST_MATRIX + CROSS_FEATURE_PAIRWISE_MATRIX executed on real Windows; security audit; performance/CPU/frame-pacing report; fixes | L1-E (matrix owner), all owners; perf L1-C | TODO |
| G9 | Release | verified installer/updater/migration/rollback/uninstall, docs/changelog, all 8 island cards recorded, benchmark reproducibility package, signed binaries if a certificate exists (otherwise explicit unsigned statement) | L0 + L1-E | TODO |

**Gate rule:** a gate is complete only when the evidence column is satisfied, an independent
reviewer has re-run it and L0 accepts it. Moving to the next gate is allowed only when safe.

## 3. Gate 0 workstreams (current tasks)

| Task | Owner | Deliverable | Status | Output path |
|---|---|---|---|---|
| task-27 G0-E | `rel-recon` | audit + program docs + matrices + benchmark plan | IN PROGRESS (this delivery) | docs/implementation/*, docs/performance/BENCHMARK_PLAN.md, docs/security/*, docs/release/* |
| task-29 G0-A | `engine-core` | reproduce P0-1 preview failure + audit preview pipeline end-to-end | IN PROGRESS | docs/implementation/BUG_LEDGER.md evidence, docs/audit/v10/** |
| task-30 G0-D | `sources-content` | reproduce P0-2 clipboard image failure (CF_DIB/CF_DIBV5 etc.) | IN PROGRESS | BUG_LEDGER evidence |
| task-28 G0-B | `design-spec` | V9 Dynamic Island + placement/settings reference extraction | IN PROGRESS | docs/design/v10/** |
| task-31 G0-C | `perf-bench` | baseline performance + V9 frame-pacing harness plan | IN PROGRESS | docs/performance/BENCHMARK_PLAN.md input, docs/performance/** |

Dependencies: G0-A and G0-D must complete before G2 starts; G0-B extraction feeds G5/G6; G0-C
feeds G8/G9 thresholds. G0-E is the documentation spine and is not blocked by code.

## 4. Shared contracts and write scopes (v10 additions)

1. Frozen IPC v1 stays the base. v10 additions must be versioned: preview payloads (typed
   preview/error/unsupported states), island placement/card settings, device capability snapshots.
2. Rust settings schema is the source of truth; TS types generated or schema-checked. Migration
   tests required for every field addition.
3. New UI components live under apps/desktop/src/ (launcher/island/settings), owned by L1-B/L2-B
   with L0 review of shared tokens.
4. Gate-0 docs/matrices are L1-E-owned: docs/implementation/**, docs/security/**, docs/release/**,
   docs/performance/**. L1-E does not edit code in this task.
5. Preview and clipboard payload contracts require cross-team review (L1-A + L1-E + UI owner)
   before implementation, because both P0 bugs cross Rust -> IPC -> React boundaries.

## 5. Verification protocol and required handoff format

Every work item carries: task ID, owner, independent reviewer, dependencies, file scope, expected
outputs, interfaces, acceptance tests, risk, rollback and a reproducible verification command.
Handoff must state: existing behavior verified with evidence; files/schemas/IPIs changed; tests run
with results + environment; screenshots/video for UI; latency/resource measurements for perf;
security/permission/migration impact; known limitations/blockers; reviewer/integration verdict and
rollback. L0 re-runs verification before accepting.

## 6. Risks and constraints

- **Single-monitor host:** no second physical display and no mixed-DPI/disconnect test environment;
  pans/DPI claims must be marked gaps (see V10_GATE0_AUDIT).
- **No signing certificate:** release binaries stay unsigned; signing is a documented gap, not a
  claim.
- **No non-Pi agent/provider credentials on the host:** live model runs are UNVERIFIED; agent tests
  use offline fake-Pi fixtures.
- **P0 bugs are user-reported, currently being reproduced:** no fabricated reproduction result is
  recorded until G0-A/G0-D evidence lands.
- **V9 island is not implemented yet:** all island claims are design/plan only at Gate 0.
- **Content retrieval p95 misses the 30 ms target** (v2 measured 43 ms at HEAD 8692997 vs legacy
  6 ms baseline) — a G3 blocker for the performance story.
- **WebView2/tauri animation fidelity** is hardware-dependent; must be measured, not asserted.

## 7. Progress ledger (append-only)

| Date | Gate | Item | Owner | Status | Evidence |
|---|---|---|---|---|---|
| 2026-11-10 | G0 | Handoff package imported; baseline tag `pre-v10-handoff` | L0 | DONE | git log 0e4c9ec; tag 7fe878e |
| 2026-11-10 | G0 | Audit env/repo/data inventory; backup tag recorded | L1-E | DONE | docs/implementation/V10_GATE0_AUDIT.md |
| 2026-11-10 | G0 | Work breakdown v10 (this file), feature test matrix, pairwise matrix, bug ledger, release gates, benchmark plan | L1-E | DELIVERED (awaiting L0 review) | this file + linked docs |
| 2026-11-10 | G0 | P0-1 preview reproduction + pipeline audit | L1-A | WIP | task-29 |
| 2026-11-10 | G0 | P0-2 clipboard image reproduction | L2-A | WIP | task-30 |
| 2026-11-10 | G0 | V9 + placement reference extraction | L1-B | WIP | task-28 |
| 2026-11-10 | G0 | Baseline perf + frame-pacing harness plan | L1-C | WIP | task-31 |
| 2026-11-10 | G0 | L0 ratified staffing: L0 + 5 L1 + 3 L2-scope = 9 members (8-teammate platform cap); no L3 workers | L0 | DONE | this section; L0 message 2026-11-10 |
| 2026-11-10 | G0 | Gate-0 backup created (repo bundle + HEAD + AppData copy + legacy DB hash) | L0 | DONE | V10_GATE0_AUDIT section 4b; %USERPROFILE%\ProtonSearch-Backups\pre-v10-20261110-0205 |
| 2026-11-10 | G0 | G0-E audit/program docs accepted by L0 | L0 | DONE | L0 message 2026-11-10 |

## 8. Prior program (superseded, evidence retained)

The v2 build (G0-G7, tag `v1.1.1-v2-baseline`, branch `v2-phoenix`) produced the current working
tree: Rust workspace (ps-ipc/ps-core/ps-index/ps-content/ps-sources/ps-windows/ps-ocr/ps-agent/
ps-bench), Tauri 2 shell, React launcher/desktop app, optional Pi agent runtime, benchmarks,
security reviews and release docs. Evidence of record: docs/audit/GATE0_* (legacy),
docs/implementation/ACCEPTANCE_REPORT.md, docs/implementation/GATE_LOG.md,
docs/implementation/FEATURE_PARITY_MATRIX.md (v2 rows), docs/performance/BENCHMARKS.md,
docs/security/SECURITY_RISK_REGISTER.md, docs/security/V2_SECURITY_REVIEW.md,
docs/security/PI_AGENT_SECURITY.md, docs/release/*. The v10 program supersedes the delivery plan
(gates and features), not the existing working code.
