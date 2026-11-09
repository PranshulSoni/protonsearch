# Release Checklist — ProtonSearch v2

Status: **no release is authorised.** This checklist exists so the first release can be
judged against evidence, not optimism. Local commits only; **no pushes, no publishes**
without explicit user authorisation.

## 0. Gate status (2026-11-09)

| Gate | Deliverable | Status | Evidence |
|---|---|---|---|
| G0 | Legacy audit, baseline, risk register, research, parity matrix | **DONE (audit) / runtime UNVERIFIED** | docs/audit/GATE0_*, docs/research/RESEARCH.md, docs/security/SECURITY_RISK_REGISTER.md |
| G1 | Workspace + IPC v1 + architecture + compile | **WIP** | cargo fmt --check red; cargo check intermittent while agents edit |
| G2 | Index + content FTS + sources, tests + benchmarks | NOT STARTED | — |
| G3 | Launcher wired + hotkey + perf | NOT STARTED | — |
| G4 | Desktop app + onboarding + settings + migration | **WIP** (migration API landed; 2/2 unit tests; contract probe wired; consent/rollback pending) | tests/migration/, docs/release/MIGRATION.md, docs/security/V2_SECURITY_REVIEW.md |
| G5 | Previews/OCR/clipboard/browser/git/actions | NOT STARTED | — |
| G6 | Agent sidecar opt-in | NOT STARTED | — |
| G7 | Installer, updater, visual audit, RC | NOT STARTED | — |

## 1. Preconditions (all must be true before building a release)

- [ ] G1-G6 acceptance evidence attached to docs/implementation/WORK_BREAKDOWN.md and re-run by L0.
- [ ] git status clean; Cargo.lock + apps/desktop/package-lock.json committed.
- [ ] CI green on the release commit (fmt, clippy -D warnings, workspace tests, frontend build).
- [ ] Security gate: legacy API key rotated (S-01); secret scan clean; no embedded/shared keys.
- [ ] Migration contract test passes (fixture + opt-in real-DB READ-ONLY dry run recorded).
- [ ] Migration review must-fixes V2-01/V2-02/V2-03 addressed before G4 acceptance.
- [ ] Known limitations reviewed and updated.
- [ ] User/L0 authorises the build and (separately) any publication.

## 2. Build and artifact verification

- [ ] Run powershell -NoProfile -File scripts/build-installer.ps1 -Execute **only after** L0
      confirms the integration build is green (dry-run first, review output).
- [ ] Record BUILD_INFO.txt (git rev, rustc/cargo/node/npm, UTC time).
- [ ] Verify SHA256SUMS.txt matches copied artifacts (Get-FileHash).
- [ ] Sign installer + exe with Authenticode; verify signtool verify /pa.
- [ ] Generate/verify signed update manifest (Ed25519/minisign) — **not implemented yet**.
- [ ] Confirm installer is per-user (installMode=currentUser), no elevation required.

## 3. Functional smoke (clean Windows VM/user)

- [ ] Install, first-run onboarding, hotkey summon, app/file/content search, open result.
- [ ] Clipboard capture is OFF by default; enabling it requires explicit consent.
- [ ] Settings persist; dark/light themes; DPI/multi-monitor; tray behaviour.
- [ ] AI sidecar: opt-in only; tool approvals shown; no idle process when disabled.
- [ ] Uninstall asks separately about user data; default keeps data; no legacy uninstaller call.

## 4. Migration verification

- [ ] Fixture contract tests pass (tests/migration).
- [ ] Real 149 MB legacy DB dry-run (read-only) on a copy; counts match Gate-0 table.
- [ ] Source DB hash unchanged after import; rollback snapshot exists and restores.
- [ ] No AI keys imported without consent; no logs/clipboard migrated silently.

## 5. Security and privacy

- [ ] cargo audit / cargo deny clean or documented exceptions.
- [ ] Secret scan clean; no secrets in artifacts/logs.
- [ ] ps-windows unsafe review complete; no new unsafe outside it.
- [ ] Threat model updated for shipped feature surface.
- [ ] Third-party licence notices generated from actual artifacts.

## 6. Docs and release notes

- [ ] CHANGELOG updated with evidence links; version matches binary/installer/manifest.
- [ ] Known limitations current (no stale "fixed" claims).
- [ ] README claims match verified behaviour (Gate-0 parity matrix is the source of truth).
- [ ] Rollback instructions published.

## 7. Evidence to attach

- [ ] CI run URL (or local command transcripts).
- [ ] SHA-256 + signature verification logs.
- [ ] Smoke screenshots (light/dark/narrow/DPI) — G7 visual audit.
- [ ] Migration report JSON + before/after hashes.
- [ ] Benchmark JSON for any performance claim (perf-bench only).
