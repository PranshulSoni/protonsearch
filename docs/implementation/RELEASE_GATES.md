# ProtonSearch v2 — Release Gates (v10)

Owner: L1-E (rel-recon); acceptance authority: L0. Applies to the v10 program (gates G0-G9 from the
master prompt section 14). A gate opens only with the evidence listed here; a gate closes only when an
independent reviewer re-runs the evidence and L0 accepts it. Nothing ships unsigned or unverified
without being stated plainly.

## 1. Gate definitions and exit criteria

| Gate | Opens when | Exit criteria (evidence) | Status (2026-11-10 re-review) |
|---|---|---|---|
| G0 Audit/baseline | handoff package read; tree clean at `pre-v10-handoff` | V10_GATE0_AUDIT with environment/repo/legal/data/backup; FEATURE_PARITY_MATRIX/FEATURE_TEST_MATRIX/CROSS_FEATURE_PAIRWISE/BUG_LEDGER/RELEASE_GATES/BENCHMARK_PLAN present; P0-1/P0-2 reproductions recorded or explicitly in progress; V9 reference inventory; baseline benchmark status | **G0-A/B/D COMPLETE + G0-E accepted by L0 (2026-11-10)**; G0-C perf report pending separately; consolidated Gate-0 closure = L0 acceptance |
| G1 Architecture/contracts | G0 accepted | ADRs for one core/two launchers/optional agents, preview payload, island settings schema, device capability model; typed IPC version/migration plan; owner/reviewer map; test strategy | **CLOSED (L1-E recommendation, 2026-11-10)** - R1 DTO move PASS, R2 preview backend/command/bridge PASS, R3 clipboard images PASS (module + real .NET), ADR-0007..0012 + additive policy, shell fixes verified; preview UI/app GUI clipboard E2E moved to G2 acceptance |
| G2 P0 repair | G0 reproductions complete | BUG-P0-01 fixed: real previews for required formats with matched-term highlight, fallbacks, tests/screenshots, both launchers once island exists; BUG-P0-02 fixed: real bitmap capture/classification/persistence/re-paste, real Windows copy/paste tests; independent review | **CONDITIONAL** - P0-2 clipboard PASS (tests + independent real .NET capture; app GUI matrix/OLE/OCR-text follow-ups); P0-1 preview backend/command/bridge PASS, UI task-38 in progress; G2 not closed until preview UI + both-launcher E2E |
| G3 Search/retrieval | G1 accepted | filename + content + OCR + sources wired; before/after benchmarks; relevance/regression gates; no fake counts; content p95 target addressed or honest gap accepted | PARTIAL (v2 engine; content p95 43 ms vs 30 ms target - task-42; OCR FTS integration pending) |
| G4 Normal Launcher | G3 search path usable | centered multi-monitor/DPI window, All/Files/Folders/Apps filters, quick destinations, previews/actions, keyboard/a11y, E2E evidence | **CONDITIONAL** - 4-chip/quick-destinations model matches Lead addendum C1/C2; design-spec micro-spec sync (SCREENS 2.4/2.11) + native Windows DPI screenshots pending |
| G5 Dynamic Island | G1 contracts + V9 extraction | all 8 cards, V9 geometry/motion fidelity, clickable dots, scroll/swipe vs inner scroll, real data, no clipping, 100-200% DPI screenshots/video, frame-pacing measurements | **PARTIAL / NOT SIGNED** - renderer geometry/motion/dots match spec in mock; clipboard thumbnails, agent hierarchy, pinning, task-35 adapters, native DPI/frame pacing, expanded 860/430 captures pending |
| G6 Desktop settings/onboarding | G4+G5 surfaces exist | home/greeting/onboarding primary-launcher choice, placement editor (drag/presets/offsets/taskbar/safe clamp/apply-cancel-reset/test/recovery), card manager (order/enable/pin/default/live dots/undo), persistence/migration, screenshots | PARTIAL |
| G7 Agents/native integrations | G1 agent contracts, capability-gated adapters | Pi sidecar on demand, provider config/keys in OS store, permissions/approvals, honest pause/cancel semantics, agent status/history both surfaces; media/output/Wi-Fi/Bluetooth/brightness/volume/notifications with capability probes and honest disabled states | PARTIAL (Pi hardened; write-consent server gate open; agents card/media/controls not started) |
| G8 Cross-feature verification | feature sets exist | FEATURE_TEST_MATRIX executed; CROSS_FEATURE_PAIRWISE pair rows executed (critical pairs on real Windows); security audit; perf/CPU/memory/frame-pacing report; fixes verified; no P0/P1 open without accepted rationale | TODO |
| G9 Release | G8 accepted | installer/updater/migration/rollback/uninstall verified; signed binaries if certificate available (else explicit unsigned statement); docs/changelog/readme; recordings of all 8 island cards + placement editor + both launchers; benchmark reproducibility package; final L0 sign-off | TODO |

## 2. Release-blocking conditions (hard No-Go)

1. P0-1 preview or P0-2 clipboard images open or unverified at G2+.
2. Any security critical/high finding open without an accepted mitigation: legacy API key rotation
   (S-01), signed/verified updater (S-02), agent write-consent server-side gate (RR-02), secret
   scanning clean, no plaintext provider keys.
3. Silent feature removal vs FEATURE_PARITY_MATRIX (never silently drop a legacy source/action).
4. Unverified migration/backup: user data loss risk, no rollback path.
5. Content/OCR/preview claims without measured evidence on real Windows.
6. Installer artifacts without recorded SHA-256 and explicit signing status.
7. Any gate opened on documentation alone while its executable criteria are unmet.

## 3. Evidence package required for a release

- Commands + logs for: build, unit/integration tests, Windows E2E, migration fixtures, benchmark runs.
- FEATURE_TEST_MATRIX and CROSS_FEATURE_PAIRWISE matrices with executed results (not plans).
- BUG_LEDGER with all P0 fixed+verified and P1 dispositions.
- Screenshots/video: Normal Launcher, all 8 island cards and states, placement editor, card manager,
  onboarding, settings, both light/dark, 100/125/150/200% scaling where available.
- Performance report: reproducible benchmark JSON + methodology + environment + competitor policy.
- Security package: threat models, secret scan, dependency audit, signing/verification evidence,
  permission/approval behavior, clipboard/agent privacy.
- Installer: artifact hashes, BUILD_INFO, signing status (signed or explicit unsigned statement),
  install/upgrade/uninstall/rollback test records.
- Honest gaps list (hardware not tested, credentials absent, signing unavailable, features deferred/CUT).

## 4. Signing, updater and data-safety gates (v10)

- **Signing:** no code-signing certificate is available on this host at Gate 0. Release artifacts are
  UNSIGNED and must be labeled as such; signing requires user-provided credentials and a documented
  verification (signtool verify /pa) step. Never claim signed without verification.
- **Updater:** the legacy insecure updater (risk register S-02) must not be reused. v10 needs a signed
  manifest + hash check + Authenticode signer pin (or an explicit no-updater statement for the alpha).
- **Migration/rollback:** pre-update backup of legacy DB (including WAL) + v2 data; migration must be
  read-only against legacy, atomic on v2, versioned, tested with rollback. Never delete user history.
- **Uninstall:** keep user data by default with explicit opt-in deletion; uninstaller must not run the
  legacy uninstaller that deletes `%APPDATA%\protonsearch`.
- **Telemetry:** none without explicit opt-in; default is off; verify no hidden network calls from the
  app/agent runtime beyond user-initiated provider calls (Pi offline/telemetry controls verified).

## 5. Gate review and sign-off

L0 owns final acceptance. Each gate review uses the exec summary in the gate's evidence page plus raw
links; reviewers must re-run at least the representative commands. For every specialty workstream the
independent reviewer is named in WORK_BREAKDOWN/handoffs. A failed gate returns to the owner with a
defect ID in BUG_LEDGER; no gate is closed by majority or by time pressure.


## 6. Reviewer sign-off (task-46 re-review, 2026-11-10)

Reviewer: L1-E (rel-recon). Evidence: docs/implementation/V10_G1_ARCHITECTURE.md section 10,
docs/qa/clipboard-audit/INDEPENDENT_REREVIEW_R3_2026-11-10.md,
docs/qa/visual/SPEC_REVIEW_G2_G4_G5_2026-11-10.md, .agents/notes/g2-rereview-tests.log.

| Area | Verdict | Evidence | Conditions / blockers | Gate |
|---|---|---|---|---|
| R1 agent DTO move to ps-ipc | PASS | canonical agent_task.rs; ps-agent re-exports; shell imports ps_ipc DetectedAgent; byte-identical fixtures; cargo test -p ps-ipc 17/17 | TS mirror remains manual; automate schema check at G8 | G1 |
| R2 preview backend/contract | PASS (backend/contract) | get_preview registered; resultId-only; bounds/cache/status/truncated/error; cargo test -p ps-core 48/48 incl 8 preview tests; bridge getPreview + ResultPreview | task-38 UI final screenshots/E2E; both-launcher parity at G8 | G2 |
| R3 clipboard images | PASS (module + real-Windows harness) | ps-windows 18/18; ps-sources 49 (1 pre-existing ignored); independent .NET SetImage -> Image row, PNG 6,315 B, thumb 3,671 B, paste-back | full app GUI matrix (Snipping/Paint/browser), OLE-only sources, image OCR text | G2 |
| G1 overall | CLOSED (recommend) | R1/R2/R3 + ADR/policy + shell fixes | preview UI and app-level clipboard E2E belong to G2 | G1 |
| G2 P0-1 previews | CONDITIONAL | backend/command/bridge/engine tests PASS | task-38 in progress; fresh native screenshots + both launchers | G2 |
| G2 P0-2 clipboard | CONDITIONAL PASS | independent module-level + real .NET reproduction; BOOT evidence | app-process GUI matrix; OLE fallback; image OCR text | G2 |
| G4 normal launcher v2 | CONDITIONAL | 4 chips + quick destinations + row/footer anatomy match Lead addendum | design-spec micro-spec sync (SCREENS 2.4/2.11, preview 2.6); native DPI screenshots | G4 |
| G5 island renderer | PARTIAL - NOT SIGNED | geometry/motion/dots match spec; mock screenshots; honest disabled cards | thumbnails, pinning, agent hierarchy, task-35 adapters, native DPI/frame pacing, 860/430 captures | G5 |

Remaining blockers to G2/G4/G5 closure are listed in V10_G1_ARCHITECTURE.md section 10.6 (task-38
preview UI; app-process clipboard GUI matrix; island card completeness + task-35 adapters + native
visual/frame-pacing evidence; design-spec micro-spec sync; single-monitor/no-signing-cert/no-non-Pi-
credentials environment gaps; content p95 task-42).


## 7. G8 round 1 verdict and sign-off requirements (task-50, 2026-11-10)

Verdict: **PARTIAL - NOT SIGNED.** Positive: frontend 18 files/237 tests pass + build 0; targeted Rust
ps-ipc 17/17 and ps-agent 100/100; live P0-1 preview payloads/screenshots (TXT/PDF/DOCX/PNG+OCR/
binary/truncation); live P0-2 real .NET clipboard image -> island Images thumbnails -> restart
persistence. Blocking-for-sign-off: full cargo workspace currently fails to compile on task-35
uncommitted ps-windows changes; pairwise matrix only partially executed; native adapter cards
Media/Controls/Notifications honest-disabled; island/launcher native DPI/High Contrast/frame-pacing
evidence missing; paste-back through the island UI not verified; several preview formats not executed;
environment (single monitor, no signing certificate, no non-Pi credentials); official content p95
target not yet reproduced.

Sign-off sequence: (1) rerun on a green commit with cargo test --workspace + frontend suites pass;
(2) execute the G8 sign-off list in docs/implementation/V10_G8_VERIFICATION_ROUND1.md section 7;
(3) independent reviewer (L1-E) re-runs a sample and records it; (4) L0 accepts; no gate is signed on
this round's partial evidence.


## 8. Final L0 recommendation (task-53, RC 4e9f589)

- **Deployment-ready locally: YES - unsigned internal alpha.** RC artifacts are hash-verified and
  match SHA256SUMS.txt; the release app launches and renders; cargo workspace 306 passed/0 failed and
  frontend 241 passed/build 0; P0-1 previews (14 formats incl DOCX/XLSX/PNG+OCR/JSON/CSV/Unicode,
  honest unsupported, stale-file error) and P0-2 clipboard images (live capture, thumbnails, restart
  persistence) are verified; content p95 8.984 ms, activation 14.289 ms and selection 6 ms meet
  targets.
- **Public release signed off: NO.** Blockers: unsigned/no updater, installer upgrade/uninstall/
  rollback drill not run, single-monitor (no mixed-DPI/HC/multi-monitor evidence), idle memory
  786.9 MiB over budget, debug input->visible p95 38.2 ms over 30 ms target, native media/Wi-Fi/
  Bluetooth deferred to post-alpha milestone 2, no live Pi credentials/approval verification, and
  the matched-term preview highlight condition open.
- Next: sign + verify, run installer drill on a clean VM, close highlight/UI parity and memory/input
  budgets, execute remaining pairwise/list items, then L0 public-release acceptance.
