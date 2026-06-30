# OmniSearch

OmniSearch is a native Windows deep-search launcher built in Rust. It is designed for fast keyboard-first retrieval across files, folders, document content, OCR text, browser data, clipboard history, Git activity, Windows settings, apps, and AI/agent chat history.

This branch (`lean-build`) focuses on the core search product: find the right thing quickly, rank it predictably, and keep the UI lightweight enough for daily use.

## What It Does

OmniSearch opens with `Alt+Space` and gives one command/search surface for local Windows data:

- **Apps & Windows/UWP Launching**: Direct COM enumeration of modern Windows Store apps (Calculator, Settings, Camera, etc.) via `IShellItem` and launching via `shell:AppsFolder\<AppID>`.
- **In-Process Calculator**: High-speed math parser (evaluates formulas like `2+2`, `15% of 340`, `sqrt(9)*4`, etc.) and copies the result to the clipboard on Enter.
- **Recent Files & PIDLs**: Displays recently used files (parsing `%APPDATA%\Microsoft\Windows\Recent`) with clean high-resolution icons, resolving shortcuts via `IShellLinkW` and stripping arrow watermarks.
- **Deep Code & Document Search**: Fast full-text search across plain text, source code files, Word DOCX, and PDF content (truncated to 50KB) powered by SQLite FTS5.
- **Browser Bookmarks & History**: Searches browser data across Chromium (Chrome, Edge, Brave) and Gecko (Firefox) user profiles, safe-copying databases before querying.
- **Clipboard History**: Tracks text/image clipboard entries with image capture, pinning, and multi-select support.
- **Git Repository Search**: Metadata-first repository discovery, HEAD branches/commits search, and codebase TODO/FIXME comment scanning. Hitting Enter on a TODO launches VS Code directly at the exact file and line number (`code -g <file>:<line>`).
- **Quick System Actions**: Lock, sleep, restart, shutdown, and empty recycle bin/trash.
- **AI Chat & Hermes Agents**: Persistent local AI chats, streaming response progress, and tool execution approval flows.

## Resume-Friendly Highlights

- Built a native Windows launcher in Rust using Win32 APIs and custom GDI rendering instead of Electron or a browser shell.
- Implemented a SQLite + FTS5 indexing pipeline for local files, document text, source code, browser data, clipboard history, and Git metadata.
- Designed a custom UWP/Windows Store application resolver using COM `IShellItem` to launch native Windows applications seamlessly.
- Built a shell shortcut resolver utilizing `IShellLinkW` and Windows PIDLs to extract clean, high-resolution application icons without shortcut arrow watermarks.
- Created an asynchronous icon-loading thread pool communicating with GDI rendering via Windows message passing (`WM_ICON_LOADED`) to eliminate UI lag.
- Implemented a high-speed, in-process math parser (recursive descent) evaluating algebraic and percentage operations.
- Added hybrid search behavior that combines SQLite queries, FTS5 content search, and custom source-specific ranking.
- Designed a 720px compact launcher UI with Segoe UI Variable fonts, dynamic result filters, keyboard navigation, and dark-mode visual polish.
- Added OCR indexing for images/screenshots using Windows OCR APIs with image-size protection.
- Integrated browser indexing for Chromium and Firefox profiles, safe-copying locked databases and capping imports at 5,000 URLs.
- Added clipboard history with image capture, pinning, multi-select, and a 500-entry retention limit.
- Added Git indexing with TODO/FIXME comments scanning and seamless launching of VS Code at exact line numbers (`code -g <file>:<line>`).
- Added AI chat and Hermes agent integrations with SQLite chat persistence and tool execution approval prompts.
- Maintained local-first storage under `%APPDATA%\opensearch-os` using SQLite.

## Performance And Optimization

OmniSearch is built as a native Win32 process instead of Electron, so the baseline cost stays close to a small desktop utility rather than a browser runtime.

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
