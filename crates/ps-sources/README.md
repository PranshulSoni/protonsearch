# ps-sources

Non-filesystem search sources for ProtonSearch v2 (owner: sources-content / L2-A2).

## Modules

- `apps` - Start Menu .lnk apps + packaged/UWP AppsFolder entries, with real
  Windows icons extracted to a PNG cache.
- `recent` - Windows Recent folder shortcuts + v2 launch history.
- `browser` - Chromium (Chrome/Edge/Brave/Vivaldi/Opera) and Firefox history +
  bookmarks, read from copied snapshots (live DBs are never opened; cookie and
  password tables are never touched). Gated by `settings.privacy.browser_history`.
- `git` - repository/commit/TODO discovery via the git CLI (CREATE_NO_WINDOW,
  structured argv only).
- `clipboard` - live clipboard history capture/search (text + PNG images, pinned,
  categorized link/email/path/code/image/text) plus one-time legacy v1 import.
  `capture_current(db, image_dir)` seeds the current clipboard so a fresh history
  is never empty; `start_listener(db, image_dir)` seeds then captures every
  subsequent change (one listener per process); `count(db)` returns the row count
  for source status. Windows 11 exposes no documented readable clipboard-history
  store, so the old %LOCALAPPDATA%\\Microsoft\\Windows\\Clipboard scan was removed -
  history is captured live from now on.
- `quicklinks` - quicklinks/snippets stored in the shared v2 database.
- `calc` - recursive-descent calculator, percentages and unit conversion.
- `actions` - executes structured `ps_ipc::ActionSpec` values via ps-windows.

## Data paths and the icon cache

`default_cache_dir()` resolves to:

- `<PROTONSEARCH_DATA_DIR>/v2/cache` when the `PROTONSEARCH_DATA_DIR` environment
  variable is set (portable installs and the E2E/migration harness), or
- `%APPDATA%/protonsearch/v2/cache` otherwise.

App icons are written as 32 px PNGs under `<cache>/icons`; the launcher renders
`AppEntry.icon_path`. This mirrors `ps_core::paths` so the engine, sources and
tests agree on one data directory. Icon extraction is bounded (at most 400 icons
per scan) and a failing target yields `icon_path: None` plus a bounded warning -
never a path that does not exist.

Start-Menu apps resolve to their real .lnk target and extract via
`ExtractIconExW`/SHGetFileInfo (no shortcut arrow baked in). Packaged/UWP apps are
addressed by `shell:AppsFolder\\<AUMID>` and rendered through
`IShellItemImageFactory::GetImage` (premultiplied BGRA is un-premultiplied before
PNG encoding); if that fails the code falls back to SHGetFileInfo and finally to
`None` + warning.

Probe/evidence command:

```powershell
$env:PROTONSEARCH_DATA_DIR = "target/ps-icon-probe-data"
cargo run -p ps-sources --example icon_probe
```