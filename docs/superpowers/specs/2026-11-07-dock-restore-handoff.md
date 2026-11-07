# ProtonDock Compact Dock Restore - Handoff Spec

Status: dock restore implemented, tested, and natively verified. Launcher untouched.
Branch: `dock`
Remote: `https://github.com/PranshulSoni/protonsearch`
HEAD: `82073c1 docs(qa): record restored compact dock`
Last code commit: `0042838 fix(dock): separate customize pill from profile tabs`
Initial restore commit: `e6727e0 Restore compact Tahoe dock rail`

## 1. Objective

Restore the original compact Tahoe-glass dock from the HTML prototype without
changing the launcher or search surfaces.

The user's exact correction:

- "not the launhcer, jjust the dock" (`m02751`).
- The larger 82px dock, removed animations, and changed proportions were rejected
  (`m02699`). The dock must feel like the original glass rail again.

Scope is dock only. Do not redesign the launcher, search, settings manager, or
widget cards in this task.

## 2. Final design contract

- Dock stage: 132px tall, full width, bottom anchored.
- Rail: 52px tall, graphite glass `rgba(14, 20, 32, 0.78)`, 18px radius,
  40px backdrop blur + 190% saturation + 104% contrast.
- Widget capsules: 36px tall, 10px radius, `rgba(255, 255, 255, 0.07)` surface.
- Rail gap: 5px; rail padding: 5px 8px.
- Profile shelf: 26px tall, 999px radius, 3px 4px padding, 10.5px semibold labels.
- Customize Widgets: its own 26px glass pill, 8px to the right of the profile pill.
  It must not sit inside the profile group.
- Daily rail content width: 1358px at standard density. Dense mode applies below
  1366px and uses the `DOCK_DENSE_WIDTHS` map.
- Hover: capsule lifts 2px with a 160ms spring transition; no magnification.
- Reduced motion: all dock transitions are disabled under
  `prefers-reduced-motion: reduce`.
- Native `fit_dock`: content width + 32px, height 132px, hit radii 15px for the
  profile shelf and 18px for the rail.

## 3. Files changed

- `src/design/dock-original.css` (new, 289 lines)
  Scoped Tahoe dock layer: rail, capsules, profile shelf, separate Customize pill,
  hover/lift, dense mode, reduced-motion.
- `src/components/dock/DockSurface.tsx`
  Compact/dense width model, rail `fit_dock`, native hit regions, width passed into
  each `DockWidget`. Renders profile tabs and Customize in a `dock-final-shelf`
  wrapper; the Customize button is a sibling of `dock-final-pills`.
- `src/components/dock/DockSurface.test.tsx`
  Geometry coverage plus a regression test that Customize is outside the profile
  tab group.
- `src/components/dock/DockShell.test.tsx`
  Native hit-region test now targets `.dock-final-shelf`; expected fit width
  unchanged.
- `src/widgets/final/DockWidget.tsx`
  Optional explicit dock width while retaining the default size map.
- `src/widgets/final/types.ts`
  Optional `width` prop.
- `src/main.tsx`
  Imports `dock-original.css` after the other dock styles.

No launcher, search, settings, Rust, provider, or package files were changed in
the dock restore commits.

## 4. Current native state

- Debug build: `src-tauri/target/debug/protondock.exe`.
- Test copy: `%TEMP%\protondock-docktest\protondock.exe`.
- The app was killed after verification. It is not running now.
- Daily dock: 1390x132 native window, restored 52px rail, separate Customize pill.
- Work dock: 1386x132.
- Founder dock can be wider because it has 10 widgets.
- Screenshots:
  - `docs/qa/native/dock-original-daily.png`
  - `docs/qa/native/dock-original-separate.png`
  - `docs/qa/native/dock-original-native.png`
- CDP screenshots show white around the transparent dock window. This is a
  compositor artifact from capturing a transparent WebView2 surface, not a CSS
  background. Desktop-transparency certification is still open.

The portable ZIP at `output/ProtonDock-0.1.0-x64-portable.zip` predates the dock
restore and still contains the old 82px dock. Do not distribute it as the restored
build. Rebuild the release and regenerate the ZIP before handoff.

## 5. Verification already run

- `npx tsc -b` passed.
- `npm test -- --reporter=dot` passed: 18 files, 186 tests.
- `npm run build` passed.
- `npm run tauri -- build --debug --no-bundle` passed with only the 5 pre-existing
  ProtonSearch warnings:
  - unused `vecs`, `dim`, `anchor_categories`
  - unused `search_local_files`, `get_conversational_results`
  - unused `mean_pool_norm`, `installed_apps`
  - crate `ProtonSearch` non-snake-case
- Native dock screenshot verified at 1390x132 with Daily profile.
- Profile switch to Work verified; the separate Customize pill remained correct.
- No `protondock` or `protonsearch` process was left running.

## 6. Build and run

From `C:\Users\Pranshul\Documents\Projects\protonsearch`:

```powershell
Get-Process protondock,protonsearch -ErrorAction SilentlyContinue | Stop-Process -Force
npx tsc -b
npm test -- --reporter=dot
npm run tauri -- build --debug --no-bundle
```

The project-tree debug executable can hit the pre-existing WebView2
`0x800700AA` location failure. Run a copy instead:

```powershell
$dir = "$env:TEMP\protondock-docktest"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Copy-Item 'src-tauri\target\debug\protondock.exe' $dir -Force
Start-Process -FilePath (Join-Path $dir 'protondock.exe') -WorkingDirectory $dir
```

For native CDP inspection:

```powershell
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9333'
Start-Process -FilePath (Join-Path $dir 'protondock.exe') -WorkingDirectory $dir
# http://127.0.0.1:9333/json exposes dock, search, settings, and widget targets
```

Port 9222 is normally occupied by Lightpanda. Use 9333.

## 7. Regression guardrails

- Do not reintroduce the 82px rail or 66px capsules.
- Do not merge Customize Widgets back into the profile pill; it must remain a
  separate glass pill.
- Do not remove hover lift or the spring transitions.
- Do not touch `src/features/search/SpotlightWindow.tsx`,
  `src/components/settings/WidgetManager.tsx`, or the launcher CSS for this task.
- Keep the profile order Daily, Work, Founder, Focus.
- Keep native hit regions aligned with the visible shelf and rail.
- Kill `protondock` and `protonsearch` before builds and before handing off.
- Commit and push after any further dock edit, per `rules.md`.

## 8. Remaining open work

1. Rebuild the release binary and portable ZIP from the restored dock source.
2. Visually certify transparency, focus-loss behavior, and 125%/150% DPI with a
   desktop compositor screenshot, not a CDP capture.
3. Optional polish: add the original gear glyph to the Customize pill using an
   inline SVG, not an emoji.
4. Optional polish: restore the original profile add (`+`) affordance only if a
   real profile-creation flow exists.
5. Installer work remains blocked by environment-level NSIS/MSI failures. The
   portable ZIP is the working artifact.
6. Search app/action icons are still generic type icons. Spotify remains empty
   without a media session. Claude remains disconnected without local usage data.
7. Pre-existing `src-tauri/src/local_calendar.rs:465` access-denied tests remain
   unaddressed.

## 9. Acceptance checklist

- [x] 52px rail, 36px capsules, 18px rail radius.
- [x] Original graphite glass and top highlight.
- [x] Hover lift and 160ms spring motion; reduced-motion fallback.
- [x] Profile tabs and Customize Widgets are separate pills.
- [x] Daily dock renders centered at 1390x132 natively.
- [x] Work profile switches and persists.
- [x] Launcher and search untouched.
- [x] 186 tests pass; tsc and debug build pass.
- [ ] Release ZIP rebuilt after restore.
- [ ] Desktop compositor transparency/DPI certification.
