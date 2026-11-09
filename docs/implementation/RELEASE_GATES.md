# ProtonSearch v2 — Release Gates (v10)

Owner: L1-E (rel-recon); acceptance authority: L0. Applies to the v10 program (gates G0-G9 from the
master prompt section 14). A gate opens only with the evidence listed here; a gate closes only when an
independent reviewer re-runs the evidence and L0 accepts it. Nothing ships unsigned or unverified
without being stated plainly.

## 1. Gate definitions and exit criteria

| Gate | Opens when | Exit criteria (evidence) | Status at Gate 0 |
|---|---|---|---|
| G0 Audit/baseline | handoff package read; tree clean at `pre-v10-handoff` | V10_GATE0_AUDIT with environment/repo/legal/data/backup; FEATURE_PARITY_MATRIX/FEATURE_TEST_MATRIX/CROSS_FEATURE_PAIRWISE/BUG_LEDGER/RELEASE_GATES/BENCHMARK_PLAN present; P0-1/P0-2 reproductions recorded or explicitly in progress; V9 reference inventory; baseline benchmark status | **IN PROGRESS** (docs delivered; reproductions tasks 29/30 in progress; V9 extraction 28; perf plan 31) |
| G1 Architecture/contracts | G0 accepted | ADRs for one core/two launchers/optional agents, preview payload, island settings schema, device capability model; typed IPC version/migration plan; owner/reviewer map; test strategy | TODO |
| G2 P0 repair | G0 reproductions complete | BUG-P0-01 fixed: real previews for required formats with matched-term highlight, fallbacks, tests/screenshots, both launchers once island exists; BUG-P0-02 fixed: real bitmap capture/classification/persistence/re-paste, real Windows copy/paste tests; independent review | TODO |
| G3 Search/retrieval | G1 accepted | filename + content + OCR + sources wired; before/after benchmarks; relevance/regression gates; no fake counts; content p95 target addressed or honest gap accepted | PARTIAL (v2 engine; content target missed; OCR FTS pending) |
| G4 Normal Launcher | G3 search path usable | centered multi-monitor/DPI window, All/Files/Folders/Apps filters, quick destinations, previews/actions, keyboard/a11y, E2E evidence | PARTIAL |
| G5 Dynamic Island | G1 contracts + V9 extraction | all 8 cards, V9 geometry/motion fidelity, clickable dots, scroll/swipe vs inner scroll, real data, no clipping, 100-200% DPI screenshots/video, frame-pacing measurements | NOT STARTED |
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
