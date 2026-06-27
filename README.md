# Omnisearch (`opensearch-os`)

**Omnisearch** is a premium, high-performance, native Windows launcher, command palette, and system-wide search tool written in Rust. By utilizing raw Win32 APIs and direct GDI graphics rendering (avoiding heavy web frameworks or Electron wrapper overhead), it offers a lightweight, sub-millisecond responsive search experience.

---

## What the Project Is

Omnisearch is a centralized control center for Windows. It acts as both an application launcher and a deep indexing search engine that lets you find, execute, and inspect anything across your system: files, bookmarks, clipboard items, browser history, Git commit logs, code TODOs, quick system power commands, and OpenAI-powered conversational chat/agent tasks.

---

## Core Capabilities & Features

### 1. Modern UI & Premium Aesthetics (Win32 & GDI)
* **Tailored Interface**: Features a modern, compact window layout (720px width, 76px row height) designed around the `Segoe UI Variable` typography.
* **Charcoal Styling**: Features a clean, solid, opaque charcoal-colored window (100% opacity, no heavy Acrylic blur rendering lag).
* **Flat List Design**: A flat-list results interface matching modern design aesthetics, with top-level filter pills, statistics sub-headers ("Results" and "Best matches first"), selection borders, and left-aligned vertical blue accent indicators.
* **Lag-Free Icon Resolving**: Resolves `.lnk` targets via `IShellLinkW` (stripping shortcut arrow watermarks) and fetches high-res icons asynchronously in background worker threads, passing them to the paint loop via Win32 message passing (`WM_ICON_LOADED`) to maintain a constant 60 FPS.

### 2. Deep Local File & Content Search
* **Throttled Crawler Indexer**: Runs a background crawler (`indexer.rs`) that indexes files in `Desktop`, `Documents`, and `Downloads`.
* **Document Text Extraction**: Extracts text content from PDF (using `pdf-extract`) and Microsoft Word DOCX (using `docx-lite`) files, caching text up to 50KB to preserve a lightweight database footprint.
* **Source Code Indexing**: Indexes source files with extensions (`.rs`, `.py`, `.js`, `.ts`, `.c`, `.cpp`, `.h`, `.hpp`, `.cs`, `.go`, `.java`, `.kt`, `.sh`, `.bat`, `.ps1`, `.yaml`, `.yml`, `.toml`, `.ini`, `.sql`, `.xml`).
* **SQLite FTS5**: Leverages SQLite's FTS5 extension (`files_fts` table) to perform sub-millisecond lexical full-text queries.

### 3. Developer & Git Repository Scanner
* **Fast Git Discovery**: Crawls system folders metadata-first (ignoring `node_modules`, `target`, etc.) to locate Git repositories in under **186ms** without recursive loops.
* **Commits & Branches**: Queries the `HEAD` branch and the last 100 commits via Git CLI tools.
* **TODO / FIXME Tasks**: Scans codebase comments for task tags. Pressing `Enter` deep-links directly into VS Code at the exact file and line using `code -g <file>:<line>`.

### 4. Multi-Browser Profiles Scanner
* **Cross-Browser Bookmarks & History**: Scans Chromium profiles (Chrome, Edge, Brave) and Gecko profiles (Firefox). Pre-copies profile database locks before parsing to prevent browse conflict locks.
* **Firefox Places Support**: Direct SQLite querying of Firefox's `places.sqlite` structure.

### 5. In-Process Tools & Quick System Actions
* **Math Parser / Calculator**: High-speed, recursive descent math parser (evaluates formulas like `2+2`, `15% of 340`, `sqrt(9)*4`, etc.) and copies results to the clipboard.
* **Quick System Actions**: Lock screen, sleep, shutdown, restart, volume control (percentage and toggle mute), hosts file editing, and recycling bin controls.
* **Recent Files Tracker**: Parses `%APPDATA%\Microsoft\Windows\Recent` to display recently used files with appropriate file-type icons.

### 6. OpenAI Chat & Agent Runs
* **Conversational AI**: Native OpenAI-compatible API connector with markdown output rendering and chat history persistence.
* **Agent Gateway**: Integrates with Hermes gateway daemon streaming runs with approve/deny dialogs.

---

## Search Prefixes & Scopes

To avoid database congestion, Omnisearch utilizes specific search prefixes:

> [!NOTE]
> Prefix-based queries bypass the mocked search results layout, querying the active indexing databases directly and displaying results in the original card/category layout.

| Category | Prefix | Empty State Placeholder | Badge | Description |
|---|---|---|---|---|
| **Bookmarks** | `bookmarks: <query>` | `📁 Browser Bookmarks` | `BOOKMARK` | Search browser favorites |
| **History** | `history: <query>` | `📁 Browser History` | `HISTORY` | Search browser history URLs |
| **Commits** | `commits: <query>` | `📁 Git Commits` | `COMMIT` | Search recent repository commits |
| **TODOs** | `todos: <query>` | `📁 Git TODOs` | `TODO` | Search code tasks / comments |
| **Local Files** | `file: <query>` | `📁 Local Files` | `FILE` | Search documents (PDF, DOCX, TXT) |
| **Source Code** | `code: <query>` | `📁 Source Code` | `CODE` | Search code files |
| **Clipboard** | `clip:` / `clipboard:` | `📁 Clipboard History` | `CLIP` | Search clipboard history |
| **Quicklinks** | `ql:` / `quicklink:` | `📁 Quicklinks` | `QL` | Browse/search web shortcuts |
| **Snippets** | `snip:` / `snippet:` | `📁 Snippets` | `SNIP` | Browse/search text snippets |
| **Windows** | `switch:` / `window:` | `📁 Active Windows` | `WIN` | Search running windows/processes |

---

## Codebase Architecture

The project is structured under the `opensearch-os/` subdirectory:

* **`src/main.rs`**: Core window management, GDI-based double-buffered rendering, input handling, and event loop.
* **`src/indexer.rs`**: Handles background threads for crawling, Chromium/Firefox profile database extraction, and document content parsing (PDF/Word).
* **`src/search.rs`**: Core ranking, prefix matching logic, and SQLite database connector configuring WAL (Write-Ahead Logging) and thread concurrency settings.
* **`build.rs`**: Embeds icons, manifests, and compilation properties for the executable.

---

## Build & Run Instructions

### Prerequisites
* Rust compiler toolchain (Stable channel target `x86_64-pc-windows-msvc`).
* SQLite runtime dependencies.

### Development Build
To compile the launcher in debug mode:
```powershell
cargo build
```

### Production Build
To compile a fully optimized release target:
```powershell
cargo build --release
```

### Build Constraints & Clean Compilation
> [!IMPORTANT]
> If the launcher is running in the background, file locks will cause compile errors (`Access is denied / os error 5`). Always terminate any running instance of the application before building:

```powershell
taskkill /F /IM opensearch-os.exe
```