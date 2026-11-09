# Gate 0 — Executive Report

**Project:** ProtonSearch v2 "Project Phoenix" · **Branch:** v2-phoenix @ de0ef19
(== tag v1.1.1-v2-baseline) · **Auditor:** L1-D (rel-recon) · **Date:** 2026-11-09

## Verdict

**Gate 0 passes for planning purposes; it does not certify that the legacy app works end to
end.** The legacy crate builds and its 85-test suite passes on this host (85 passed / 0 failed /
1 ignored, exit 0), the real 149 MB index is intact and forensically documented, and every
feature row in the parity matrix now has file:line evidence. The legacy GUI was not launched
(workspace rule), so all runtime/UI claims remain **UNVERIFIED**.

## What works (verified by code + tests and/or real data)

- Global hotkey launcher, custom Win32 UI, installed-app search (incl. UWP/AppsFolder), 89,813-row
  file index, 14,609-document FTS5 content search, calculator, native commands, browser history/
  bookmarks (4,523 rows), recent files, git activity (15 repos / 516 commits / 132 TODOs),
  settings catalog (238 rows), quicklinks/snippets, window management, focus categories, notes
  (code only), open-window search, OCR code path (WinRT).
- Engineering strengths worth preserving: subprocess-isolated PDF/DOCX extraction with a 30 s
  watchdog, WAL + busy_timeout everywhere, in-memory filename index, CREATE_NO_WINDOW on all
  shell-outs, browser DB copy with RAII cleanup.

## What is broken / not proven

- **Clipboard history is empty** in the real DB (0 rows) and timeline events are empty (0 rows)
  despite code; end-to-end clipboard behaviour is UNVERIFIED. is_indexing=1 is a stale unclean-
  shutdown marker; 23.8 MB of WAL is un-checkpointed.
- **Steam games** only scan 3 hardcoded steamapps paths (no libraryfolders.vdf); **voice input**
  is not implemented (Cargo feature only) -> CUT candidate.
- **Updater is insecure**: fetches mutable update.json, runs an unsigned/unverified EXE.
- **Wi-Fi password command is broken** (literal PROFILE placeholder) yet advertised in the
  settings catalog.
- **Version drift**: crate 1.1.0 vs tag v1.1.1 vs update.json 1.1.0 vs winget OmniSearch 1.0.0.
- Dead semantic/vector code (compiler warnings) should not be ported.

## Top risks (details: docs/security/SECURITY_RISK_REGISTER.md)

1. **CRITICAL S-01** — a full AI API key was committed to public git history (masked
   sk-Hr...***, 67 chars). **Rotate/revoke now**; history rewrite is secondary.
2. **CRITICAL S-02** — self-updater executes unverified downloads (supply-chain RCE). v2 must
   require signed manifest + SHA-256 + Authenticode signer pin.
3. **HIGH S-03/S-04** — static Hermes loopback key with PC-control tools enabled; AI keys stored
   plaintext and passed on child argv.
4. **HIGH S-05/S-06** — arbitrary powershell execution from the search box; prompt-injection +
   SSRF via "summarize URL" into a tool-enabled agent.
5. **HIGH S-07** — clipboard capture/import persists secrets and screenshots without opt-in.
6. **MEDIUM** — focus-mode wildcard process killing, predictable temp files, browser DB copies in
   %TEMP%, git-from-PATH, 215+ raw unsafe blocks, legacy uninstaller deleting user data.

## Migration constraints (v2 must honour)

- v2 owns %APPDATA%\protonsearch\v2\; open the legacy DB **read-only and WAL-aware**; never
  mutate it, never run the legacy uninstaller (it deletes {userappdata}\protonsearch).
- Import: files/metadata (89,813), content docs (14,609), browser items (4,523), memory events
  (5,286), quicklinks (4), snippets (1), ignored folders (2), focus (1). Do **not** import
  settings_catalog_fts (regenerated), logs, or 0-byte index.db.
- ai_settings has only endpoint/model at baseline — no key to migrate. Any key found on other
  machines must require explicit consent + OS secret storage; never log values.
- Clipboard/timeline are empty at baseline; migration must handle empty sources gracefully.
- Snapshot + SHA-256 rollback before import; dry-run report first; idempotent second run.

## Gate-0 exit criteria

| Criterion | Status |
|---|---|
| Baseline commit/tag, legacy unmodified | MET |
| Legacy builds + tests recorded (85/0/1, exit 0) | MET |
| DB forensic dump + user-data inventory | MET |
| Feature parity matrix verified per row (+39 rows, appendix) | MET |
| Security risk register with evidence + remediation | MET |
| Research with cited URLs + licence notes | MET |
| CI skeleton for v2 workspace | MET (YAML validated; pipeline not yet green) |
| Migration fixture + contract test | MET for fixture/read-only invariants; **import test BLOCKED** on ps_core::migration API |
| Runtime smoke of legacy GUI | **NOT DONE / UNVERIFIED** by rule |

## What v2 must not regress

Sub-10 ms warm filename search; FTS content search + open-at-match; app/UWP launch + icons;
hotkey reliability; prefix/scope semantics users already know; calculator; browser/git/clipboard
sources; settings-catalog search; per-user install with no elevation; local-first privacy (no
telemetry); and the good safety patterns listed above.

*Evidence files: docs/audit/GATE0_BASELINE.md, docs/audit/LEGACY_ARCHITECTURE.md,
docs/security/SECURITY_RISK_REGISTER.md, docs/research/RESEARCH.md,
docs/implementation/FEATURE_PARITY_MATRIX.md, .agents/notes/legacy-tests-clean.log.*
