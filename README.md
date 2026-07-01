<p align="center">
  <img src="icons/OmniSearchLogo.png" alt="OmniSearch logo" width="150" />
</p>

<h1 align="center">OmniSearch</h1>

<p align="center">
  <em>Find everything on your PC. From one shortcut.</em>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/platform-Windows-0078D4?style=flat-square" />
  <a href="LICENSE"><img alt="license" src="https://img.shields.io/badge/license-MIT-green?style=flat-square" /></a>
  <a href="https://github.com/PranshulSoni/omnisearch/stargazers"><img alt="stars" src="https://img.shields.io/github/stars/PranshulSoni/omnisearch?style=flat-square&label=stars&color=yellow" /></a>
  <a href="https://github.com/PranshulSoni/omnisearch/releases"><img src="https://badgen.net/github/release/PranshulSoni/omnisearch" alt="Release"/></a>
  <a href="https://github.com/PranshulSoni/omnisearch/releases"><img alt="downloads" src="https://img.shields.io/github/downloads/PranshulSoni/omnisearch/total?style=flat-square&label=downloads&color=brightgreen" /></a>
</p>

OmniSearch is a fast, local-first Windows launcher that helps users find and open anything already on their PC from one keyboard shortcut.

A fast, local-first Windows launcher to search apps, files, browser history, clipboard, OCR text, Git activity, settings, commands, and AI chats from one shortcut.

<p align="center">
  <img src="icons/Banner.png" alt="OmniSearch banner" />
</p>

## Why OmniSearch?

Your work is already on your PC, but Windows spreads it across too many places.

Apps are in Start Menu. Files are in Explorer. Browser pages are hidden in history. Clipboard items disappear. Screenshots need OCR. Git activity lives somewhere else. AI chats become another place to manually search.

OmniSearch brings all of it into one fast, keyboard-first command center. Press `Alt + Space`, type what you remember, and open the right thing instantly.

## What Makes It Different?

- **One search box for everything** — apps, files, folders, browser history, clipboard, OCR text, Git activity, settings, commands, and AI chats.
- **Local-first by design** — your indexed data stays on your PC.
- **Built for speed** — native Rust/Win32 app with SQLite FTS5 search.
- **Keyboard-first workflow** — open with `Alt + Space`, search naturally, press `Enter`.
- **More than app launching** — search document content, screenshots, clipboard history, code, commits, TODOs, agents, and chats.
- **A real clipboard workflow** — search clipboard history, pin important items, select multiple clips, copy images, edit text clips, and paste combined selections.
- **Hermes agent support** — use Hermes to run autonomous tasks, execute approved commands, and help control your PC from the launcher.
- **Made for Windows power users** — replaces the friction of jumping between Start Menu, File Explorer, browser history, Settings, and AI tools.

## Demo Video

<!-- [This text is completely hidden in preview mode](https://github.com/user-attachments/assets/d37e6edf-e9ba-46a8-98e5-5a96454c4971) -->
<video src="https://github.com/user-attachments/assets/d37e6edf-e9ba-46a8-98e5-5a96454c4971" controls width="100%">
  Your browser does not support the video tag.
</video>

## Star On GitHub

If OmniSearch looks useful, star the repo so more Windows users can find it:

[Star OmniSearch on GitHub](https://github.com/PranshulSoni/omnisearch)

## Installation

> **Recommended install**
>
> Run this in Windows PowerShell:
>
> ```powershell
> curl.exe -fsSL https://raw.githubusercontent.com/PranshulSoni/omnisearch/lean-build/scripts/install.ps1 | powershell -NoProfile -ExecutionPolicy Bypass -
> ```
>
> This downloads the latest OmniSearch release from GitHub and opens the Windows installer.

Manual download: get the latest Windows build from the [OmniSearch releases page](https://github.com/PranshulSoni/omnisearch/releases), then run the installer.

After installation:

1. Launch OmniSearch.
2. Press `Alt + Space`.
3. Add or confirm indexed folders in Settings > Database.
4. Let the first index finish.
5. Start searching.

If `Alt + Space` is already used by another app, change the launcher hotkey in Settings > Hotkeys.

## What You Can Search

| Source | What OmniSearch finds |
|---|---|
| Apps | Installed desktop apps, Microsoft Store apps, and Windows utilities |
| Files and folders | Indexed local files, folders, recent files, documents, downloads, and projects |
| File content | Text inside supported documents, PDFs, Markdown, text files, and source files |
| Images and screenshots | Image files plus OCR text extracted from screenshots and pictures |
| Browser data | Bookmarks and recent history from Chromium-based browsers and Firefox |
| Clipboard | Text and image clipboard history, pinned clips, multiselect actions, image copy, editing, and bulk cleanup |
| Git | Repositories, commits, branches, and TODO/FIXME comments |
| Windows Settings | Modern Windows Settings pages and classic Control Panel pages |
| Commands | Local OmniSearch actions like clipboard, agents, windows, settings, and system actions |
| Agents | Saved AI agents, agent chats, and AI chat history |

## Useful Details

OmniSearch also includes smaller workflow features that make it useful every day:

- **Clipboard pinning** — keep important snippets, links, IDs, commands, and copied text at the top of clipboard search.
- **Clipboard multiselect** — select multiple clipboard items, paste them together, or clean them up in bulk.
- **Image clipboard support** — keep copied screenshots and images searchable, then copy them back when needed.
- **Editable clipboard text** — fix or update a saved clipboard item before copying it again.
- **Clipboard-first commands** — paste recent items sequentially, paste the newest screenshot, clear clipboard contents, or ask AI about clipboard text.
- **Content search** — search inside supported documents, code files, OCR output, screenshots, and notes instead of only matching filenames.
- **Browser recall** — find old pages from browser bookmarks and history without opening the browser first.
- **Windows control surface** — open Windows Settings, Control Panel pages, local commands, windows, and system actions from the same launcher.

## Hermes Agents

OmniSearch includes Hermes agent integration for users who want more than search.

Hermes can help with autonomous workflows such as running local tasks, executing approved commands, managing agent chats, and assisting with PC control from inside the launcher. It gives OmniSearch an agent layer without turning the search bar into a chatbot first.

## Search Prefixes

Prefixes are optional, but useful when you want to search one source directly.

| Prefix | Use it for |
|---|---|
| `file:` | Files, folders, and document content |
| `code:` | Source files and code content |
| `img:` or `screenshots:` | Image files and OCR text |
| `bookmarks:` | Browser bookmarks |
| `history:` | Browser history |
| `clip:` or `clipboard:` | Clipboard history |
| `commits:` | Git commits |
| `agents:` | Available AI agents |
| `agentchats:` | Agent chat history |

## Settings App

OmniSearch also includes a settings app for controlling the launcher.

From settings you can manage:

- General launcher behavior
- Appearance/theme
- Hotkeys
- Agents and AI endpoint configuration
- Indexed folders and database/index status

The app runs from the Windows system tray, so the launcher can stay available in the background without keeping a large window open.

## Privacy

OmniSearch is local-first.

Runtime data is stored on your PC under:

```text
%APPDATA%\omnisearch
```

The local database stores indexed metadata, searchable text, browser items, clipboard history, chats, agents, and settings. Expensive or large data is capped where needed so the app stays responsive instead of trying to keep everything in memory.

## Performance

OmniSearch is a native Rust/Win32 app, not an Electron app.

Current implementation details:

- Native Windows UI and system integration.
- SQLite FTS5 for fast full-text search.
- Debounced search input to avoid doing heavy work on every keystroke.
- Background indexing for files, browser data, Git activity, clipboard, and OCR.

The goal is simple: open fast, search fast, and stay light enough to leave running all day.
