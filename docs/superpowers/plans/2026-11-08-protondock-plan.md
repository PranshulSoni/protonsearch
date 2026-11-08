# ProtonDock implementation plan (phase by phase)

Date: 2026-11-08. Branch: `dock`. Spec: `docs/superpowers/specs/2026-11-08-protondock-spec.md` (decisions D, BD, O and resolutions R cited below). Detailed contracts: DS = `docs/superpowers/specs/2026-11-08-dock-design-system.md`, BS = `docs/superpowers/specs/2026-11-08-dock-backend-spec.md`. Supersedes `docs/superpowers/plans/2026-11-07-macos-dock-plan.md` (its Phases 1 and 2 landed as `5011a06` to `16cf1c2`; its follow-ups T2.4 hot-on-blur and T5.3 last-tile extent are folded into tasks A10, B and J below).

Rolling wave: Phase A has full task specs. Every later phase has goal, outcome, packages, files, dependencies and exit checks only; the planner expands phase N+1 into full task specs when phase N's IC passes, using what phase N taught. No implementer starts a phase that has no full task specs.

## 0. How work runs

Roles. Opus (medium) plans and expands the next phase. Haiku (high) implements exactly one task at a time. Sonnet (high) reviews every task commit (spec compliance first, then quality; read-only; APPROVE or REVISE with `file:line`) and runs the integration checkpoint (IC) that ends every phase. Every agent first invokes the `ponytail` and `caveman` skills, loads Headroom (`headroom_compress` on outputs over about 120 lines), and reads `CLAUDE.md`, `AGENTS.md`, `rules.md`.

Implementer protocol (every task):
1. `git status --short -- <task files>`: if a task file has changes you did not make, stop and report (peer sessions share this tree).
2. Kill before any build: `taskkill //F //IM protondock.exe //T ; taskkill //F //IM protonsearch.exe //T` ("not found" is fine).
3. Implement exactly the spec. If the spec is wrong or ambiguous, stop and report; make no design choice.
4. Run the task's verify commands. Never `cargo run`; never start the executable.
5. Commit and push by pathspec only: `git add <files>; git commit -m "<message>" -- <files>; git push origin dock`. If the push is rejected, stop and report (never pull, rebase, stash or revert).

Concurrency. At most 2 implementers at once: one UI lane (`src/`, `scripts/`, `docs/`) and one backend lane (`src-tauri/`, `protonsearch/`). Never two tasks in the same lane at once (they share one compiler). Hot files, one task at a time across both lanes: `src-tauri/src/main.rs`, `src-tauri/build.rs`, `src-tauri/permissions/commands.toml`, `src-tauri/capabilities/*`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src/services/native.ts`, `src/components/dock/layout.ts`, `src/components/dock/DockSurface.tsx`, `src/widgets/final/meta.ts`, `src/design/dock-tiles.css`, `src/design/dock-final.css`.

HTML drift. Authority is the pinned copy `docs/design/dock-editable.2126c49.html` (task A1). At each phase start the planner runs `node scripts/html-pin.mjs` and `git diff --no-index --stat docs/design/dock-editable.2126c49.html dock-editable.html`; if drift touches a range cited by that phase, re-triage per DS 1.4 step 3 before dispatch. If the user supplies a new authoritative HTML (`CLAUDE.md`), pin it as a new snapshot and re-run the affected audit sections first. Known drift today: the worktree file (`0d8c471`) differs only in the search launcher script (pinned lines 5831 to 6385), which matters to phase I only.

Rust conventions (BS 1.6, 7.6, 7.7). Provider bodies are synchronous inside `spawn_blocking`; no lock guard crosses an `.await`; every `unsafe` block has a `// SAFETY:` comment; COM and WinRT objects and GDI handles are released on every path (`CoUninitialize` guard per STA thread, `WlanFreeMemory`, `CloseHandle`, `DeleteObject`, `ReleaseDC`, `DestroyIcon` only for icons we created); no listening socket; any unavoidable console child sets `creation_flags(0x08000000)` (`CREATE_NO_WINDOW`). New Rust tests never write under `std::env::temp_dir()` (it fails here with Windows error 5, the cause of the known `local_calendar::tests::` failures); integration tests use `env!("CARGO_TARGET_TMPDIR")`.

Gates. TS task: `npx tsc -b` then `npm test -- --reporter=dot`. Rust task: the named `cargo test --manifest-path src-tauri/Cargo.toml <filter>` plus the full run `cargo test --manifest-path src-tauri/Cargo.toml -- --skip local_calendar::tests::` (0 failures).

IC (Sonnet, end of every phase):
1. Kill both processes. `node scripts/html-pin.mjs` (prints `html-pin: ok`).
2. `npx tsc -b`; `npm test -- --reporter=dot`; `npm run widgets:validate`; `npm run build`.
3. `cargo test --manifest-path src-tauri/Cargo.toml -- --skip local_calendar::tests::` (0 failures); `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::tests::` (only the known fixture `create_dir` access-denied failures; record the count); `cargo test --manifest-path protonsearch/Cargo.toml --lib <filter>` when the engine crate changed.
4. Kill, `npx tauri build --debug --no-bundle`, copy `src-tauri/target/debug/protondock.exe` to `%TEMP%\protondock-ic\` and launch the copy (never from the project tree: WebView2 `0x800700AA`).
5. Wiring checks named by the phase. Then the phase's native checks with the Computer Use tool (read the `anthropic-skills:computer-use` skill; tools `mcp__computer-use__*`). When the tool is absent, record the exact blocker; nothing is marked verified without an observation. Never change system settings (display scale, animation effects, location consent, wallpaper): checks needing another setting are recorded as "needs user: <setting>". A maximized white Notepad window is the bright backdrop. Respect physical-Escape stops and locked desktops.
6. Record: `docs/qa/phase-<x>.md` (IC-owned: commands, results, screenshots under `docs/qa/native/phase-<x>/`, blockers) and a short pointer section appended to `docs/qa/STATUS.md`. Commit STATUS.md only when `git diff docs/qa/STATUS.md` shows nothing but that append; otherwise report the pending pointer to the orchestrator.
7. Refresh the graph with `graphify update .` if the CLI exists (no watch mode); never stage `graphify-out/`.

## 1. Phases

| Phase | Outcome the user sees | Tasks | Lanes | Exit checks |
|---|---|---|---|---|
| A Foundation and landed fixes | Sharper icons for apps with small window icons; shelf returns when the dock loses focus; dock starts even when Alt+Space is taken; a corrupt layout write recovers; build inputs tracked; HTML pinned; jitter and icon fixes reviewed and natively checked | 12 | UI 2, BE 8, IC 1, review 1 | IC-A gate, N-A1 to N-A6 |
| B Tokens and motion (bar 3) | Profile switch, reorder and new widgets animate; hover lift and press; magnification at the HTML numbers; reduced motion honoured | ~10 | UI 7, BE 3 | Bar 3 and bar 1 regression |
| C Surfaces, themes, honest states (bar 4) | Neutral 48px tiles, working Light and Glass themes, solid Windows flag, honest state visuals, no fabricated values, Start opens the real Start menu | ~11 | UI 8, BE 3 | Bar 4 sampling, theme captures, Start |
| D Widths, layout v3, glyph set (bar 5, bar 2 at DPR) | Width steps, lossless v3 migration with backup, new glyph set, DPR-correct app icons, Codex replaces ChatGPT | ~15 | UI 13, BE 2 | Glyph sheet, DPR icons, migration relaunch |
| E Edit mode, shelf, profiles, cards | Edit mode with drag reorder and width cycle, custom profiles, five shelf pills, anchored cards with caret | ~9 | UI 8, BE 1 | Persistence in two profiles, keyboard pass, caret |
| F System widgets, real data | Battery, music with artwork, system stats capsule, calendar week, clock; demand-driven polling | ~10 | UI 5, BE 5 | Values match Windows; honest empty states |
| G User widgets | Weather setup (city search, my location), quick notes, Claude and Codex cards, pinned apps | ~11 | UI 5, BE 6 | Searched city weather, note found in search, pinned launch |
| H Connectivity | Wi-Fi, Bluetooth, peripherals battery | ~8 | UI 3, BE 5 | Real SSID and networks, device levels |
| I Spotlight (Kensaku) UI | Quick Look, list or grid, calculator hero, engine footer, reveal and copy path | ~7 | UI 4, BE 3 | Keyboard flow, preview, reveal |
| J Edges and alignment (D5 override) | Left and right docks, start, center, end alignment, free placement | ~7 | UI 5, BE 2 | Bar 1 on all edges, cards per edge |
| K Taskbar (BD-9) | AppBar reservation, "Hide Windows taskbar" switch, capability split, trash confirmation | ~7 | UI 2, BE 5 | Maximized windows stop above the dock; restore on exit and crash |
| L Gated and blocked | Each item only after its named user decision | per decision | | per item |

Total: 12 specified now, about 95 more estimated (expanded per phase).

## 2. Phase A: foundation and landed fixes (full specs)

Order: A0 alone; then the UI lane (A1, A10) beside the backend lane (A2 to A9, strictly in that order); A11 (read-only review) any time after A0; then IC-A. Parallel-safe pairs: any UI-lane task with any backend-lane task.

### A0 Track the build inputs (IC-owned, Sonnet)
- Why: a clean checkout of `dock` does not compile. `main.rs` declares `mod local_calendar` and others that are untracked; tracked `src/widgets/sdk/primitives.tsx` imports untracked `src/design/Icon`; later task commits would otherwise add whole files.
- Files (exactly): `src-tauri/Cargo.lock`, `src-tauri/build.rs`, `src-tauri/capabilities/calendar-settings.json`, `src-tauri/capabilities/dock.json`, `src-tauri/capabilities/search.json`, `src-tauri/src/calendar_ics.rs`, `src-tauri/src/clipboard_widget.rs`, `src-tauri/src/dock_lifecycle.rs`, `src-tauri/src/local_calendar.rs`, `src-tauri/tests/local_calendar.rs`, `src/design/Icon.tsx`, `src/design/icons.svg`, `src/design/reference-widgets.json`, `src/widgets/{CalendarWidget,ClipboardWidget,FocusTimer,LocalProviders,RunningApps,SystemControls,WeatherWidget}.tsx` and their `.test.tsx` files, `src/widgets/builtins/storage-health/{Preview.tsx,README.md,Widget.test.tsx,Widget.tsx,fixtures.ts,manifest.ts,types.ts}`, `src/widgets/{catalog.tsx,catalog.test.tsx,index.tsx,providers.css,weather.css}`. Never `src-tauri/gen/` or `src-tauri/permissions/autogenerated/` (generated).
- Steps: `git status --short -uall -- src src-tauri` must list exactly these as `??` (plus the two generated folders); commit only the intersection and report any difference. Kill; `npx tsc -b`; `npm test -- --reporter=dot`; `cargo check --manifest-path src-tauri/Cargo.toml`. Commit `chore(build): track source files the dock build already depends on` by pathspec; push.

### A1 Pin the design HTML (U0.4)
- Lane UI. Depends A0. Parallel-safe with A2 to A9.
- Files: `docs/design/dock-editable.2126c49.html` (new), `scripts/html-pin.mjs` (new; not `.test.mjs`, so Vitest never collects it).
- Create the copy with `mkdir -p docs/design && git cat-file -p 2126c4990818e68d157e2207977702f5ab349d19 > docs/design/dock-editable.2126c49.html`.
- Script (ESM, only `node:fs`, `node:crypto`, `node:assert/strict`): read the copy as UTF-8, replace `\r\n` with `\n`, compute the git blob id as SHA-1 of `blob <byteLength>\0` followed by the bytes, assert it equals `2126c4990818e68d157e2207977702f5ab349d19`. In the text between the first `<style` and the first `</style>` assert these regexes match: `--tile-h:\s*48px`, `--dock-gap:\s*6px`, `--dock-pad:\s*6px`, `--dock-radius:\s*18px`, `--mag-strength:\s*0\.22`, `--mag-radius:\s*86px`, `--lift:\s*-4px`, and that `@media` occurs 0 times. Assert `var DEFAULT_DOCK_ORDER = [ ... ];` contains exactly 19 quoted ids. Inside `var DEFAULT_PROFILES = { ... };` assert each list deep-equals: daily `start, search, peripherals, system-stats, calendar, running-apps, music-player, weather, clock`; work `start, search, calendar, system-stats, running-apps, pinned-apps, weather, status-battery, wifi, bluetooth, notifications, quick-notes, clock`; founder `start, search, claude, codex, system-stats, calendar, running-apps, pinned-apps, music-player, wifi, system-tray, clock`; focus `start, search, calendar, system-stats, music-player, quick-notes, clock`. From `function cycleWidgetWidth` to the next `function ` assert `/48[\s\S]*96[\s\S]*140[\s\S]*168/`. Then compute the same blob id for `dock-editable.html` and print `dock-editable.html drift: none` or `dock-editable.html drift: yes (<id>)` (never fails). Last line `html-pin: ok`. Any failed assertion exits 1.
- Verify: `node scripts/html-pin.mjs` prints `html-pin: ok`; after staging, `git ls-files -s docs/design/dock-editable.2126c49.html` shows blob `2126c4990818e68d157e2207977702f5ab349d19` (this also makes the object reachable, so `git gc` cannot prune it).
- Commit: `docs(design): pin dock-editable.html snapshot 2126c49 with a fact check`.
- Review: copy byte-identical to the blob; script has no dependency and no network; drift line never fails the run.

### A2 Provider and action registry (BE-01)
- Lane backend. Depends A0. Hot: `main.rs`.
- Files: `src-tauri/src/providers.rs` (new), `src-tauri/src/main.rs`, `src-tauri/src/widget_data.rs`.
- `providers.rs`: `use serde_json::Value;` `pub fn snapshot(id: &str) -> Result<Value, String> { match id { _ => Err("Unknown provider".into()) } }` and `pub fn action(id: &str) -> Result<(), String> { match id { _ => Err("Unsupported widget action".into()) } }` with the comment `// One arm per line, alphabetical, so parallel packages merge cleanly.`; tests `unknown_provider_is_rejected` (`snapshot("nope")` error equals `Unknown provider`) and `unknown_action_is_rejected` (`action("nope:x")` error equals `Unsupported widget action`).
- `main.rs`: add `mod providers;` between `mod local_calendar;` and `mod running_apps;`; in `provider_snapshot_data` replace `_ => return Err("Unknown provider".into()),` (line 780) with `_ => return providers::snapshot(&provider_id),`.
- `widget_data.rs` `action` (lines 337 to 339): inside the `if !["media:toggle", "media:next", "media:previous"].contains(&action)` block replace the `return Err(...)` with `drop(_guard);` then `return crate::providers::action(action);` so `NATIVE_LOCK` is released before delegating. Nothing else changes.
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml providers`; full Rust run. Existing tests asserting `Unknown provider` and `Unsupported widget action` must still pass.
- Commit: `feat(native): add provider and action registry for new widgets`.
- Review: lock dropped before delegation; error strings byte-identical; no other arm touched.

### A3 IPC manifest sync (BE-03)
- Lane backend. Depends A2. Hot: `build.rs`.
- Files: `src-tauri/build.rs`, `src-tauri/tests/ipc_manifest.rs` (new).
- `build.rs`: add `"known_folders_snapshot"` after `"provider_snapshot"`, then `"open_widget_card"`, `"hide_widget_card"`, `"widget_card_state"` after `"fit_search"`, and `"ai_usage_snapshot"` after `"running_apps_activate"`, so the list has the same 25 commands as `generate_handler!`.
- Test file reads `concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs")`, `/build.rs` and `/permissions/commands.toml` as text. Handlers: text between `tauri::generate_handler![` and the next `]`, split on `,`, trimmed, empties dropped. Manifest: text between `.commands(&[` and the next `]`, quoted strings (split on `"`, odd indices). Sets: pair each `identifier = "<id>"` line with the next `commands.allow = [...]` line's quoted strings. Tests: `every_handler_is_in_the_manifest` (sorted lists equal, no duplicates), `every_manifest_command_is_allowed_by_some_set`, `no_set_allows_an_unknown_command`.
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml --test ipc_manifest`; `cargo check --manifest-path src-tauri/Cargo.toml`.
- Commit: `fix(native): list every invoked command in the app manifest`.
- Review: the five names added, nothing removed; if N-A4 shows a window losing a command, the IC reverts only the `build.rs` hunk and keeps the test with those five as a named exception (BS BE-03).

### A4 Crash-safe layout store (BE-04)
- Lane backend. Depends A3. Hot: `main.rs`.
- Files: `src-tauri/src/layout_store.rs` (new, pure: no `crate::`), `src-tauri/src/main.rs`, `src-tauri/tests/layout_store.rs` (new).
- API: `pub const FILE: &str = "protondock-layout.json"; pub const BACKUP: &str = "protondock-layout.json.bak"; pub fn load(dir: &Path) -> Result<Value, String>; pub fn save(dir: &Path, value: &Value) -> Result<(), String>`.
- `load`: missing `FILE` gives `Ok(Value::Null)`. Read and parse `FILE`; on success return it. On a read or parse failure, if `BACKUP` reads and parses to a JSON object, return it (files untouched); otherwise return the original error string (`e.to_string()`, as today).
- `save`: `to_vec_pretty`; not an object or over 1048576 bytes gives `Err("Invalid layout")` before touching disk. If `FILE` exists and parses as a JSON object, `std::fs::copy(FILE, BACKUP)`; a copy failure returns `Err("Could not back up the dock layout")` and writes nothing. A corrupt `FILE` is never copied. Then write `protondock-layout.json.tmp` and move it with the existing `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` block, moved verbatim with a `// SAFETY:` comment (both wide strings NUL-terminated and alive for the call).
- `main.rs`: `mod layout_store;` between `mod known_folders;` and `mod local_calendar;`. `load_dock_settings` body: keep the lock line, then `layout_store::load(&data_dir()?)`. `save_dock_settings`: keep the lock, `layout_store::save(&data_dir()?, &value)?;`, keep the `settings-updated` emit and `Ok(())`. Signatures unchanged; delete the moved code.
- Tests (`#[path = "../src/layout_store.rs"] mod layout_store;`, each in a fresh `CARGO_TARGET_TMPDIR` subfolder): `save_then_load_round_trips`, `missing_file_loads_null`, `object_and_size_limits_are_enforced` (array and a 1048577-character string rejected, no file created), `previous_good_copy_is_kept` (save A, save B: backup is A, file is B), `corrupt_file_falls_back_to_backup` (save A, save B, overwrite file with `{trunc`: load returns A), `corrupt_file_never_replaces_a_good_backup` (then save C: backup still A, file C), `both_bad_returns_error_and_modifies_nothing` (both bytes unchanged).
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml --test layout_store`; full Rust run.
- Commit: `fix(native): keep a previous-good dock layout and recover from a corrupt file`.
- Review: mutex and emit unchanged; no write before validation; backup failure blocks the write.

### A5 Canonical ids and layout v3 acceptance (BE-06)
- Lane backend. Depends A4. Hot: `main.rs`.
- Files: `src-tauri/src/main.rs`, `protonsearch/src/weather.rs`.
- `main.rs`: `const WIDGET_CARD_IDS: [&str; 30] = ["start", "search", "running-apps", "pinned-apps", "system-stats", "calendar", "peripherals", "music-player", "weather", "status-battery", "wifi", "bluetooth", "notifications", "quick-notes", "claude", "codex", "assistant", "system-tray", "clock", "agent", "files", "folders", "trash", "volume", "windows", "apps", "system", "spotify", "siri-ai", "chatgpt"];`. Test `widget_card_ids_accept_canonical_and_alias_ids_only`: `start`, `codex`, `assistant`, `volume`, `windows`, `chatgpt` valid; `""`, `Start`, `siri`, `../x`, `calendar ` invalid.
- `weather.rs`: add `fn profile_id(p: &str) -> bool` (1 to 24 bytes, first `[a-z0-9]`, rest `[a-z0-9-]`); `allowed_profile` gains `3 => profile_id(profile),`; `location` filter becomes `matches!(version, 1 | 2 | 3)`. Update the two tests that use version 3 as "unknown": line 391 `layout_for(3, ...)` becomes `layout_for(4, ...)`, line 437 `json!(3)` becomes `json!(4)`. New tests `weather_location_reads_version_three_and_custom_profiles` (version 3, profile `deep-work`, enabled settings with latitude 1.0 and longitude 2.0: same expectation as `version_two_accepts_all_profiles_and_reads_the_active_profile_item`) and `weather_rejects_invalid_profile_ids` (`""`, `Daily`, `-x`, `a_b` and 25 `x` are errors).
- Verify: `cargo test --manifest-path protonsearch/Cargo.toml --lib weather`; `cargo test --manifest-path src-tauri/Cargo.toml widget_card`; full Rust run.
- Commit: `feat(native): accept canonical widget ids and layout v3 profiles`.
- Review: 30 ids exactly; versions 1 and 2 behave as before. If the engine crate fails to build because of peer edits in other `protonsearch/` files, stop and report.

### A6 Calendar accepts v2 and v3 layouts (BE-07)
- Lane backend. Depends A5. File: `src-tauri/src/local_calendar.rs`.
- `profile_name`: accept the same grammar as A5 (copy `fn profile_id` into this file; error text unchanged).
- Add pure `fn profile_is_active(layout: Option<&Value>, profile: &str) -> Result<bool, String>`: `None` gives `Ok(profile == "daily")`; version 1 keeps today's rule; version 2 or 3 with `profile` a valid id and `profiles` an object gives `Ok(layout["profile"] == profile && layout["profiles"].get(profile).is_some())`; anything else `Err("Native dock profile could not be validated")`. `active_profile` becomes `profile_name(profile)?`, the existing bounded read, then `profile_is_active(layout.as_ref(), profile)`.
- Extract everything in `grants` after the bounded read into pure `fn check_grants(value: Value) -> Result<Value, String>`; replace `for profile in ["daily", "founder"]` with a loop over every key of `value["profiles"]`: a key failing `profile_id` returns the existing invalid-grants error; each record keeps the existing checks.
- Tests in a new `#[cfg(test)] mod profile_tests` at the end of the file (pure, no filesystem): `version_two_layout_activates_its_profile`, `version_three_custom_profile_is_valid`, `inactive_profile_is_denied_not_an_error`, `unknown_layout_version_is_rejected` (version 4; version 2 with profile `Bad Name`), `missing_layout_keeps_daily_active`, `grants_for_custom_profiles_are_validated` (`deep-work` with source token `0123456789abcdef0123456789abcdef` and path `a.ics` is Ok; key `Bad Key` is Err; record `{"token":"bad","path":42}` is Err).
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::profile_tests`; full Rust run; `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::tests::` failure count equal to before the edit (run it first and note the count).
- Commit: `fix(native): accept layout v2 and v3 profiles for calendar grants`.
- Review: grant file format untouched; tokens of other profiles still validated; no fixture-based test added.

### A7 Non-fatal hotkeys with a status provider (BE-08)
- Lane backend. Depends A6. Hot: `main.rs`.
- Files: `src-tauri/src/hotkey_status.rs` (new, pure), `src-tauri/src/providers.rs`, `src-tauri/src/main.rs`.
- `hotkey_status.rs`: `static STATUS: Mutex<(Vec<String>, Vec<String>)> = Mutex::new((Vec::new(), Vec::new()));`, `pub fn hotkeys(configured: &str) -> Vec<String>` (`["Alt+Space"]` plus `configured` when it differs), `pub fn record(shortcut: &str, ok: bool)` (push to registered or failed; a poisoned lock is ignored), `pub fn read() -> Value` (`{"registered":[...],"failed":[...]}`). Tests: `records_success_and_failure_separately` (unique names, `contains` checks), `snapshot_has_both_lists`, `default_shortcut_is_always_first`.
- `providers.rs`: `#[path = "hotkey_status.rs"] pub mod hotkey_status;`, a private `fn envelope(status: &str, data: Value, message: &str) -> Value` returning `{"status","data","updatedAt" (Unix ms from SystemTime),"message"}`, and the arm `"hotkey" => Ok(envelope("live", hotkey_status::read(), "")),`.
- `main.rs` setup: replace lines 1099 to 1103 with `for shortcut in providers::hotkey_status::hotkeys(&protonsearch::settings::AppSettings::load().global_hotkey) { let ok = app.global_shortcut().register(shortcut.as_str()).is_ok(); providers::hotkey_status::record(&shortcut, ok); }`. No `?` remains on a registration.
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml hotkey`; full Rust run.
- Commit: `fix(native): keep running when a search hotkey cannot be registered`.
- Review: the shortcut handler is untouched; envelope keys match BS 5.2.

### A8 Wi-Fi permission state (BE-10)
- Lane backend. Depends A7. Files: `src-tauri/src/widget_data.rs`, `docs/widgets/NATIVE_PROVIDERS.md`.
- `current_wifi_connection` returns `Result<Option<WLAN_CONNECTION_ATTRIBUTES>, u32>`: non-zero code frees `data` when non-null and returns `Err(code)`; null data or a short size frees and returns `Ok(None)`; else `Ok(Some(..))` after `WlanFreeMemory` (memory freed on every path).
- `WifiStatus` gains `access_denied: bool` (false in every existing constructor). New pure `fn wifi_from_query(query: Result<Option<WLAN_CONNECTION_ATTRIBUTES>, u32>) -> WifiStatus`: `Err(5)` gives connected true, ssid none, signal none, state `connected`, api available, access_denied true; `Ok(Some(c))` with `c.isState == wlan_interface_state_connected` gives today's connected result; anything else gives today's `disconnected` result. `read_wifi` calls it in place of lines 614 to 635.
- `network()` wifi object adds `"accessDenied":wifi.access_denied`. `settings_uri` adds `"settings:location" => Some("ms-settings:privacy-location"),`.
- Tests: `access_denied_query_reports_connected_without_a_name`, `other_query_failures_stay_disconnected` (`Err(87)`, `Ok(None)`, `Ok(Some(default))`), `settings_location_has_the_exact_route`.
- `NATIVE_PROVIDERS.md`: append `## Wi-Fi access denied (2026-11-08)` with: "`network.wifi.accessDenied` is true when Windows withholds connection details for lack of precise-location consent; the dock then reports connected without a name. `settings:location` opens `ms-settings:privacy-location`. The same consent gates Wi-Fi scans and Geolocator."
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml widget_data`; full Rust run.
- Commit: `fix(native): report Wi-Fi as connected when Windows withholds details`.
- Review: no new WLAN call; existing network tests pass.

### A9 Prefer the 256px executable icon for small window icons (a3d9e11 follow-up)
- Lane backend. Depends A8. File: `src-tauri/src/running_apps.rs`.
- Extract lines 114 to 130 of `icon` into `unsafe fn process_path(hwnd: HWND) -> Option<Vec<u16>>` returning the path including its NUL (`CloseHandle` still runs on every path).
- Add pure `fn prefer_executable_icon(window_px: Option<u32>, exe: &[u16]) -> bool`: false when the file name after the last `\` (NUL ignored) equals `applicationframehost.exe` ignoring ASCII case; otherwise `window_px.map_or(true, |px| px < 64)`.
- New tail of `icon` after the handle lookup: `let window = (handle != 0).then(|| HICON(handle as *mut _)); let window_px = window.and_then(icon_size); let path = process_path(hwnd); let prefer = path.as_deref().is_some_and(|p| prefer_executable_icon(window_px, p)); if prefer { if let Some(i) = path.as_deref().and_then(executable_icon) { return Some(i); } } if let Some(i) = window.and_then(render_icon) { return Some(i); } if prefer { None } else { path.as_deref().and_then(executable_icon) }`. Never `DestroyIcon` a window or class icon (shared); `executable_icon` already destroys its own.
- Tests: `prefers_executable_icon_below_64px` (`Some(32)` and `None` with `C:\Apps\tool.exe`), `keeps_window_icon_at_64px_and_above` (`Some(64)`, `Some(256)`), `never_prefers_application_frame_host_icon` (`Some(16)` with `...\ApplicationFrameHost.exe` and the lowercase form).
- Verify: `cargo test --manifest-path src-tauri/Cargo.toml running_apps`; full Rust run.
- Commit: `fix(native): prefer the 256px executable icon when a window icon is small`.
- Review: UWP frame-host windows keep their window icon; cache behaviour unchanged.

### A10 Leave the hot state on window blur (old T2.4)
- Lane UI. Depends A0. Hot: `DockSurface.tsx`. Parallel-safe with A2 to A9.
- Files: `src/components/dock/DockSurface.tsx`, `src/components/dock/DockSurface.test.tsx`.
- After the unmount-cleanup effect (line 57) add an effect with `[]` deps: `const settle = () => { clearTimeout(cool.current); setHot(false); }; const hidden = () => { if (document.hidden) settle(); };` added as `window` `blur` and `document` `visibilitychange` listeners and removed in the cleanup.
- Tests (render as the file's existing tests do): `leaves the hot state when the window loses focus` (`fireEvent.pointerEnter` on `.dock-final-rail`, `.dock-final` has `data-hot="true"`, `act(() => window.dispatchEvent(new Event('blur')))`, attribute is null) and `leaves the hot state when the document is hidden` (define `document.hidden` as true with `configurable: true`, dispatch `visibilitychange`, attribute null, delete the override after).
- Verify: TS gate. Commit: `fix(dock): leave the hot state when the window loses focus`.
- Review: no change to fit, dedupe or magnify code.

### A11 Review the landed jitter and icon fixes (Sonnet, read-only)
- `git show fa60e4b` and `git show a3d9e11` against `docs/qa/dock-defects-2026-11-07.md` items 1 and 2 and DS 7 rows CL-Q1 and CL-Q2: root cause addressed, dedupe compares width, height and regions, `place` skips unchanged size and position, icon order large-first, no upscaling, GDI objects released. Output APPROVE or numbered fix items; the planner turns each fix item into a task (A12 onward) before IC-A.

### IC-A
- Gate: section 0 IC steps 1 to 4, with `cargo test --manifest-path protonsearch/Cargo.toml --lib weather`.
- Wiring: `provider_snapshot_data` falls through to `providers::snapshot`; `widget_data::action` delegates after `drop(_guard)`; both settings commands call `layout_store`; `setup` has no `register(...)?`; `ipc_manifest` test green; `git status --short -uall -- src src-tauri` shows no untracked source files.
- Native checks (Computer Use):
  - N-A1 jitter (bar 1, `fa60e4b`): idle 60 s with a screenshot every 2 s, rail left and right edge pixel columns identical; ten hover enter and leave cycles on tile 3, edges never move; Daily to Focus and back five times, three shots 1 s apart equal after settling; at the user's current scale only (other scales: "needs user: display scale").
  - N-A2 icons (bar 2, `a3d9e11` plus A9): zoomed tile of Notepad and of one app whose window icon is small, beside the same app's Windows taskbar icon, at rest and at hover peak; edge sharpness at least equal.
  - N-A3 hot on blur (A10): hover the rail, Alt+Tab away: the shelf is visible again within 300 ms.
  - N-A4 ACL (A3): a widget card opens from the dock, Customize opens the settings window, Spotlight opens from the search tile.
  - N-A5 layout safety (A4): switch profile, relaunch the copy: profile restored; `%APPDATA%\protonsearch\protondock-layout.json.bak` exists after a save.
  - N-A6 hotkey (A7): only if a program already holds Alt+Space: the dock still starts; otherwise record "unverified: no program holds Alt+Space".
- Records: `docs/qa/phase-a.md`, STATUS.md pointer. BE-10's denied branch stays an unverified gate (needs location consent off: "needs user").

## 3. Later phases (expanded when the previous IC passes)

### Phase B: tokens and motion (bar 3)
- Outcome: the dock moves like the HTML: magnification 0.22 within 86px without spreading neighbours, hover lift -4px and press .96, FLIP for reorder (220ms), profile switch and resize morph (380ms), entering slots (320ms); reduced motion and reduced transparency honoured.
- UI packages: U0.1 (tokens file imported last; old `:root` variables stay until phase C rewrites the rules using them), U2.1 (fit constants: headroom 63, constant `ALONG_PAD = 12` on both sides, which also covers old T5.3 last-tile extent; `hot` changes only vertical extent), U2.2, U2.3 (`useFlip`, stable `key={item.id}`), U2.4, U2.5 (rail morph: glass stays a pseudo-element animated by `--glass-l` and `--glass-r` per R12; two-phase fit; `data-morphing` skips the observer), U2.6.
- Backend packages: BE-02 (features one per line), BE-09 (stable keys), BE-52 (security guard test).
- Hot files: `DockSurface.tsx`, `dock-tiles.css`, `fit.ts`, `magnify.ts`, `src/main.tsx`, `src-tauri/Cargo.toml`. Depends: A.
- Exit: DS 7 row CL-Q3 (animations present within 50ms in tests; six-shot burst shows intermediate widths), bar 1 regression (exactly 2 `fit_dock` calls per morph, none idle), reduced-motion run only if the user turns off Windows animation effects ("needs user").

### Phase C: surfaces, themes, honest states (bar 4, honesty)
- Outcome: neutral 48px tiles and rail roles in Dark, Light and Glass; solid Windows flag distinct from Apps; one state visual per DS 4.4 including `unsupported`; badge, dot, state dot and tooltip per DS 4.6; card surfaces without tints; every simulated or sample value gone from the native path (O-1); Start tile opens the real Start menu.
- UI packages: U0.2, U0.3 (+R1), U1.1 (on the glass pseudo-element, R12), U1.5, U4.2 (defect-doc rows), U4.3. Backend: BE-14 (Start, needs BE-02), BE-12 (AI `working` from session activity, replaces the test toggle).
- Hot files: `dock-tiles.css`, `dock-final.css`, `DockSurface.tsx`, `meta.ts`, `main.rs`. Depends: B.
- Exit: DS 7 row CL-Q4 (19 tiles sampled in Dark, channel spread 6 or less; flag opaque over the white backdrop; Light and Glass over white and black backdrops); grep proves no `sampleFor` on the native path; Start opens and closes Start.

### Phase D: widths, layout v3, glyph set (bar 5, bar 2 at DPR)
- Outcome: width steps 48 to 220 with per-widget sets; layout v3 with lossless migration, `layoutV2Backup`, aliases (`chatgpt` to `codex`, BD-1), Ahmedabad weather materialized then default unconfigured (BD-8), unknown ids kept; new glyph families; app icons requested at the DPR ladder (R2).
- UI packages: U1.2 (+R4 whitelist, delete `sizing.ts`), U1.2b, U1.3.0, U1.3.a to U1.3.h, U1.4 with BE-05 (types and `native.ts` wrappers). Backend: BE-13 (running-app `key`, after U1.4 on `running_apps.rs`).
- Hot files: `layout.ts`, `DockSurface.tsx`, `meta.ts`, `native.ts`, `running_apps.rs`. Depends: C (and A5, A6 for v3 readers).
- Exit: BS 6.10 tests 1, 2, 3, 5 green; a copy of the user's real `protondock-layout.json` migrates in a unit test; relaunch shows the same widgets in all profiles; DS 7 row CL-Q5 glyph sheet at 32, 48, 64, 96 beside `preview-tahoe-glass.html`; `naturalWidth >= ceil(24 * 1.22 * dpr)`.

### Phase E: edit mode, shelf, profiles, cards
- Outcome: Edit pill with drag reorder, width cycle, Alt-click and Shift+Enter morph; roving-tabindex rail and live region; five shelf pills (profiles with "+", theme, style, edit, customize); new-profile dialog (custom profiles); Customizer segmented width control; cards anchored to the slot with a caret, tile resting while open.
- Packages: U2.7, U2.8, U3.1, U3.2, U3.3 (Apps tab waits for phase G), U4.1 (`main.rs` placement and `caret_x`).
- Hot files: `DockSurface.tsx`, `layout.ts`, `dock-final.css`, `main.rs`, `WidgetManager.tsx`. Depends: D.
- Exit: edits persist across relaunch in two profiles; keyboard-only pass of DS 4.9; caret within 1px of the tile centre; Escape priority.

### Phase F: system widgets with real data
- Outcome: status-battery, music-player with artwork and transport, system-stats capsule with history ring buffer, calendar card with week strip, clock modes; providers polled only for mounted widgets.
- Packages: XD-11 demand-driven subscriptions first (R11), BE-11 plus battery tile and card, BE-15, BE-16 and `media_artwork` registration (BE-49 part) plus music UI, system-stats UI, BE-39 plus calendar card. BE-21 only if the user asks for battery health rows.
- Hot files: `useFinalWidgetData.ts`, `widget_data.rs`, `local_calendar.rs`, `main.rs`, `build.rs`, `commands.toml`. Depends: E.
- Exit: values match Windows Settings and Task Manager; honest states for no battery and nothing playing.

### Phase G: user widgets
- Outcome: weather setup with city search and "use my location" (copy says the typed city leaves the machine), quick notes saved as `.txt` and found by `notes:` in Spotlight, Claude (tokens only) and Codex cards, pinned apps from the installed catalog.
- Packages: BE-18, BE-19, BE-20 (BD-2), BE-34, BE-35, BE-36, BE-49 registrations, weather, notes, AI and pinned UI, U3.3 Apps tab.
- Hot files: `main.rs`, `build.rs`, `commands.toml`, capabilities, `native.ts`, `WidgetManager.tsx`, `protonsearch/src/search.rs` (one line). Depends: F.
- Exit: BS 6.10 tests 7 and 8; pinned icons crisp at the user's scale over light and dark rails.

### Phase H: connectivity
- Outcome: Wi-Fi tile and card (SSID, signal, band, saved-network connect and disconnect, Settings handoff), Bluetooth tile and card, peripherals battery levels.
- Packages: BE-31 spike first, BE-30 (split enumeration and provider arm), BE-32 (split `wifi` and `wifi-networks`), BE-33 (BD-3), tiles and cards with `permission-denied` remedies.
- Hot files: `widget_data.rs`, `providers.rs`, `wifi.rs`, `src-tauri/Cargo.toml`. Depends: G.
- Exit: real SSID and networks; disconnect and reconnect a saved network only with the user present (never automated); device level equals Windows Settings.

### Phase I: Spotlight (Kensaku) UI on the existing engine
- Outcome: Quick Look drawer, list or grid, calculator hero, engine footer, reveal and copy path, search sub-label from the `hotkey` provider. The engine stays authoritative; no second index.
- Packages: U5 (re-pin first: the HTML search script drifted), BE-40, BE-41, BE-42, remaining BE-49.
- Hot files: `SpotlightWindow.tsx`, `main.rs`, `build.rs`, `commands.toml`. Depends: H.
- Exit: query, arrows, Enter, Escape; preview of a text file; reveal in Explorer; copy path.

### Phase J: edges and alignment (D5 override)
- Outcome: dock on bottom, left or right, aligned start, center or end; Start or any widget anywhere in the order.
- Packages: layout v3 additive `dock: {edge, align}` (R13, whitelisted, default bottom and center), shelf control, edge and align sent to `fit_dock` (old T5.3), vertical rail CSS for 48px-high mixed-width tiles (old T5.4 adapted), vertical `ALONG_PAD` for the last-tile extent, card placement per edge in `main.rs`.
- Hot files: `layout.ts`, `DockSurface.tsx`, `fit.ts`, `dock-tiles.css`, `main.rs`, `window_geometry.rs`. Depends: I.
- Exit: bar 1 checks on all three edges; cards anchored per edge; edge survives relaunch.

### Phase K: taskbar (BD-9 decided)
- Outcome: the dock reserves its space as an AppBar; maximized windows stop at the dock; "Hide Windows taskbar" switch in Customize (off by default) uses auto-hide and is restored on exit and after a crash.
- Packages: BE-70 (dedicated message thread, `ABM_NEW`, `ABM_QUERYPOS`, `ABM_SETPOS`, `ABM_REMOVE`; placement from the AppBar rect, not the shrunken work area), BE-71 (`ABM_SETSTATE` with `ABS_AUTOHIDE`, previous state restored on exit and by `--restore-taskbar`, never `SW_HIDE`), BE-50 capability split, BE-51 trash confirmation (BD-10).
- Hot files: `main.rs`, capabilities, `commands.toml`, `WidgetManager.tsx`. Depends: J.
- Exit: maximized window bottom equals the reservation at the user's scale; toggle on and off; kill the process and run `--restore-taskbar`; Start, tray flyout and toasts still work.

### Phase L: gated and blocked
- BE-37 then BE-38 notifications: needs the user's consent to read their own notifications for the spike and the opt-in widget; badges BE-69 need BD-14 reversed. Tray: BE-63 spike, else BE-64 (BD-5). AirPods left, right, case: BD-4 reversed. Clipboard: BD-12 (BE-65 first). Recurrence: BD-11 (BE-66). Assistant: a product definition (BD-6). Admin usage APIs and credentials: excluded by BD-13.

## 4. Carry-over notes from reviews (orchestrator)

- A9 also applies the two A11 fixes for a3d9e11: (1) `src-tauri/src/running_apps.rs:169`: change `if GetIconInfo(handle, &mut info).is_err() || info.hbmColor.is_invalid() {` to `if GetIconInfo(handle, &mut info).is_err() {` (the existing `read == 0` branch then frees both bitmaps); (2) in `renders_windows_shared_icon_as_png` (about line 407) bind `let png = render_icon(icon).unwrap();`, keep the prefix assert, add `let side = icon_size(icon).unwrap(); assert_eq!(png_dimensions(&png), (side, side));`.
- Phase B (U2.5 two-phase fit) also takes A11 fix 3: in `src/components/dock/DockSurface.tsx` change the fit `.catch(() => {})` to `.catch(() => { if (lastFit.current === next) lastFit.current = null; })`, with a DockShell test where the first `fit_dock` rejects, a `resize` follows and a second `fit_dock` is expected. Also note: a profile wider than the window grows about 80 px per fit round trip until U2.5 lands.
- Later backend phases (F, H): registry providers run outside `NATIVE_LOCK`; the first provider that touches WinRT or Core Audio must make that lock reachable and take it.
- IC-A: `ai_usage::tests::reads_newest_codex_limits_and_agent_metadata_without_inventing_quota` is flaky across a minute rollover (fixture `now + 4h12m`); rerun once, and if it fails again fix it by injecting a fixed `now` in the test only. Run N-A1 exactly as A11 listed (idle 60 s shots every 2 s with edge crops; ten hover cycles judged 700 ms after leave; five Daily to Focus round trips plus one narrow-to-wide burst; three relaunches; current display scale only) and N-A2 after A9 (Notepad tile beside its taskbar icon at rest and at hover peak; one small-icon app; hover smoothness with many apps). Update the "review pending" headings at `docs/qa/dock-defects-2026-11-07.md` lines 5 and 37 with the A11 verdicts.
- A12 (new, from the A10 review; UI lane, runs alone in the UI lane, before IC-A): blur must reset magnification and pointer movement must re-heat the rail. (1) `src/components/dock/magnify.ts`: after the `pointerleave` listener add `window.addEventListener('blur', leave);` and in the cleanup, after the `pointerleave` removal, `window.removeEventListener('blur', leave);`. (2) `src/components/dock/DockSurface.tsx`: add `onPointerMove={heat}` to the `.dock-final-rail` element. (3) `src/components/dock/magnify.test.tsx`, after `leaving restores rest scale`: test `window blur restores rest scale` (pointerMove clientX 82, dispatch window `blur`, expect `cells[1]` `--mag` `'1.000'`). (4) `src/components/dock/DockSurface.test.tsx`: test `heats again on pointer move after blur` (pointerEnter, act(blur), pointerMove on the rail, expect `data-hot` `'true'`). Optional only if N-A3 shows a tooltip sliver over the returned shelf: `.dock-final:not([data-hot="true"]) .dock-cell:hover > .dock-tip { opacity: 0; }` in dock-tiles.css. Phase B U2.2 (useMagnify rewrite) must keep the blur reset and its test. N-A3 keeps the mouse still.

### A13 Weather and calendar read the layout through layout_store

All paths are relative to `C:\Users\Pranshul\Documents\Projects\protonsearch`.

- **Lane and order.**
  - Backend lane. Depends on A9, and needs A5 and A6 landed.
  - `main.rs` is hot, so one task at a time.
  - `local_calendar.rs` editor order: A6, then A13, then any BE-39 task.
  - Land before IC-A.
- **Why (BE-04 follow-up from the A4 review).**
  - `weather_snapshot` and `local_calendar::active_profile` still open `protondock-layout.json` themselves. With a corrupt file and a good backup, the dock loads the backup but weather errors and every calendar command fails.
  - After A13, `grep -rn "protondock-layout" src-tauri/src protonsearch/src` prints only the two constant lines in `src-tauri/src/layout_store.rs`.
- **Files (exact).**
  - `protonsearch/src/weather.rs`
  - `src-tauri/src/local_calendar.rs`
  - `src-tauri/src/main.rs`
  - `src-tauri/tests/layout_store.rs` (append one test)
  - `src-tauri/tests/local_calendar.rs` (add one include, one `use`, one test)
- **Stop and report** if A6's `active_profile` or `profile_is_active` differs from A6's spec, or if any caller of the changed functions exists outside `main.rs` and `local_calendar.rs`'s `mod tests`.

**1. `weather.rs`**

Replace the whole `snapshot` function (the doc line plus the body that opens the file by path):
```rust
/// Read the saved layout's active-profile opt-in before any external request. `Null` means nothing is saved.
pub fn snapshot(layout: &Value) -> Result<Value, String> {
    let coordinates = if layout.is_null() { None } else { location(layout)? };
    let Some(coordinates) = coordinates else {
        return Ok(json!({"status":"disconnected","updatedAt":0}));
    };
    snapshot_for(coordinates)
}
```
- Delete the `path::Path,` line from the `use std::{...}` block. `Path` has no other use in the file.
- Keep `io::Read` and `bounded_json`; `fetch` and an existing test still use them.

**2. `local_calendar.rs` (written against the post-A6 file)**

- Replace `active_profile` entirely:
```rust
// The layout is loaded on every check on purpose: select_source and snapshot re-check after slow work, so a profile switch in between must be seen.
fn active_profile(
    load_layout: &dyn Fn() -> Result<Value, String>,
    profile: &str,
) -> Result<bool, String> {
    profile_name(profile)?;
    let layout = load_layout().map_err(|_| "Native dock profile could not be validated")?;
    profile_is_active((!layout.is_null()).then_some(&layout), profile)
}
#[cfg(test)]
fn no_layout() -> Result<Value, String> {
    Ok(Value::Null)
}
```
- Add `load_layout: &dyn Fn() -> Result<Value, String>` as the last parameter of three functions:
  - `pub fn select_source(directory: PathBuf, profile: String, owner: isize, load_layout: ...) -> Result<Option<Value>, String>`
  - `pub fn snapshot(directory: &Path, profile: &str, token: &str, load_layout: ...) -> Result<Value, String>`
  - `pub fn forget_source(directory: &Path, profile: &str, token: &str, load_layout: ...) -> Result<(), String>`
- Update the five call sites:
  - `select_source`, two places: `active_profile(&directory, &profile)?` becomes `active_profile(load_layout, &profile)?`.
  - `snapshot`, two places, and `forget_source`, one place: `active_profile(directory, profile)?` becomes `active_profile(load_layout, profile)?`.
- Existing `mod tests`:
  - Add `&no_layout` as the last argument to all 11 calls: 7 `snapshot(&fixture.0, ...)` and 4 `forget_source(&fixture.0, ...)`.
  - `no_layout` means no saved layout with daily active, which is the old NotFound behaviour.
  - These fixture tests still fail at runtime here (known `create_dir` error 5). They only have to compile, and the failure count must not change.
- Unchanged:
  - No `crate::` in this file.
  - `bounded_json` stays, because `grants` uses it for `calendar-sources.json`.
  - `mod ics;`, `GRANTS`, `grants`, `save_grants`, `profile_name`, `profile_is_active`, `check_grants` and all existing error strings stay as they are.

**3. `main.rs`**

`weather_snapshot`, the `None` arm (the coordinates arm is untouched). Replace `None => protonsearch::weather::snapshot(&data_dir()?.join("protondock-layout.json")),` with:
```rust
        None => {
            let layout = layout_store::load(&data_dir()?)
                .map_err(|_| "Cannot read weather settings".to_string())?;
            protonsearch::weather::snapshot(&layout)
        }
```
Calendar commands, inside their existing `spawn_blocking(move || ...)`:
```rust
local_calendar::select_source(directory.clone(), profile, owner, &|| {
    layout_store::load(&directory)
})
local_calendar::snapshot(&directory, &profile, &token, &|| layout_store::load(&directory))
local_calendar::forget_source(&directory, &profile, &token, &|| layout_store::load(&directory))
```
- `directory.clone()` is required in `select_source`, because the closure borrows `directory` after the call moves it.
- No persistence lock is added. These readers were lock-free before, `save` replaces the file atomically, and it writes the backup only while the file is valid.
- No new command, `build.rs`, `permissions/commands.toml`, capability, `Cargo.toml` or `Cargo.lock` change, and no frontend change.

**Behaviour after**
- Layout file missing: weather is `disconnected` and calendar is daily-only, as before.
- File corrupt, backup valid: both read the backup (new).
- Both bad: weather returns `Err("Cannot read weather settings")` and calendar returns `Err("Native dock profile could not be validated")`.
  - The old texts were "Invalid weather response" and "Native calendar configuration is invalid".
  - A grep of `src/`, `docs/` and the Rust sources found no consumer of those strings.
- The old 1 MiB read bound in the two readers is gone. `layout_store::save` caps the file at 1 MiB, and `load_dock_settings` already reads unbounded.

**Tests**

`weather.rs`, appended to the existing `mod tests` (uses its `layout(settings)` helper):
```rust
    #[test]
    fn snapshot_without_a_saved_layout_or_opt_in_is_disconnected_without_a_fetch() {
        for saved in [Value::Null, layout(json!({})), layout(json!({"weatherEnabled":false}))] {
            let result = snapshot(&saved).unwrap();
            assert_eq!(result["status"], "disconnected");
            assert_eq!(result["updatedAt"], 0);
        }
    }

    #[test]
    fn snapshot_rejects_an_invalid_layout_before_any_fetch() {
        assert_eq!(
            snapshot(&json!({"version":9,"profile":"daily"})).unwrap_err(),
            "Invalid weather layout version"
        );
        assert_eq!(
            snapshot(&layout(
                json!({"weatherEnabled":true,"weatherLatitude":91,"weatherLongitude":0})
            ))
            .unwrap_err(),
            "Invalid weather latitude"
        );
    }
```

`local_calendar.rs`, appended to A6's `mod profile_tests` (pure; uses `super::*` only, as the file's `mod tests` does):
```rust
    #[test]
    fn active_profile_reads_the_loader_on_every_call() {
        let calls = std::cell::Cell::new(0);
        let layout = || -> Result<Value, String> {
            calls.set(calls.get() + 1);
            let profile = if calls.get() == 1 { "daily" } else { "founder" };
            Ok(json!({"version":1,"profile":profile}))
        };
        assert_eq!(active_profile(&layout, "daily"), Ok(true));
        assert_eq!(active_profile(&layout, "daily"), Ok(false));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn active_profile_treats_a_null_layout_as_daily_and_hides_load_errors() {
        assert_eq!(active_profile(&no_layout, "daily"), Ok(true));
        assert_eq!(active_profile(&no_layout, "founder"), Ok(false));
        let broken = || -> Result<Value, String> { Err("os error 5 at secret path".into()) };
        assert_eq!(
            active_profile(&broken, "daily"),
            Err("Native dock profile could not be validated".to_string())
        );
    }
```

`tests/layout_store.rs`, appended (uses the file's existing `fresh`). Corrupt FILE with a valid BACKUP still yields a weather snapshot:
```rust
#[test]
fn weather_reads_the_backup_when_the_file_is_corrupt() {
    let dir = fresh("weather_reads_the_backup_when_the_file_is_corrupt");
    let layout = |settings: Value| {
        json!({"version":1,"profile":"daily","profiles":{"daily":[{"id":"weather","settings":settings}]}})
    };
    layout_store::save(&dir, &layout(json!({"weatherEnabled":false}))).unwrap();
    layout_store::save(
        &dir,
        &layout(json!({"weatherEnabled":true,"weatherLatitude":91,"weatherLongitude":0})),
    )
    .unwrap();
    // FILE errors before any fetch; only BACKUP answers "disconnected".
    let snapshot = || protonsearch::weather::snapshot(&layout_store::load(&dir).unwrap());
    assert_eq!(snapshot().unwrap_err(), "Invalid weather latitude");
    std::fs::write(dir.join(layout_store::FILE), "{trunc").unwrap();
    assert_eq!(snapshot().unwrap()["status"], "disconnected");
}
```

`tests/local_calendar.rs`. Keep the existing three lines (the `#[allow(dead_code)]` comment, the `#[path = "../src/local_calendar.rs"]` include and `mod local_calendar;`) and add below them. Corrupt FILE with a valid BACKUP yields the calendar active profile from the backup:
```rust
#[path = "../src/layout_store.rs"]
mod layout_store;

use serde_json::json;

#[test]
fn calendar_profile_comes_from_the_backup_when_the_layout_file_is_corrupt() {
    const TOKEN: &str = "0123456789abcdef0123456789abcdef";
    const INACTIVE: &str = "Calendar profile changed; source was not removed";
    const ACTIVE: &str = "Calendar source is not authorized for this profile";
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("calendar_profile_comes_from_the_backup");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // FILE will say daily; the previous good copy says founder.
    layout_store::save(&dir, &json!({"version":1,"profile":"founder"})).unwrap();
    layout_store::save(&dir, &json!({"version":1,"profile":"daily"})).unwrap();
    let layout = || layout_store::load(&dir);
    let forget =
        |profile: &str| local_calendar::forget_source(&dir, profile, TOKEN, &layout).unwrap_err();
    assert_eq!(forget("daily"), ACTIVE);
    assert_eq!(forget("founder"), INACTIVE);
    std::fs::write(dir.join(layout_store::FILE), "{trunc").unwrap();
    assert_eq!(forget("daily"), INACTIVE);
    assert_eq!(forget("founder"), ACTIVE);
}
```
No test writes under `std::env::temp_dir()`.

**Verify (from the repo root)**
1. Before editing, run `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::tests::` and note the failed count.
2. After editing, run these:
   - `cargo test --manifest-path protonsearch/Cargo.toml --lib weather`
   - `cargo test --manifest-path src-tauri/Cargo.toml --test layout_store`: 8 passed.
   - `cargo test --manifest-path src-tauri/Cargo.toml --test local_calendar -- --skip local_calendar::tests::`: 0 failed.
   - `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::profile_tests`
   - `cargo test --manifest-path src-tauri/Cargo.toml -- --skip local_calendar::tests::`: 0 failures.
   - `cargo test --manifest-path src-tauri/Cargo.toml local_calendar::tests::`: failed count equals step 1.
   - `grep -rn "protondock-layout" src-tauri/src protonsearch/src`: only the two lines in `src-tauri/src/layout_store.rs`.
   - `git diff --stat`: only the five files above.

**Commit:** `fix(native): read the layout through layout_store in weather and calendar`

Push by pathspec on the five files, per the section 0 protocol.

**Review**
- `tests/local_calendar.rs` keeps its original three lines and compiles.
- All five `active_profile` call sites pass `load_layout`.
- `weather.rs` no longer imports `Path`.
- Only the two fixed error strings appear on load failure.
- `no_layout` is defined once and all 11 test calls are updated.
- Both corrupt-file tests flip from the file's answer to the backup's answer.

**IC-A wiring addition:** `weather_snapshot` and the three calendar commands load through `layout_store::load`.

## 5. More carry-over notes (orchestrator)

- A13 also takes the A7 review follow-up (BE-08 says a hotkey failure is logged): in `src-tauri/src/main.rs` setup, replace the `let ok = ...register(...)...is_ok()` line and the `record` call with `let result = app.global_shortcut().register(shortcut.as_str()); if let Err(error) = &result { protonsearch::applog::log(&format!("Hotkey {shortcut} not registered: {error}")); } providers::hotkey_status::record(&shortcut, result.is_ok());`.
- UI phase that renders Wi-Fi (C or H): A8 added the `settings:location` action and the `accessDenied` field in Rust. Update the TS action unions (`src/services/native.ts`, `src/widgets/final/types.ts`, `src/components/dock/actions.ts`), read `accessDenied` in `useFinalWidgetData.ts`, and add `settings:location` to `docs/widgets/CONTRACT.md` (quick-controls list).
- IC-A records as "needs user" (not verified unless the user switches the setting): Wi-Fi with location consent off (A8); the dock starting while another program holds Alt+Space, with the failure listed by the `hotkey` provider (A7, BE-08).
