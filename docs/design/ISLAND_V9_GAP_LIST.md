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
| 7 | Browser/Git real scoped rows; disabled Media/Quick Settings/Notifications with reasons, no mock data | island-v9/{browser,git,media,controls,notifications}-1150 |
| 8 | Placement editor + card manager + Launcher/Onboarding primary choice | ui-app task-40; ISLAND_SETTINGS_SPEC (embedded-frame adaptation allowed) |

## Remaining gaps

### P1 — must be tracked before G5 sign-off / release

| # | Gap | Current evidence | Required / owner |
|---|---|---|---|
| 1 | No native Media/Quick Settings/Notifications adapters; cards show honest disabled states, not V9 controls/drill-downs | island-v9 media/controls/notifications-1150; audit §3.1 | task-35 native adapters; then restore V9 card bodies/drill-downs |
| 2 | Agent task-tree/hierarchy/log IPC missing; flat task list only | island-v9/agents-*; audit §3.2 | Lead/IPC owner; wire when agent task tree APIs exist |
| 3 | Clipboard pinning IPC missing; Pinned tab disabled | island-v9/clipboard-*; audit §3.3 | G2/clipboard IPC; then real pinning |
| 4 | Compact page dots keep 11×37 visual/hit box (expanded 24×30 hit pseudo); WCAG target-size deviation | island compact captures; audit D2.1 | Accept as documented a11y deviation; showDots opt-out + keyboard/wheel; revisit touch target design with Lead |
| 5 | Real Windows/WebView2 visual/DPI/frame-pacing evidence missing (Chromium/mock only) | audit §3.5; live-shots partial | L0/task-31: capture real window at 125/150/200 %, taskbar/fullscreen, shadow |
| 6 | Evidence gaps: zero-results, reduced-motion and media/controls/notification drill-down captures absent because adapters unavailable | audit D2.8 / this list | Add when adapters land; keep reduced-motion test evidence |
| 7 | Preview error detail can show raw bridge error; preview skeleton uses a pulse gradient (design says flat) | `ResultPreview.tsx:149-153`, `launcher.css:1216-1235` | sanitize/Details only; flat skeleton; tracked in REVIEW_3 |
| 8 | Agent copy nits: "1 TASKS", "1 permissions granted"; both Pause and Resume rendered (one disabled but not visually distinct) | island-v9/agents-1150 | pluralise + render only the available action (or clear disabled style); ui-launcher |
| 9 | Settings IPC gaps: no per-monitor placement map, cursor/active-window monitor policy, pin/undo, Normal Launcher-specific options | ui-app task-40 caveats | Add IPC fields or keep explicit no-fake-controls notes; do not silently drop |
| 10 | Stale live-shots: `island-shown.png`/`island-placeholder.png` still show the old letter-tile placeholder | `docs/qa/visual/live-shots` | Refresh or label stale; current renderer shown in island-live-compact |

### P2 — polish / verification

| # | Item | Required |
|---|---|---|
| 11 | V9 demo stage/bar must not ship | Only island surface binds; no product stage chrome |
| 12 | Icon set stroke/size exactness | 24 viewBox, 1.85 stroke, 19/23/18 sizes; no stretch |
| 13 | Reference-vs-production comparison harness | Use captures + per-state metrics; compare real Windows screenshots |
| 14 | Settings dark-theme treatment | Reference settings is light; embed app theme intentionally, document if dark |
| 15 | Tail-copy/pluralisation sweep | Check remaining counts ("1 items" etc.) across island/settings |

## Ownership

- **ui-launcher:** island cards/dots/interaction, agent copy/action-state nits, stale live-shot refresh, capability card
  structure readiness.
- **ui-app:** settings pages/persistence; expose only real IPC fields; keep gap notes for cursor/active policy, pin/undo.
- **L0:** native shadow margin (done), real monitor/DPI/taskbar/fullscreen verification, IPC extensions for card data.
- **sources/agent owners:** Media/Quick Settings/Notifications, agent task tree/logs, clipboard pinning.
