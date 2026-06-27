# Project-Raycast: Premium Native Windows Launcher (`opensearch-os`)

`opensearch-os` is a premium, high-performance, native Windows launcher and system-wide search tool written in Rust utilizing the native Win32 API directly. It is designed to be lightweight, lag-free, and highly visual, providing instant access to applications, files, bookmarks, git repositories, and system commands.

---

## Key Features & Achievements

### 1. Modern UI & Premium Aesthetics (Win32 & GDI)
* **Tailored Form Factor**: Compact layout featuring an enlarged window (720px width), results (76px height per row), and typography centered around the modern `Segoe UI Variable` font family.
* **Sleek Backdrop**: Opaque, charcoal-colored window backdrop (100% opacity, no heavy Acrylic backdrop blur overhead) for a clean, distraction-free aesthetic.
* **Modern Mock Search Layout**: A modern layout showing filter pills at the top, a results sub-header ("Results" & "Best matches first"), flat results formatting, accent selection borders, and left-aligned vertical indicator bars.

### 2. High-Performance Application Launching & Icon Loading
* **UWP & Windows Store Support**: Directly enumerates modern Windows Store apps (Calculator, Settings, Camera, etc.) via COM `IShellItem` and triggers launches using `shell:AppsFolder\<AppID>`.
* **Watermark-free Icons**: Resolves target paths of `.lnk` shortcuts using `IShellLinkW` and extracts high-resolution icons using clean PIDLs to strip the shortcut arrow overlay.
* **Async Icon Loading**: Spawns asynchronous background worker threads to load app and file-type icons, passing them to the main thread via custom Windows message passing (`WM_ICON_LOADED`) to keep the search UI entirely lag-free.
* **Favicon Integration**: Uses Google's official favicon service for clean web search/URL results.

### 3. In-Process Tools & Quick System Actions
* **Recursive Math Parser**: High-speed, in-process calculator. Evaluates formulas (e.g., `2+2`, `15% of 340`, `sqrt(9)*4`, etc.) instantly and copies the output to the clipboard on Enter.
* **Power & Quick Actions**: Instant execution of system power controls: `lock`, `sleep`, `shutdown`, `restart`, and recycle bin empty (`empty recycle bin`/`empty trash`).
* **Recent Documents**: Parses `%APPDATA%\Microsoft\Windows\Recent` to display recently used files with appropriate file-type icons.

### 4. Local Filesystem & Document Content Search
* **Throttled Indexer (`indexer.rs`)**: Runs a throttled background indexer that monitors and crawls directories like `Desktop`, `Documents`, and `Downloads`.
* **Document Text Extraction**: Extracts and parses textual content from PDF files (using `pdf-extract`) and Microsoft Word DOCX files (using `docx-lite`), truncating them to 50KB to maintain a lightweight SQLite index.
* **Source Code Indexing**: Indexes source files with extensions (`.rs`, `.py`, `.js`, `.ts`, `.c`, `.cpp`, `.h`, `.hpp`, `.cs`, `.go`, `.java`, `.kt`, `.sh`, `.bat`, `.ps1`, `.yaml`, `.yml`, `.toml`, `.ini`, `.sql`, `.xml`).
* **SQLite FTS5**: Leverages SQLite's FTS5 extension (`files_fts` table) to perform sub-millisecond full-text queries across document contents and code metadata.

### 5. Multi-Browser Profiles Indexer (Bookmarks & History)
* **Profile Scanner**: Scans user profiles for Chromium browsers (Chrome, Edge, Brave) and Gecko-based browsers (Firefox), creating temporary lock-free database copies before reading to prevent profile lock conflicts.
* **Firefox Places Support**: Direct SQLite querying of Firefox's `places.sqlite` structure to pull bookmarks and search histories.

### 6. Developer & Git Repository Integration
* **Superfast Git Discovery**: Crawls system folders metadata-first (ignoring `node_modules`, `target`, etc.) to discover Git repositories in under **186ms** without recursive loops.
* **Commits & Branches**: Directly queries the `HEAD` branch and the last 100 commits via Git CLI tools.
* **Code Task Scanner**: Scans repository comments for `TODO` / `FIXME` tasks. Selecting a task and pressing `Enter` deep-links directly into VS Code at the exact file and line using `code -g <file>:<line>`.

---

## Search Prefixes & Scopes

To minimize read overhead and database congestion, `opensearch-os` supports dedicated search scopes using query prefixes. 

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

---

## Codebase Architecture

The project is structured under the `opensearch-os/` subdirectory:

* **`src/main.rs`**: Core entry point containing the Win32 window initialization, Windows message loop (`WndProc`), GDI-based double-buffered graphics rendering, keyboard/mouse input handling, and active view state management.
* **`src/indexer.rs`**: Handles background threads for crawling, Chromium/Firefox profile database extraction, and document content parsing (PDF/Word).
* **`src/search.rs`**: Core ranking, prefix matching logic, and SQLite database connector configuring WAL (Write-Ahead Logging) and thread concurrency settings.
* **`build.rs`**: Embeds icons, manifests, and compilation properties for the executable.

---

## Build & Run Instructions

### Prerequisites
* Rust compiler toolchain (Stable channel target `x86_64-pc-windows-msvc`).
* SQLite runtime dependencies (configured automatically during `rusqlite` compilation).

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