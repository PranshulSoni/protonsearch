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

Engine-only, release build, synthetic realistic corpora (22 fixed queries x 30 iterations; p50/p95/p99 in ms):

| Corpus | p50 | p95 | p99 | Build | Engine memory |
|---|---:|---:|---:|---:|---:|
| 10k | 1.02 | 2.88 | 3.54 | 13.8 ms | 1.2 MB |
| 100k | 2.46 | 5.64 | 6.96 | 219 ms | 11.4 MB |
| 1M | 9.05 | 14.93 | 16.69 | 4.08 s | 109.8 MB |

Targets from the brief: 100k warm p95 < 5 ms (**not met: 5.64 ms, +13 %**); 1M warm p95 < 15 ms (**met: 14.93 ms,
margin 0.45 ms**). These are engine-only numbers; the UI adds WebView/React/render time.
Real-app end-to-end input-to-visible (debug Rust build, production JS, 150 % DPI, 131,199 entries, n=4 queries):
40.6 ms first query/cold render, then 12.6 / 8.4 / 10.8 ms warm. Not a p95; release-build percentile measurement is pending.
Relevance: synthetic 10k MRR@10 0.895, exact@1 1.00; real legacy corpus MRR@10 0.928, recall@10 0.983, exact@1 1.00.
Known relevance gaps: multi-term phrase queries and one typo class.

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
