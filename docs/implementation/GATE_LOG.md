# Gate Log — ProtonSearch v2 (Project Phoenix)

Evidence-first record. Every entry: what was done, command/output proving it, owner, date.
Status per gate lives in docs/implementation/WORK_BREAKDOWN.md.

## Gate 0 — Reconnaissance

- 2026-11-09 L0: baseline branch v2-phoenix + tag v1.1.1-v2-baseline created; working tree committed.
- 2026-11-09 L0: legacy test suite: 85 passed, 0 failed, 1 ignored (cargo test --no-fail-fast in protonsearch/; log .agents/baseline-old-tests.log).
- 2026-11-09 L0: forensic SQLite dump of the live legacy index (immutable read-only):
  files 89,813 · files_fts 14,609 · browser_items 4,523 · memory_events 5,286 · git_repos 15 · git_commits 516 ·
  git_todos 132 · quicklinks 4 · snippets 1 · settings_catalog_fts 238 · ai_settings 2 · clipboard_history 0 ·
  timeline_events 0. DB size 149,372,928 bytes (+23.8 MB WAL).
- Gate 0 deliverables (feature parity matrix with evidence, security register, research) assigned to rel-recon; **status: see reports**.
- Missing artifacts (recorded, not silently ignored): assets/protonsearch-mark.svg, assets/protonsearch-logo-concept.png,
  assets/original-logo-crop-reference.png do not exist in the workspace; the brand mark in the supplied HTML is the visual source.

## Gate 1 — Architecture and contracts

- Cargo workspace (crates/ps-ipc, ps-core, ps-index, ps-content, ps-sources, ps-windows, ps-ocr, ps-agent, ps-bench,
  apps/desktop/src-tauri) committed; legacy proto crate excluded and left untouched.
- IPC contract v1 frozen in crates/ps-ipc (PROTOCOL_VERSION=1, DATA_SCHEMA_VERSION=1) with serde round-trip tests;
  TypeScript mirror in apps/desktop/src/bridge/types.ts; browser mock bridge for UI development.
- ADR-0001..0006 written (Tauri migration, search core, FTS5 vs Tantivy, OCR engine, agent runtime, IPC).
- Design system v1.0 (docs/design/DESIGN_SYSTEM.md) landed by design-spec; tokens.css v1.0 implemented by L0, Inter Variable bundled.
- Build environment finding: host MSVC intermittently fails (cl D8050) writing /Z7 debug records for bundled SQLite;
  fixed with workspace [profile.dev] debug = 0 (recorded in root Cargo.toml). Evidence: cargo check -p ps-content passes after fix.

## Gate 2 — Engine (in progress)

## Gate 3 — Launcher (in progress)

## Gate 4 — Desktop app / onboarding (in progress)

## Gate 5 — Rich search (pending)

## Gate 6 — Agent sidecar (pending)

## Gate 7 — Release candidate (pending)
- Test-environment limitation recorded: processes launched by the agent harness cannot write %APPDATA%/%LOCALAPPDATA%
  (access denied; the harness sandbox grants writes under the workspace only). The app therefore supports the
  PROTONSEARCH_DATA_DIR override (also useful for portable installs); real-app E2E runs use .devdata/ inside the repo.
  End-user default remains %APPDATA%\\protonsearch\\v2. Verified with a compiled probe: writes under the workspace succeed,
  writes to %APPDATA%/%TEMP% fail with os error 5 from child executables while PowerShell itself is allowed.
## Gate 2 / 3 — engine + launcher integration (evidence)

- 2026-11-09: **First real end-to-end app run.** Tauri debug binary (WebView2 157), production frontend served by Vite preview,
  ps-core engine + ps-index over the real user profile (Documents/Desktop/Downloads).
  - Filesystem: 131,199 entries indexed at warm startup (3 roots, names-only depth).
  - Real search verified through WebView2 CDP against the actual window: "arch" -> 50 matches (ARCHITECTURE.md first),
    "readme" -> real files, typo query fuzzy-matched, zero-match state rendered.
  - End-to-end input-to-visible latency (page timestamp before input event -> double rAF after first row, debug Rust build,
    150% DPI, production JS, n=4 queries only): 40.6 ms first query/cold render; 12.6 ms, 8.4 ms, 10.8 ms warm.
    Raw: docs/qa/visual/app-captures/latency-real-app.json. Debug-build caveat recorded; release numbers pending.
  - Real window captures: docs/qa/visual/app-captures/real-launcher-*.png, real-app-*.png (150% DPI, live data).
- 2026-11-09: **Legacy migration verified on real data** (copy of the 149.4 MB v1 index; source opened read-only via SQLite backup API):
  - v2 DB 149.8 MB; files 89,813 imported; documents 14,491 (legacy files_fts) imported; source SHA-256 identical before/after
    (BE255E99... -> BE255E99...), legacy_migrated=true in settings.
  - NOTE: extra migrated tables (browser_items/quicklinks/snippets) were absent in that run; migration error surfaced as a warning
    instead of aborting (by design), and content extraction then indexed real PDFs. Root cause under investigation (see known issues).
  - pdf-extract 0.10 panics internally on malformed PDF content streams (InvalidContentStream). This was observed as isolated worker
    thread panics in debug; **release profile panic=abort was removed** so a malformed document can never abort the whole app.
    Extraction must additionally catch panics in the worker (assigned to sources-content, tracked as a release blocker).
- 2026-11-09: Tauri shell now supports live hotkey re-registration (set_hotkey), cursor-monitor launcher placement, WebView2
  user-data-dir override for portable/testing installs, CDP test hook (PROTONSEARCH_CDP_PORT), and optional agent-sidecar commands
  (agent_status/start/cancel/approve/stop) with events forwarded on agent:event. The sidecar is never started at setup/search.
- 2026-11-09: ps-ipc AgentEvent payload fields corrected to camelCase (rename_all_fields) + contract test; ADR-0005 accepted
  (Pydantic AI 2.66.0 over Nanobot 0.4.1, measured).
## Gate 7 — release candidate artifacts (evidence)

- 2026-11-09: release build succeeds: target/release/ProtonSearch.exe (17.9 MB) + NSIS installer
  target/release/bundle/nsis/ProtonSearch_2.0.0-alpha.1_x64-setup.exe (4.69 MiB; unsigned).
  SHA-256 of the first installer build: 2D82B2E8542BBD6AFD72CAA0858ED1C48D408E314A6BC2444EF381D1AA0C548C.
  It is rebuilt after the v2 security-review fixes; the final artifact plus checksums is recorded in
  dist-installer/SHA256SUMS.txt by scripts/build-installer.ps1.
- 2026-11-09: security review docs/security/V2_SECURITY_REVIEW.md reviewed; actionable items fixed:
  migration retry semantics + WAL/SHM sidecar hashing + imported counts, server-side destructive-action
  confirmation handshake (engine + Tauri command + bridge), CDP gated to debug builds only, clipboard history
  default OFF, migration URI percent-encoding. Remaining lower-risk items tracked in docs/release/KNOWN_LIMITATIONS.md.
- 2026-11-09: OCR pipeline implemented in ps-ocr (Windows.Media.Ocr platform + text store + tests); the engine
  owns OCR text storage/search. perf-bench measured the native OCR engine at 244 images/s (p50 3.6 ms, English
  char accuracy 92.4% on the synthetic corpus; CJK unverified). End-to-end screenshot indexing through the app
  still needs an exercised run (settings-gated; stated in the acceptance report).
## Gate 7 — final RC evidence (build 4, committed tree 4066df7)

- Final installer rebuilt from the committed tree: dist-installer/ProtonSearch_2.0.0-alpha.1_x64-setup.exe
  (SHA-256 3CB375B4EB48C791FFE9A90C33666DBB2997A4C5E2DB73A9B021D0C73EAC12DD) and
  dist-installer/ProtonSearch.exe (SHA-256 A251D784B3E4B372FD73DDF00EB86AAD9A6B49559D9CD4C64DAC7BE256D97E33),
  BUILD_INFO git_rev=4066df7, unsigned, install mode currentUser.
- Release binary smoke test: launched from dist-installer with PROTONSEARCH_DATA_DIR; process stayed alive, setup completed
  (engine/windows/tray/hotkey), WebView2 profile + v2 index created under the data dir; a real window was captured at
  1195x837 (docs/qa/visual/app-captures-release/release-1-1195x837.png). The pre-warmed launcher is intentionally hidden on
  startup for returning users; CDP-based automation is deliberately disabled in release builds (security V2-05), so
  interaction-level release screenshots are not automatable — documented limitation.
- Final test totals: cargo test --workspace 129 passed / 0 failed / 1 ignored (action smoke verified separately);
  frontend 143 passed / 9 files; ps-bench 2 passed; migration harness 6 contract checks.
- Formal filename benchmark at a28042d (valid at HEAD): 10k p95 0.451 ms; 100k p95 1.047 ms (target <5 ms MET);
  1M p95 7.228 ms (target <15 ms MET); 110.1 MiB at 1M; long_path regression fixed (0 -> 1 result, p50 1.073 ms).
- v2 content retrieval re-measured after optimisation: p95 43.0 ms (was 61.1; 108.2 before the first fix) — target <30 ms NOT MET.
