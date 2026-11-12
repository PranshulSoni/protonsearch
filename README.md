<p align="center"><img src="assets/brand/protonsearch-mark.svg" width="96" height="96" alt="ProtonSearch logo" /></p>

# ProtonSearch

**Find anything on your PC from one keyboard shortcut — files, folders, documents, apps, actions, clipboard, browser history, Git activity and (optionally) text inside screenshots.**

ProtonSearch is a fast, local-first, Windows-native search and command centre. Press **Alt + Space**, type what you remember, press **Enter**. Nothing leaves your device unless you explicitly enable an external integration.

> **Project Phoenix — v2 alpha.** This branch is a ground-up rebuild of the v1 Win32/Slint application into a modular Rust engine + Tauri 2 / React desktop app. The legacy app is preserved in `protonsearch/` and is still buildable for comparison. v2 is an alpha: the sections below mark what is verified, what is partial and what is not implemented.


## v10 release candidate (2026-11-10) — unsigned internal alpha

Source commit 4e9f589; artifacts in dist-installer/ (binary + NSIS setup + BUILD_INFO + SHA256SUMS).
SIGNED=false — do not distribute publicly.

### Verified on the RC
- Suites: cargo workspace 306 passed / 0 failed / 1 pre-existing ignored; frontend 18 files /
  241 tests passed; production build clean.
- P0-1 previews: 14 live formats including DOCX/XLSX/PNG+OCR/JSON/CSV/Unicode, honest unsupported
  (binary/scanned PDF), large-file truncation and stale-file error; matched-term highlight is the
  one open condition (BUG-G8-01).
- P0-2 clipboard images: real .NET clipboard image captured live (320x200 PNG), bounded thumbnail
  in IPC and island Images tab, restart persistence verified; UI paste-back path remains a
  verification item (harness-level paste-back passes).
- Launcher/island: Normal Launcher v2 (All/Files/Folders/Apps, quick destinations, previews) and
  the V9 Dynamic Island renderer (8 cards, dots, wheel/swipe, motion) captured on real Windows.

### Benchmarks (honest)
| Metric | Result | Target | Status |
|---|---:|---:|---|
| Content v2 retrieval p95 | 8.984 ms (official quiet, n=960) | 30 ms | MET |
| Release hotkey to window visible p95 | 14.289 ms | 50 ms | MET |
| Selection paint | 6 ms | 16 ms | MET |
| Input to first visible result p95 (debug) | 38.2 ms | 30 ms | NOT MET |
| Idle process-tree working set | 786.9 MiB | ~150 MB budget | NOT MET |

### Artifact hashes (unsigned)
- ProtonSearch.exe: 3010FBEED2AE5BB9EA0FC91960F0D8325BBF5BC589D9F3596B631BEFB6FE15EF (20,750,336 B)
- ProtonSearch_2.0.0-alpha.1_x64-setup.exe: F5D15C2D89CA2857C6DDB71B9289645A693B37C958AAA462E7B662CEBD6FBF8D (5,567,529 B)

### Honest blockers / recommendation
Unsigned installer (no public distribution), installer upgrade/uninstall drill not run, single
monitor only (mixed-DPI/High-Contrast/multi-monitor untested), idle memory and debug input latency
over budget, native media/Wi-Fi/Bluetooth post-alpha milestone 2, no live Pi provider credentials,
preview highlight open. Deployment-ready locally as an unsigned internal alpha: YES. Public release
signed off: NO. Full detail: docs/release/ACCEPTANCE_REPORT.md and
docs/implementation/V10_G8_VERIFICATION_ROUND2.md.

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
| Desktop app (Home / Search & Indexes / Activity / Workflows / Preferences), onboarding, dark mode, hotkey capture | Works | real-app-*.png; 241 frontend tests |
| Pi agent workflows (on demand) | Pi installation checks, supervised runs and durable task history | crates/ps-agent (14 tests), agent-sidecar (18 tests) |
| Marketing/telemetry/silent downloads | None. Ever. | settings model has telemetry forced off |

Verified in the v10 release candidate (source 4e9f589, unsigned internal alpha, 2026-11-10): real previews for TXT/code/Markdown/JSON/CSV/PDF/DOCX/XLSX/images/OCR with explicit unsupported and stale-file fallbacks; clipboard text+image history with island thumbnails and restart persistence; V9 Dynamic Island with 8 cards/dots/motion; two-launcher settings/placement/card manager; agent task APIs with read-only Pi gate. Still open: matched-term preview highlight, island preview hookup, native Media/Wi-Fi/Bluetooth (post-alpha milestone 2), live Pi provider verification, signed installer/updater, mixed-DPI/High-Contrast/multi-monitor evidence.

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
├── ps-agent     Pi process supervision and durable task history
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

Prerequisites: Rust 1.99+ (MSVC), Node 26+, WebView2 (preinstalled on Windows 11). Pi workflows require Pi and its Node.js prerequisite.

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