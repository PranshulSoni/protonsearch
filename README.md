<p align="center"><img src="assets/brand/protonsearch-mark.svg" width="96" height="96" alt="ProtonSearch logo" /></p>

# ProtonSearch

**Find anything on your PC from one keyboard shortcut — files, folders, documents, apps, actions, clipboard, browser history, Git activity and (optionally) text inside screenshots.**

ProtonSearch is a fast, local-first, Windows-native search and command centre. Press **Alt + Space**, type what you remember, press **Enter**. Nothing leaves your device unless you explicitly enable an external integration.

> **Project Phoenix — v2 alpha.** This branch is a ground-up rebuild of the v1 Win32/Slint application into a modular Rust engine + Tauri 2 / React desktop app. The legacy app is preserved in `protonsearch/` and is still buildable for comparison. v2 is an alpha: the sections below mark what is verified, what is partial and what is not implemented.

---

## What works today (verified on this machine)

Measured and captured with the real app over a real user profile (131,199 indexed entries):

| Area | State | Evidence |
|---|---|---|
| Global hotkey launcher (Alt+Space, pre-warmed, frameless) | Works | docs/qa/visual/app-captures/real-launcher-*.png |
| Filename/folder search, exact + prefix + fuzzy, grouped results | Works | 50 real matches for "arch"; 8.4–12.6 ms warm input-to-visible (debug build) |
| Calculator ("2+2", "15% of 340", unit conversion) | Works | engine tier 0; unit-tested in crates/ps-sources |
| Native actions (lock, sleep, shutdown, restart, empty bin, Settings, Task Manager, Terminal) | Results + engine wiring work; execution lands with ps-windows actions | crates/ps-core catalog + ps-sources actions |
| Document/code content search (PDF, DOCX, XLSX, text, Markdown, source) | Works when content indexing is enabled; retrieval speed target not met yet | crates/ps-content FTS5 + extractors (16 tests); docs/performance |
| Applications, browser history/bookmarks, recent files, Git, clipboard sources | Implemented and visible with real data | real-launcher-long-list.png; crates/ps-sources |
| Screenshot OCR search | Pipeline implemented (Windows.Media.Ocr); native engine measured; end-to-end UI run pending | crates/ps-ocr; docs/performance/BENCHMARKS.md |
| Legacy v1 migration (settings, 89,813 file rows, 14,609 document texts) | Works, source read-only and hash-verified | docs/implementation/GATE_LOG.md; migration tests |
| Desktop app (Home / Search & Indexes / Activity / Workflows / Preferences), onboarding, dark mode, hotkey capture | Works | real-app-*.png; 136 frontend tests |
| Optional Python agent sidecar (Pydantic AI, on-demand, approval-gated) | Sidecar + Rust client + shell commands work; Workflows panel integration in progress | crates/ps-agent (14 tests), agent-sidecar (18 tests) |
| Marketing/telemetry/silent downloads | None. Ever. | settings model has telemetry forced off |

Not yet wired in this alpha: OCR pipeline (Windows.Media.Ocr extraction is next), clipboard capture UI, applications/clipboard/browser/Git sources are implemented behind the engine but still being finalised, previews beyond metadata/snippets, quicklink/snippet editing, signed installer.

## Measured performance (honest, reproducible)

Engine-only, release build, this repo's benchmark harness (`cargo run -p ps-bench -- ...`):

| Corpus | warm p50 | warm p95 | build time | engine memory |
|---|---:|---:|---:|---:|
| 10k realistic paths | 0.070 ms | 0.584 ms | 15 ms | 1.2 MB |
| 100k | 0.600 ms | 2.195 ms | 220 ms | 12.4 MB |
| 1M | 2.713 ms | 7.583 ms | 3.5 s | 110 MB (~115 B/entry) |

Committed engine hash `3906836`, 22 fixed queries x 30 iterations (n=660 per corpus), release profile, this machine.
Targets from the brief: 100k warm p95 < 5 ms is **met** (2.195 ms); 1M warm p95 < 15 ms is **met** (7.583 ms).
Earlier pre-optimisation revision measured 5.64 / 14.93 ms and one contended run measured 28.4 ms — variance is disclosed in
`docs/performance/BENCHMARKS.md`. Content retrieval does **not** meet its 30 ms p95 target yet (v2 p95 61.1 ms; see the report).
Real-app input-to-visible samples on a debug build: 3.8–32.3 ms (n=4 exploratory; release-build percentile run pending).
No "world's fastest" claim is made; the report includes an explicit veto list of claims it does not support.

## Architecture

```text
ProtonSearch.exe (Tauri 2 host, Rust)
├── ps-core      engine facade: tiered query coordinator, ranking, settings, activity, migration
├── ps-index     compact in-memory filename index (arena + bloom/trigram prefilter + top-k)
├── ps-content   SQLite FTS5 content index + extractors (PDF/DOCX/XLSX/text/code)
├── ps-sources   applications, recent files, clipboard, browser history/bookmarks, Git, calculator
├── ps-windows   Win32 helpers (icons, shell open, clipboard, AppsFolder, windows)
├── ps-ocr       OCR extraction pipeline (Windows.Media.Ocr) — in progress
├── ps-agent     optional Python sidecar client (process lifecycle + JSONL protocol)
└── ps-ipc       versioned contract shared by all layers and mirrored in TypeScript
```

Design decisions and trade-offs are recorded in `docs/adr/`. The interface follows `protonsearch-v2-ui-reference.html` and `docs/design/` (tokens, screens, keyboard/accessibility).

## Brand assets (single source of truth)

Every ProtonSearch logo — exe/taskbar/tray icon, window icon, installer icon, installer header/sidebar artwork, WebView favicons and the in-app mark — is generated from `assets/brand/protonsearch-mark.svg`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-brand-assets.ps1
```

That script renders the app icon source, regenerates all platform icon sizes (`npx tauri icon`), creates the favicons and the NSIS installer artwork, and installs `favicon.ico`. Re-run it and rebuild to update the logo *everywhere*; no other file needs to be edited.

## Build and run (Windows)

Prerequisites: Rust 1.99+ (MSVC), Node 26+, WebView2 (preinstalled on Windows 11), Python 3.14 only if you want the optional agent sidecar.

```powershell
# frontend dependencies
cd apps/desktop; npm ci; cd ../..

# debug build + run with the Vite dev server
cd apps/desktop
npm run tauri dev

# production build (frontend + binary + NSIS installer)
cd apps/desktop
npm run build
cd src-tauri
cargo build --release
# optional installer: npx tauri build (see docs/release/BUILD.md)
```

Full reproducible instructions, troubleshooting (including this host's MSVC D8005/D8050 C-compiler quirk) and packaging notes live in `docs/release/BUILD.md`.

## Privacy and data

- All indexes and settings live in `%APPDATA%\protonsearch\v2`. A portable override exists via `PROTONSEARCH_DATA_DIR`.
- The legacy v1 folder is opened read-only for one-time migration and is never modified (hash-verified in tests).
- No telemetry, no accounts, no background network calls. Browser history/Git/clipboard/OCR are opt-in per source.
- The AI/agent layer is optional, starts only when you run a workflow, and every sensitive action requires explicit approval.

## Compatibility

Existing ProtonSearch / OmniSearch users: v2 detects the v1 database, imports file names and extracted document text, and skips credentials. See `docs/release/MIGRATION.md`. Rollback: keep the v1 data folder; it is untouched.

## Known limitations

See `docs/release/KNOWN_LIMITATIONS.md` and the truthful "complete / partial / not tested" report in `docs/implementation/ACCEPTANCE_REPORT.md`.

## Licence

MIT. Third-party licences: `docs/release/THIRD_PARTY_LICENSES.md`.