# ProtonSearch v2 — Work Breakdown, Ownership & Progress Ledger
**Project Phoenix. L0 = Team Lead (this file's owner). Started 2026-11-09. Baseline tag: `v1.1.1-v2-baseline` (branch `v2-phoenix`).**

This is the plan of record. It is updated at every gate. Status legend: `TODO` / `WIP` / `REVIEW` / `DONE` / `BLOCKED` / `CUT` (with reason).

## 0. Operating rules

- Every task follows **Investigation → Specification → Implementation → Tests → Independent review → Integration → Acceptance**. A task is only `DONE` when evidence (command output, test report, screenshot, benchmark JSON) is attached here and the L0 has re-run the verification.
- One writer per path. Write scopes are listed below; agents must not edit outside their scope without L0 approval. Shared contracts (`crates/ps-ipc`, `apps/desktop/src/bridge`, `apps/desktop/src/styles/tokens.css`) are owned by L0 and change only via L0.
- Legacy app (`protonsearch/`) is **read-only reference** after baseline: no edits for v2 work. Port by re-implementation, not by copy-paste of coupled UI code.
- Commit messages are conventional (`feat:`, `fix:`, `test:`, `docs:`, `perf:`, `chore:`). Local commits only; **no pushes** without user authorization.
- No fabricated evidence. Numbers in `docs/performance/` must come from commands run on this machine; environment recorded.
- Never run a GUI app as an unbounded background daemon: launch, verify, kill. Kill `ProtonSearch.exe` before rebuilding.

## 1. Team structure (Agent Teams)

Reality note: only the **Team Lead can spawn teammates** in this runtime (teammates cannot spawn more teammates). The required 4-lead × 4-specialist hierarchy is therefore staffed as **L1 leads plus L2 specialists under L0**, with L3 work absorbed as narrowly-scoped tasks inside the owning teammate. Every role below has a named owner; the missing deeper layers are handled sequentially, not faked.

| Role | Owner | Scope |
|---|---|---|
| L0 Program Director | `lead` | Architecture coherence, shared contracts, integration, final verification, docs of record. |
| L1-A Chief Architect / Integration | `engine-core` (also L2-A1/C1) | Rust workspace health, ps-core coordinator/ranker, ps-index filename engine, IPC semantics. |
| L1-B Product Design & Experience | `design-spec` → `ui-launcher` + `ui-app` + `ux-qa` | Design system, launcher, desktop app, onboarding, visual QA. |
| L1-C Performance & Search Quality | `perf-bench` | Benchmark harness, datasets, golden relevance fixtures, perf report, veto on claims. |
| L1-D Reliability, Security & Release | `rel-recon` → `release` | Gate-0 audit, threat model, migration, CI, installer, release QA, docs. |
| L2-A2 Windows native & filesystem | `sources-content` | Win32 helpers, apps/clipboard/browser/recent/git sources, calculator/actions. |
| L2-C2/C3 Content, document & OCR | `sources-content` | Extractors, FTS5 content index, OCR pipeline (Windows.Media.Ocr), semantic (optional). |
| L2-A4 Agent sidecar | `agent-sidecar` | On-demand Python sidecar, versioned JSONL protocol, approvals, UI workflow integration. |
| L2-B1 Launcher UI | `ui-launcher` | `apps/desktop/src/launcher/**`. |
| L2-B2/B3 Desktop app + onboarding | `ui-app` | `apps/desktop/src/app/**`, `src/onboarding/**`. |
| L2-B4 Interaction / a11y / visual QA | `ux-qa` | `docs/qa/visual/**`, screenshot evidence, a11y audit (read-only on UI). |
| L2-D1/D3/D4 Tests, installer, CI, release | `rel-recon` / `release` | `tests/**`, `.github/**`, `packaging/**`, release documentation. |

## 2. Target architecture (frozen at Gate 1)

```text
repo root (Cargo workspace, excludes legacy protonsearch/)
├── crates/
│   ├── ps-ipc       # versioned DTOs + event schemas (SHARED CONTRACT, owner: L0)
│   ├── ps-core      # query parsing, coordinator, ranking, cancellation, Engine facade
│   ├── ps-index     # in-memory compact filename index, scan/persist/incremental update
│   ├── ps-content   # SQLite FTS5 content index + text extractors + open-at-match offsets
│   ├── ps-sources   # apps, recent, clipboard, browser history/bookmarks, git, quicklinks, calculator
│   ├── ps-windows   # Win32 helpers: icons, shell open, foreground window, monitors, DPI
│   ├── ps-ocr       # bounded OCR pipeline + OCR FTS (Windows.Media.Ocr first)
│   ├── ps-agent     # optional sidecar lifecycle + JSONL protocol client (gate 6)
│   └── ps-bench     # benchmark CLI (dev-only, not shipped)
├── apps/desktop/    # Tauri 2 shell (src-tauri/, owner: L0) + React/TS UI (owner: UI team)
├── agent-sidecar/   # optional Python runtime (owner: agent-sidecar)
├── docs/            # architecture, ADRs, audit, design, performance, security, release, qa
├── tests/           # cross-crate golden fixtures
└── protonsearch/    # LEGACY v1 app — read-only reference + baseline
```

**Data location:** v2 owns `%APPDATA%\protonsearch\v2\` (index.db, settings.json, logs, icon-cache). Legacy `%APPDATA%\protonsearch\file_index.db` is opened read-only for one-time migration of settings/clipboard/snippets/quicklinks/browser history and optionally file paths; it is never mutated.

**Query path:** UI → Tauri command `search` (async, on blocking pool) → `ps_core::Engine` → Tier 0 exact/prefix/app/recent/compact filename scan (parallel, bounded) → Tier 1 fuzzy + FTS content → Tier 2 OCR/semantic/preview. Stale request IDs are dropped; ranking merge is deterministic; no Tier 2 work blocks Tier 0 results.

## 3. Gates, deliverables and acceptance evidence

| Gate | Deliverable | Acceptance evidence | Owner | Status |
|---|---|---|---|---|
| G0 | Legacy repo audit, build, feature inventory, baseline metrics, risk register, research notes | `docs/audit/GATE0_BASELINE.md`, `docs/implementation/FEATURE_PARITY_MATRIX.md`, legacy `cargo test` report | rel-recon | WIP |
| G1 | Architecture, ADRs, IPC v1 contract, workspace that compiles, owner map, security model, test strategy | `cargo check --workspace` clean; ADR-0001..0005; `crates/ps-ipc` frozen | L0 + engine-core | WIP |
| G2 | Filename index + content FTS + sources with correctness tests and measured baselines | `cargo test -p ps-index -p ps-content -p ps-core`; bench JSON 10k/100k/1M | engine-core + sources-content + perf-bench | TODO |
| G3 | Frameless launcher wired to real search results, global hotkey, keyboard model | Real app screenshot + input-to-visible p50/p95 + E2E test | ui-launcher + L0 | TODO |
| G4 | Desktop app (Home/Search&Indexes/Activity/Workflows/Preferences), onboarding, settings, migration, dark mode | Screenshots light/dark/narrow/DPI; onboarding E2E; migration test fixture | ui-app + rel-recon | TODO |
| G5 | Previews/open-at-match, OCR search, clipboard/browser/git actions, quicklinks/snippets, calculator, semantic opt-in | feature tests + benchmark tables | sources-content + ui teams | TODO |
| G6 | Optional Python agent sidecar: framework ADR, on-demand launch, streaming, approvals, no idle cost | protocol tests + sidecar E2E + idle process check | agent-sidecar | TODO |
| G7 | Installer, updater metadata, full checklist, visual audit, docs, RC | `tauri build` artifacts; signed-off checklist; truth report | release + ux-qa + L0 | TODO |

## 4. Shared contracts (frozen after G1)

1. `crates/ps-ipc/src/lib.rs` — `PROTOCOL_VERSION = 1`; `SearchRequest`, `SearchResponse`, `SearchResult`, `SourceKind`, `ActionSpec`, `EngineStatus`, `Settings`, `OnboardingState`, `ActivityItem`, `AgentEvent`. Changes require version bump + L0 review.
2. `apps/desktop/src/bridge/types.ts` mirrors the Rust DTOs; `bridge/api.ts` is the only module allowed to call Tauri `invoke` (or the browser mock).
3. `ps_core::SearchSource` trait: `fn search(&self, q: &Query, limit: usize) -> Vec<ScoredHit>`; sources register with the engine at construction. Tier assignment is done by the coordinator, not by sources.
4. Tauri command surface: `search`, `open_result`, `get_status`, `get_settings`, `update_settings`, `get_activity`, `get_sources`, `index_command` (pause/resume/rebuild), `onboarding` (get/complete), `agent_*` (start/cancel/approve — gate 6), `window_* (show/hide/resize launcher)`.

## 5. Dependency graph

```text
L0 scaffold ──┬─> engine-core (ps-index, ps-core) ──┐
              ├─> sources-content (ps-content, ps-sources, ps-windows, ps-ocr) ─┤
              ├─> ui-launcher / ui-app (React views) ──> L0 Tauri shell wiring ──┤
              ├─> perf-bench (dataset gen + harness) <───────────────────────────┤
              ├─> rel-recon (audit/security/CI docs) ──> release (installer/docs) ─┤
              └─> design-spec (tokens/screens) ──> ui teams ──> ux-qa (screenshots/a11y)
```
Blocking edges: `ps-ipc` → everything; `ps-core` facade → Tauri shell; UI shell → visual QA; installers → `tauri build`.

## 6. Progress ledger

| Date | Gate | Item | Owner | Status | Evidence |
|---|---|---|---|---|---|
| 2026-11-09 | G0 | Baseline branch/tag; legacy working tree committed | L0 | DONE | `git log`, tag `v1.1.1-v2-baseline` |
| 2026-11-09 | G0 | Legacy DB forensic dump (tables/row counts; 89,813 files, 14,609 content docs, 4,523 browser items) | L0 | DONE | `.agents/dump_schema.py` output in GATE0_BASELINE |
| 2026-11-09 | G1 | Workspace + IPC contract + compiling stubs | L0 | WIP | `cargo check --workspace` |

## 7. Known constraints / risks
- Tauri 2 first build is heavy; validate toolchain in Gate 1 before UI work depends on it.
- Legacy `main.rs` (542 KB) / `search.rs` (428 KB) must be treated as evidence, not as a template to port wholesale.
- Real OCR/Windows APIs need this Windows host; Linux-only validation is not acceptable evidence.
- The machine has a real 149 MB legacy index and 89,813 files — use it for migration and realistic benchmarks.
- Missing artifacts: `assets/protonsearch-mark.svg`, `assets/protonsearch-logo-concept.png`, `assets/original-logo-crop-reference.png` are **not present** in the workspace. The mark embedded in the supplied HTML is the visual source of truth; a reconstructed vector is stored in `assets/brand/` and documented as reconstructed.
## Status ledger additions (2026-11-09, evening)

| Gate | Item | Owner | Status | Evidence |
|---|---|---|---|---|
| G2 | ps-index compact engine: scan/arena/matching/persist/incremental + tests | engine-core | WIP (finishing perf pass) | crates/ps-index src+tests; bench JSONs |
| G2 | Engine facade: tiered coordinator, settings/activity/onboarding/migration/background | L0 | DONE (first integration) | crates/ps-core engine.rs/db.rs/catalog.rs/migration.rs; real app run |
| G2 | Content FTS + extractors + legacy import | sources-content | DONE (tests 16) | crates/ps-content; real migration run |
| G2 | Win32 helpers: icons/shell/clipboard/AppsFolder | sources-content | DONE (tests 7) | crates/ps-windows |
| G2/G5 | ps-sources (actions/apps/recent/browser/git/clipboard) | sources-content | IN PROGRESS (compile) | crates/ps-sources |
| G5 | OCR pipeline (Windows.Media.Ocr) | sources-content | TODO (next) | crates/ps-ocr stub |
| G3 | Launcher UI + review fixes + real-window captures | ui-launcher | DONE (visual QA pending) | apps/desktop/src/launcher; docs/qa captures |
| G3 | Real-app E2E + latency harness (CDP) | L0 | DONE | tools/visual-qa/e2e-drive.mjs; latency JSON |
| G4 | Desktop app + onboarding + review fixes | ui-app | DONE (visual QA pending) | apps/desktop/src/app,onboarding |
| G4 | Real-data legacy migration (read-only source) | L0 | VERIFIED with caveat | gate log; tests/migration; ps-core migration tests |
| G6 | Pydantic AI sidecar + Rust client + docs + tests | agent-sidecar | DONE | crates/ps-agent (14 tests), agent-sidecar (18 tests) |
| G6 | Tauri shell wiring for agent commands/events | L0 | CODE DONE (UI panel pending) | src-tauri main.rs |
| G7 | Benchmarks final report | perf-bench | IN PROGRESS | docs/performance/results |
| G7 | Visual QA + screenshots audit | ui-launcher/ui-app (acting as L2-B4) | IN PROGRESS | docs/qa/visual |
| G7 | Installer/updater/CI/release docs | rel-recon | READY; installer run pending green build | .github, scripts, docs/release |
