<p align="center">
  <img src="icons/OmniSearchLogo.png" alt="OmniSearch logo" width="150" />
</p>

<h1 align="center">OmniSearch</h1>

<p align="center">
  <em>Deep search for everything already on your Windows PC.</em>
</p>

<p align="center">
  <img alt="platform" src="https://img.shields.io/badge/platform-Windows-0078D4?style=flat-square" />
  <a href="LICENSE"><img alt="license" src="https://img.shields.io/badge/license-MIT-green?style=flat-square" /></a>
  <a href="https://github.com/PranshulSoni/omnisearch/stargazers"><img alt="stars" src="https://img.shields.io/github/stars/PranshulSoni/omnisearch?style=flat-square&label=stars&color=yellow" /></a>
  <a href="https://github.com/PranshulSoni/omnisearch/releases"><img alt="release" src="https://img.shields.io/badge/release-v1.0.0-blue?style=flat-square" /></a>
  <a href="https://github.com/PranshulSoni/omnisearch/releases"><img alt="downloads" src="https://img.shields.io/badge/downloads-coming%20soon-lightgrey?style=flat-square" /></a>
  <img alt="views" src="https://img.shields.io/badge/views-184-brightgreen?style=flat-square" />
</p>

OmniSearch is a fast Windows launcher for finding what is already on your PC.

Press `Alt + Space`, type what you remember, and open the right app, file, folder, browser page, clipboard item, Windows setting, command, image text, code result, or agent chat from one place.

It is built for people who lose time jumping between Start Menu, File Explorer, browser history, Settings, screenshots, copied text, Git repos, and AI chats just to find one thing.

## What OmniSearch Solves

Windows already has your work, but it is scattered across too many places:

- Start Menu knows apps, but not your document content.
- File Explorer can find file names, but not always PDF text, OCR text, browser history, clipboard history, or Git activity.
- Browser search only knows browser data.
- Windows Settings and Control Panel are split across old and new interfaces.
- Clipboard content disappears unless you saved it.
- AI and agent chats become another place to search manually.

OmniSearch puts those sources behind one keyboard-first search box.

## What You Can Search

| Source | What OmniSearch finds |
|---|---|
| Apps | Installed desktop apps, Microsoft Store apps, and Windows utilities |
| Files and folders | Indexed local files, folders, recent files, documents, downloads, and projects |
| File content | Text inside supported documents, PDFs, Markdown, text files, and source files |
| Images and screenshots | Image files plus OCR text extracted from screenshots and pictures |
| Browser data | Bookmarks and recent history from Chromium-based browsers and Firefox |
| Clipboard | Text and image clipboard history, including pinned clipboard items |
| Git | Repositories, commits, branches, and TODO/FIXME comments |
| Windows Settings | Modern Windows Settings pages and classic Control Panel pages |
| Commands | Local OmniSearch actions like clipboard, agents, windows, settings, and system actions |
| Agents | Saved AI agents, agent chats, and AI chat history |

## Daily Use

Open OmniSearch with:

```text
Alt + Space
```

Then type naturally:

```text
chrome
search settings
my resume pdf
history: chatgpt
clip: api key
img: invoice number
code: build index
commits: readme
agents:
```

Press `Enter` to open the selected result.

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
| `todos:` | TODO/FIXME comments in code |
| `agents:` | Available AI agents |
| `agentchats:` | Agent chat history |
| `chats:` | AI chat history |
| `switch:` or `window:` | Open windows and running apps |

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
- Async icon loading so search results do not block on slow shell icon extraction.
- Clipboard retention is capped, with pinned items preserved.
- Browser imports are capped to avoid pulling huge locked profile databases into memory.
- Document text extraction is capped per file to keep the database compact.

The goal is simple: open fast, search fast, and stay light enough to leave running all day.

## Install

Download the latest Windows build from the project release page, then run the installer.

After installation:

1. Launch OmniSearch.
2. Press `Alt + Space`.
3. Add or confirm indexed folders in Settings > Database.
4. Let the first index finish.
5. Start searching.

If `Alt + Space` is already used by another app, change the launcher hotkey in Settings > Hotkeys.

## Build From Source

Most users do not need this section. It is only for contributors or local builds.

Requirements:

- Windows
- Rust stable toolchain
- MSVC build tools

Build:

```powershell
cd omnisearch
cargo build --release --bin omnisearch
```

Run:

```powershell
.\target\release\omnisearch.exe
```

Run tests:

```powershell
cargo test --bin omnisearch -- --test-threads=1
```

If Windows locks the executable during rebuild, close OmniSearch from the tray or run:

```powershell
taskkill /F /IM omnisearch.exe
```

## Tech Stack

- Rust
- Win32 API
- GDI custom rendering
- SQLite and FTS5
- Windows shell APIs
- Windows OCR APIs
- Browser profile indexing
- Local SQLite-backed agents and chat history

## Product Focus

OmniSearch is not trying to be another note app, browser, or file manager.

It is a command center for finding and opening the things already on your Windows PC.
