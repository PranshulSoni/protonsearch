# Changelog — ProtonSearch v2

All notable changes to the v2 "Project Phoenix" line. This file covers **v2 only**; the
legacy v1.1.1 baseline is audited in docs/audit/GATE0_BASELINE.md.

## [2.0.0-alpha.1] — 2026-11-10 — RELEASE CANDIDATE (UNSIGNED, internal alpha)

First RC of the v2 "Project Phoenix" rebuild. Source commit 4e9f589; unsigned NSIS installer and
portable binary produced. Internal alpha only: do not distribute.

### Added / verified
- Normal Launcher v2: All/Files/Folders/Apps filters, real quick destinations, result previews,
  keyboard/a11y and live input-latency captures.
- V9 Dynamic Island renderer: 8 cards, page dots, wheel/swipe, motion tokens, V9 geometry, real data
  for Search/Clipboard/Agents/Browser/Git; Media/Quick Settings/Notifications capability-honest.
- Preview pipeline P0-1: resultId-only get_preview IPC + bounded engine previews with LRU cache;
  live-verified TXT/code/MD/JSON/CSV/PDF/DOCX/XLSX/PNG+OCR/binary/unknown/large/Unicode and
  stale-file fallbacks. Known gap: matched-term highlight ranges (BUG-G8-01).
- Clipboard image P0-2: real CF_DIB/BI_BITFIELDS/DIBV5/CF_BITMAP/registered PNG capture, PNG storage
  + bounded thumbnails (<=128 KiB, dims <=8192, image/png allowlist), island Images thumbnails,
  restart persistence, paste-back harness coverage, master privacy gate + format toggles.
- Agent task DTOs moved to ps-ipc; runtime-agnostic task history; read-only Pi alpha gate
  (allow_writes=false, RR-02); detect-agents and agent task APIs wired.
- Settings: two-launcher model, island placement/card manager, clipboard/privacy toggles, validation
  and additive-serde migration policy; ADR-0007..0012 + versioning policy.
- Release artifacts: dist-installer/ProtonSearch.exe, NSIS setup.exe, BUILD_INFO.txt, SHA256SUMS.txt;
  UNSIGNED=true documented.

### Performance
- Content retrieval p95 8.984 ms MET (30 ms target, official quiet, n=960, 13,296 docs).
- Release activation p95 14.289 ms MET; selection 6 ms MET.
- Debug input->visible p95 38.2 ms NOT MET (30 ms target); idle tree WS 786.9 MiB NOT MET
  (~150 MB budget). Both recorded honestly for post-alpha work.

### Known issues (public-release gates)
- Artifacts unsigned; no updater; installer upgrade/uninstall drill not run.
- Single-monitor host: mixed-DPI/High-Contrast/multi-monitor untested.
- Native media/Wi-Fi/Bluetooth adapters post-alpha milestone 2 (honest disabled states).
- No live Pi provider credentials; write-enabled agent path closed.
- Preview matched-term highlight missing; island preview hookup/both-launcher parity pending.
- Public release NOT signed off; see docs/release/ACCEPTANCE_REPORT.md.

## [2.0.0-alpha.1] — 2026-11-09 — historical Gate-0 scaffold

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
- tests/migration/: deterministic synthetic legacy DB fixture, read-only contract tests, and a
  dev-only Rust probe (tests/migration/harness) that calls ps_core::migration::migrate_legacy_db
  (migration unit tests 2/2 pass).
- docs/security/V2_SECURITY_REVIEW.md: security review of migration, engine, Tauri command
  surface, ps-windows and ps-content (V2-01...V2-12; no criticals; must-fixes listed).
- Pi coding-agent CLI backend (crates/ps-agent/src/pi): one-shot pi --mode json runs, read-only
  tool allowlist by default, writes/shell opt-in, timeouts/tree kill, redaction, offline tests.
  Legacy Hermes runtime dropped; Pydantic sidecar kept as optional fallback (G6 WIP).
- docs/security/PI_AGENT_SECURITY.md: Pi threat model, code-verified controls and release gates
  (PI-SEC-01..15; must-fix items before UI enablement).
- Release documentation set: BUILD, MIGRATION, SECURITY, KNOWN_LIMITATIONS,
  THIRD_PARTY_LICENSES, RELEASE_CHECKLIST, CHANGELOG (this file).

### Security
- Post-review hardening applied (docs/security/V2_SECURITY_REVIEW.md, "Resolved in tree"):
  migration retry on failure (V2-01), DB+WAL+SHM source fingerprint (V2-02), imported-count
  reporting (V2-03), server-side 15 s confirmation for destructive native actions (V2-04),
  CDP disabled in release builds (V2-05), clipboard capture opt-in/off by default (V2-06),
  migration URI percent-encoding (V2-09).
- **Action required:** rotate the AI API key leaked in legacy git history (S-01). Do not reuse
  it. v2 ships no embedded/shared keys.
- v2 update design is gated on signed manifest + Authenticode + SHA-256 (S-02); updater not
  implemented in this alpha.

### Known issues
- Migration API landed; consent/rollback flow and imported-count reporting remain G4 work
  (review V2-01/V2-02/V2-03).
- No v2 artifact built/run; installer unsigned; lockfiles untracked; CI red by design.
- Frontend vitest suites are landing with the UI work; no E2E/coverage gate yet; transient TS errors while UI agents edit.
- Pi backend is not yet wired into the Tauri shell at review time (PI-SEC-10); must-fixes
  PI-SEC-01..06 pending (MCP/extensions/context, final-Output redaction, env, UI disclosure).
- Sandbox test environment requires PROTONSEARCH_DATA_DIR (see BUILD.md section 3).

### Notes
- Version: 2.0.0-alpha.1 (Cargo.toml [workspace.package], apps/desktop/package.json,
  apps/desktop/src-tauri/tauri.conf.json). No git tag created for v2.
