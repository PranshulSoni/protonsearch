# Omnisearch (`opensearch-os`)

Omnisearch is a native Windows deep-search launcher built in Rust. It is designed for fast keyboard-first retrieval across files, folders, document content, OCR text, browser data, clipboard history, Git activity, Windows settings, apps, and AI/agent chat history.

This branch (`lean-build`) focuses on the core search product: find the right thing quickly, rank it predictably, and keep the UI lightweight enough for daily use.

## What It Does

Omnisearch opens with `Alt+Space` and gives one command/search surface for local Windows data:

- Launch apps and Windows settings.
- Search files and folders by name.
- Search PDF, DOCX, text, source-code, and OCR-indexed image content.
- Search browser bookmarks and history across Chrome, Edge, Brave, and Firefox profiles.
- Search clipboard history, including saved clipboard images and pinned entries.
- Search Git commits, branches, repositories, and TODO/FIXME comments.
- Search AI chats, agent runs, and persistent agents.
- Run selected Windows actions such as volume control, DNS flush, lock, sleep, restart Explorer, and settings shortcuts.

## Resume-Friendly Highlights

- Built a native Windows launcher in Rust using Win32 APIs and custom GDI rendering instead of Electron or a browser shell.
- Implemented a SQLite + FTS5 indexing pipeline for local files, document text, source code, image OCR, browser data, clipboard history, and Git metadata.
- Added hybrid search behavior that combines Everything IPC metadata search, SQLite fallback queries, FTS5 content search, curated source filters, and custom ranking.
- Designed a 720px compact launcher UI with a 64px search bar, 76px result rows, dynamic result filters, horizontal filter scrolling, high-resolution icon rendering, keyboard navigation, mouse hover states, and dark-mode visual polish.
- Built asynchronous search and icon-loading flows with Win32 message passing (`WM_SEARCH_RESULTS`, `WM_ICON_LOADED`) and a 150ms keystroke debounce so expensive work stays out of the paint loop.
- Added OCR indexing for images/screenshots using Windows OCR APIs with image-size protection to avoid hangs or out-of-memory failures.
- Integrated browser indexing for Chromium and Firefox profiles, including a capped import of the latest 5,000 browser-history URLs.
- Added clipboard history with image capture, pinning, multi-select support, and retention capped to the latest 500 non-pinned entries.
- Added Git indexing with a 15-minute background refresh for repositories, commits, branches, and TODO/FIXME task discovery.
- Added AI chat and Hermes agent integrations with SQLite chat persistence, streaming run progress, and approval prompts for tool execution.
- Maintained local-first storage under `%APPDATA%\opensearch-os` with SQLite as the primary persistence layer.

## Performance And Optimization

The app is built as a native Win32 process instead of Electron, so the baseline cost stays close to a small desktop utility rather than a browser runtime.

| Area | Current implementation |
|---|---|
| UI runtime | Native Rust + Win32 + GDI, no Electron/webview runtime |
| Search input | 150ms debounce per query to avoid searching on every keystroke |
| Paint loop | Search, indexing, AI, and icon loading run outside the paint path |
| Result bounds | Dedicated source searches usually return 50-100 rows before final ranking/truncation |
| Browser indexing | Imports recent browser history with capped profile reads, including a 5,000 URL cap in the browser indexer |
| Clipboard retention | Keeps pinned items and caps non-pinned clipboard history at 500 entries |
| Document extraction | Caps extracted text at 50KB per file to keep SQLite/FTS compact |
| Background refresh | Browser index refreshes every 10 minutes; Git index refreshes every 15 minutes |
| Observed memory | Current local long-running dev session: ~27MB private memory, ~74MB working set |
| Optimization target | 15MB idle private memory and ~30MB private memory during normal active search |

For resume wording, the defensible version is:

> Built a native Rust/Win32 deep-search launcher with SQLite FTS5, 150ms debounced query execution, capped background indexing, 500-entry clipboard retention, 5,000-entry browser-history import caps, and a measured ~27MB private-memory footprint in local testing.

## Core Search Sources

| Source | What is indexed/searched | Notes |
|---|---|---|
| Apps | Installed Win32/UWP apps and settings entries | Uses shell/app enumeration and async icon lookup |
| Files | Desktop, Downloads, Pictures, Documents, Program Files, and fixed drives | Metadata-first scan with ignored heavy folders |
| Content | PDF, DOCX, text, Markdown, and source files | Extracted content is capped at 50KB per file for DB size and speed |
| Images/OCR | Screenshots and image files | OCR text is stored in FTS for text search |
| Browser | Chrome, Edge, Brave, and Firefox bookmarks/history | Browser indexer refreshes every 10 minutes |
| Clipboard | Text and image clipboard history | Pinned items are preserved; non-pinned retention is capped |
| Git | Repositories, branches, commits, TODO/FIXME comments | Git scanner refreshes every 15 minutes |
| AI | AI chats, agent chats, persistent agents | Stored locally in SQLite |
| Windows | Settings, control-panel actions, windows/processes, quick actions | Keyboard-first execution |

## Search Prefixes

General search shows the curated deep-search result set. Prefixes let you jump directly into a specific source.

| Prefix | Purpose |
|---|---|
| `file:` | Search indexed local files and document content |
| `code:` | Search indexed source-code files and code content |
| `img:` / `screenshots:` | Search image files and OCR text |
| `bookmarks:` | Search browser bookmarks |
| `history:` | Search browser history |
| `clip:` / `clipboard:` | Search clipboard history |
| `commits:` | Search Git commits |
| `todos:` | Search TODO/FIXME code tasks |
| `agents:` | Browse persistent AI agents |
| `agentchats:` | Browse agent chat history |
| `chats:` | Browse AI chat history |
| `switch:` / `window:` | Search running windows/processes |
| `ql:` / `quicklink:` | Search custom web shortcuts |
| `snip:` / `snippet:` | Search reusable snippets |

## UI And Interaction

- Global hotkey: `Alt+Space`.
- Centered launcher window with fast expand/collapse behavior.
- Dark-mode UI optimized for repeated daily use.
- Dynamic filter row with real result counts for `All`, `Files`, `Content`, `Images`, `Code`, `Settings`, and `Commands`.
- Horizontal filter scrolling for narrow result sets.
- Search result badges are de-emphasized so they do not dominate the result title.
- Agent chat uses a separate chat input rather than hijacking the main search field.
- Escape behavior routes agent chat back to agent history first, then back to the main launcher flow.
- Clipboard image results can render thumbnails and support pinning/multi-select flows.

## Architecture

The Rust app lives in `opensearch-os/`.

| File | Responsibility |
|---|---|
| `src/main.rs` | Win32 window lifecycle, hotkeys, input handling, GDI rendering, result UI, clipboard UI, AI panel, and launcher event loop |
| `src/search.rs` | Query routing, ranking, prefix search, SQLite schema, FTS search, browser/clipboard/Git/AI result assembly |
| `src/indexer.rs` | File crawling, document extraction, OCR extraction, SQLite/FTS population, folder watching |
| `src/browser_indexer.rs` | Chromium/Firefox bookmark and history indexing |
| `src/git_indexer.rs` | Repository discovery, commit indexing, branch indexing, TODO/FIXME scanning |
| `src/ai.rs` | OpenAI-compatible chat, Hermes agent config, streaming runs, approval handling |
| `src/launcher.rs` | Windows action execution and system integrations |
| `src/voice.rs` | Windows speech recognition input |
| `src/markdown.rs` | Lightweight markdown parsing/render support for AI output |
| `src/uninstall.rs` | Native uninstaller |

## Storage

Runtime data is stored under:

```text
%APPDATA%\opensearch-os
```

Main storage uses SQLite with tables for indexed files, FTS content, browser items, clipboard history, chats, agents, and settings. The app keeps expensive work disk-backed and capped where needed instead of keeping everything in RAM.

## Build And Run

### Prerequisites

- Windows
- Rust stable toolchain for `x86_64-pc-windows-msvc`
- MSVC build tools

### Build

```powershell
cd opensearch-os
cargo build
```

### Release Build

```powershell
cd opensearch-os
cargo build --release
```

### Run

```powershell
.\target\release\opensearch-os.exe
```

If the app is already running, kill it before rebuilding because Windows can lock the executable:

```powershell
taskkill /F /IM opensearch-os.exe
```

### Test

Use a single test thread because some tests share temporary SQLite/appdata state:

```powershell
cd opensearch-os
cargo test -- --test-threads=1
```

## Current Product Focus

The current branch is about making deep search feel trustworthy:

- Faster real search results instead of hardcoded demo rows.
- Better source-specific ranking.
- Cleaner Raycast-style result presentation.
- Correct icons for apps, settings, folders, files, images, commands, and AI results.
- Search filters driven by actual result counts.
- Clipboard image paste, pinning, and multi-select reliability.
- Agent chat/history flows that do not trap the launcher in chat mode.

## Tech Stack

- Rust 2021
- Win32 API via the `windows` crate
- GDI custom rendering
- SQLite via `rusqlite` with bundled SQLite
- SQLite FTS5
- Everything IPC integration
- Windows OCR and speech APIs
- `pdf-extract` for PDF text
- `docx-lite` for DOCX text
- `notify` and `walkdir` for indexing
- `ureq` for AI/Hermes HTTP calls
