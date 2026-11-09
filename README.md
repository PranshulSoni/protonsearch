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
| Document/code content search (PDF, DOCX, XLSX, text, Markdown, source) | Works when content indexing is enabled | crates/ps-content FTS5 + extractors (16 tests) |
| Legacy v1 migration (settings, 89,813 file rows, 14,609 document texts) | Works, source read-only and hash-verified | docs/implementation/GATE_LOG.md; migration tests |
| Desktop app (Home / Search & Indexes / Activity / Workflows / Preferences), onboarding, dark mode, hotkey capture | Works | real-app-*.png; 136 frontend tests |
| Optional Python agent sidecar (Pydantic AI, on-demand, approval-gated) | Sidecar + Rust client + shell commands work; Workflows panel integration in progress | crates/ps-agent (14 tests), agent-sidecar (18 tests) |
| Marketing/telemetry/silent downloads | None. Ever. | settings model has telemetry forced off |

Not yet wired in this alpha: OCR pipeline (Windows.Media.Ocr extraction is next), clipboard capture UI, applications/clipboard/browser/Git sources are implemented behind the engine but still being finalised, previews beyond metadata/snippets, quicklink/snippet editing, signed installer.

## Measured performance (honest, reproducible)

Engine-only, release build, this repo's benchmark harness (`cargo run -p ps-bench -- ...`):

| Corpus | warm p50 | warm p95 | warm p99 | build time | engine memory |
|---|---:|---:|---:|---:|---:|
| 10k realistic paths | 1.02 ms | 2.88 ms | 3.54 ms | 13.8 ms | 1.2 MB |
| 100k | 2.46 ms | 5.64 ms | 6.96 ms | 219 ms | 11.4 MB |
| 1M | 9.05 ms | 14.93 ms | 16.69 ms | 4.08 s | 109.8 MB |

Real-app end-to-end input-to-visible (debug Rust build, production JS, 150% DPI, 131,199 entries, n=4 queries): 40.6 ms first query, then 12.6 / 8.4 / 10.8 ms. Release-build and percentile numbers are in `docs/performance/BENCHMARKS.md`; the 100k p95 target of 5 ms is **not** met (5.64 ms) and is reported as such. No "world's fastest" claim is made.

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
