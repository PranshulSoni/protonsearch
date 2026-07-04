# Workspace Rules & Project Progress Summary

This file serves as a guide for AI agents working in this repository. It documents everything that has been built so far, the repository architecture, and the behavioral rules/constraints requested by the user.

---

## 1. Project Achievements & Features Built

We have built a premium, high-performance, native Windows launcher and search tool (**`protonsearch`**, formerly `omnisearch`) written in Rust (Win32 API). It features:

### UI & Aesthetics
* **Window Size & Scale**: Enlarged window (720px width), results (76px height), and fonts (Segoe UI Variable) for a modern, high-contrast feel.
* **Backdrop Style**: Opaque, charcoal-colored window (no Acrylic backdrop blur, 100% opacity) for a solid, clean layout.

### Application Launching & Icons
* **Modern Windows/UWP Apps**: Directly enumerates modern Windows Store apps (Calculator, Cam, Settings, etc.) via COM `IShellItem` and launches them through `shell:AppsFolder\<AppID>`.
* **Clean Icons**: Resolves `.lnk` targets via `IShellLinkW` and extracts high-res icons using clean PIDLs to strip the shortcut arrow watermarks.
* **Lag-Free Async Icons**: Loads application and file-type icons asynchronously in background worker threads using Windows message passing (`WM_ICON_LOADED`) to keep the search UI extremely responsive.
* **Google Favicon**: Uses Google's official favicon for web search results.

### In-Process Tools & Actions
* **Calculator**: High-speed, in-process recursive descent math parser (evaluates formulas like `2+2`, `15% of 340`, `sqrt(9)*4`, etc.) and copies results to clipboard on Enter.
* **Quick Actions**: Immediate execution of system commands: `lock`, `sleep`, `shutdown`, `restart`, `empty recycle bin`/`empty trash`.
* **Recent Files**: Parses `%APPDATA%\Microsoft\Windows\Recent` to show recently used files with their respective file-type icons.

### Local File & Document Content Search
* **Background Crawler Indexer**: Runs a throttled background indexer (`indexer.rs`) that indexes files in `Desktop`, `Documents`, and `Downloads`.
* **Document Text Extraction**: Parses and extracts text content from PDF (using `pdf-extract`) and Word DOCX (using `docx-lite`) files, truncating content to 50KB to keep index sizes low.
* **Broader Code Indexing**: Indexes content and metadata for plain text and source code extensions (`.rs`, `.py`, `.js`, `.ts`, `.c`, `.cpp`, `.h`, `.hpp`, `.cs`, `.go`, `.java`, `.kt`, `.sh`, `.bat`, `.ps1`, `.yaml`, `.yml`, `.toml`, `.ini`, `.sql`, `.xml`).
* **FTS5 FTS Search**: Stores extracted text inside SQLite FTS5 (`files_fts`) for sub-millisecond full-text queries.

### Browser Scopes (Bookmarks & History)
* **Profiles Indexer**: Scans Chromium browsers (Chrome, Edge, Brave) and Gecko browsers (Firefox) across all user profiles, safe-copying locks before parsing.
* **Firefox Places Support**: Direct SQLite querying of Firefox `places.sqlite` bookmarks and history.

### Git Repository Search
* **Crawler Discovery**: Walks scan folders (ignoring node_modules, target, etc.) metadata-first to identify git repositories in **186ms** without recursive loops.
* **Commits & Branches**: Queries HEAD branch and last 100 commits via git CLI tools.
* **Code Tasks (TODO/FIXME)**: Scans codebase comments for TODO/FIXME tags and logs them with file paths and line numbers. Hitting Enter launches VS Code directly at the exact line (`code -g <file_path>:<line>`).

---

## 2. Search Prefixes & Folder Navigation

To avoid result cluttering and SQLite read overhead, the search engine utilizes prefixes:

| Category | Prefix | Empty Input Folder | Badge | Description |
|---|---|---|---|---|
| **Bookmarks** | `bookmarks: <query>` | `📁 Browser Bookmarks` | `BOOKMARK` | Search browser favorites |
| **History** | `history: <query>` | `📁 Browser History` | `HISTORY` | Search browser history URLs |
| **Commits** | `commits: <query>` | `📁 Git Commits` | `COMMIT` | Search recent repo commits |
| **TODOs** | `todos: <query>` | `📁 Git TODOs` | `TODO` | Search code tasks/comment lines |
| **Local Files** | `file: <query>` | `📁 Local Files` | `FILE` | Search documents (PDF, DOCX, TXT) |
| **Source Code** | `code: <query>` | `📁 Source Code` | `CODE` | Search code files |

* *Note: Bookmarks, history, and commits are strictly excluded from general search merging and are only scanned when their respective prefixes are active.*

---

## 3. Custom Agent Rules & Constraints

AI coding agents must follow these rules without exception:

### Development & Build
1. **Windows API Safety**: Keep standard Win32 structures intact and safely delete fonts and GDI objects on `WM_DESTROY`.
2. **SQLite WAL & Concurrency**: Ensure all database connections configure `busy_timeout` (e.g. 5 seconds) and run writes inside short transactions to prevent `database is locked` error code 5.
3. **Suppress flashing windows**: Configure background git/shell command execution with `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) to prevent command windows from flashing.

### Agent Behavior (CRITICAL)
4. **Never run the launcher in the background**: **Do not** run `cargo run`, `cargo run --release`, or `protonsearch.exe` as background tasks within the agent's workspace. Running the daemon blocks the workspace environment and causes access denial (`os error 5`) during subsequent builds. Allow the user to launch and run the app manually.
5. **Do NOT use graphify**: Do not use graphify or run any graphify-watch/rebuild commands. Graphify is completely disabled.
6. **Commit and Push after every edit**: Perform a git commit and push after every single edit you make to the codebase.
7. **Kill app before build or edit**: Before building the app or editing it, make sure you kill any running instance of the app (`taskkill /F /IM protonsearch.exe`).

