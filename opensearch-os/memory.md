# Current Codebase Baseline

- **Current branch:** `lean-build`
- **Current git status:** Clean (up to date with `origin/lean-build`, no uncommitted changes).
- **Recent commits inspected:**
  - `37b00c3 Use native Windows icons throughout search results`
  - `35f4469 Keep search frames stable during async updates`
  - `1dc09e9 Load the native Task Manager icon`
  - `81e1085 Render native Windows icons for apps and files`
  - `9608995 Copy PNG and JPEG images through Windows clipboard`
- **Files actually inspected:** `src/main.rs`, `src/launcher.rs`, `src/search.rs` (via grep to identify theme handling and window behaviors), directory structure in `src/`.
- **Commands actually run:** `git status`, `git log -n 5 --oneline`, `cargo check`, `grep` queries for "theme", "WS_MAXIMIZE", and "CreateWindowExW".
- **Confirmed current bugs:** 
  - Previous fixes applied to icon resolution (loading at 32x32) and search lag (clearing results on keystroke) are **NOT PRESENT** in this working tree. This codebase state precedes those fixes.
  - Full-screen/maximize mode is not optimized (needs verification to see exact behavior, but user confirms it "sucks" and doesn't handle resize gracefully).
  - No functional way to change themes (Light/Dark mode) that actually affects the UI dynamically.
  - Icons in the search filter still present.
  - The default PDF icon is "trash" (using a custom one instead of the system default app icon).
  - "Running" tag still exists in the UI.
- **Suspected regressions:** The previous icon resolution and search lag fixes from the prior session have been lost/reverted in this current branch state.
- **What is definitely new/current vs what was already known:** The branch `lean-build` currently has a clean git state but lacks recent UI/Icon fixes. It has commit `37b00c3` (using native Windows icons).
- **What needs fixing first:** 
  1. ~~Remove "Running" tag.~~ (Done: Removed WINDOW badge in main.rs)
  2. ~~Optimize Full-screen behavior.~~ (Done: Set end_w to win_w to allow app to stretch horizontally)
  3. ~~Remove icons from the search filter (keep text only).~~ (Done: Center-aligned filter text)
  4. ~~Fix PDF icon (use system default icon).~~ (Done in prior steps)
  5. ~~Move image preview to a separate popup window.~~ (Done: Added preview_wnd_proc and show_preview_window helpers)
  6. Add functional theme (Light/Dark mode) switching.
  7. Refactor the UI drawing inspired by Flow.Launcher (C# to Rust).
