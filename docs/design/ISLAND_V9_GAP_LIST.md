# ISLAND V9 — gap list vs the current app (post-task-41/task-47)

**Status:** task-47 sync, 2026-11-10. No code in this task. **Binding:** `ISLAND_V9_SPEC.md`,
`ISLAND_SETTINGS_SPEC.md`, captures `docs/qa/island-reference/` + `island-v9/` + `live-shots/`.
**Summary:** renderer v1 and the settings UI now exist. P0 implementation blockers are closed; remaining items are
capability adapters, IPC gaps and evidence/a11y polish. No fake data is accepted.

## Closed / present

| # | Item | Evidence |
|---|---|---|
| 1 | V9 surface + all 8 cards + compact/expanded + drill-down structure | `apps/desktop/src/island/**`; 20 refreshed captures |
| 2 | Card geometry/dots/motion/tokens | matches ISLAND_V9_SPEC §2; audit D2 |
| 3 | Native shadow pad 112 | `src-tauri/src/main.rs:34`; shadow extent 107 fits |
| 4 | Search real results/filters (All/Files/Folders/Apps/Actions), borderless focus line, no recency meta | island-v9/search-* |
| 5 | Clipboard real categories/snippets/image thumbnails; counts removed; neutral row copy; Pinned honestly disabled | island-v9/clipboard-* + live-shots/island-clipboard-images |
| 6 | Agents flat real Pi tasks + gated Pause/Resume + progress/permissions/logs summary; no fake tree | island-v9/agents-* |
| 7 | Browser/Git real scoped rows; Media honest-disabled; Quick Settings (volume/brightness/audio output) and Notifications now real via the shell controls API | island-v9/{browser,git,media,controls,notifications}-1150 |
| 8 | Placement editor + card manager + Launcher/Onboarding primary choice | ui-app task-40; ISLAND_SETTINGS_SPEC (embedded-frame adaptation allowed) |
| 9 | Zero-results, reduced-motion and expanded 860/430 evidence captured; agent copy/action code pluralised and action-gated; spec-recheck JSON landed | island-v9 zero-results/reduced-motion/{search,clipboard,agents}-860,430; task49-spec-recheck.json |

## Remaining gaps

## P0 — user-rejection quality fixes (task-56, reviewed 2026-11-10: PASS — notes closed)

After set at docs/qa/visual/live-shots/task54-after-* (1150/860/430 over a white Notepad/text window) verified: opaque
surface (no bleed), notifications 0/1/3 without overlap, usable Quick Settings + real Open Windows settings fallback
(no "Unsupported"), island search parity (four filters, group labels/order, shared rows/icons, empty copy), enter/exit
transform-only CSS. Notes closed: paused animation frames and forced brightness fallback captured; Quick Settings Airplane/Night light/Energy saver compact buttons should gain an external/chevron cue or move
under the fallback row; split Wi-Fi/Bluetooth tile uses a separate On/Off pill + chevron (usable deviation). Full G5 still
conditional on Media adapter and real Windows/DPI evidence, not on these surface fixes.

| # | Issue | Required |
|---|---|---|
| 0.1 | Island surface bleed/translucency in user screenshots: opacity/alpha/fade treatment lets desktop/previous content show through | Fully opaque surface/cards at all times; no surface opacity/fade; only shadow pad transparent; verify over light/text window. See `ISLAND_VISUAL_UX_FIX.md` §1. |
| 0.2 | Enter/exit motion is not a defined, opaque transition | Show 180 ms transform translateY(-8) scale(.97)->0/1; hide 120 ms translateY(-6) scale(.985); opacity constant 1; reduced motion 0 ms; no fade-through. |
| 0.3 | Notifications overlap (row content/dismiss/footer) | Fixed 18 / 1fr / 30 grid, 52 px rows, 156 px scroll list, 30x30 dismiss reserved, footer below list; 0/1/many states; no test toast. |
| 0.4 | Quick Settings shows "Unsupported" / dead controls; content overflows | Real volume/brightness/audio + split Wi-Fi/Bluetooth tiles with chevron lists + real "Open Windows settings" fallback; never "Unsupported"; layout fits 376. |
| 0.5 | Island Search does not match Normal Launcher | Same four filters, shared group order/labels, ResultIcon source, row data and empty copy; shared pure search model; no Actions chip. |


### P1 — must be tracked before G5 sign-off / release

| # | Gap | Current evidence | Required / owner |
|---|---|---|---|
| 1 | Media adapter still absent (honest disabled card). Quick Settings/Notifications now use the shell controls API, but media/Wi-Fi/BT drill-downs and notification dismiss/clear commands remain pending | island-v9 media/controls/notifications-1150; task-49 | Media task-35; wire drilldowns when milestone-2/adapters land; gate test-only toast out of production |
| 2 | Agent task-tree/hierarchy/log IPC missing; flat task list only | island-v9/agents-*; audit §3.2 | Lead/IPC owner; wire when agent task tree APIs exist |
| 3 | Clipboard pinning IPC missing; Pinned tab disabled | island-v9/clipboard-*; audit §3.3 | G2/clipboard IPC; then real pinning |
| 4 | Compact page dots keep 11×37 visual/hit box (expanded 24×30 hit pseudo); WCAG target-size deviation | island compact captures; audit D2.1 | Accept as documented a11y deviation; showDots opt-out + keyboard/wheel; revisit touch target design with Lead |
| 5 | Real Windows/WebView2 visual/DPI/frame-pacing evidence missing (Chromium/mock only) | audit §3.5; live-shots partial | L0/task-31: capture real window at 125/150/200 %, taskbar/fullscreen, shadow |
| 6 | Zero-results/reduced-motion evidence is now present; media output/Wi-Fi/BT drill-down captures still absent | audit D2.8; task-49 evidence | Add drill-downs when adapters land; keep reduced-motion test evidence |
| 7 | Preview error detail can show raw bridge error; preview skeleton uses a pulse gradient (design says flat) | `ResultPreview.tsx:149-153`, `launcher.css:1216-1235` | sanitize/Details only; flat skeleton; tracked in REVIEW_3 |
| 8 | Agent copy/action code is fixed ("1 TASK"/"1 permission", only allowed actions) but agents-1150.png is stale and still shows the old "1 TASKS" + Pause/Resume pair | code fixed; agents-1150 timestamp 02:45 | Refresh agents-1150/520/335 captures; no code change needed |
| 9 | Settings IPC gaps: no per-monitor placement map, cursor/active-window monitor policy, pin/undo, Normal Launcher-specific options | ui-app task-40 caveats | Add IPC fields or keep explicit no-fake-controls notes; do not silently drop |
| 10 | Stale live-shots: `island-shown.png`/`island-placeholder.png` still show the old letter-tile placeholder | `docs/qa/visual/live-shots` | Outside ui-launcher scope; L0/design refresh or label stale; current renderer shown in island-live-compact |
| 11 | Controls card overflows: at 516×376 the bottom Wi-Fi/Bluetooth tiles are cut behind the dots; V9 controls must show 2×2 tiles + brightness/volume without clipping | island-v9/controls-1150/430 | Reorder/compact controls or move audio-output to the Media drill-down so the V9 structure fits; body scroll only if intentionally signalled |
| 12 | Zero-results copy exists in the DOM but is not visible at the canonical 185 px expanded card (blank results area) | island-v9/zero-results-1150/860/430; task49 JSON | Show a compact 1–2 line empty state inside the available results area (no 110 px min-height); keep card height 185; never ship a blank zero state |
| 13 | Notifications: dismiss is disabled pending a bridge command and a test-only "Test local toast" control appears; disabled × is not visually distinct | island-v9/notifications-1150; task-49 | Gate the test action out of production; show dismiss disabled with reason or hide; keep Clear all only with a real command |

### P2 — polish / verification

| # | Item | Required |
|---|---|---|
| 14 | V9 demo stage/bar must not ship | Only island surface binds; no product stage chrome |
| 15 | Icon set stroke/size exactness | 24 viewBox, 1.85 stroke, 19/23/18 sizes; no stretch |
| 16 | Reference-vs-production comparison harness | Use captures + per-state metrics; compare real Windows screenshots |
| 17 | Settings dark-theme treatment | Reference settings is light; embed app theme intentionally, document if dark |
| 18 | Tail-copy/pluralisation sweep | Check remaining counts ("1 items" etc.) across island/settings |

## Ownership

- **ui-launcher:** island cards/dots/interaction, controls overflow/zero-state visibility, notification test-action gating, refresh stale agent captures; capability card structure readiness.
- **ui-app:** settings pages/persistence; expose only real IPC fields; keep gap notes for cursor/active policy, pin/undo.
- **L0:** native shadow margin (done), live-shots refresh/label (outside ui-launcher scope), real monitor/DPI/taskbar/fullscreen verification, IPC extensions for card data.
- **sources/agent owners:** Media/Quick Settings/Notifications, agent task tree/logs, clipboard pinning.
