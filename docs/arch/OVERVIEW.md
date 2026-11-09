# ProtonSearch v2 — Architecture Overview

Status: Gate 1 snapshot (2026-11-09). Owner: L0. Details: ADR-0001..0006.

## Process model

```text
ProtonSearch.exe (Tauri host, Rust)
├── ps-core Engine (in-process, single owner of indexes)
│   ├── ps-index    filename arena + scanner + persistence
│   ├── ps-content  SQLite FTS5 documents + extractors
│   ├── ps-ocr      OCR extraction pipeline (Windows.Media.Ocr)
│   ├── ps-sources  apps / recent / clipboard / browser / git / calc / workflows
│   └── ps-windows  Win32 helpers (icons, shell open, clipboard, foreground window)
├── windows: launcher (pre-warmed, frameless, hidden) + main (desktop app)
├── tray + global hotkey (Alt+Space default)
└── optional agent sidecar (Python, on-demand, JSONL over stdio) — Gate 6
```

## Query path

1. Global hotkey -> Rust handler shows/focuses the pre-created launcher window (no window creation, no WebView boot).
2. Keystroke -> `search(SearchRequest{requestId, query, scope, filters})` -> Engine.
3. Engine parses prefixes/filters, then runs Tier 0 (exact/prefix filename via memory scan, apps, recents, calculator, actions, quicklinks) and Tier 1 (fuzzy filename + FTS5 content) under a time budget; Tier 2 (OCR text, optional semantic) only when enabled.
4. Ranker merges deterministically, returns top-k with per-tier timings and status. UI drops stale ids.
5. `open_result(ActionSpec)` executes a structured action through ps-windows; the engine records activity if enabled.

## Storage

- `%APPDATA%\\protonsearch\\v2\\index.db` — SQLite WAL: content FTS, OCR FTS, activity, workflows, migration state.
- `%APPDATA%\\protonsearch\\v2\\settings.json` — settings (atomic write).
- `%APPDATA%\\protonsearch\\v2\\filename-index.bin` — versioned compact filename index.
- `%APPDATA%\\protonsearch\\v2\\cache\\icons\\*` — extracted app icons (PNG).
- Legacy `%APPDATA%\\protonsearch\\file_index.db` is opened read-only for migration; never mutated.

## Failure containment

- Filename search has no dependency on content/OCR/AI/network/Python; a failure in any of those degrades results, never the launcher.
- Engine open is fast and non-blocking; scanning starts on a background thread. Search works against a partial index during the first scan.
- Corrupt index/DB: validated on load; rebuild paths exposed via IndexCommand; no panic across the IPC boundary.
- WebView crash: the Rust process survives; windows can be recreated (resilience check in Gate 7).
