# Known Limitations — v2.0.0-alpha.1 (Gate 0)

This is an **internal alpha scaffold**, not a shipped product. Everything below is a known
limitation at the time of writing (2026-11-09); remove an entry only with evidence.

## Build / packaging
1. **No v2 artifact has been built or run.** The release installer has not been produced;
   npx tauri build is **UNVERIFIED** on this host. scripts/build-installer.ps1 is
   dry-run-tested only.
2. **Unsigned.** No Authenticode signing, no signed update manifest, no updater implemented.
   The wrapper emits SIGNED=false and refuses to publish (it never publishes at all).
3. **Lockfiles untracked.** Cargo.lock and apps/desktop/package-lock.json exist but are not
   committed yet; release builds are not reproducible until they are.
4. **CI is red by design at Gate 0.** cargo fmt --all -- --check currently reports diffs on the
   in-progress scaffold; frontend has transient TS errors in UI-agent files.
5. **NSIS toolchain not validated.** Tauri's NSIS bundler may require NSIS on PATH depending on
   the Tauri version; not verified here. installMode=currentUser.
6. **winget manifests are stale** (legacy OmniSearch 1.0.0) and must not be used for v2.

## Migration
7. **Migration API landed** (ps_core::migration::migrate_legacy_db; 2 unit tests pass) and the
   contract test calls it via tests/migration/harness. Onboarding/consent/rollback and real-DB
   acceptance remain G4 work (see docs/release/MIGRATION.md).
8. **WAL replay not exercised** on the real 23.8 MB un-checkpointed WAL. The contract now
   fingerprints DB+WAL+SHM before/after and sets wal_or_shm_changed (V2-02 fixed); a real import
   (non-dry-run) against a WAL-heavy legacy DB is still untested.
9. **AI keys are not migrated** (by design); no consent UI yet. Legacy keys must be rotated.

## Features / runtime
10. **Frontend:** vitest unit suites are landing with the UI work, but there is no E2E suite
    and no coverage threshold yet; visual QA and dark-mode/DPI screenshots are pending (G3/G4).
11. **Engine:** filename/content/OCR/sources are scaffold/stub-level at Gate 0; no measured
    performance claims. Legacy baseline numbers only (docs/audit/GATE0_BASELINE.md).
12. **Legacy app runtime was not exercised** (GUI not launched by rule); legacy "working"
    claims are code/test/DB evidence only.
13. **Agent sidecar** is not committed as a manifest-managed component (agent-sidecar/.venv and
    logs exist locally; the directory is excluded from the workspace). pydantic-ai is installed
    in that local venv (2.66.0, MIT) but there is no committed pyproject.toml yet.
14. **AI is opt-in and not wired** in v2; no embedded/shared keys will ship.
15. **Clipboard capture is opt-in/off by default** (privacy.clipboard_history=false). Capture,
    search and image storage exist in ps-sources/ps-windows, but source-app exclusions, TTL,
    DPAPI encryption and the capture UI are follow-ups.
16. **OCR pipeline** (Windows.Media.Ocr) is planned (G5); no v2 OCR evidence yet.
17. **No telemetry/crash reporting** — intentionally absent; support relies on local logs that
    must be redacted before sharing.

## Environment
18. **Sandbox %APPDATA% constraint (test environment only):** child processes cannot write
    %APPDATA% here, so E2E/migration tests must set PROTONSEARCH_DATA_DIR to a temp dir (v2 data
    goes to <temp>/v2). This is a harness limitation, not product behaviour; the same override
    supports portable installs.
19. **Windows-only** target for alpha (Tauri shell + Win32 sources). No macOS/Linux claim.

## Release process
20. No public release, no pushes, no tag for v2 yet. Do not distribute these artifacts.

## Alpha cuts (deliberate, not shipped in v2.0.0-alpha.1)

- No AI sidecar is bundled and no API keys are embedded; AI/Hermes is opt-in and not shipped.
- No self-updater and no code signing; the installer is unsigned (see Below).
- No plugin/extension ecosystem (internal ps-sources only).
- No privileged USN/MFT volume indexer; v2 indexes user-selected roots in user space.
- No Google Lens/circle-to-search, voice input, Steam games, notes, focus sessions, window
  controls, process kill, or semantic/vector search in the alpha (legacy features remain in the
  parity matrix as PLANNED-V2/CUT).
- OCR pipeline exists (Windows.Media.Ocr) but has not passed an end-to-end acceptance run.

## Residual security risks (post-review)

See docs/security/V2_SECURITY_REVIEW.md "Resolved in tree": V2-01/02/03/04/05/06/09 are fixed or
mitigated; residual items are V2-07 (clipboard DIB pixel cap, assigned), V2-08 (app-target
validation depth), V2-10 (activity-log retention UX). Legacy release gates remain: rotate the
leaked AI key (S-01) and replace the unsigned updater (S-02) before any public build.
