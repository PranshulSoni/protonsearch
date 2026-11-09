# Changelog — ProtonSearch v2

All notable changes to the v2 "Project Phoenix" line. This file covers **v2 only**; the
legacy v1.1.1 baseline is audited in docs/audit/GATE0_BASELINE.md.

## [2.0.0-alpha.1] — 2026-11-09 — UNRELEASED, NOT DISTRIBUTABLE

Internal Gate-0/Gate-1 scaffold. No installer, no signed artifacts, no public release.

### Added
- Cargo workspace (crates/ps-ipc, ps-core, ps-index, ps-content, ps-sources, ps-windows,
  ps-ocr, ps-agent, ps-bench) plus Tauri 2 shell (apps/desktop/src-tauri) and React/TypeScript
  UI shell.
- PROTONSEARCH_DATA_DIR override in ps-core::paths so tests/portable installs never touch
  legacy %APPDATA%\protonsearch data.
- Gate-0 forensic audit: docs/audit/GATE0_BASELINE.md, docs/audit/LEGACY_ARCHITECTURE.md,
  docs/audit/GATE0_REPORT.md; feature parity matrix verified against legacy source with
  file:line evidence (docs/implementation/FEATURE_PARITY_MATRIX.md).
- Security baseline: docs/security/SECURITY_RISK_REGISTER.md (threat model + S-01...S-20),
  consolidated release view in docs/release/SECURITY.md.
- Competitive research with citations: docs/research/RESEARCH.md (Kensaku, Everything, Flow
  Launcher, PowerToys Command Palette, Raycast) and licence/attribution notes.
- CI skeleton .github/workflows/ci.yml: Windows Rust fmt/clippy/test, frontend npm ci + tsc +
  vitest + vite build, manual unsigned NSIS build with artifact upload. YAML validated locally;
  pipeline currently expected red while G1 lands.
- scripts/build-installer.ps1: dry-run-by-default unsigned NSIS wrapper with preflight, artifact
  copy, SHA-256 checksums and BUILD_INFO.txt; -Execute gated on Lead approval.
- tests/migration/: deterministic synthetic legacy DB fixture + read-only contract tests
  (4 pass, 1 skipped) + Rust skeleton documenting the missing ps_core::migration API.
- Release documentation set: BUILD, MIGRATION, SECURITY, KNOWN_LIMITATIONS,
  THIRD_PARTY_LICENSES, RELEASE_CHECKLIST, CHANGELOG (this file).

### Security
- **Action required:** rotate the AI API key leaked in legacy git history (S-01). Do not reuse
  it. v2 ships no embedded/shared keys.
- v2 update design is gated on signed manifest + Authenticode + SHA-256 (S-02); updater not
  implemented in this alpha.

### Known issues
- Migration is **BLOCKED** on the engine API; import correctness is untested.
- No v2 artifact built/run; installer unsigned; lockfiles untracked; CI red by design.
- Frontend vitest suites are landing with the UI work; no E2E/coverage gate yet; transient TS errors while UI agents edit.
- Agent sidecar is not yet a committed manifest-managed component.
- Sandbox test environment requires PROTONSEARCH_DATA_DIR (see BUILD.md section 3).

### Notes
- Version: 2.0.0-alpha.1 (Cargo.toml [workspace.package], apps/desktop/package.json,
  apps/desktop/src-tauri/tauri.conf.json). No git tag created for v2.
