# ProtonSearch v2 — Acceptance & Truth Report (Project Phoenix)

Owner: L0 (Program Director). Date: 2026-11-09. Status: **v2.0.0-alpha.1, release candidate in progress**.
This document is the single honest answer to: what is complete, what is partial, what is unsupported, and what was not tested.
No claim here is stronger than its evidence. Numbers are measured on the host described in docs/performance/BENCHMARKS.md.

## 1. Executive summary

ProtonSearch v2 is a working rebuilt application, not a plan:

- A new Rust workspace (ps-ipc, ps-core, ps-index, ps-content, ps-sources, ps-windows, ps-ocr, ps-agent, ps-bench)
  and a Tauri 2 / React / TypeScript desktop app implementing the supplied design reference (launcher, desktop app, onboarding).
- The real app runs on this machine, indexes the real user profile (131,199 entries in Documents/Desktop/Downloads),
  returns real grouped results, supports keyboard-first navigation, settings, onboarding, activity, diagnostics and dark mode.
- The legacy v1 index (149.4 MB, 89,813 files, 14,609 document texts) migrates into v2 with the source file verified
  unchanged by SHA-256; the legacy app remains buildable and untouched for comparison and rollback.
- An optional Python agent sidecar (Pydantic AI) exists with a versioned JSONL protocol, approval-gated tools and a Rust client;
  it is never started for search.
- Reproducible benchmarks, golden relevance fixtures, a security risk register, ADRs, migration docs, CI and installer scaffolding exist.

The alpha is not a finished product. The most important gaps are listed in §5 and §6: OCR extraction, finalisation of
the applications/clipboard/browser/Git sources and action execution, signed installer, and complete UI visual QA at all DPI scales.

## 2. Gate status

| Gate | Scope | Status | Evidence |
|---|---|---|---|
| G0 | Reconnaissance: legacy build/tests, feature inventory, DB forensics, risks, baseline | **DONE** | docs/audit/GATE0_BASELINE.md, GATE0_REPORT.md, LEGACY_ARCHITECTURE.md; docs/implementation/GATE_LOG.md |
| G1 | Architecture, IPC contract v1, ADRs, design system, security model, test strategy | **DONE** | docs/adr/ADR-0001..0006; crates/ps-ipc; docs/design/*; docs/arch/OVERVIEW.md |
| G2 | Fast Rust engine: filename index, content FTS, sources, ranking, cancellation, persistence | **PARTIAL — core done, sources finalising** | crates/ps-index (tests), ps-content (tests), ps-core engine; docs/performance |
| G3 | Integrated frameless launcher on real results, keyboard model, real-window captures | **DONE (visual QA pending)** | docs/qa/visual/app-captures/real-launcher-*.png; tools/visual-qa; latencies in GATE_LOG |
| G4 | Desktop app, onboarding, settings, migration, dark mode, accessibility | **DONE (visual QA pending)** | docs/qa/visual/app-captures/real-app-*.png; apps/desktop tests (136); migration evidence |
| G5 | Rich search: previews/open-at-match, OCR, clipboard/browser/Git actions | **PARTIAL** | content FTS + open-at-match fields done; OCR + sources finalising; preview bodies limited |
| G6 | Optional Python agent sidecar with real execution | **PARTIAL — sidecar+protocol+client done, UI panel pending** | crates/ps-agent tests; agent-sidecar tests; docs/agent/* |
| G7 | Release candidate: installer, updater, full checklist, visual audit, docs | **IN PROGRESS** | .github/workflows/ci.yml; scripts/build-installer.ps1; docs/release/*; this report |

## 3. What is complete (verified)

1. **Workspace and contracts.** Everything compiles with cargo check/build for the shipped binary; ps-ipc has serde
   round-trip contract tests and a TypeScript mirror; the browser mock bridge lets UI states be developed and tested without Rust.
2. **Filename engine.** Compact in-memory index with scan, arena storage, prefix/scan/fuzzy tiers, bloom and directory prefilter,
   deterministic ordering, versioned persistence, incremental upsert/remove/compact, tests, and a benchmark example.
3. **Content engine.** SQLite FTS5 documents index with external-content table, BM25 ranking, snippet + 1-based match line,
   extractors for PDF/DOCX/XLSX/text/code/Markdown (bounded bytes, truncated flag), legacy import, sanitised FTS queries, tests.
4. **Win32 helpers.** Icon extraction to cached PNG, shell open/reveal, http(s)-only URLs, AppsFolder enumeration, shortcut
   resolution, clipboard text read/write, foreground window; tests including a real PNG icon extraction from notepad.exe.
5. **Engine facade.** Tiered coordinator, calculator/action/app/name/content/source/OCR tiers, deterministic finalise + dedupe,
   per-tier timings, settings load/patch persistence (telemetry forced off), activity log, onboarding suggestions,
   legacy migration (read-only source, SHA-256 verified), background scan/content/OCR pipeline with pause/rebuild commands.
6. **Launcher UI.** Frameless, pre-warmed window; search row with brand mark; grouped results; 2 px selected-state bar;
   keyboard model (arrows, Enter, Esc chain, Tab, Ctrl+Space preview, context-menu key, copy-path); stale-request protection;
   exact empty / 0-match / error copy; footer hints only; index status; honest limitations.
7. **Desktop app + onboarding.** Five routes with real bridge data; time-of-day greeting with optional name; index health and
   per-root depth controls; activity with privacy pause; workflows/agent honest state; preferences (theme/density/motion/privacy/
   diagnostics); live hotkey capture and re-registration; 7-step resumable onboarding with legacy-detected notice; dark theme.
8. **Real-data migration.** Copy of the live v1 DB imported read-only: 89,813 file rows + 14,491 document texts imported,
   source hash unchanged, legacy_migrated recorded; no credentials imported.
9. **Agent sidecar (optional).** Pydantic AI 2.66.0 selected over Nanobot with measured evidence; JSONL v1 protocol
   (started/plan/progress/approval_request/tool_result/output/error/finished), cancel and approvals; tools are approval-gated and
   cannot execute shell/file/network operations themselves; Rust client with spawn, framing bounds, idle kill, crash restart (max 1);
   14 Rust + 18 Python tests; never started for search.
10. **Release engineering foundations.** CI workflow (Windows fmt/clippy/test + frontend + manual installer job), installer script
    (dry-run by default, checksums, unsigned caveat), migration fixtures and contract tests, security register/threat model,
    research notes with citations, third-party licence inventory, build/migration/release documentation.

## 4. Measured performance (summary; full report in docs/performance/BENCHMARKS.md)

Engine-only, release build, committed engine hash 3906836, this host (22 fixed queries x 30 iterations, n=660 per corpus):

| Corpus | p50 | p95 | p99 | Build | Engine memory |
|---|---:|---:|---:|---:|---:|
| 10k realistic | 0.070 | 0.584 | n/a | 15 ms | 1.2 MB |
| 100k | 0.600 | 2.195 | n/a | 220 ms | 12.4 MB |
| 1M | 2.713 | 7.583 | n/a | 3,529 ms | 110.1 MiB (~115 B/entry) |

Targets from the brief: 100k warm p95 < 5 ms (**MET: 2.195 ms**); 1M warm p95 < 15 ms (**MET: 7.583 ms**).
Variance is disclosed in the report: an earlier pre-perf-fix revision measured 5.64/14.93 ms, and one contended 1M run measured 28.4 ms
(concurrent agent compiles). Cold process: spawn+load 3.3/45-72/505-525 ms at 10k/100k/1M; the 1M cold load motivates pre-warming at app start
rather than on the hotkey path (the launcher window is pre-created; the index is loaded during setup, not on summon).
Content retrieval (v2, ps-content): warm p50 10.86 ms / p95 61.10 ms / p99 110.7 ms on 13,296 docs — **the 30 ms p95 target is NOT met**;
the legacy v1 FTS5 baseline measured 6.07 ms p95 and is reported only as a migration baseline. Profiling is assigned as a release blocker.
OCR (Windows.Media.Ocr, English only on this host): 244 images/s warm, p50 3.6 ms per image, p95 7.3 ms; 92.4 % character accuracy,
59.7 % image-exact on the synthetic 72-image corpus; CJK/language variance UNVERIFIED (no CJK language pack installed).
Relevance: synthetic 10k MRR@10 1.000/recall@10 1.000/exact@1 1.000; real 89,813-entry corpus MRR@10 0.928/recall@10 0.983/exact@1 1.000 (64/65).
Real-app end-to-end input-to-visible (debug Rust build, production JS, 150 % DPI, 131,199 entries): n=4 exploratory queries
(3.8-32.3 ms depending on cold render); not a p95; release-build percentile pass is pending the release binary.
Competitors: no cross-tool latency table is published. Everything 1.4.1 is installed but its HTTP/ETP interfaces are disabled and no CLI/SDK
is present; Kensaku 1.0.16 is installed without a CLI; PowerToys is present; Flow Launcher absent. Scope mismatch is stated in the report.
Honest veto list (do not claim): sub-5 ms at 100k without naming commit 3906836; content speed without the snippet/rank split; UI under 30 ms
p95 from the n=4 debug sample; CJK OCR; Everything/Kensaku parity.

## 5. What is partial (works, with documented limits)

1. **Sources tier.** ps-sources is implemented but was still compiling when this report was written; applications, clipboard,
   browser history/bookmarks and Git results may show empty or "starting" in the UI until that lands. The engine path and UI
   states are honest about this (no fake data).
2. **Action execution.** Action results and the reviewed NativeCommand catalog exist; actual opening/launching depends on
   ps-sources::actions (landing with the same change set). Until then the shell returns a structured not-implemented outcome.
3. **OCR.** ps-ocr extraction is the next pipeline step; engine storage/search for OCR text exists (ocr_documents + FTS) and is
   disabled by default. No OCR accuracy numbers are claimed until measured.
4. **Previews.** Metadata and content snippet/line are real; image/PDF rendering and a preview IPC payload are not implemented
   (the UI says so honestly).
5. **Agent UI.** Sidecar, protocol, Rust client and shell commands exist; the Workflows page does not yet expose a run/approve/
   stream panel (in progress). Agent actions are approval-gated by design; host-side execution of approved capabilities is v1-scoped.
6. **Visual QA.** Real-window captures exist at 150 % DPI (light). Full matrix (100/125/150/200 %, dark, narrow,
   empty/loading/error, high contrast, keyboard focus) is being captured; findings are tracked in docs/qa/visual.

## 6. Not implemented / unsupported in this alpha (explicit)

- Signed installer / code signing, automatic updater with signature validation (scaffolding only; unsigned NSIS build planned).
- Clipboard multi-select and "paste N" (CUT for alpha; documented in docs/design/SCREENS.md §10).
- Exact-command ACTIONS pinning (no contract marker; documented deviation).
- Folder picker, quicklink/snippet editors, clear-history UI, replay-search, updater/log-export UI (no IPC commands yet;
  the UI omits dead controls rather than faking them).
- NTFS MFT/USN fast path (design permits it later; current scanner uses portable traversal).
- Semantic/local embeddings retrieval (settings placeholder only; no model download).
- OCR engine comparison (Windows.Media.Ocr vs PaddleOCR/Tesseract) beyond a documented plan.
- Signed release notes, winget submission, auto-update channel.
- Non-Windows platforms.

## 7. Not tested / unverified (honest)

- Real UI latency percentiles on a **release** build (only debug-build warm samples above).
- Full DPI matrix, high-contrast mode and screen-reader passes (code follows the design contract; runtime evidence pending).
- Agent sidecar against a real model provider (no credentials available by design); tested with the deterministic test model.
- Agent tool execution on the host (capability results are deferred; approval flow tested).
- Installer/uninstaller execution and upgrade-in-place (script is dry-run validated; NSIS build pending green integration).
- Clipboard listener behaviour across real clipboard managers; browser-profile variants not present on this machine;
  Git repository edge cases beyond the local fixtures.
- Power/battery impact and idle CPU over long sessions (short observations only).
- Multi-monitor placement verification with more than one active display (code positions on the cursor's monitor).

## 8. Security posture

Legacy v1 issues (docs/security/SECURITY_RISK_REGISTER.md) are **not** carried into v2 by design:
no telemetry, no hardcoded credentials, no shell-string action execution, structured ActionSpecs, http(s)-only URL opens,
read-only browser DB copies, opt-in sensitive sources, agent approval gates, read-only legacy migration.

**Action required by the owner (non-code):** a full AI API key is present in the legacy git history (masked in the report;
commits 41b56c8/1bfe506/11f0dfe/d55c3ad). It must be revoked/rotated with the provider before any release announcement.
See SECURITY_RISK_REGISTER.md S-01/S-02/S-03.

## 9. How to reproduce the evidence

See docs/release/BUILD.md for build/run, docs/performance/METHODOLOGY.md for benchmarks, and docs/implementation/GATE_LOG.md for
the command/output trail. App screenshots: docs/qa/visual/app-captures/*.png (real WebView2 windows, live data).
Machine: AMD Ryzen AI 9 465 (10C/20T), 31.2 GB RAM, Windows 11 build 26300, WebView2 157, rustc 1.99, Node 26.7.

## 10. Bottom line

The core promise — press the shortcut, find what you remember, open it — is implemented and demonstrably works on real data.
The remaining work to call this a finished product is finite and listed above; the largest items are source/action finalisation,
OCR, the agent UI panel, signed packaging, and complete visual QA and release gating.
## 11. Late-integration updates (same day, after §1-§10 were drafted)

- SOURCES/ACTIONS: ps-sources, ps-windows and ps-content are integrated; the real launcher now shows real installed
  applications (Start Menu/UWP scan), browser history/bookmarks (Chromium/Firefox profiles, read-only snapshots), recent
  files, clipboard (opt-in) and Git results, and executes open/reveal/launch actions through structured ActionSpecs.
  Evidence: docs/qa/visual/app-captures/real-launcher-long-list.png (real application results, dark theme) and the
  updated captures list.
- OCR: the ps-ocr pipeline is implemented (Windows.Media.Ocr bitmap decode, pixel caps, text store, tests). The native
  engine was independently measured by perf-bench (244 img/s warm, p50 3.6 ms, 92.4% char accuracy on the synthetic
  English corpus). End-to-end screenshot indexing through the running app is settings-gated and has not yet been
  exercised in a UI session; OCR results are therefore reported as implemented + unit/integration tested, not as
  user-validated.
- SECURITY: docs/security/V2_SECURITY_REVIEW.md findings addressed in code: destructive native commands now require a
  server-side 15 s one-shot confirmation handshake (engine + Tauri command + bridge + launcher confirm bar); CDP is
  debug-build only; clipboard history defaults OFF; migration marks success only on success, hashes DB+WAL+SHM and
  reports imported (not source) row counts; legacy URI is percent-encoded. Lower-risk items (clipboard source-app
  exclusions/TTL, code signing, app-target allowlisting) remain open and are listed in docs/release/KNOWN_LIMITATIONS.md.
- INSTALLER: a real release build produces target/release/ProtonSearch.exe (~17.9 MB) and an NSIS installer
  (ProtonSearch_2.0.0-alpha.1_x64-setup.exe, ~4.7 MiB, unsigned). The final hashes/checksums are produced by
  scripts/build-installer.ps1 into dist-installer/SHA256SUMS.txt; no signing certificate is available, so SmartScreen
  warnings are expected (documented).
- RELEASE BUILD UI LATENCY: the release binary intentionally disables the CDP test hook, so the E2E latency harness
  cannot drive release builds. End-to-end latency evidence therefore remains debug-build (production JS) only; engine
  numbers remain release-build. This limitation is deliberate (security) and recorded rather than worked around.
- COMPETITOR COMPARISON: not published. Everything 1.4.1 is installed but its HTTP/ETP interface and CLI are absent;
  Kensaku 1.0.16 has no CLI; PowerToys is present; Flow Launcher absent. Scope mismatch makes any single-number
  comparison misleading; docs/performance/BENCHMARKS.md documents what was and was not measured.
## 12. Final RC numbers and evidence index (supersedes earlier tables where they differ)

Filename engine, formal revision a28042d (ps-index frozen and unchanged through HEAD; quiet-machine run, release):
- 10k: p50 0.059 ms / p95 0.451 ms / p99 0.488 ms; build 12 ms
- 100k: p50 0.471 ms / p95 1.047 ms / p99 1.552 ms; build 171 ms -> target <5 ms MET
- 1M: p50 2.433 ms / p95 7.228 ms / p99 8.545 ms; build 2.87 s; 110.1 MiB (~115 B/entry) -> target <15 ms MET
- cold binary load 3 ms / 52-65 ms / 442-448 ms at 10k/100k/1M; cold first query 0.13 / 1.30-2.07 / 3.22-4.01 ms.
- 1M full-path regression fixed in a28042d: long_path 0 -> 1 result (top-1 anchor), per-query p50 1.073 ms; new regression tests.

Content retrieval (v2 ps-content, HEAD 8692997): warm p50 9.4 ms / p95 43.0 ms / p99 67.5 ms after the bounded line-lookup
optimisation (was p95 61.1 ms; 108.2 ms before the first fix). Target <30 ms is NOT met. Top remaining gap; the remaining
cost is FTS snippet() over external-content rows plus the rank path. Legacy v1 FTS5 baseline (6.07 ms p95) is baseline-for-migration only.

OCR (native engine, measured independently): 244 images/s warm, p50 3.6 ms, p95 7.3 ms per image; 92.4% character accuracy on the
synthetic English corpus; exact recognition of two generated PNGs verified through ps_ocr::extract_image; CJK UNVERIFIED (no CJK
language pack installed). End-to-end screenshot indexing through a running UI session is not user-validated.

Tests at final commit: cargo test --workspace -> 129 Rust tests passed, 0 failed, 1 ignored (action smoke, separately verified:
real ShellExecute marker file); frontend npm test -> 143 passed / 9 files; legacy v1 suite unchanged at 85 passed / 1 ignored.
Visual QA: docs/qa/visual/VISUAL_AUDIT_APP.md (74 mock-state captures) + VISUAL_AUDIT_LAUNCHER.md (10 real captures) + real-app
captures under docs/qa/visual/app-captures/. Security: docs/security/V2_SECURITY_REVIEW.md with resolved-in-tree mapping.

Installable artifacts (unsigned):
- dist-installer/ProtonSearch.exe (17,917,952 bytes; SHA-256 57A0233740905CFE31D507283144F1C59190770DD19EEF8577EF522D43C4B82F)
- dist-installer/ProtonSearch_2.0.0-alpha.1_x64-setup.exe (4,935,678 bytes; SHA-256 76DD0FC38FE03478130F44786334F9A0772E580062C0023480E0935E3C841C5E)
- dist-installer/SHA256SUMS.txt + BUILD_INFO.txt; Authenticode status: NotSigned (do not distribute publicly without signing).

Remaining work for a GA release (unchanged): Authenticode signing + signed update manifest; content p95 <30 ms; OCR end-to-end
UI validation + CJK; clipboard source-app exclusions/TTL/DPAPI; signed installer + rollback drill; visual QA on real 100/125/200%
DPI, High Contrast and screen readers; optional agent Workflows panel finishing; legacy key rotation by the repository owner (S-01).
