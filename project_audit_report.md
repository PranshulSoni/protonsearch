# Project Audit Report

**Project:** Omnisearch / Project-Raycast  
**Report date:** 2026-07-03  
**Scope:** Consolidated audit from uploaded notes covering `main.rs`, `search.rs`, `launcher.rs`, `settings_ui.rs`, `indexer.rs`, `browser_indexer.rs`, `git_indexer.rs`, installer/version files, and related UI/database paths.

---

## 1. Executive Summary

The audit found several release-blocking defects, mainly in Win32 lifecycle handling, Rust reentrancy/aliasing, clipboard synchronization, Unicode string slicing, and search/indexer worker resilience.

### Highest-priority blockers

| ID | Severity | Area | Summary |
|---|---:|---|---|
| C-01 | Critical | Win32/Rust state aliasing | `animate_window()` holds `&mut State` across synchronous Win32 calls that can re-enter `wnd_proc_inner`, producing overlapping mutable aliases and possible undefined behavior. |
| C-02 | Critical | Window teardown | `WM_DESTROY` frees `State` with `Box::from_raw` while `GWLP_USERDATA` still points to freed memory and most timers remain active. |
| C-03 | Critical | Search calculator | `try_pct_of()` uses an index from a lowercased string to slice the original string, allowing a Unicode query like `K% of 5` to panic and kill search workers. |
| H-01 | High | Clipboard | Most `OpenClipboard`/`SetClipboardData`/`GetClipboardData` paths bypass `clipboard_lock()`, so the lock does not actually serialize clipboard access. |
| H-02 | High | Indexer | `IS_INDEXING` is not reset if `run_indexer_folders_inner()` panics, permanently disabling indexing until restart. |
| H-03 | High | Search engine init | `SearchEngine::new()` raw-slices embedded `CATALOG` data and can panic on malformed/corrupt catalog assets, killing engine initialization. |
| H-04 | High | Git/search timestamp | Forged git commit timestamps can overflow `format_timestamp_local()` and produce garbage dates or debug-build panics. |
| H-05 | High | Search result metadata | ChatGPT quick action source tag changed from `LIVE` to empty string, causing the renderer to show the wrong badge. |

### Consolidated finding count

| Severity | Count |
|---|---:|
| Critical | 3 |
| High | 6 |
| Medium | 10 |
| Low | 7 |
| Verified clean / explicitly checked | 18 |

> Duplicate findings across the uploaded notes were merged into one consolidated entry where appropriate, especially clipboard-lock coverage.

---

## 2. Critical Findings

### C-01 — Reentrant Win32 calls create overlapping `&mut State` aliases

**Severity:** Critical  
**Files/locations:** `src/main.rs`, especially `animate_window()`, `ShowWindow(hwnd, SW_SHOWNOACTIVATE)`, `force_foreground(hwnd)`, and `wnd_proc_inner()` re-entry paths.

#### Issue

`animate_window()` materializes a mutable reference to `State` from the window's `GWLP_USERDATA` pointer and keeps it live across synchronous Win32 calls such as:

- `ShowWindow(hwnd, SW_SHOWNOACTIVATE)`
- `SetFocus(hwnd)` inside `force_foreground()`
- `SetForegroundWindow(hwnd)` inside foregrounding logic

These calls can synchronously send messages such as `WM_ACTIVATE`, `WM_SETFOCUS`, `WM_NCACTIVATE`, `WM_WINDOWPOSCHANGED`, etc. back to the same window. During that reentrant dispatch, `wnd_proc_inner()` again reads `GWLP_USERDATA` and creates another `&mut State` pointing to the same object.

That creates two live mutable references to the same `State`, violating Rust aliasing rules and risking undefined behavior or optimizer-dependent miscompilation.

#### Impact

Concrete effects can include:

- animation state corruption;
- window stuck hidden/visible incorrectly;
- timers left in an inconsistent state;
- state fields mutated while the outer animation frame still assumes they are stable;
- undefined behavior under optimization.

#### Recommended fix

- Do not hold `&mut State` across any Win32 call that may synchronously re-enter the window procedure.
- Restrict mutable borrows to short lexical scopes before the call.
- Prefer posting messages for follow-up state changes rather than calling focus/show APIs while state is mutably borrowed.
- Consider a reentrancy guard for animation transitions.

---

### C-02 — `WM_DESTROY` frees `State` while `GWLP_USERDATA` remains dangling

**Severity:** Critical  
**Files/locations:** `src/main.rs`, `WM_DESTROY` handler, `Box::from_raw(sp)`, timer cleanup.

#### Issue

`WM_DESTROY` frees the application `State` using `Box::from_raw(sp)`, but:

- `GWLP_USERDATA` is not cleared with `SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0)`;
- `WM_NCDESTROY` is not used for final state cleanup;
- only `TIMER_SEARCH_ANIM` is killed;
- `TIMER_DEBOUNCE`, `TIMER_CURSOR_BLINK`, `TIMER_AI_ANIM`, and `TIMER_ICON_BATCH` remain possible pending sources of `WM_TIMER`.

After `Box::from_raw(sp)`, the window still contains a stale pointer. Any later dispatched message before full teardown can dereference freed heap memory.

#### Impact

Normal exit paths can trigger this, including tray-menu exit and command-triggered `WM_CLOSE`. A pending timer or late message can re-enter `wnd_proc_inner()`, retrieve the freed pointer, and use it as valid `State`.

This is a classic use-after-free window.

#### Recommended fix

- Move final `State` deallocation to `WM_NCDESTROY`.
- In `WM_DESTROY`, kill **all** timers and initiate shutdown only.
- Before freeing state, call `SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0)`.
- Make every message handler tolerate `sp == null` after shutdown begins.

---

### C-03 — Unicode query can panic `try_pct_of()` and kill search workers

**Severity:** Critical  
**Files/locations:** `src/search.rs`, `try_pct_of()`, around `% of` calculator parsing; worker threads in `src/main.rs`.

#### Issue

`try_pct_of()` lowercases the query and calls `.find("% of ")` on the lowercased copy, then uses that byte offset to slice the original string:

- index source: `lower.find("% of ")`
- slicing target: original `s[..idx]` and `s[idx + 5..]`

This is unsafe because lowercasing can change UTF-8 byte length. A concrete repro is a query using the Unicode Kelvin Sign `K` before `% of`, such as:

```text
K% of 5
```

The lowercase string has a different byte layout, so the index can land inside a multibyte character in the original string, causing a Rust panic: byte index is not a char boundary.

#### Impact

The calculator path is reached from ordinary query handling without requiring a special prefix. Search work runs on dedicated fast/slow worker threads that do not have `catch_unwind` protection. A panic kills the worker thread. Since sends ignore errors and there is no worker respawn/health check, search can silently stop working until app restart.

#### Recommended fix

- Never reuse byte offsets from a transformed string to slice the original string.
- Use an ASCII-only case-insensitive search helper that returns offsets into the original string, similar to the already-audited `find_ascii_ci()` approach.
- Alternatively parse `% of` from the original string directly.
- Add regression tests for `K% of 5`, Turkish dotted/dotless I cases, and other Unicode case-mapping edge cases.

---

## 3. High-Severity Findings

### H-01 — Clipboard synchronization is incomplete across the app

**Severity:** High  
**Files/locations:** `src/main.rs`, `src/launcher.rs`, all `OpenClipboard`/`GetClipboardData`/`SetClipboardData` paths.

#### Issue

The codebase defines `clipboard_lock()` to serialize Windows clipboard access, but only a small subset of paths actually acquire it:

- `WM_CLIPBOARDUPDATE` path uses it.
- `launcher.rs::paste_sequentially()` uses it.

Many other paths bypass it, including:

- `copy_to_clipboard()`
- `paste_from_clipboard()`
- `copy_image_to_clipboard()`
- `capture_clipboard_dib_bmp_bytes()`
- `capture_clipboard_image_data()`
- `paste_clipboard_into_chat()`
- `paste_clipboard_into_query()`
- `save_clipboard_image()` via some call paths
- `launcher.rs` `clearclip`
- multiple Ctrl+C/Ctrl+V handlers and clipboard-history actions

`copy_image_to_clipboard()` is especially important because one audited path calls it from a spawned background thread, making cross-thread clipboard races concrete.

#### Impact

While `paste_sequentially()` is running on a background thread and repeatedly writing/pasting clipboard contents, main-thread actions can simultaneously open or mutate the clipboard without the mutex. Failures are often silently swallowed, so the user may see:

- wrong item pasted;
- empty clipboard reads;
- image copy silently failing;
- `clearclip` silently no-oping;
- sequential paste corruption.

#### Recommended fix

- Move locking inside the low-level clipboard helper functions, not only at selected callers.
- Make every function that touches the Windows clipboard acquire the same mutex.
- Replace `.lock().unwrap()` with poison-tolerant handling because the mutex protects no data, only access sequencing.
- Consider retry/backoff for `OpenClipboard()` because transient failure is normal on Windows.

---

### H-02 — `IS_INDEXING` can stay `true` forever after panic

**Severity:** High  
**File/location:** `src/indexer.rs`, `run_indexer_folders()`.

#### Issue

`run_indexer_folders()` sets `IS_INDEXING` to `true`, calls `run_indexer_folders_inner()`, then resets it to `false` only after normal return.

If `run_indexer_folders_inner()` panics, the reset line is skipped. Future indexing requests hit the early guard and return immediately forever.

#### Impact

A single panic during indexing permanently disables indexing until app restart. The user receives stale results and may not get a visible error.

#### Recommended fix

- Use an RAII guard whose `Drop` resets `IS_INDEXING`.
- Optionally wrap the inner run with `catch_unwind` and convert panics into logged errors.
- Keep the existing `Result`-based error propagation, but make the global flag panic-safe.

---

### H-03 — Corrupt embedded catalog can panic `SearchEngine::new()`

**Severity:** High  
**File/location:** `src/search.rs`, `SearchEngine::new()`, embedded `CATALOG` parsing.

#### Issue

`SearchEngine::new()` raw-slices the embedded catalog buffer with expressions like:

- `CATALOG[off..off + vb]`
- `CATALOG[off..off + 2]`

The surrounding code otherwise uses `Result` and `bail!`, but these slices can panic if the embedded catalog header and payload length disagree due to asset corruption, bad build packaging, or future format mismatch.

#### Impact

The search engine is initialized inside a spawned background thread. A panic there prevents `WM_ENGINE_READY` from being posted. The app can open without search becoming available, and the graceful error path is skipped because a panic is not an `Err`.

#### Recommended fix

- Replace raw indexing with `.get(range).ok_or_else(...)`.
- Validate header fields before using them for offsets.
- Convert malformed catalog data into a visible initialization error.
- Add a unit test with truncated catalog bytes.

---

### H-04 — Forged git timestamps can overflow date formatting

**Severity:** High  
**Files/locations:** `src/git_indexer.rs`, `src/search.rs`, `format_timestamp_local()`.

#### Issue

`git_indexer.rs` parses git author timestamps from `git log --format=...%at...` as `i64` without upper-bound validation. A hostile repository can contain commits with extreme author dates near `i64::MAX`.

This value flows into `format_timestamp_local()`, which computes:

```rust
(timestamp + 11644473600) * 10000000
```

using plain `i64`. Unlike `format_unix_date()`, this path does not use `i128` or negative/overflow guards.

#### Impact

- Debug builds can panic on overflow.
- Release builds wrap silently and display garbage dates.
- Reachable by indexing a malicious or malformed git repository.

#### Recommended fix

- Validate git timestamps before storing them.
- Clamp to a reasonable supported date range.
- Reuse the safer `i128` conversion pattern from `format_unix_date()`.
- Add tests for `i64::MAX`, negative timestamps, and far-future commits.

---

### H-05 — ChatGPT quick action lost the `LIVE` source tag

**Severity:** High  
**Files/locations:** `src/search.rs`, `src/main.rs` badge rendering.

#### Issue

The ChatGPT AI-prefix quick-action result changed from:

```rust
source: "LIVE".to_string()
```

to:

```rust
source: String::new()
```

The renderer has a live branch for source `LIVE` that displays a distinct green `LIVE` badge. With an empty source, the result falls through to the default badge branch and can be shown as `SET`, making an AI action look like a Settings result.

#### Impact

This is a UI regression and cross-file contract mismatch between `search.rs` result generation and `main.rs` rendering.

#### Recommended fix

- Restore `source: "LIVE".to_string()` for this branch, or introduce a shared enum/constant for result sources.
- Add a regression test that verifies ChatGPT quick actions produce the expected source and badge.

---

### H-06 — Unsafe blocks lack documented invariants

**Severity:** High  
**File/location:** `src/search.rs`, unsafe Win32/COM usage.

#### Issue

The audit found multiple unsafe blocks in `search.rs` with no `// SAFETY:` comments. The highest-risk instance is manual walking of a `PWSTR` returned by COM's `IShellItem::GetDisplayName()` using raw pointer arithmetic until a null terminator is found, without a defensive length cap.

#### Impact

The immediate code may work under normal COM contracts, but the invariants are undocumented and not locally reviewable. Future edits can easily break assumptions around pointer ownership, null termination, or buffer lifetime.

#### Recommended fix

- Add `// SAFETY:` comments before every unsafe block.
- For COM strings, document ownership and deallocation responsibility.
- Add a conservative maximum length while scanning raw pointers.
- Prefer helper wrappers that convert Win32 strings safely.

---

## 4. Medium-Severity Findings

### M-01 — `CoInitializeEx()` in Settings “Add Folder” thread is not balanced

**Severity:** Medium  
**File/location:** `src/settings_ui.rs`, add-folder background thread.

#### Issue

The add-folder thread calls `CoInitializeEx(None, COINIT_MULTITHREADED)` but never calls `CoUninitialize()` before the thread exits.

#### Impact

The leak is bounded to the thread lifetime, but it violates the Win32 COM contract and is inconsistent with other COM-using threads in the project.

#### Recommended fix

Use a small RAII COM guard:

```rust
struct ComGuard(bool);
impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize(); }
        }
    }
}
```

---

### M-02 — `CoUninitialize()` is unconditional in rebuild-index thread

**Severity:** Medium  
**File/location:** `src/settings_ui.rs`, rebuild-index background thread.

#### Issue

The rebuild-index thread calls `CoUninitialize()` even if `CoInitializeEx()` failed.

#### Impact

On a fresh thread this is usually low-risk, but it is still the wrong pattern. `CoUninitialize()` must only be paired with a successful `CoInitializeEx()`/`OleInitialize()` call.

#### Recommended fix

Capture `CoInitializeEx(...).is_ok()` and call `CoUninitialize()` only when true, matching the safer pattern used elsewhere.

---

### M-03 — Deleted files remain searchable indefinitely

**Severity:** Medium  
**File/location:** `src/indexer.rs`, watcher and full crawl.

#### Issue

The file watcher handles create/modify events but ignores remove events. The full crawl also explicitly skips deleted-file cleanup to avoid memory bloat.

#### Impact

Deleted files remain in `files` and `files_fts`, so search can return stale paths that no longer exist, or paths later reused for different content.

#### Recommended fix

- Handle `EventKind::Remove` by deleting the path from `files` and `files_fts`.
- Add a bounded cleanup strategy during full crawl, such as per-folder delete reconciliation or batched stale-path pruning.

---

### M-04 — `DrawTextW` empty-string guard is inconsistent

**Severity:** Medium  
**Files/locations:** `src/main.rs`, rendering paths around `control_name`, `breadcrumb_path`, and `ai_title`.

#### Issue

The codebase already documents that `DrawTextW` must not be called with an empty UTF-16 slice. Some paths are guarded, but others still pass state-derived strings without empty checks:

- `res.entry.control_name`
- `res.entry.breadcrumb_path`
- `s.ai_title`

The current audited construction paths mostly appear to produce non-empty values, but the defensive rule is not consistently enforced.

#### Impact

A future data source or ordering bug that produces an empty title/path can reproduce the native crash class the adjacent code comments already warn about.

#### Recommended fix

- Guard every `DrawTextW` call with `if !buf.is_empty()`.
- Add a small helper like `draw_text_nonempty(...)` and ban direct calls with dynamic buffers.
- Add debug assertions for fields expected to be non-empty.

---

### M-05 — `WM_CLIPBOARDUPDATE` spawns one OS thread per clipboard event

**Severity:** Medium  
**File/location:** `src/main.rs`, clipboard update handler.

#### Issue

Every clipboard update spawns a new `std::thread::spawn` for database write work. There is no debounce, pool, or cap.

#### Impact

Normal use is likely fine, but a script or misbehaving app that writes clipboard data in a tight loop can cause proportional OS thread growth and many SQLite connections.

#### Recommended fix

- Use a single clipboard worker thread with a channel.
- Coalesce rapid clipboard updates.
- Add queue limits or drop duplicate consecutive events.

---

### M-06 — Search/Icon request channels are unbounded

**Severity:** Medium  
**File/location:** `src/main.rs`, search/icon worker channels.

#### Issue

Search and icon workers use unbounded `std::sync::mpsc::channel()` queues.

The consumers coalesce requests, which reduces practical risk, but the queue itself has no capacity limit or backpressure.

#### Impact

Under pathological producer load, memory can grow without a hard cap.

#### Recommended fix

Use bounded channels or a latest-request slot protected by a mutex/atomic sequence number.

---

### M-07 — Unit conversion formatter can display non-finite values

**Severity:** Medium  
**File/location:** `src/search.rs`, `try_unit_convert()` / `fmt_conv()`.

#### Issue

The unit converter handled tested edge cases without panics, including malformed numbers and very large numbers. However, `fmt_conv()` has no explicit `NaN`/`inf` rejection.

#### Impact

Not currently reachable with the audited linear unit table, but future zero multipliers or extreme conversions could display `NaN` or `inf` to the user.

#### Recommended fix

Add:

```rust
if !v.is_finite() { return None; }
```

before formatting conversion results.

---

### M-08 — Dead embedding/anchor-category subsystem remains in `search.rs`

**Severity:** Medium  
**File/location:** `src/search.rs`, `mean_pool_norm`, `AnchorCategory`, related fields/functions.

#### Issue

Clippy reported several dead fields/functions around embeddings, anchor categories, and conversational translation. The code appears disconnected from the live query path.

#### Impact

Dead code increases maintenance cost and can mislead future work. The audited math itself is safe, including the normalization divide guard.

#### Recommended fix

Either remove the subsystem or reconnect it with tests proving it is live.

---

### M-09 — Test mutates real production app database

**Severity:** Medium  
**File/location:** `src/search.rs`, `test_search_file`.

#### Issue

A test constructs its DB path from real `%APPDATA%\omnisearch\file_index.db` instead of using a temp fixture DB.

Running the test can execute `ensure_settings_catalog_fts()`, which deletes and repopulates a derived table in the developer's live app database.

#### Impact

Not destructive to source data, but still an unintended mutation of production user data during test execution.

#### Recommended fix

- Use a temp directory and fixture database.
- Mark environment-dependent tests with `#[ignore]` if needed.
- Never use `%APPDATA%` production paths in tests.

---

### M-10 — `clearclip` bypasses clipboard lock

**Severity:** Medium  
**File/location:** `src/launcher.rs`, `clearclip` command.

#### Issue

`clearclip` directly opens and empties the clipboard without acquiring `clipboard_lock()`.

#### Impact

If another background clipboard operation is active, `clearclip` can fail silently or interfere with sequential paste.

#### Recommended fix

Fold this into the systemic clipboard-lock fix from H-01.

---

## 5. Low-Severity Findings

### L-01 — Version strings use inconsistent formats

**Severity:** Low  
**Files/locations:** `Cargo.toml`, `installer.iss`, `src/settings_ui.rs`, `ui/settings.slint`.

#### Issue

Different files report different version formats:

- Cargo / installer: `1.0.5`
- Settings display: `1.05`
- Slint default literal: stale `1.02`

The update-check path uses `CARGO_PKG_VERSION` and is not broken, but the user-facing version can confuse users.

#### Recommended fix

Use one source of truth and one format. Generate UI/installer display strings from Cargo metadata where possible.

---

### L-02 — API key, endpoint, and model are saved without trimming

**Severity:** Low  
**File/location:** `src/settings_ui.rs`, AI settings save path.

#### Issue

Snippet fields are trimmed before saving, but AI settings are persisted as-is.

#### Impact

A copied API key with trailing whitespace/newline can produce malformed authorization headers.

#### Recommended fix

Trim values before saving and optionally validate empty/invalid fields.

---

### L-03 — `show_preview_window()` relies on fragile `selected >= scroll_offset` invariant

**Severity:** Low  
**File/location:** `src/main.rs`, preview positioning.

#### Issue

`visual_idx = selected - scroll_offset` uses plain `usize` subtraction. The audited navigation paths maintain the invariant, but the operation is not self-defending.

#### Impact

A future code path that changes `selected` and `scroll_offset` independently could underflow in release builds and compute a garbage row position.

#### Recommended fix

Use `saturating_sub()` or explicitly guard the invariant with a debug assertion.

---

### L-04 — Clipboard mutex poisoning can permanently break clipboard operations

**Severity:** Low  
**Files/locations:** `src/main.rs`, `src/launcher.rs`, `clipboard_lock().lock().unwrap()`.

#### Issue

The few current lock acquisitions use `.lock().unwrap()`. If any panic occurs while holding the mutex, subsequent clipboard operations will panic on poison.

#### Impact

Because the mutex guards only a unit value and not data consistency, poisoning is unnecessary and can cause permanent feature failure until restart.

#### Recommended fix

Recover from poison:

```rust
let guard = clipboard_lock().lock().unwrap_or_else(|e| e.into_inner());
```

---

### L-05 — Headless/desktop-dependent test can fail CI

**Severity:** Low  
**File/location:** `src/search.rs`, `test_enumerate_apps_folder`.

#### Issue

The test calls real Win32 shell/COM enumeration APIs and uses `.unwrap()`.

#### Impact

It may fail in headless CI or minimal Windows environments without a normal Explorer shell context.

#### Recommended fix

Mark as `#[ignore]`, gate on environment, or mock shell enumeration.

---

### L-06 — Duplicate timestamp formatting logic diverged

**Severity:** Low  
**File/location:** `src/search.rs`, `format_timestamp_local()` and `format_unix_date()`.

#### Issue

Two functions perform similar Unix-to-FILETIME conversions, but only one uses safer `i128`/negative-guard logic.

#### Impact

This inconsistency contributed directly to the git timestamp overflow issue.

#### Recommended fix

Unify both functions behind one safe conversion helper.

---

### L-07 — Clippy warnings remain unresolved

**Severity:** Low  
**Area:** Whole crate.

#### Issue

`cargo check` and `cargo fmt --check` passed in the uploaded audit, but `cargo clippy -D warnings` failed with many warnings. Most were style/dead-code issues rather than correctness blockers.

#### Impact

Clippy cannot currently be used as a clean CI gate.

#### Recommended fix

Triage warnings into:

1. correctness-relevant warnings;
2. dead-code cleanup;
3. style fixes;
4. intentionally allowed lints.

---

## 6. Verified Clean / Not Bugs

The audit explicitly checked the following areas and did **not** find bugs:

### Win32 / UI lifecycle

- `preview_wnd_proc` safely null-checks `sp` before dereference in `WM_PAINT`.
- `WM_NCCREATE`, `WM_CREATE`, `WM_NCCALCSIZE`, `WM_ERASEBKGND`, and related preview-window messages fall through safely where applicable.
- `TrackPopupMenu` path holds no `RefCell` borrow and no live `&mut State` across the nested menu loop.
- `Box::from_raw(sp)` occurs exactly once; no double-free was found. The issue is dangling pointer/timer cleanup, not duplicate free.
- Apparent `\ ponytail` / syntax anomalies in reads were confirmed to be rendering artifacts; actual source bytes are valid comments.

### Keyboard/string handling

- Cursor arithmetic using `cursor_pos - 1` and `chat_cursor_pos - 1` is guarded by `> 0` and followed by UTF-8 char-boundary walk-back logic.
- `word_left`, `word_right`, `floor_char_boundary`, and `delete_word_before` are char-boundary-aware and defensively clamped.
- Main result rendering and hit testing use `get()`/`saturating_sub()` patterns correctly in the audited paths.
- Query dispatch `strip_prefix(...).unwrap()` sites were checked and found to be guarded by matching prefix checks on the same string.
- `extract_path_or_url()` / `find_ascii_ci()` is implemented correctly for ASCII case-insensitive search without reusing lowercased-string offsets.

### Search / ranking / SQL

- All audited FTS `MATCH ?` sites use bound parameters with sanitized/alphanumeric-filtered query strings.
- SQL in settings, browser indexer, git indexer, and related DB paths uses parameterized queries; no SQL injection was found.
- Score sorting uses `partial_cmp(...).unwrap_or(Ordering::Equal)` consistently; no NaN sort panic found.
- Calculator parser is generally well-hardened with `Option` propagation, recursion-depth cap, division-by-zero checks, and non-finite result rejection in the main calculator path.
- Unit converter handled tested malformed inputs without panics.

### Indexing / filesystem / DB paths

- `WalkDir` default behavior does not follow symlinks, so symlink cycles were not found to be a real issue.
- `spawn_extractors`, `start_watcher`, and `start_indexer` correctly balance `CoInitializeEx()` / `CoUninitialize()` on the same thread, gated on success.
- Browser indexer timestamp units are internally consistent:
  - Chromium time converted from microseconds since 1601 to Unix microseconds;
  - Firefox `last_visit_date` already stored as Unix microseconds;
  - memory event insertion receives Unix seconds consistently.
- Temp DB file cleanup in browser indexer occurs after the SQLite connection is dropped.
- Persistent app DB paths consistently resolve under `%APPDATA%\omnisearch\file_index.db`; no stray `index.db` was found.
- `%APPDATA%`-derived paths are consistently suffixed with the `omnisearch` directory where applicable.

### Settings / update flow

- `slint::quit_event_loop().ok()` correctly lets the `--settings` process return from `main()` and exit.
- The installer flow waits for the main launcher process before spawning the installer; no blocking lingering-process issue was found.
- Hotkey validation is clean and avoids unwrap panics.
- `is_newer_version()` uses `CARGO_PKG_VERSION`, not `DISPLAY_VERSION`, so the displayed `1.05` string does not corrupt update comparison logic.

---

## 7. Recommended Fix Order

### Release blockers

1. **Fix C-03** — Unicode panic in `try_pct_of()` because it is trivial to trigger from normal typing and kills search workers.
2. **Fix C-02** — move `State` teardown to `WM_NCDESTROY`, clear `GWLP_USERDATA`, and kill all timers.
3. **Fix C-01** — remove live `&mut State` across synchronous Win32 calls.
4. **Fix H-01** — centralize clipboard locking in helper functions.
5. **Fix H-02** — make `IS_INDEXING` reset panic-safe.
6. **Fix H-03** — convert catalog panics into recoverable init errors.

### Quick wins

1. Restore ChatGPT quick-action `source: "LIVE".to_string()` or replace result sources with shared constants.
2. Replace all `DrawTextW` direct dynamic-buffer calls with a guarded helper.
3. Trim AI settings before saving.
4. Replace clipboard `.lock().unwrap()` with poison-tolerant handling.
5. Unify timestamp conversion helpers.
6. Move tests off real `%APPDATA%` paths.

### Follow-up hardening

1. Add `// SAFETY:` comments to all unsafe blocks.
2. Add panic guards or health checks to worker threads.
3. Add bounded/coalescing queues for clipboard, search, and icon workers.
4. Add deletion handling to the file indexer.
5. Clean or reconnect dead embedding/anchor-category code.

---

## 8. Regression Tests to Add

| Area | Test case |
|---|---|
| Unicode calculator | Query `K% of 5` must not panic and must return no result or a valid result safely. |
| Search workers | A panicking query path should not kill the worker permanently; worker should recover or be respawned. |
| Window teardown | Simulate shutdown with pending timers; no handler should dereference freed `State`. |
| Clipboard | Concurrent `paste_sequentially`, `clearclip`, and image copy should serialize through one lock. |
| Catalog loading | Truncated/corrupt `catalog.bin` should return `Err`, not panic. |
| Git timestamps | `i64::MAX`, negative, and far-future commit timestamps should be clamped/rejected. |
| DrawTextW | Empty strings passed through result rendering helpers must be skipped safely. |
| Indexer delete | Delete a watched file and verify it is removed from `files` and `files_fts`. |
| Settings tests | Running tests must not touch `%APPDATA%\omnisearch\file_index.db`. |

---

## 9. Final Risk Assessment

The project has several mature defensive patterns already in place: parameterized SQL, many UTF-8-safe cursor operations, safe score sorting, and partially hardened Win32 rendering. The remaining serious risks are concentrated in a few categories:

1. **Win32 reentrancy and teardown:** highest memory-safety risk.
2. **Worker-thread panics:** highest reliability risk for search/indexing.
3. **Clipboard synchronization:** highest data-race/user-visible correctness risk.
4. **Cross-file source-tag contracts:** UI regressions from stringly typed result sources.
5. **Test/environment hygiene:** avoid touching real user data during tests.

Fixing the critical/high items should be prioritized before another public release.


Some other errors:

High: overlay can hang forever after DestroyWindow because PostQuitMessage is not guaranteed on every destroy path
[omnisearch/src/circle_to_search.rs](C:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\omnisearch\src\circle_to_search.rs:200) handles WM_DESTROY by freeing data but does not call PostQuitMessage(0). Some paths do call it manually after DestroyWindow, but if Windows destroys the overlay from another path, the thread message loop at lines 35-39 can keep waiting forever. Put PostQuitMessage(0) in WM_DESTROY and remove duplicate manual calls.

High: selection can miss secondary monitors / negative-coordinate monitors
[omnisearch/src/circle_to_search.rs](C:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\omnisearch\src\circle_to_search.rs:52) uses SM_CXSCREEN / SM_CYSCREEN, which only covers the primary screen dimensions from (0,0). On multi-monitor setups, especially monitors left/up of primary, Circle to Search will not cover/capture the full desktop. Use virtual screen metrics: SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, and account for offsets in crop coordinates.

Medium: temp upload HTML path is predictable and can collide across rapid uses
[omnisearch/src/circle_to_search.rs](C:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\omnisearch\src\circle_to_search.rs:355) writes omnisearch_lens_<pid>.html. Two Lens searches within 60 seconds from the same process can overwrite each other, and a stale file from a failed cleanup can be reused. Include a timestamp/counter/random suffix and write with create_new(true) where practical.

Medium: the whole screenshot image is embedded in a local HTML file in temp for up to 60 seconds
[omnisearch/src/circle_to_search.rs](C:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\omnisearch\src\circle_to_search.rs:355) stores the selected region as base64 in a temp HTML file. That is a privacy-sensitive artifact. It is cleaned after 60 seconds only if the cleanup thread runs. Prefer direct browser upload only if needed, but at minimum use a unique temp file, delete sooner after browser launch, and document this behavior.

Medium: overlay dimming uses PATINVERT, which creates unpredictable colors rather than a controlled dim
[omnisearch/src/circle_to_search.rs](C:\Users\Pranshul Soni\Documents\Projects\Backend\Project-Raycast\omnisearch\src\circle_to_search.rs:229) creates a black brush but then calls PatBlt(..., PATINVERT), which ignores normal alpha/dim semantics and inverts pixels. This can look broken depending on the screen content. Use AlphaBlend with a semi-transparent black layer or a compatible 32-bit DIB overlay.