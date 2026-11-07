# ProtonDock macOS (Tahoe) Dock Pass - Implementation Plan

Date: 2026-11-07. Branch: `dock`. Status: planned; nothing implemented or verified.
Workflow: Haiku (`dock-implementer`) implements one task, Sonnet (`dock-reviewer`) reviews it, Sonnet
(`dock-integrator`) runs the integration checkpoint (IC) that closes each phase.

## 0. Ground rules

- Read CLAUDE.md, AGENTS.md, rules.md; widget tasks also docs/widgets/CONTRACT.md and QA_CHECKLIST.md. This plan overrides
  docs/design-reference.md where they conflict (T10.1 rewrites it).
- One task = one commit = one push. `git add` only the named paths (never `-A`/`.`, logs, zips, graphify-out, protonsearch/), then `git push origin dock`.
- Kill before builds/tests: `powershell -Command "Get-Process protondock,protonsearch -ErrorAction SilentlyContinue | Stop-Process -Force"`.
- **TS** gate = `npx tsc -b ; npm test -- --reporter=dot` (186 pass today; the count grows). **RS(f)** = `cargo test --manifest-path src-tauri/Cargo.toml f`
  (full-suite `local_calendar.rs:465` access-denied failures are pre-existing). Paste summary lines.
- Budget: 1-3 source files per task. Tests and named one-line call-site edits ("one-liners") are extra. Exceptions are flagged.
- `src-tauri/src/running_apps.rs` and `window_geometry.rs` are untracked; their first commit adds the whole file (expected).
- `src/widgets/final/*` sources must stay free of en/em dashes and emoji (test enforced). Use ASCII punctuation in new strings.
- graphify-out is stale (built from 5be3ef09): read named files directly. Integrators run `graphify update .` and never commit graphify-out.
- CONTRACT.md: the renderer never sends paths, commands or URIs; Rust validates every ID. Absent data renders `--`, a dimmed glyph or an honest label.
- Every animation/transition has a `prefers-reduced-motion: reduce` off-switch. Every control keeps a focus ring (`2px solid #0a84ff`, offset 2px) and a name.
- Ambiguous or wrong spec: stop and report NEEDS_CONTEXT. jsdom has no `PointerEvent`; pointer tests define
  `window.PointerEvent ??= class extends MouseEvent { pointerId: number; constructor(t: string, i: PointerEventInit = {}) { super(t, i); this.pointerId = i.pointerId ?? 1; } }`.

## 1. Decisions (no judgment left to implementers)

1. Every widget is a 46px square tile in a 60px glass rail. Size (Pill/Tile/Card) no longer changes the dock; size controls leave Customize;
   saved `size` stays in the layout and is still sent to `open_widget_card`.
2. Click: Start tile opens the Windows Start menu, Search opens Spotlight, others open their card. Right-click or Shift+F10 opens any card.
   Group squares (running/pinned apps): click activates/launches, right-click opens the group card; an empty group shows one dimmed placeholder.
3. Magnification: macOS fisheye, on by default, togglable, off under reduced motion.
4. `dock` placement (`edge`, `align`, `magnify`) and `pinned` app IDs are global (layout v3). Alignment positions the content-sized dock along
   its edge; magnification grows away from the aligned side.
5. While the rail is hot (hovered/focused) the profile shelf fades out and the native hit region grows for magnified tiles and tooltips;
   it shrinks 240ms after pointer/focus leave.
6. Pinned ID = `app-` + 16 lowercase hex FNV-1a-64 of the lowercased AppsFolder parsing name; max 24. Rust reads them from the saved layout.
7. Spotlight Ctrl+D pins the selected app result: Rust emits `dock:pin-app` to the dock, which pins it and enables Pinned Apps in the active profile.
8. A running pinned app gets a dot and still appears in Running Apps (merging is out of scope).
9. Pinned Apps is a final-system widget. `npm run widget:create` targets the legacy SDK registry the dock no longer renders, so it is not used (T10.2 records it).
10. Old `.fw-dock*` rules in final-widgets.css become unused; left for a follow-up deletion (STATUS note).

## 2. Design contract

| Item | Value |
|---|---|
| Rail | 60px, radius 18px, padding `0 7px` (vertical `7px 0`), gap 6px; glass `rgba(14,20,32,.78)`; border `1px rgba(255,255,255,.14)`; shadow `0 24px 60px rgba(0,0,0,.55), 0 8px 20px rgba(0,0,0,.35), inset 0 1px 0 rgba(255,255,255,.32)`; `backdrop-filter: blur(40px) saturate(190%) contrast(104%)` |
| Tile | 46x46, radius 11px, border `1px rgba(255,255,255,.14)`, background `linear-gradient(160deg, rgba(var(--fw-tint-rgb),.34), rgba(var(--fw-tint-rgb),.10)), rgba(255,255,255,.06)`, shadow `0 4px 10px rgba(0,0,0,.28), inset 0 1px 0 rgba(255,255,255,.24)`; app squares `rgba(255,255,255,.08)` + 32px icon |
| Bottom stage | window 146px tall: 14px bottom inset, rail 60, headroom 64; shelf 8px above the rail |
| Magnify | `s(d) = 1 + 0.55*(1 + cos(PI*d/150))/2` for `abs(d) < 150`, else 1 (peak 1.55 = 71px); neighbours shift by the extra width; origin bottom-center / left-center / right-center; rAF; `transform 140ms cubic-bezier(0.16,1,0.3,1)` |
| Allowances | MAG_ALONG 80px along the axis; MAG_HEADROOM 64px inward |
| Tooltip | 10px past the tile, follows magnification; padding `4px 10px`, radius 8px, `500 12px/16px "Segoe UI Variable Text"`, `rgba(255,255,255,.96)` on `rgba(16,22,34,.90)`, border `1px rgba(255,255,255,.14)`, shadow `0 8px 20px rgba(0,0,0,.40), inset 0 1px 0 rgba(255,255,255,.18)`, opacity 120ms ease-out, max-width 280px ellipsis |
| Running dot | 4px `rgba(255,255,255,.85)`, 5px outside the square on the screen-edge side |
| Easing | `--dock-spring: cubic-bezier(0.16,1,0.3,1)`; `--dock-pop: cubic-bezier(0.34,1.36,0.64,1)` |
| Card | in 260ms `--dock-pop` from `translate(dx,dy) scale(.92)` opacity 0; out 160ms `cubic-bezier(0.4,0,1,1)` to 60% offset, `scale(.96)`, opacity 0; 26px close circle inside the card, 12px from top/right |
| Spotlight | width 638; bar 56px; chip row 36px; empty window 94px; max 500px; input `400 22px/30px "Segoe UI Variable Display"`; rows 44px; Top Hit 56px; section titles 26px; selection `rgba(10,132,255,.88)`; radius 22px; open 200ms |

## 3. Full gate (every IC)

Kill the app; `npx tsc -b ; npm test -- --reporter=dot ; npm run widgets:validate ; npm run build ; cargo test --manifest-path src-tauri/Cargo.toml`
(only the pre-existing local_calendar failures may fail); `graphify update .` (not committed). Fix breaks with the smallest diff, commit explicit
paths, push. ICs marked "+native" also run `npm run tauri -- build --debug --no-bundle` and launch a temp copy (handoff spec section 6) for a
smoke look; native behaviour is only accepted through Phase 11.

---

## Phase 1 - Square tiles and glyphs

Parallel-safe: T1.1 and T1.4. T1.2 needs T1.1; T1.3 needs T1.2.

### T1.1 Glyph primitives [TS]
- **Files:** `src/widgets/final/glyphs.tsx` (new). Test: `src/widgets/final/glyphs.test.tsx` (new, jsdom).
- **Spec:** `Ring({ value, warnAt?, children? })`: clamp value to 0..100, `null` when nullish or non-finite. Root `<span className="dock-ring">`
  plus ` dock-ring--warn` when `warnAt != null && value >= warnAt`. SVG `viewBox="0 0 34 34"` 34x34, `aria-hidden`, `focusable="false"`:
  `circle.dock-ring-track` cx 17 cy 17 r 15; when value is not null `circle.dock-ring-fill` same geometry, `strokeDasharray={\`${C * v / 100} ${C}\`}`
  with `C = 2 * Math.PI * 15`, `transform="rotate(-90 17 17)"`. Children in `span.dock-ring-center`.
  `ClockFace({ now, seconds })`: `s = getSeconds()`, `m = getMinutes() + s/60`, `h = getHours() % 12 + m/60`. SVG `svg.dock-clock` 36x36
  `viewBox="0 0 36 36"`: `circle.dock-clock-face` r 16.5 at 18,18; four `line.dock-clock-tick` (18,3.5)-(18,6) with `rotate(0|90|180|270 18 18)`;
  `line.dock-clock-hour` (18,18)-(18,10) `rotate(${h*30} 18 18)`; `line.dock-clock-minute` (18,18)-(18,6.5) `rotate(${m*6} 18 18)`; if seconds
  `line.dock-clock-second` (18,20)-(18,5.5) `rotate(${s*6} 18 18)`; `circle.dock-clock-pin` r 1.4.
- **Tests:** `ring omits the fill for unknown values`; `ring fills by percent and warns at the threshold` (50: first dasharray number
  toBeCloseTo(47.12, 1); 85 with warnAt 80: `.dock-ring--warn`); `clock face hands follow the time` (3:00:00: hour `rotate(90 18 18)`,
  minute `rotate(0 18 18)`, no second hand; `seconds` true adds it).
- **Verify:** TS. **Commit:** `feat(widgets): add ring and clock-face dock glyphs`

### T1.2 DockWidget renders glyph tiles [TS]
- **Files:** `src/widgets/final/DockWidget.tsx`, `src/widgets/final/types.ts`. One-liners: `src/components/dock/DockSurface.tsx` (`<DockWidget>`
  keeps `id settings data preview`), `src/components/settings/WidgetManager.tsx` (drop `size="compact"`). Test: `final-widgets.test.tsx`.
- **Spec:** `DockWidgetProps = { id; settings?; data?; preview? }`. `source = preview ? sampleFor(id) : data`. Return
  `<span className={\`dock-tile fw-tint-${meta.tint}${off ? ' is-off' : ''}\`} role="img" aria-label={label} data-widget={id} data-preview={preview ? 'true' : undefined}>{glyph}</span>` with:

| id | glyph | label | off |
|---|---|---|---|
| default (windows, search, apps, folders) | `<Icon name={meta.icon} size={22}/>` | `meta.title` | no |
| spotify | `<Icon name="music" size={20}/>` + when playing with a title `<span className="dock-eq" aria-hidden="true"><i/><i/><i/></span>` | `Playing ${t}` / `Paused ${t}` / `Nothing playing` | no title |
| weather | `<span className="dock-weather">` + `<WeatherIcon kind={weatherKind(code)} size={16}/>` (no data: `<Icon name="weather" size={16}/>`) + `<b className="dock-glyph-value">{temperature(t, s.units) or '--°'}</b>` | `${temp}, ${weatherLabelShort(code)}`; none: `No weather data` if `weatherEnabled` else `Weather location not set` | no data |
| system | `<Ring value={cpu}><b className="dock-glyph-value">{cpu == null ? '--' : Math.round(cpu)}</b></Ring>` | `CPU ${n}%` / `CPU unavailable` | cpu null |
| claude, chatgpt | live: `<Ring value={sessionPercent} warnAt={s.warnAt}><Icon name={id} size={14}/></Ring>`; else `<Icon name={id} size={22}/>` | `Session ${pct(x)}` / `Not connected` | not live |
| agent | `<Icon name="agent" size={22}/><StatusDot state={state}/>` (state rule unchanged) | `Agent ${state}` | offline |
| files | `<Icon name="files" size={22}/>` | `${n} recent files` / `Recent files unavailable` | no |
| trash | `<Icon name="trash" size={22}/>` + when `itemCount > 0` `<span className="dock-badge">{n > 99 ? '99+' : n}</span>` | `${n} items in Trash` / `Trash unavailable` | no |
| volume | `<Icon name={muted ? 'volumeMute' : 'volume'} size={20}/>` + when level known and not muted `<span className="dock-level" aria-hidden="true"><span style={{width: \`${Math.round(level)}%\`}}/></span>` | `Volume ${n}%` / `Muted` / `Volume unavailable` | level null |
| clock | `<ClockFace now={c?.now ?? Date.now()} seconds={s.showSeconds}/>` | `clockParts(...).time` + ` ${suffix}` when set | no |

  Delete `appsSlot`, `usageSlot`, `emit`, `workspaceLabel`, `tintStyle`, `baseClass`, `mediaProgress`, `TINT_COLORS`, `mediaDuration`,
  `systemTempLabel`, `usageToneFor`, `SlotProps` and imports only they used. Keep `dockActionFor`, `weatherLabelShort`.
- **Tests:** drop `size` from the search render; size loop becomes `every dock tile and card renders without emoji or mojibake`
  (`[data-widget="${id}"]`); AI test: role img `Not connected`, no `62` text, no `.dock-ring-fill` for disconnected and missing data, img
  `Session 62%` with a fill for `{status:'live', sessionPercent:62}`; delete the 3 DockWidget lines in the app-icon test; replace the Wi-Fi test with
  `system tile shows CPU load honestly` (cpu 20: img `CPU 20%`, text `20`; null: `CPU unavailable`, `--`, no fill); weather: `Weather location not set`
  and `No weather data` with `--°`, live F `88°` + img `88°, Clear`, preview `24°` + img `24°, Partly cloudy`; new
  `volume, trash and clock tiles show live values only` (68: `Volume 68%` + `.dock-level`; muted: `Muted`, no bar; none: `Volume unavailable`;
  itemCount 3: badge `3`; 0: no badge; 16:05 with hour12 false: img `16:05`).
- **Verify:** TS. **Commit:** `feat(dock): render widgets as 46px glyph tiles`

### T1.3 Tile slots and app groups [TS]
- **Files:** `src/components/dock/DockGroup.tsx` (new), `src/components/dock/DockSurface.tsx`. Tests: `DockGroup.test.tsx` (new), `DockSurface.test.tsx`, `DockShell.test.tsx`.
- **Spec:** `export interface DockGroupItem { id: string; title: string; icon: string | null; running: boolean }`;
  `DockGroup({ label, placeholder: FinalIconName, items, disabled = false, onOpen(id), onCard(anchor: HTMLElement) })`. Empty items: one
  `<button className="dock-cell" aria-label={\`Open ${label} card\`}>` with `<span className="dock-tile dock-tile--app is-off"><Icon name={placeholder} size={22}/></span>`
  and `<span className="dock-tip" aria-hidden="true">{label}</span>`; click and contextmenu call `onCard(currentTarget)`. Otherwise
  `<div className="dock-group" role="group" aria-label={label}>` of `<button className="dock-cell dock-app" aria-label={title}>` each holding
  `<span className="dock-tile dock-tile--app"><AppGlyph title icon size={32}/></span>`, the tip, and `<span className="dock-dot" aria-hidden="true"/>`
  when `running`; click `onOpen(id)`; contextmenu preventDefault then `onCard(currentTarget)`; all buttons honour `disabled`.
  DockSurface: delete `availableWidth`, `DOCK_WIDTHS`, `DOCK_DENSE_WIDTHS`, `railWidth`, `useDockWidth`, `dense`, `visibleRail`, rail `style` and
  `is-dense`; fit deps `[layout]`. Running items: `s = settingsFor('apps', item.settings)`; data `views.apps.preview ? sampleFor('apps') : views.apps.data`;
  `s.activeOnly` keeps only the active item; `slice(0, s.maxApps)`; map to `{ id, title, icon, running: true }`. `apps` renders
  `<div className="dock-final-slot dock-final-slot--group" data-widget-id="apps"><DockGroup label={meta.title} placeholder="apps" .../></div>` with
  `onOpen={(id) => void performWidgetAction(id).catch(() => {})}` and `onCard={(anchor) => openCard(item, anchor)}`. Other ids render
  `<div className="dock-final-slot dock-cell" data-widget-id role="button" tabIndex>` with `<DockWidget/>` and `<span className="dock-tip" aria-hidden="true">{meta.title}</span>`;
  `primary = dockActionFor(id)`; aria-label `Open Start` / `Open Spotlight search` / `Open ${meta.title} card`; click and Enter/Space run
  `performWidgetAction(primary)` or `openCard`; `onContextMenu` preventDefault + `openCard`. Remove the `closest('button')` guard.
- **Tests:** DockGroup: `renders safe icons, tooltips and running dots` (data-URL Chrome running + `https://x/y.png` Code: 1 `img.fw-app-img`,
  1 `.dock-dot`, tips `Chrome`,`Code`); `click opens and right-click asks for the card`; `empty group shows a placeholder that opens the card`.
  DockSurface: delete both width tests and the `DOCK_*` import; add `renders single widgets as tiles and running apps as a group` (preview daily:
  8 `.dock-final-slot.dock-cell`, 4 `.dock-app`, 4 `.dock-dot`, clock tip `Clock`) and `right-click opens a card even for Start` (dialog
  `Windows Start card`). DockShell: `starts Windows from the Start tile` (invoke `open_start`).
- **Verify:** TS. **Commit:** `feat(dock): render tiles and running-app groups in the rail`

### T1.4 Tahoe tile CSS [CSS]
- **Files:** `src/design/dock-tiles.css` (new), `src/design/dock-original.css`, `src/main.tsx` (`import "./design/dock-tiles.css";` after dock-original.css).
- **Spec:** dock-original.css: delete lines 2-6 and 83-317, append `@media (prefers-reduced-motion: reduce) { .dock-final-pill, .dock-final-customize { transition: none; } }`.
  dock-tiles.css, in order:
  - `:root { --dock-tile:46px; --dock-tile-radius:11px; --dock-rail:60px; --dock-gap:6px; --dock-glass:rgba(14,20,32,.78); --dock-glass-border:rgba(255,255,255,.14); --dock-spring:cubic-bezier(0.16,1,0.3,1); --dock-pop:cubic-bezier(0.34,1.36,0.64,1); --dock-focus:#0a84ff }`
  - `.dock-final { height:146px; gap:8px; padding:0 16px 14px }`; `.dock-final-shelf { transition:opacity 120ms ease-out }`; `.dock-final[data-hot="true"] .dock-final-shelf { opacity:0 }`; `.dock-final[data-hot="true"] .dock-final-shelf > * { pointer-events:none }`
  - `.dock-final-rail { position:relative; isolation:isolate; display:flex; align-items:center; height:var(--dock-rail); max-width:calc(100vw - 32px); gap:var(--dock-gap); padding:0 7px; box-sizing:border-box; border:0; border-radius:18px; background:transparent; box-shadow:none; backdrop-filter:none; -webkit-backdrop-filter:none; overflow:visible; transition:none }`
  - `.dock-final-rail::after { content:""; position:absolute; z-index:-1; top:0; bottom:0; left:calc(-1 * var(--mag-before, 0px)); right:calc(-1 * var(--mag-after, 0px)); pointer-events:none; transition:left 140ms var(--dock-spring), right 140ms var(--dock-spring), top 140ms var(--dock-spring), bottom 140ms var(--dock-spring) }` + section 2 rail border, radius, background, shadow, both backdrop-filters
  - `.dock-final-slot { position:relative; flex:0 0 auto; display:block; width:var(--dock-tile); height:var(--dock-tile); border-radius:var(--dock-tile-radius); background:none; outline:none; cursor:default; transition:none }`, `:hover { background:none }`, `:focus-visible { outline:none }`; `.dock-final-slot--group { width:auto }`; `.dock-group { display:flex; gap:var(--dock-gap); height:100% }`
  - `.dock-cell { position:relative; flex:0 0 var(--dock-tile); width:var(--dock-tile); height:var(--dock-tile); padding:0; border:0; background:none; color:inherit; font:inherit; outline:none; cursor:default }`
  - `.dock-tile { --fw-dim:#6f747b; --fw-green:#5fd99a; --fw-amber:#f0bd6a; position:relative; box-sizing:border-box; display:grid; place-items:center; width:var(--dock-tile); height:var(--dock-tile); color:var(--fw-tint, #c3c9d1); transform-origin:50% 100%; transform:translate(var(--shift, 0px), 0) scale(var(--mag, 1)); transition:transform 140ms var(--dock-spring), border-color 160ms var(--dock-spring), filter 120ms ease-out; will-change:transform }` + section 2 tile border/radius/background/shadow with `var(--fw-tint-rgb, 195, 201, 209)`
  - `.dock-tile--app { background:rgba(255,255,255,.08); color:#fff }`; `.dock-tile.is-off > svg, .dock-tile.is-off .dock-weather { opacity:.55 }`; `.dock-cell:hover > .dock-tile { border-color:rgba(255,255,255,.26) }`; `.dock-cell:active > .dock-tile { filter:brightness(.82) }`; `.dock-cell:focus-visible > .dock-tile { outline:2px solid var(--dock-focus); outline-offset:2px }`
  - `.dock-glyph-value { font:600 13px/1 "Segoe UI Variable Display","Segoe UI",sans-serif; color:#fff; letter-spacing:-.2px }`; `.dock-weather { display:grid; justify-items:center; gap:2px }`; `.dock-ring { position:relative; display:grid; place-items:center; width:34px; height:34px }`; `.dock-ring > svg { position:absolute; inset:0 }`; `.dock-ring-track { fill:none; stroke:rgba(255,255,255,.16); stroke-width:3 }`; `.dock-ring-fill { fill:none; stroke:var(--fw-tint, #74b7ff); stroke-width:3; stroke-linecap:round }`; `.dock-ring--warn .dock-ring-fill { stroke:#f0bd6a }`; `.dock-ring-center { position:relative; display:grid; place-items:center; color:#fff }`; `.dock-ring-center .dock-glyph-value { font-size:11px }`
  - `.dock-clock-face { fill:rgba(255,255,255,.94) }`; `.dock-clock-tick { stroke:rgba(20,22,26,.5); stroke-width:1.4; stroke-linecap:round }`; `.dock-clock-hour { stroke:#14161a; stroke-width:2.4; stroke-linecap:round }`; `.dock-clock-minute { stroke:#14161a; stroke-width:1.6; stroke-linecap:round }`; `.dock-clock-second { stroke:#ff9f0a; stroke-width:1; stroke-linecap:round }`; `.dock-clock-pin { fill:#ff9f0a }`
  - `.dock-badge { position:absolute; top:-5px; right:-5px; min-width:18px; height:18px; padding:0 5px; box-sizing:border-box; border-radius:9px; background:#ff453a; color:#fff; font:700 10.5px/18px "Segoe UI",sans-serif; text-align:center; box-shadow:0 1px 3px rgba(0,0,0,.45) }`; `.dock-tile > .fw-dot { position:absolute; top:6px; right:6px }`; `.dock-level { position:absolute; left:10px; right:10px; bottom:7px; height:3px; border-radius:2px; background:rgba(255,255,255,.18); overflow:hidden }`; `.dock-level > span { display:block; height:100%; background:currentColor }`
  - `.dock-eq { position:absolute; bottom:7px; display:flex; align-items:flex-end; gap:2px; height:8px }`; `.dock-eq i { width:2px; height:3px; border-radius:1px; background:currentColor; animation:dock-eq 900ms ease-in-out infinite alternate }`; `nth-child(2)` delay `-300ms`, `nth-child(3)` `-600ms`; `@keyframes dock-eq { to { height:8px } }`
  - `.dock-tip { position:absolute; left:50%; bottom:calc(100% + 10px); z-index:4; pointer-events:none; opacity:0; transform:translate(calc(-50% + var(--shift, 0px)), calc(-46px * (var(--mag, 1) - 1))); transition:opacity 120ms ease-out, transform 140ms var(--dock-spring); white-space:nowrap; overflow:hidden; text-overflow:ellipsis }` + section 2 tooltip values; `.dock-cell:hover > .dock-tip, .dock-cell:focus-visible > .dock-tip { opacity:1 }`
  - `.dock-dot { position:absolute; left:50%; bottom:-5px; width:4px; height:4px; border-radius:50%; background:rgba(255,255,255,.85); pointer-events:none; transform:translateX(calc(-50% + var(--shift, 0px))); transition:transform 140ms var(--dock-spring) }`
  - Lines mode: `.dock-final[data-appearance="lines"] .dock-tile:not(.dock-tile--app) { border-color:transparent; background:transparent; box-shadow:none }`; `.dock-final[data-appearance="lines"] .dock-final-slot:not(:first-child)::before { left:-4px; top:10px; bottom:10px; background:rgba(255,255,255,.12) }`
  - `@media (prefers-reduced-motion: reduce) { .dock-tile, .dock-tip, .dock-dot, .dock-final-rail::after, .dock-final-shelf { transition:none } .dock-tile { transform:none } .dock-tip { transform:translate(-50%, 0) } .dock-dot { transform:translateX(-50%) } .dock-eq i { animation:none; height:6px } }`
- **Verify:** TS; `npm run build`. **Commit:** `style(dock): add Tahoe tile, tooltip and dot styles`

**IC1:** full gate. Wiring: dock-tiles.css is the last CSS import; DockSurface renders no `.fw-dock`; `views.apps` feeds DockGroup; the
WidgetManager preview strip shows tiles. Web preview `?window=dock&preview=1`: tiles, tooltips, sample clock face (not native evidence).

---

## Phase 2 - Magnification, hot hit regions, fit geometry

T2.1 first (types), then T2.2; T2.3 needs both.

### T2.1 Native fit geometry [TS]
- **Files:** `src/components/dock/fit.ts` (new). Test: `src/components/dock/fit.test.ts` (new).
- **Spec:**
```ts
import type { DockRegion } from '../../services/native';
export type DockEdge = 'bottom' | 'left' | 'right';
export type DockAlign = 'start' | 'center' | 'end';
export interface Box { left: number; top: number; width: number; height: number }
export const MAG_ALONG = 80, MAG_HEADROOM = 64, BOTTOM_STAGE = 146;
export const ANCHOR: Record<DockAlign, number> = { start: 0, center: 0.5, end: 1 };
/** Window size and hit regions after the native resize; mirrors the edge/align layout in dock-tiles.css. */
export function dockFit(edge: DockEdge, align: DockAlign, shelf: Box, rail: Box, viewport: { width: number; height: number }, hot: boolean) {
  const a = ANCHOR[align], vertical = edge !== 'bottom';
  const left = Math.min(shelf.left, rail.left), top = Math.min(shelf.top, rail.top);
  const right = Math.max(shelf.left + shelf.width, rail.left + rail.width), bottom = Math.max(shelf.top + shelf.height, rail.top + rail.height);
  const width = vertical ? Math.ceil(Math.max(right - left, rail.width + MAG_HEADROOM)) + 22 : Math.ceil(right - left + MAG_ALONG) + 32;
  const height = vertical ? Math.ceil(bottom - top + MAG_ALONG) + 32 : BOTTOM_STAGE;
  const dx = (width - viewport.width) * (edge === 'bottom' ? a : edge === 'left' ? 0 : 1);
  const dy = (height - viewport.height) * (edge === 'bottom' ? 1 : a);
  const place = (box: Box, radius: number): Required<DockRegion> => ({ x: Math.round(box.left + dx), y: Math.round(box.top + dy), width: Math.ceil(box.width), height: Math.ceil(box.height), radius });
  const r = place(rail, 18);
  if (hot && vertical) { r.y -= Math.round(MAG_ALONG * a); r.height += MAG_ALONG; r.width += MAG_HEADROOM; if (edge === 'right') r.x -= MAG_HEADROOM; }
  else if (hot) { r.x -= Math.round(MAG_ALONG * a); r.width += MAG_ALONG; r.y -= MAG_HEADROOM; r.height += MAG_HEADROOM; }
  return { width, height, regions: [place(shelf, 15), r] };
}
```
- **Tests** (`toEqual`, boxes as `{left,top,width,height}`):
  1. bottom/center cold, shelf `332,38,360,26`, rail `112,72,800,60`, viewport `1024x146` -> `{width:912,height:146,regions:[{x:276,y:38,width:360,height:26,radius:15},{x:56,y:72,width:800,height:60,radius:18}]}`.
  2. same hot -> rail `{x:16,y:8,width:880,height:124,radius:18}`.
  3. bottom/start hot, shelf `16,38,360,26`, rail `16,72,800,60` -> width 912, shelf x 16, rail `{x:16,y:8,width:880,height:124,radius:18}`.
  4. left/center, shelf `14,233,360,26`, rail `14,267,60,400`, viewport `400x900` -> width 382, height 546, shelf `{x:14,y:56,...}`, rail `{x:14,y:90,width:60,height:400,radius:18}`; hot rail `{x:14,y:50,width:124,height:480,radius:18}`.
  5. right/end hot, shelf `26,450,360,26`, rail `326,484,60,400`, viewport `400x900` -> width 382, height 546, shelf `{x:8,y:96,width:360,height:26,radius:15}`, rail `{x:244,y:50,width:124,height:480,radius:18}`.
- **Verify:** TS. **Commit:** `feat(dock): compute edge-aware native fit regions`

### T2.2 Magnification math and hook [TS]
- **Files:** `src/components/dock/magnify.ts` (new). Test: `src/components/dock/magnify.test.tsx` (new, jsdom).
- **Spec:**
```ts
import { ANCHOR, type DockAlign, type DockEdge } from './fit';
export const MAG_MAX = 1.55, MAG_RADIUS = 150;
export function magnifyScale(distance: number): number {
  const d = Math.abs(distance);
  return d >= MAG_RADIUS ? 1 : 1 + ((MAG_MAX - 1) * (1 + Math.cos((Math.PI * d) / MAG_RADIUS))) / 2;
}
/** centers/sizes: un-magnified values along the dock axis. anchor 0 grows toward the end, 1 toward the start. */
export function magnifyLayout(centers: number[], sizes: number[], pointer: number | null, anchor: number) {
  const scales = centers.map((c) => (pointer == null ? 1 : magnifyScale(pointer - c)));
  const extras = scales.map((s, i) => sizes[i] * (s - 1));
  const total = extras.reduce((sum, e) => sum + e, 0);
  let before = 0;
  const shifts = extras.map((e) => { const shift = before + e / 2 - total * anchor; before += e; return shift; });
  return { scales, shifts, total };
}
```
  `useMagnify(rail: RefObject<HTMLElement | null>, edge, align, enabled)`: one `useEffect` keyed on `[rail, edge, align, enabled]`. Bail when there
  is no node, `!enabled`, or `window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches`. Locals `pointer: number | null = null`, `frame = 0`,
  `vertical = edge !== 'bottom'`, `anchor = ANCHOR[align]`. `apply()`: `frame = 0`; measure every `.dock-cell` rect relative to the rail rect
  (center = `top - rail.top + height/2` vertical, else `left - rail.left + width/2`; size = height or width); run `magnifyLayout(centers, sizes,
  rail.dataset.dragging === 'true' ? null : pointer, anchor)`; set `--mag` (`toFixed(3)`) and `--shift` (`toFixed(2)` + `px`) on each cell, and
  `--mag-before` = `total * anchor`, `--mag-after` = `total * (1 - anchor)` (`toFixed(2)` + `px`) on the rail. `schedule()` requests one
  `requestAnimationFrame(apply)` when `frame` is 0. Native listeners on the rail: `pointerenter`/`pointermove` set `pointer` to
  `clientY - rail.top` (vertical) or `clientX - rail.left` and schedule; `pointerleave` sets `null` and schedules. Cleanup removes the three
  listeners, cancels the frame, sets `pointer = null` and calls `apply()` once (cells return to rest).
- **Tests:** `scale peaks under the pointer and fades to 1 at the radius` (0 -> 1.55; +-75 -> 1.275; 150 and 400 -> 1);
  `neighbours shift symmetrically for a centered dock` (`[23,75,127]`, sizes 46, pointer 75, anchor .5: scales[1] 1.55, scales[0]/[2]
  toBeCloseTo(1.4024, 3), shifts[1] ~0, shifts[0] ~ -shifts[2], total toBeCloseTo(62.32, 1)); `start-aligned docks grow toward the end only`
  (pointer 23, anchor 0: all shifts >= 0, shifts[0] toBeCloseTo(12.65, 2)); `no pointer leaves every cell at rest`. Hook harness: rail + 3
  `.dock-cell`, synchronous `requestAnimationFrame` stub, rects rail `0,0,174,60`, cell i `7+52*i,7,46,46`: `pointer over the middle cell
  magnifies it most` (pointerMove clientX 82: cell1 `--mag` `1.550`, cell0 `1.402`, rail `--mag-before` `31.16px`); `leaving restores rest
  scale` (`1.000`); `disabled or reduced motion sets nothing` (`enabled` false, and `matchMedia` stubbed `{ matches: true }`: `--mag` is `''`).
- **Verify:** TS. **Commit:** `feat(dock): add macOS magnification math and pointer hook`

### T2.3 Wire magnification, hot state and fit [TS]
- **Files:** `src/components/dock/DockSurface.tsx`. Test: `DockShell.test.tsx`.
- **Spec:** `useMagnify(railRef, 'bottom', 'center', true)`. Hot: `const [hot, setHot] = useState(false); const cool = useRef(0);`
  `heat = () => { clearTimeout(cool.current); setHot(true) }`, `relax = () => { clearTimeout(cool.current); cool.current = window.setTimeout(() => setHot(false), 240) }`,
  unmount clears the timer. Rail: `onPointerEnter={heat} onPointerLeave={relax} onFocus={heat}`, `onBlur` relaxes only when
  `!event.currentTarget.contains(event.relatedTarget as Node | null)`. Root `data-hot={hot ? 'true' : undefined}`. Fit: return when either rect
  width is 0; `next = dockFit('bottom', 'center', shelfBox, { left, top, width: Math.max(r.width, rail.scrollWidth), height: Math.max(r.height, rail.scrollHeight) }, { width: innerWidth, height: innerHeight }, hot)`;
  `native.fitDock(next.width, next.height, next.regions)`; deps `[layout, hot]`; keep rAF, ResizeObserver and resize listener.
- **Tests:** both fit tests set `window.innerHeight` 146 (restore after) and mock rail `112,72,800,60`, shelf `332,38,360,26`: `fit_dock`
  args equal T2.1 vector 1. Clipped (rail left 12, scrollWidth 1100): `{width:1212,height:146,regions:[{x:426,y:38,width:360,height:26,radius:15},{x:106,y:72,width:1100,height:60,radius:18}]}`.
  New `expands the rail hit region while the dock is hovered` (pointerEnter rail: regions[1] `{x:16,y:8,width:880,height:124,radius:18}` and
  `[data-hot="true"]`; pointerLeave, wait 300ms: regions[1] back to vector 1).
- **Verify:** TS. **Commit:** `feat(dock): magnify tiles and grow hit regions while hovered`

**IC2 (+native):** `useMagnify` bound to `railRef`; fit goes through `dockFit`; `data-hot` toggles; tooltips are not clipped in the smoke look.

---

## Phase 3 - Card motion and in-card close

Parallel-safe: T3.1 and T3.2.

### T3.1 Card window motion [TS+CSS]
- **Files:** `src/features/widget-card/WidgetCardWindow.tsx`, `src/design/dock-final.css`. Test: `WidgetCardWindow.test.tsx`.
- **Spec:** state `opened` (number), `closing`, ref `closingRef`. Every accepted card (mount `widget_card_state` and `widget-card:show`) goes through
  `show(v)`: `closingRef.current = false; setClosing(false); setOpened((n) => n + 1); setCard(v)`. `close` (useCallback): web preview unchanged;
  return if `closingRef.current`; set it; if `window.matchMedia?.('(prefers-reduced-motion: reduce)')?.matches` hide now, else `setClosing(true)` and
  hide after 160ms. Escape calls `close`. Markup: `.wc-window > .wc-card(key={opened}, + ' is-closing') > [button.wc-close (same label), div.wc-scroll > WidgetCard]`.
  CSS (replace the existing `.wc-window`, `.wc-card`, `.wc-close`, `.wc-close:hover` blocks): `.wc-window { position:fixed; inset:0; display:flex; align-items:flex-end; justify-content:center; padding:12px; box-sizing:border-box; --wc-dx:0px; --wc-dy:12px; --wc-origin:50% 100% }`;
  `.wc-card { position:relative; width:460px; max-width:calc(100vw - 24px); transform-origin:var(--wc-origin); animation:wc-in 260ms var(--dock-pop, cubic-bezier(0.34,1.36,0.64,1)) both }`;
  `.wc-card.is-closing { animation:wc-out 160ms cubic-bezier(0.4,0,1,1) both; pointer-events:none }`;
  `.wc-scroll { max-height:calc(100vh - 24px); overflow-y:auto; scrollbar-width:none }` (swap `.wc-card` for `.wc-scroll` in the shared `::-webkit-scrollbar` rule);
  `@keyframes wc-in { from { opacity:0; transform:translate(var(--wc-dx), var(--wc-dy)) scale(.92) } to { opacity:1; transform:none } }`;
  `@keyframes wc-out { from { opacity:1; transform:none } to { opacity:0; transform:translate(calc(var(--wc-dx) * .6), calc(var(--wc-dy) * .6)) scale(.96) } }`;
  `.wc-close { position:absolute; top:12px; right:12px; z-index:2; appearance:none; width:26px; height:26px; display:grid; place-items:center; padding:0; border:1px solid rgba(255,255,255,.12); border-radius:50%; background:rgba(255,255,255,.08); color:rgba(255,255,255,.72); font-size:15px; line-height:1; cursor:pointer; transition:background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out }` (no backdrop-filter);
  `.wc-close:hover { background:rgba(255,255,255,.18); border-color:rgba(255,255,255,.2); color:#fff }`; `.wc-close:focus-visible { outline:2px solid #0a84ff; outline-offset:2px }`;
  `.wc-card .fw-card-head { padding-right:34px }`; `@media (prefers-reduced-motion: reduce) { .wc-card, .wc-card.is-closing { animation:none } .wc-close { transition:none } }`.
- **Tests:** existing Escape/close test emits a show between Escape and the click (still 2 hides); `keeps the close button inside the card and
  replays the pop per show` (`.wc-card > .wc-close`; `.wc-card` node changes after a second show); `animates out before hiding unless reduced
  motion is requested` (click: `.is-closing`, no hide yet, hide within waitFor; with `matchMedia` `{ matches: true }`: immediate hide, no class).
  `vi.unstubAllGlobals()` in afterEach.
- **Verify:** TS. **Commit:** `feat(cards): pop cards from the dock with an in-card close`

### T3.2 Anchor cards above their tile [RUST]
- **Files:** `src-tauri/src/main.rs`.
- **Spec:** in `widget_card_position`: `let top = anchor.map_or(dock_position.1 as f64, |a| dock_position.1 as f64 + a.y * scale); let desired_y = top - card_h as f64 - 4.0 * scale;`.
- **Tests:** first expectation in `widget_card_position_centers_above_anchor_and_clamps_to_work_area` becomes `[560, 554, 520, 430]`.
- **Verify:** RS(widget_card). **Commit:** `fix(cards): place widget cards 16px above their tile`

**IC3 (+native):** card payload unchanged; `.wc-window` fills the 520x430 window; card bottom sits near its tile.

---

## Phase 4 - Layout v3 and placement controls

### T4.1 Layout v3 migration [TS]
- **Files:** `src/components/dock/layout.ts`. Tests: `layout.test.ts`, `DockShell.test.tsx` (`version: 2` -> `3`).
- **Spec:** add
```ts
import type { DockAlign, DockEdge } from './fit';
export interface DockPlacement { edge: DockEdge; align: DockAlign; magnify: boolean }
export const DEFAULT_PLACEMENT: DockPlacement = { edge: 'bottom', align: 'center', magnify: true };
export const PINNED_LIMIT = 24;
export const PINNED_ID = /^app-[0-9a-f]{16}$/;
function parsePlacement(value: unknown): DockPlacement {
  const raw = isRecord(value) ? value : {};
  return { edge: raw.edge === 'left' || raw.edge === 'right' ? raw.edge : 'bottom',
    align: raw.align === 'start' || raw.align === 'end' ? raw.align : 'center',
    magnify: typeof raw.magnify === 'boolean' ? raw.magnify : true };
}
function parsePinned(value: unknown): string[] {
  const pinned: string[] = [];
  for (const id of Array.isArray(value) ? value : []) {
    if (typeof id === 'string' && PINNED_ID.test(id) && !pinned.includes(id) && pinned.length < PINNED_LIMIT) pinned.push(id);
  }
  return pinned;
}
export const pinApp = (layout: DockLayout, id: string): DockLayout =>
  !PINNED_ID.test(id) || layout.pinned.includes(id) || layout.pinned.length >= PINNED_LIMIT ? layout : { ...layout, pinned: [...layout.pinned, id] };
```
  Also export `setPlacement(layout, patch: Partial<DockPlacement>)` (`{ ...layout, dock: { ...layout.dock, ...patch } }`), `unpinApp(layout, id)`
  (filter it out) and `movePinned(layout, from, to)` (same splice and bounds rules as `moveWidget`, on `layout.pinned`).
  `DockLayout`: `version: 3`, `dock: DockPlacement`, `pinned: string[]`. `defaultLayout`: version 3, `dock: { ...DEFAULT_PLACEMENT }`, `pinned: []`.
  Rename `parseV2` to `parseCurrent`: accepts versions 2 and 3, returns version 3 with `dock: parsePlacement(value.dock)` and
  `pinned: parsePinned(value.pinned)` (profiles, unknown IDs, appearances, theme unchanged). `parseLayout`: 2|3 -> `parseCurrent`, 1 -> `parseV1`,
  else `null` (future files stay untouched). `isLayout`: require `value.version === 3`; allowed keys add `dock`, `pinned`; `dock` must be exactly
  `{edge,align,magnify}` with valid values; `pinned` must equal `parsePinned(value.pinned)` (compare `JSON.stringify`). Commit body: the Rust
  weather fallback reads only v1/v2 but the dock always sends coordinates, so Rust is unchanged.
- **Tests:** describe `dock layout v3`; defaults expect version 3, `DEFAULT_PLACEMENT`, `pinned []`; v1 migration expects version 3;
  `migrates v2 to v3 keeping profiles, unknown ids, settings, appearances and theme` (focus weather `{units:'F'}`, unknown `['legacy']`,
  appearances `{work:'lines'}`, theme `glass`); `parses v3 placement and pinned ids, dropping invalid entries` (`{left,end,false}` kept;
  `{top,middle,'yes'}` -> defaults; `['app-0123456789abcdef','bad','app-0123456789abcdef','APP-0123456789ABCDEF']` -> first only); null test uses
  `{ version: 4, profiles: {} }`; structural test rejects `version: 2`, `dock.edge 'top'`, `pinned ['x']`; `pins, unpins, reorders pinned apps
  and patches placement` (duplicate/invalid/25th ignored; `movePinned(0,1)`; `setPlacement({edge:'right'})` keeps align and magnify).
- **Verify:** TS. **Commit:** `feat(layout): migrate dock layout to v3 with placement and pinned apps`

### T4.2 Placement controls in Customize [TS+CSS]
- **Files:** `src/components/settings/WidgetManager.tsx`, `src/design/dock-final.css`, `src/widgets/final/meta.ts` (delete orphaned `SIZE_LABELS`).
  One-liner: DockSurface `useMagnify(railRef, 'bottom', 'center', layout.dock.magnify)`. Test: `WidgetManager.test.tsx`.
- **Spec:** remove `SizeControl`, `SIZE_ORDER`, `setSize`, `onSize`, every size control and orphaned imports. Wrap the Installed toolbar's
  `Dock tile style` group in `<div className="wm-toolbar-controls">` and append: group `aria-label="Dock edge"` of `wm-style-pill` buttons
  `Bottom` `Left` `Right` (`aria-pressed`, `onChange(setPlacement(layout, { edge }))`); group `aria-label="Dock alignment"` `Start` `Center` `End`;
  `<span className="wm-toolbar-label" aria-hidden="true">Magnify</span>` and a `wm-enable` switch (`role="switch"`, `aria-checked={layout.dock.magnify}`,
  `aria-label="Magnification"`, same track/thumb). CSS after `.wm-style-pill[aria-pressed="true"]`:
  `.wm-toolbar-controls { display:flex; align-items:center; flex-wrap:wrap; justify-content:flex-end; gap:8px }`, `.wm-toolbar-label { color:var(--dock-secondary); font-size:11px; font-weight:600 }`.
- **Tests:** drop size assertions from the settings-reset and appearance tests; replace the size-detail test with `shows install state in the
  widget detail view`; new `changes dock edge, alignment and magnification` (Left and End pressed; `Magnification` true -> false).
- **Verify:** TS. **Commit:** `feat(settings): add dock edge, alignment and magnification controls`

**IC4:** saved layout is v3 (web preview localStorage); the magnify toggle reaches `useMagnify`.

---

## Phase 5 - Native left/right edges

T5.1 -> T5.2 (both main.rs). T5.3 and T5.4 are disjoint and may run alongside.

### T5.1 Edge-aware dock bounds and fit_dock args [RUST]
- **Files:** `src-tauri/src/window_geometry.rs`, `src-tauri/src/main.rs`.
- **Spec (window_geometry.rs):** `pub enum Edge { Bottom (#[default]), Left, Right }` and `pub enum Align { Start, Center (#[default]), End }`, both
  `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]`. `Edge::parse(Option<&str>) -> Result<Self, String>`: None or `bottom`, `left`, `right`, else
  `Err("Invalid dock edge")`; `Edge::as_str(self) -> &'static str`. `Align::parse`: None or `center`, `start`, `end`, else `Err("Invalid dock alignment")`.
```rust
pub fn dock_bounds(area: [i32; 4], scale: f64, width: f64, height: f64, edge: Edge, align: Align) -> Result<[i32; 4], String> {
    let [left, top, right, bottom] = area;
    if !scale.is_finite() || !(0.5..=8.0).contains(&scale) || right - left < 64 || bottom - top < 64 { return Err("Invalid monitor geometry".into()); }
    let gap = (10.0 * scale).ceil() as i32;
    let w = (width * scale).ceil().min(f64::from(right - left - 2 * gap)).max(1.0) as i32;
    let h = (height * scale).ceil().min(f64::from(bottom - top - 2 * gap)).max(1.0) as i32;
    let along = |start: i32, end: i32, size: i32| match align { Align::Start => start + gap, Align::Center => start + (end - start - size) / 2, Align::End => end - gap - size };
    Ok(match edge {
        Edge::Bottom => [along(left, right, w), bottom - gap - h, w, h],
        Edge::Left => [left + gap, along(top, bottom, h), w, h],
        Edge::Right => [right - gap - w, along(top, bottom, h), w, h],
    })
}
```
  `DockGeometry` gains `pub edge: Edge, pub align: Align`. `bounds` starts with
  `if dock { return dock_bounds(area, scale, width, height, Edge::Bottom, Align::Center); }` and the rest keeps only the search case (gap 16,
  search `y`). `validate`: height `64.0..=16384.0`; region bound `r.y + r.height > 32768.0`. `fit` calls `dock_bounds` with the geometry's edge
  and align. Existing test literals add `edge: Edge::Bottom, align: Align::Center`. main.rs: `fit_dock(..., regions, edge: Option<String>, align: Option<String>)`
  builds `DockGeometry { width, height, regions, edge: Edge::parse(edge.as_deref())?, align: Align::parse(align.as_deref())? }`.
- **Tests:** `dock_bounds_place_every_edge_and_alignment` (area `[0,0,3000,1950]`, scale 1; bottom 400x146: Start `[10,1794,400,146]`, Center
  `[1300,1794,400,146]`, End `[2590,1794,400,146]`; 146x600: Left Start `[10,10,146,600]`, Left Center `[10,675,146,600]`, Left End
  `[10,1340,146,600]`, Right Center `[2844,675,146,600]`; `bounds(area,1.0,1200.0,132.0,true)` equals Bottom/Center);
  `dock_edges_and_alignments_parse_strictly`; `vertical_dock_geometry_is_accepted` (382x1400 with a 60x1200 region).
- **Verify:** RS(window_geometry). **Commit:** `feat(native): place the dock on bottom, left or right edges`

### T5.2 Cards beside vertical docks [RUST]
- **Files:** `src-tauri/src/main.rs`.
- **Spec:** `WidgetCardState` gains `edge: String`. `open_widget_card` reads
  `let edge = state.dock_geometry.lock().map_err(|e| e.to_string())?.as_ref().map_or(Edge::Bottom, |g| g.edge);`, stores `edge.as_str()` in the
  card and passes `edge` to `widget_card_position(..., scale, edge)`, whose desired position becomes:
```rust
let (dock_x, dock_y, dock_w, dock_h) = (dock_position.0 as f64, dock_position.1 as f64, dock_size.0 as f64, dock_size.1 as f64);
let center_x = anchor.map_or(dock_x + dock_w / 2.0, |a| dock_x + (a.x + a.width / 2.0) * scale);
let center_y = anchor.map_or(dock_y + dock_h / 2.0, |a| dock_y + (a.y + a.height / 2.0) * scale);
let (desired_x, desired_y) = match edge {
    Edge::Bottom => (center_x - card_w as f64 / 2.0, anchor.map_or(dock_y, |a| dock_y + a.y * scale) - card_h as f64 - 4.0 * scale),
    Edge::Left => (anchor.map_or(dock_x + dock_w, |a| dock_x + (a.x + a.width) * scale) + 4.0 * scale, center_y - card_h as f64 / 2.0),
    Edge::Right => (anchor.map_or(dock_x, |a| dock_x + a.x * scale) - card_w as f64 - 4.0 * scale, center_y - card_h as f64 / 2.0),
};
```
  then the existing clamps using `desired_x`/`desired_y`.
- **Tests:** existing calls pass `Edge::Bottom` (values unchanged); left: dock `(10,300)` size `(146,600)` card `(520,430)` anchor `{14,200,46,46}` ->
  `[74,308,520,430]`; right: dock `(1764,300)`, anchor x 86 -> `[1326,308,520,430]`; `widget_card_state_reports_the_dock_edge` (serialized `edge` = `left`).
- **Verify:** RS(widget_card). **Commit:** `feat(cards): open cards beside left and right docks`

### T5.3 Send edge and alignment from the dock; edge-aware card window [TS]
- **Files:** `src/components/dock/DockSurface.tsx`, `src/services/native.ts`, `src/features/widget-card/WidgetCardWindow.tsx`. Tests: `DockShell.test.tsx`, `WidgetCardWindow.test.tsx`.
- **Spec:** `const { edge, align, magnify } = layout.dock`; `useMagnify(railRef, edge, align, magnify)`; `dockFit(edge, align, ...)`; root
  `data-edge={edge} data-align={align}`. native.ts: `fitDock(width, height, regions, edge: DockEdge, align: DockAlign)` -> `fit_dock`
  `{ width, height, regions, edge, align }` (type import from `../components/dock/fit`); `WidgetCardState.edge?: 'bottom' | 'left' | 'right'`.
  WidgetCardWindow: `.wc-window` gets `data-edge={card.edge ?? 'bottom'}`.
- **Tests:** `fit_dock` expectations add `edge: 'bottom', align: 'center'`; new `places the dock on the saved edge` (saved `dock {left,start,true}`:
  `.dock-final[data-edge="left"]`, `fit_dock` called with `expect.objectContaining({ edge: 'left', align: 'start' })`); card show payload with
  `edge: 'left'` -> `.wc-window[data-edge="left"]`.
- **Verify:** TS. **Commit:** `feat(dock): send dock edge and alignment to the native host`

### T5.4 Vertical rail and card edge CSS [CSS]
- **Files:** `src/design/dock-tiles.css`, `src/design/dock-final.css`.
- **Spec:** dock-tiles.css before the reduced-motion block (write each `left`/`right` pair as two selectors, prefix `.dock-final`):
  `[data-align="start"] { align-items:flex-start }`, `[data-align="end"] { align-items:flex-end }`;
  `[data-edge="left"], [data-edge="right"] { top:0; bottom:0; height:auto; justify-content:center }`;
  `[data-edge="left"] { align-items:flex-start; padding:16px 0 16px 14px }`; `[data-edge="right"] { align-items:flex-end; padding:16px 14px 16px 0 }`;
  left/right with `[data-align="start"]` -> `justify-content:flex-start`, with `[data-align="end"]` -> `flex-end`;
  vertical `.dock-final-rail { flex-direction:column; width:var(--dock-rail); height:auto; max-width:none; max-height:calc(100vh - 32px); padding:7px 0 }`;
  vertical `.dock-final-rail::after { top:calc(-1 * var(--mag-before, 0px)); bottom:calc(-1 * var(--mag-after, 0px)); left:0; right:0 }`;
  vertical `.dock-group { flex-direction:column }`, vertical `.dock-final-slot--group { width:var(--dock-tile); height:auto }`;
  left `.dock-tile { transform-origin:0% 50%; transform:translate(0, var(--shift, 0px)) scale(var(--mag, 1)) }`, right the same with `100% 50%`;
  left `.dock-tip { left:calc(100% + 10px); right:auto; top:50%; bottom:auto; transform:translate(calc(46px * (var(--mag, 1) - 1)), calc(-50% + var(--shift, 0px))) }`;
  right `.dock-tip { right:calc(100% + 10px); left:auto; top:50%; bottom:auto; transform:translate(calc(-46px * (var(--mag, 1) - 1)), calc(-50% + var(--shift, 0px))) }`;
  left `.dock-dot { left:-5px; top:50%; bottom:auto; transform:translateY(calc(-50% + var(--shift, 0px))) }`, right `.dock-dot { left:auto; right:-5px; top:50%; bottom:auto; same transform }`;
  vertical lines separators `left:10px; right:10px; top:-4px; bottom:auto; width:auto; height:1px`. Reduced-motion block adds vertical
  `.dock-tile { transform:none }`, `.dock-tip { transform:translate(0, -50%) }`, `.dock-dot { transform:translateY(-50%) }`.
  dock-final.css: `.wc-window[data-edge="left"] { align-items:center; justify-content:flex-start; --wc-dx:-12px; --wc-dy:0px; --wc-origin:0% 50% }`,
  `.wc-window[data-edge="right"] { align-items:center; justify-content:flex-end; --wc-dx:12px; --wc-dy:0px; --wc-origin:100% 50% }`.
- **Verify:** TS; `npm run build`. **Commit:** `style(dock): lay out vertical rails and edge-aware cards`

**IC5 (+native):** edge flows layout -> DockSurface -> `fit_dock(edge, align)` -> `DockGeometry` -> card placement and payload; CSS insets match
`dockFit` (14px edge side, 16px along-axis padding, 146px bottom stage).

---

## Phase 6 - Drag reorder

### T6.1 Drag model [TS]
- **Files:** `src/components/dock/drag.ts` (new). Test: `drag.test.ts` (new).
- **Spec:**
```ts
export function dropIndex(centers: number[], from: number, pointer: number): number {
  return centers.reduce((index, center, i) => (i !== from && center < pointer ? index + 1 : index), 0);
}
```
  `useDockDrag(railRef, vertical, onMove(from, to))` keeps `state` (ref: `{ from, target, start, centers, slots, active, pointerId }` or null) and
  `suppress` (ref boolean); `axis(e) = vertical ? e.clientY : e.clientX`. Returns handlers:
  - `onPointerDown(event, index)`: only `button === 0` with a rail; `slots = rail.querySelectorAll(':scope > .dock-final-slot')`; `centers` = slot
    rect centers relative to the rail rect along the axis; store `{ from: index, target: index, start: axis(event), centers, slots, active: false, pointerId }`.
  - `onPointerMove(event)`: ignore other `pointerId`s; `delta = axis(event) - start`; while inactive return until `abs(delta) >= 6`, then set active,
    `rail.dataset.dragging = 'true'`, add `is-dragging` to `slots[from]`, `slots[from].setPointerCapture?.(pointerId)`. Set `--drag: ${delta}px` on
    `slots[from]`; `target = dropIndex(centers, from, axis(event) - rail start)`; give `slots[target]` (when `!== from`) `data-drop` = `before` if
    `target < from` else `after`, and remove `data-drop` from every other slot.
  - `finish(commit)`: clear `state`; delete `rail.dataset.dragging`; strip `is-dragging`, `data-drop`, `--drag` from all slots; when it was active set
    `suppress = true` and, if `commit && target !== from`, call `onMove(from, target)`. `onPointerUp` = `finish(true)`; `onPointerCancel` = `finish(false)`;
    `onKeyDown`: Escape during an active drag -> preventDefault + `finish(false)`; `onClickCapture`: when `suppress`, reset it, preventDefault, stopPropagation.
- **Tests:** `dropIndex([30,82,134,186], 0, 140) === 2`; `(..., 3, 10) === 0`; `(..., 1, 75) === 1`.
- **Verify:** TS. **Commit:** `feat(dock): add drag-reorder model`

### T6.2 Drag and keyboard reorder in the rail [TS+CSS]
- **Files:** `src/components/dock/DockSurface.tsx`, `src/design/dock-tiles.css`. Test: `DockSurface.test.tsx`.
- **Spec:** `const drag = useDockDrag(railRef, layout.dock.edge !== 'bottom', (from, to) => onChange(moveWidget(layout, from, to)));` Rail gets
  `onPointerMove/onPointerUp/onPointerCancel/onKeyDown/onClickCapture` from `drag` (omitted when `disabled`). Every slot (single and group):
  `onPointerDown={(e) => drag.onPointerDown(e, index)}` and, before Enter/Space handling, `Alt+ArrowLeft|ArrowUp` -> `index - 1`,
  `Alt+ArrowRight|ArrowDown` -> `index + 1` (preventDefault, `onChange(moveWidget(layout, index, next))`, then `requestAnimationFrame` to focus the
  slot when it has `tabindex`, else its first `button`). CSS: `.dock-final-slot.is-dragging { z-index:3 }`;
  `.dock-final-slot.is-dragging .dock-tile { transform:translate(var(--drag, 0px), -6px) scale(1.08); transition:none; box-shadow:0 10px 24px rgba(0,0,0,.45) }`
  (left edge `translate(6px, var(--drag, 0px))`, right `translate(-6px, var(--drag, 0px))`);
  `.dock-final-slot[data-drop]::after { content:""; position:absolute; top:4px; bottom:4px; width:3px; border-radius:2px; background:var(--dock-focus) }`,
  before `left:-5px`, after `right:-5px`; vertical: `left:4px; right:4px; width:auto; height:3px; top:auto; bottom:auto`, before `top:-5px`, after `bottom:-5px`.
- **Tests (preview; slot i rect left `7+52*i` width 46; rail rect left 0):** `drags a widget to a new position` (pointerDown windows clientX 30,
  move 140, up: ids start `search, apps, windows`); `a drag does not open a card`; `Alt+ArrowLeft moves the focused widget` (clock -> index 7).
- **Verify:** TS. **Commit:** `feat(dock): reorder widgets by dragging or Alt+Arrow`

**IC6:** magnification pauses while `data-dragging`; drops persist through the normal save path.

---

## Phase 7 - Pinned apps

Rust chain T7.1 -> T7.2 -> T7.3 -> T7.9; TS chain T7.4 -> T7.5 -> T7.6, then T7.7 and T7.8 (disjoint). The chains may run side by side.

### T7.1 Cached 48px shell icons [RUST]
- **Files:** `src-tauri/src/running_apps.rs`, `src-tauri/Cargo.toml` (windows features += `"Win32_UI_Controls"`).
- **Spec:** `render_icon` becomes `render_icon_sized(handle: HICON, size: i32)` (every literal 32 -> `size`; buffer `(size * size * 4) as usize`) and
  `fn render_icon(handle: HICON) -> Option<String> { render_icon_sized(handle, 32) }`. Add (import `COINIT_APARTMENTTHREADED`):
```rust
/// Shell work on a fresh STA: Tauri workers may already be MTA (widget_data::Apartment keeps RoInitialize alive).
pub(crate) fn on_sta_thread<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    std::thread::spawn(move || {
        // SAFETY: this thread owns its apartment; COM objects from `work` are dropped before CoUninitialize.
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let value = work();
        if initialized { unsafe { CoUninitialize() }; }
        value
    }).join().ok()
}
fn extract_shell_icon(parsing_name: &str) -> Option<String> {
    use windows::Win32::UI::{Controls::{IImageList, ILD_TRANSPARENT}, Shell::{Common::ITEMIDLIST, ILFree, SHGetImageList, SHParseDisplayName, SHGFI_PIDL, SHGFI_SYSICONINDEX, SHIL_EXTRALARGE}};
    let wide: Vec<u16> = parsing_name.encode_utf16().chain(Some(0)).collect();
    // SAFETY: the PIDL is freed exactly once; the extracted icon is destroyed after rendering.
    unsafe {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None).ok()?;
        let mut info = SHFILEINFOW::default();
        let found = SHGetFileInfoW(PCWSTR(pidl as *const u16), Default::default(), Some(&mut info), std::mem::size_of::<SHFILEINFOW>() as u32, SHGFI_PIDL | SHGFI_SYSICONINDEX);
        ILFree(Some(pidl.cast_const()));
        if found == 0 { return None; }
        let list: IImageList = SHGetImageList(SHIL_EXTRALARGE as i32).ok()?;
        let icon = list.GetIcon(info.iIcon, ILD_TRANSPARENT.0).ok()?;
        let rendered = render_icon_sized(icon, 48);
        let _ = DestroyIcon(icon);
        rendered
    }
}
```
  `pub fn shell_icon(parsing_name: &str) -> Option<String>`: return the hit from `static SHELL_ICONS: OnceLock<Mutex<HashMap<String, Option<String>>>>`
  when present; otherwise `on_sta_thread(move || extract_shell_icon(&name)).flatten()`, clear the map when it already holds 512 entries
  (`// ponytail: whole-cache reset; LRU if icon churn shows up`), insert, return.
- **Tests:** `shell_icon_renders_paths_and_rejects_missing_items` (notepad from `SystemRoot` -> `data:image/png;base64,` prefix;
  `C:\definitely\missing\protondock.exe` -> None); existing icon tests pass.
- **Verify:** RS(running_apps). **Commit:** `feat(native): extract cached 48px shell icons`

### T7.2 Pinned apps catalog [RUST]
- **Files:** `src-tauri/src/pinned_apps.rs` (new). One-liner: `mod pinned_apps;` in main.rs.
- **Spec:**
```rust
//! Start-menu (AppsFolder) apps pinned to the dock. IDs are opaque; the renderer never sends paths or commands.
use crate::running_apps::{on_sta_thread, shell_icon};
use serde_json::{json, Value};
use std::{sync::{Mutex, OnceLock}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
pub const LIMIT: usize = 24;
const CATALOG_LIMIT: usize = 600;
const TTL: Duration = Duration::from_secs(60);
#[derive(Clone, Debug)]
pub struct App { pub id: String, pub name: String, pub target: String }
static CATALOG: OnceLock<Mutex<Option<(Instant, Vec<App>)>>> = OnceLock::new();
pub(crate) fn fnv1a64(text: &str) -> u64 {
    text.to_lowercase().bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}
pub fn id_for(target: &str) -> String { format!("app-{:016x}", fnv1a64(target)) }
pub fn valid_id(id: &str) -> bool { id.len() == 20 && id.starts_with("app-") && id[4..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
pub fn id_for_launch_command(command: &str) -> Option<String> { command.strip_prefix("shell:AppsFolder\\").filter(|t| !t.is_empty()).map(id_for) }
fn skip(name: &str, target: &str) -> bool {
    let (name, target) = (name.to_lowercase(), target.to_lowercase());
    ["uninstall", "readme", "read me", "help", "documentation", "release notes", "license", "website"].iter().any(|w| name.contains(w))
        || [".txt", ".url", ".chm", ".pdf", ".html", ".htm"].iter().any(|ext| target.ends_with(ext))
}
fn enumerate() -> Vec<App> {
    use windows::Win32::{Foundation::HANDLE, System::Com::CoTaskMemFree, UI::Shell::{BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, SHGetKnownFolderItem, KF_FLAG_DEFAULT, SIGDN, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY}};
    unsafe fn display(item: &IShellItem, kind: SIGDN) -> Option<String> {
        let text = item.GetDisplayName(kind).ok()?;
        let value = text.to_string().ok();
        CoTaskMemFree(Some(text.0 as *const _));
        value
    }
    // SAFETY: runs on a fresh STA thread; every shell string is freed exactly once.
    on_sta_thread(|| unsafe {
        let mut apps: Vec<App> = Vec::new();
        let Ok(folder) = SHGetKnownFolderItem::<_, IShellItem>(&FOLDERID_AppsFolder, KF_FLAG_DEFAULT, HANDLE::default()) else { return apps };
        let Ok(items) = folder.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) else { return apps };
        let (mut slot, mut fetched) = ([None], 0);
        while items.Next(&mut slot, Some(&mut fetched)).is_ok() && fetched == 1 {
            let Some(item) = slot[0].take() else { break };
            let (Some(name), Some(target)) = (display(&item, SIGDN_NORMALDISPLAY), display(&item, SIGDN_DESKTOPABSOLUTEPARSING)) else { continue };
            let id = id_for(&target);
            if !skip(&name, &target) && !apps.iter().any(|app| app.id == id) { apps.push(App { id, name, target }); }
        }
        apps.sort_by_key(|app| app.name.to_lowercase());
        apps
    }).unwrap_or_default()
}
```
  - `pub fn catalog() -> Vec<App>`: `CATALOG` caches `(Instant, Vec<App>)`; return a clone while younger than `TTL`, else `enumerate()` and store.
  - `fn envelope(items: Vec<Value>) -> Value`: `{"status": "empty" if no items else "live", "data": {"items": items}, "updatedAt": epoch ms}`.
  - `pub fn catalog_snapshot() -> Value`: envelope of the first `CATALOG_LIMIT` apps as `{"id", "name"}`.
  - `pub fn saved_ids(layout: &Value) -> Vec<String>`: strings in `layout["pinned"]` passing `valid_id`, deduplicated, in order, at most `LIMIT`.
  - `pub fn snapshot(ids: &[String]) -> Value`: envelope, in `ids` order, of catalog matches as
    `{"id", "name", "icon": shell_icon(&format!("shell:AppsFolder\\{}", target))}`; unknown IDs are skipped.
  - `pub fn launch(id: &str) -> Result<(), String>`: `Err("Invalid app ID")` unless `valid_id`; catalog match else `Err("App is no longer installed")`;
    `crate::widget_data::open_uri(&format!("shell:AppsFolder\\{}", target))`.
- **Tests:** `ids_are_opaque_stable_and_case_insensitive`; `malformed_or_unknown_ids_are_rejected` (`""`, `app-`, `app-XYZ`, `app-0123456789abcdeg`,
  `app-0123456789ABCDEF`, `../app`, 21 chars; `launch("app-0000000000000000")` is Err); `engine_launch_commands_map_to_catalog_ids`
  (`shell:AppsFolder\Chrome` -> `id_for("Chrome")`; exe path and empty target -> None); `saved_ids_are_validated_deduplicated_and_bounded`
  (30 valid + `"../evil"` + `7` -> 24, first `app-0000000000000000`); `catalog_lists_installed_start_menu_apps` (non-empty; `id == id_for(target)`, valid, names non-empty).
- **Verify:** RS(pinned_apps). **Commit:** `feat(native): add pinned apps catalog with opaque IDs`

### T7.3 Pinned app commands and permissions [RUST]
- **Files:** `src-tauri/src/main.rs`, `src-tauri/permissions/commands.toml`.
- **Spec:** `WIDGET_CARD_IDS: [&str; 15]` appends `"pinned"`. Add:
```rust
fn pinned_id_for_result(engine: &Engine, result_id: &str) -> Result<String, String> {
    let result = engine.results.get(result_id).ok_or("Result expired; search again")?;
    if result.entry.source != "app" { return Err("Only applications can be pinned".into()); }
    pinned_apps::id_for_launch_command(&result.entry.launch_command).ok_or_else(|| "This application cannot be pinned".into())
}
#[tauri::command]
fn pin_search_result(result_id: String, state: tauri::State<'_, NativeState>, app: tauri::AppHandle) -> Result<String, String> {
    let id = pinned_id_for_result(&state.engine.lock().map_err(|e| e.to_string())?, &result_id)?;
    app.emit_to("dock", "dock:pin-app", &id).map_err(|e| e.to_string())?;
    Ok(id)
}
```
  Async commands on `spawn_blocking`: `pinned_apps_snapshot()` reads `data_dir()?/protondock-layout.json` (unreadable or invalid JSON -> `Value::Null`)
  and returns `pinned_apps::snapshot(&pinned_apps::saved_ids(&layout))`; `pinned_apps_catalog()` returns `catalog_snapshot()`;
  `pinned_apps_launch(id)` rejects `!valid_id` before spawning, then `pinned_apps::launch`. Register all four in `generate_handler!`.
  commands.toml: `dock-commands` += `pinned_apps_snapshot`, `pinned_apps_catalog`, `pinned_apps_launch`; `widget-card-commands` +=
  `pinned_apps_snapshot`, `pinned_apps_launch`, `open_settings`; `search-commands` += `pin_search_result`.
- **Tests:** `only_engine_app_results_can_be_pinned` (Engine with `1:0` app `shell:AppsFolder\Chrome` -> `id_for("Chrome")`; `1:1` FILE, `1:2` app
  with an exe path and missing `9:9` -> Err; build entries like the clipboard test); allowlist test asserts `valid_widget_card_id("pinned")`.
- **Verify:** RS(pinned), RS(widget_card); `npm run tauri -- build --debug --no-bundle` (permissions compile). **Commit:** `feat(native): expose pinned app commands and pin-from-search`

### T7.4 Pinned provider plumbing [TS]
- **Files:** `src/widgets/sdk/types.ts` (`ProviderId` += `'pinned-apps'`), `src/services/native.ts`, `src/widgets/final/types.ts` (types only),
  plus the mapper in `useFinalWidgetData.ts`. Test: `useFinalWidgetData.test.ts`.
- **Spec:** native.ts: snapshot routes `'pinned-apps'` to `invoke('pinned_apps_snapshot')`; add `pinnedAppsCatalog()` (`pinned_apps_catalog`),
  `launchPinnedApp(id)` (`pinned_apps_launch`, `{ id }`), `pinSearchResult(resultId)` (`pin_search_result`, `{ resultId }`). types.ts:
  `PinnedAppItem { id: string; title: string; icon: string | null; running: boolean }`, `PinnedData { items: PinnedAppItem[] | null }`.
  `mapPinnedApps(snapshot)`: undefined unless `data.items` is an array; keep entries with an `id` matching `/^app-[0-9a-f]{16}$/` and a string `name`;
  `{ id, title: name, icon: text(icon), running: item.running === true }`.
- **Tests:** `maps pinned apps and drops malformed ids` (bad/numeric id and missing name dropped; running defaults false; non-array -> undefined).
- **Verify:** TS. **Commit:** `feat(widgets): add pinned apps provider plumbing`

### T7.5 Pinned Apps widget ID [TS] (exception: 6 type-coupled files must change together for tsc)
- **Files:** `src/widgets/final/types.ts`, `meta.ts`, `sample.ts`, `icons.tsx`, `WidgetCard.tsx`, `useFinalWidgetData.ts`. Tests: `final-widgets.test.tsx`, `WidgetManager.test.tsx`.
- **Spec:** `FinalWidgetId` += `'pinned'`; `PinnedSettings = Record<string, never>`; settings map `pinned: PinnedSettings`; data map `pinned: PinnedData`.
  meta: append `'pinned'` to `FINAL_WIDGET_IDS` and `STORE_WIDGET_IDS`; `FINAL_SETTING_FIELDS.pinned = []`; META
  `{ id:'pinned', title:'Pinned Apps', shortTitle:'Pinned', description:'Start menu apps you pin, launched by the native host.', category:'System', tint:'blue', tintRgb:'116, 183, 255', icon:'pinned', defaultSize:'compact', sizes:SIZES, defaultWidth:64, widths:{compact:64, standard:174, wide:208}, cardWidth:452, providers:provider(['pinned-apps']), permissions:['pinned-apps:read','pinned-apps:launch'], optional:true, settingsDefaults:{} }`.
  sample: `app-00000000000000a1` Browser (running), `...a2` Terminal, `...a3` Notes, icons null. icons.tsx `pinned: <path d="M9 4h6M10 4v5l-3 3h10l-3-3V4M12 12v8" />`.
  WidgetCard `SOURCE_LABELS.pinned = 'Start menu apps'`. useFinalWidgetData: `const pinnedApps = useProvider('pinned-apps', 5000)`; view
  `pinned: { data: mapPinnedApps(pinnedApps), status: statusOf(pinnedApps), preview }`; add to deps.
- **Tests:** metadata expects 15 IDs and store `['chatgpt','files','folders','trash','volume','pinned']`; WidgetManager `shows all fifteen widgets`.
- **Verify:** TS; `npm run widgets:validate`. **Commit:** `feat(widgets): register the Pinned Apps widget`

### T7.6 Pinned group in the dock and pin events [TS]
- **Files:** `src/components/dock/DockSurface.tsx`, `src/components/dock/actions.ts`, `src/components/dock/DockShell.tsx`. Test: `DockShell.test.tsx`.
- **Spec:** DockSurface renders `pinned` like `apps` (`placeholder="pinned"`; items from `views.pinned`, `sampleFor('pinned')` in preview; `running`
  from data; `onOpen={(id) => void performWidgetAction(\`pinned:launch:${id}\`).catch(() => {})}`). actions.ts, before the running-app fallback:
  `pinned:launch:` prefix -> `native.launchPinnedApp(rest)`; `'customize'` -> `native.openSettings()`. DockShell (not `settingsOnly`): listen
  `dock:pin-app`; string payload -> `setLayout((current) => current ? enableWidget(pinApp(current, payload), 'pinned') : current)`, cleanup like the
  settings listener; the `settings-updated` handler calls `refreshProvider('pinned-apps')` after `setLayout(parsed)`.
- **Tests:** record `listen` handlers by name. `launches a pinned app from its square` (saved v3, daily includes `pinned`, `pinned ['app-0123456789abcdef']`,
  snapshot returns Chrome: click `Chrome` -> `pinned_apps_launch` `{ id }`); `pins an app announced by Spotlight` (emit `dock:pin-app`; within 1500ms
  the last saved layout has the ID in `pinned` and `{ id: 'pinned' }` in daily).
- **Verify:** TS. **Commit:** `feat(dock): launch pinned apps and accept pins from Spotlight`

### T7.7 Pinned Apps card [TS+CSS]
- **Files:** `src/widgets/final/WidgetCard.tsx`, `types.ts`, `final-widgets.css`. Test: `final-widgets.test.tsx`.
- **Spec:** `WidgetAction` += `'customize' | \`pinned:launch:${string}\``. `AppTile` root `div` -> `span`. `case 'pinned'`: header; items in
  `.fw-app-row` as `<button className="fw-app-launch" aria-label={\`Open ${title}\`} disabled={!onAction} onClick={() => onAction?.(\`pinned:launch:${id}\`)}><AppTile title icon showTitles/></button>`;
  none -> `EmptyState` `No pinned apps` / `Pin apps from Spotlight with Ctrl+D or from Customize.`; footer right
  `onAction ? <button className="fw-link" onClick={() => onAction('customize')}>Manage pinned apps</button> : 'Start menu apps'`.
  CSS: `.fw-app-launch { appearance:none; padding:0; border:0; background:none; color:inherit; font:inherit; cursor:pointer; border-radius:7px }`,
  `.fw-app-launch:hover .fw-app-tile { background:rgba(255,255,255,.08) }`, `.fw-app-launch:focus-visible { outline:2px solid #0a84ff; outline-offset:2px }`.
- **Tests:** `pinned card launches apps and links to Customize` (`Open Chrome` -> `pinned:launch:app-0123456789abcdef`; `Manage pinned apps` -> `customize`; `items: []` -> `No pinned apps`).
- **Verify:** TS. **Commit:** `feat(widgets): add the Pinned Apps card`

### T7.8 Pinned apps editor in Customize [TS+CSS]
- **Files:** `src/components/settings/PinnedAppsEditor.tsx` (new), `WidgetManager.tsx`, `src/design/dock-final.css`. Test: `PinnedAppsEditor.test.tsx` (new).
- **Spec:** props `{ layout, onChange }`. Native: load `native.pinnedAppsCatalog()` once into `{id,name}[]` (IDs checked with `PINNED_ID`), errors in
  `role="alert"`; web preview: `Open the Windows app to list installed apps.`. Markup `<section className="wm-pinned" aria-label="Pinned apps"><h3>Pinned apps</h3>`;
  `ol.wm-pinned-list` of `layout.pinned` rows `.wm-pinned-row` (name or `Unavailable app`; `wm-customize` buttons `Up`/`Down`/`Unpin` labelled
  `Move {name} up|down` (disabled at the ends, `movePinned`) and `Unpin {name}` (`unpinApp`)); input `.wm-pinned-filter` `type="search"`
  `aria-label="Find an app to pin"` placeholder `Find an app`; `role="status"` `Loading installed apps` while loading; `ul.wm-pinned-results` of the
  first 50 case-insensitive name matches with button `Pin {name}` (text `Pin`; text `Pinned` and disabled when pinned; disabled at 24). WidgetManager renders
  `<PinnedAppsEditor layout={layout} onChange={onChange} />` after `WidgetSettings` when `detail === 'pinned'`. CSS: `.wm-pinned { display:grid; gap:10px; margin-top:16px }`;
  `.wm-pinned h3 { margin:0; font-size:13px; font-weight:650 }`; `.wm-pinned-list, .wm-pinned-results { display:grid; gap:6px; margin:0; padding:0; list-style:none; max-height:220px; overflow-y:auto }`;
  `.wm-pinned-row { display:flex; align-items:center; gap:8px; min-height:32px; padding:4px 8px; border:1px solid rgba(255,255,255,.08); border-radius:8px; background:rgba(255,255,255,.035) }`;
  `.wm-pinned-row > span { flex:1; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap }`;
  `.wm-pinned-filter { height:30px; padding:0 10px; border:1px solid rgba(255,255,255,.12); border-radius:8px; background:rgba(255,255,255,.05); color:var(--dock-text); font:inherit }`; `.wm-pinned-filter:focus-visible { outline:2px solid #0a84ff; outline-offset:2px }`.
- **Tests:** catalog Chrome `app-...01`, Code `app-...02`, pinned `[...02]`: `lists pinned apps with unpin and reorder` (`Unpin Code` -> `[]`);
  `filters the catalog and pins an app` (`chr` -> only `Pin Chrome`; click -> `[...02, ...01]`); `explains the web preview`.
- **Verify:** TS. **Commit:** `feat(settings): pin, unpin and reorder apps in Customize`

### T7.9 Running dots for pinned apps [RUST]
- **Files:** `src-tauri/src/pinned_apps.rs`, `src-tauri/src/running_apps.rs`, `src-tauri/src/main.rs`.
- **Spec:** `App` gains `pub exe: Option<String>`: in `enumerate`, `item.cast::<IShellItem2>()` (imports `windows::core::Interface`,
  `Win32::UI::Shell::IShellItem2`, `Win32::UI::Shell::PropertiesSystem::PROPERTYKEY`, `std::collections::HashSet`) ->
  `GetString(&LINK_TARGET)` -> string (freed with `CoTaskMemFree`) -> lowercase, kept only when it ends with `.exe`; failures -> None, where
  `const LINK_TARGET: PROPERTYKEY = PROPERTYKEY { fmtid: windows::core::GUID::from_u128(0xb9b4b3fc_2b51_4a42_b5d8_324146afcf25), pid: 2 }; // System.Link.TargetParsingPath`.
  Extract `fn item(app: &App, running: &HashSet<String>) -> Value` adding `"running": app.exe.as_ref().is_some_and(|exe| running.contains(exe))`;
  `snapshot(ids, running: &HashSet<String>)`. running_apps.rs: extract `fn process_path(hwnd: HWND) -> Option<String>` (OpenProcess +
  QueryFullProcessImageNameW, reused by `icon()`) and `pub fn running_executables(results: &[SearchResult]) -> HashSet<String>` (lowercased paths
  of `window:<hwnd>` results). main.rs: `pinned_apps_snapshot(state)` clones `state.engine`, takes `search_windows("")` inside the blocking task
  (empty when the engine is not ready) and passes `running_executables`.
- **Tests:** `pinned_items_report_running_only_for_matching_executables`; `running_executables_ignore_non_window_results`; `process_path(HWND::default())` is None.
- **Verify:** RS(pinned_apps), RS(running_apps). **Commit:** `feat(native): mark running pinned apps`

**IC7 (+native):** commands registered and permitted per window; `dock:pin-app` reaches only the dock; saved `pinned` drives the snapshot;
card Manage and editor paths work; no frontend paths or commands anywhere.

---

## Phase 8 - Spotlight redesign

Parallel-safe: T8.1, T8.2, T8.5. T8.3 needs T8.2; T8.4 needs T8.3.

### T8.1 App icons on results and compact height [RUST]
- **Files:** `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json` (search `height` 174 -> 94).
- **Spec:** `ResultRow` gains `#[serde(skip_serializing_if = "Option::is_none")] icon: Option<String>` (`None` in `publish_results`). In `search`,
  after `publish_results` collect `(index, launch_command)` for rows with `r#type == "app"` from `state.results`, `drop(state)`, set
  `results[index].icon = running_apps::shell_icon(&command)`, measure `elapsed_ms` after icons. `fit_search`: `height.clamp(94.0, 500.0)`;
  `NativeState.search_height` starts at `94.0`.
- **Tests:** `response_matches_typescript_contract`: row without icon has no `icon` key; a second row with `Some("data:image/png;base64,AA==")` serializes it.
- **Verify:** RS(response_matches). **Commit:** `feat(search): attach real app icons to app results`

### T8.2 Spotlight section model [TS]
- **Files:** `src/features/search/sections.ts` (new). Test: `sections.test.ts` (new).
- **Spec:** `SectionTitle = 'Top Hit' | 'Applications' | 'Documents' | 'Folders' | 'Other'`; `Section<T> = { title; items: T[] }`.
  `sectionFor(type)` on `type.toUpperCase()`: `APP` -> Applications; `FOLDER` -> Folders; `FILE`, `FILE_CONTENT`, `CODE`, `CODE_CONTENT`, `OCR`,
  `RECENT` -> Documents; anything else -> Other. `groupResults(rows)`: `[]` for no rows; else Top Hit `[rows[0]]`, then Applications, Documents,
  Folders, Other from the remaining rows in engine order, omitting empty sections. `SPOTLIGHT_BAR = 94`;
  `spotlightHeight(sections, hasCalc, showStatus)`: `rows` = sum of 56 per Top Hit item and 44 per other item; no rows and no calc -> `showStatus ? 150 : 94`;
  else `Math.min(500, 94 + 16 + (hasCalc ? 76 : 0) + sections.length * 26 + rows)`.
- **Tests:** `sectionFor` for `app`, `FOLDER`, `FILE`, `OCR`, `CODE_CONTENT`, `ACTION`; `groupResults` keeps engine order and puts the first row in
  Top Hit; `spotlightHeight`: empty 94, empty with status 150, calc + one row 268, 30 rows 500.
- **Verify:** TS. **Commit:** `feat(search): add Spotlight section model`

### T8.3 Sectioned Spotlight with real app icons [TS]
- **Files:** `src/features/search/SpotlightWindow.tsx`. Test: `SpotlightWindow.test.tsx`.
- **Spec:** `Result.icon?: string | null`. `sections = groupResults(rows)`, `ordered = sections.flatMap((s) => s.items)`, `items = calc ? [calc, ...ordered] : ordered`,
  `rowIndexOf = new Map(ordered.map((r, i) => [r.id, i]))`; `activeId` uses `ordered`. The stale guard's `nextItems` uses the same order
  (`[...(nextCalc ? [nextCalc] : []), ...groupResults(nextRows).flatMap((s) => s.items)]`). Listbox renders each section as
  `<div className="ps-section" role="group" aria-label={title}><div className="ps-section-title" aria-hidden="true">{title}</div>...</div>`. Rows keep
  `id={\`result-${rowIndex}\`}`, add class `is-top` in Top Hit; icon box `data-icon={image ? 'image' : kind}` with
  `<img className="ps-result-img" src={image} alt="" draggable={false} />` when `image = safeAppIcon(result.icon)` (from `../../widgets/final/primitives`),
  else the SVG. Placeholder `Spotlight Search`. The status div adds `ps-sr-only` when the query is empty. Height: `fit_search` with
  `spotlightHeight(sections, hasCalc, Boolean(error) || (!!query.trim() && !searching && !items.length))`.
- **Tests:** fit heights 174 -> 94 (two tests), 236 -> 268, 500 kept; `[data-icon]` order `tsx, terminal, vscode, spotify, cpp, rust, folder`;
  `groups results into Top Hit and typed sections` (FILE, app, FOLDER, ACTION -> listbox groups `Top Hit`, `Applications`, `Folders`, `Other`);
  `shows Rust app icons as images and falls back for unsafe icons` (one `img.ps-result-img`; unsafe `Notes` row keeps `data-icon="app"`).
- **Verify:** TS. **Commit:** `feat(search): sectioned Spotlight results with real app icons`

### T8.4 Open animation and pin from Spotlight [TS]
- **Files:** `src/features/search/SpotlightWindow.tsx`. Test: `SpotlightWindow.test.tsx`.
- **Spec:** `rootRef` on the section; effect on `opened` (skip 0 and reduced motion):
  `rootRef.current?.animate?.([{ opacity: 0, scale: '0.97', translate: '0 -8px' }, { opacity: 1, scale: '1', translate: '0 0' }], { duration: 200, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' })`
  (individual `scale`/`translate` sidestep native.css `transform: none !important`). Input keydown Ctrl/Cmd+D: preventDefault; when the selected item's
  `type.toLowerCase() === 'app'`, `invoke('pin_search_result', { resultId })`; success sets `pinnedId`, failure sets `error`. Selected app rows show
  `<span className="ps-result-hint">{pinnedId === id ? 'Pinned to Dock' : 'Ctrl+D to pin'}</span>`.
- **Tests:** `Ctrl+D pins the selected app result to the dock`; `Ctrl+D ignores non-app results`; `replays the open animation per show unless reduced motion`
  (stub and restore `HTMLElement.prototype.animate`; a `search-query` event calls it once with the arguments above; with `matchMedia` `{ matches: true }` no call).
- **Verify:** TS. **Commit:** `feat(search): animate Spotlight open and pin apps with Ctrl+D`

### T8.5 Tahoe Spotlight CSS [CSS]
- **Files:** `src/design/spotlight.css` (new), `src/design/native.css`, `src/main.tsx` (import spotlight.css last).
- **Spec:** native.css `.ps-spotlight.native-spotlight` (keep `!important`): background `rgba(16,22,34,.90)`; border `1px solid rgba(255,255,255,.14)`;
  radius 22px; shadow `0 30px 80px rgba(0,0,0,.55), 0 10px 24px rgba(0,0,0,.35), inset 0 1px 0 rgba(255,255,255,.22)`; both backdrop-filters
  `blur(48px) saturate(190%) contrast(104%)`. spotlight.css, every selector prefixed `.native-spotlight`:
  `.ps-search-top { height:56px; padding:0 18px; gap:12px; border-bottom:1px solid rgba(255,255,255,.08) }`; `.ps-search-brand { width:22px; height:22px; flex-basis:22px; color:rgba(235,235,245,.6) }` (svg 22px);
  `.ps-search-input { height:30px; font:400 22px/30px "Segoe UI Variable Display","Segoe UI",sans-serif; letter-spacing:-.2px; color:#f5f5f7 }`, placeholder `rgba(235,235,245,.38)`;
  `.ps-scope-bar { height:36px; padding:0 14px; gap:6px; background:transparent; border-bottom:1px solid rgba(255,255,255,.06) }`;
  `.ps-scope { height:24px; padding:0 11px; border:1px solid rgba(255,255,255,.08); border-radius:999px; background:rgba(255,255,255,.06); color:rgba(235,235,245,.72); font:600 11.5px/22px "Segoe UI Variable Text","Segoe UI",sans-serif; transition:background 140ms ease-out, color 140ms ease-out }`,
  hover `background:rgba(255,255,255,.12); color:#fff`, pressed `background:rgba(10,132,255,.9); border-color:#0a84ff; color:#fff`, focus-visible ring;
  `.ps-results-column { padding:6px 8px 10px; mask-image:none; -webkit-mask-image:none }`;
  `.ps-section-title { height:26px; padding:0 10px; font:600 11px/26px "Segoe UI Variable Text","Segoe UI",sans-serif; letter-spacing:.02em; color:rgba(235,235,245,.5) }`;
  `.ps-result { min-height:44px; padding:6px 10px; gap:12px; border:0; border-radius:10px; box-sizing:border-box }`, hover `rgba(255,255,255,.06)`,
  `.ps-result.is-selected { background:rgba(10,132,255,.88) }` (title `#fff`, detail `rgba(255,255,255,.82)`), `.ps-result.is-top { min-height:56px }`;
  `.ps-result-ic { width:32px; height:32px; flex:0 0 32px; background:transparent }` (svg 26px; Top Hit box 40px, svg 32px); `.ps-result-img { width:100%; height:100%; object-fit:contain }`;
  `.ps-result-copy b { font:500 13.5px/18px "Segoe UI Variable Text","Segoe UI",sans-serif; color:#f5f5f7 }` (Top Hit `600 15px/20px`); `.ps-result-copy small { font-size:11.5px; line-height:15px; color:rgba(235,235,245,.55) }`;
  `.ps-result-hint { margin-left:auto; flex:none; font:600 11px/1 "Segoe UI Variable Text","Segoe UI",sans-serif; color:rgba(255,255,255,.82) }`;
  `.ps-calc-hero { height:72px; margin:0 0 4px; border-radius:12px; box-sizing:border-box }`; `@media (prefers-reduced-motion: reduce) { .native-spotlight .ps-scope { transition:none } }`.
- **Verify:** TS; `npm run build`. **Commit:** `style(search): Tahoe glass Spotlight`

**IC8 (+native):** `ResultRow.icon` reaches `safeAppIcon`; `fit_search` minimum equals `SPOTLIGHT_BAR`; Ctrl+D -> `pin_search_result` ->
`dock:pin-app` -> DockShell; keyboard, scope and stale-guard tests still pass.

---

## Phase 9 - Per-widget functionality audit and fixes

| Widget | Tile click | Wheel | Right-click | Card actions | Gap found | Fix |
|---|---|---|---|---|---|---|
| Start | `open_start` opens a `shell:AppsFolder` window | - | card | Open Start | Not the Start menu; card "Recent" lists open windows | T9.1, T9.6 |
| Search | Spotlight | - | card | hero opens Spotlight | Card claims "Local index is empty"/"Index offline" with no data | T9.5 |
| Running Apps | activate (T1.3) | - | card | none | Card tiles do not activate | T9.6 |
| Pinned Apps | launch (T7.6) | - | card | launch, manage | - | - |
| Spotify | card | - | card | prev/play/next, volume set | - | - |
| Weather, System, Claude, ChatGPT, Agent, Clock | card | - | card | read-only or settings links | - (honest states kept) | - |
| Recent Files | card | - | card | rows send `folder:open:<label>` | Wrong target; fails unless the label is a known folder | T9.2-T9.4 |
| Quick Folders | card | - | card | `folder:open:<id>` | - | - |
| Trash | card | - | card | Empty | Permanent empty in one click | T9.5 |
| Volume | card | none | card | mute, up, down, set | No wheel control | T9.7 |

Parallel-safe: T9.1, T9.2, T9.3, T9.7. T9.4 -> T9.5 -> T9.6 share WidgetCard.tsx.

### T9.1 Start opens the Windows Start menu [RUST]
- **Files:** `src-tauri/src/main.rs`, `src-tauri/Cargo.toml` (features += `"Win32_UI_Input_KeyboardAndMouse"`).
- **Spec:** import `SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VK_LWIN` from
  `windows::Win32::UI::Input::KeyboardAndMouse`. `fn start_key_inputs() -> [INPUT; 2]`: two `INPUT_KEYBOARD` inputs
  (`ki: KEYBDINPUT { wVk: VK_LWIN, wScan: 0, time: 0, dwExtraInfo: 0, dwFlags }`), flags `KEYBD_EVENT_FLAGS(0)` then `KEYEVENTF_KEYUP`. `open_start`:
  `// SAFETY: two initialized keyboard inputs; SendInput copies them.` then return Ok when
  `unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } == 2`; otherwise keep the existing `shell:AppsFolder` fallback and its comment.
- **Tests:** `start_key_presses_and_releases_the_windows_key` (inspect both inputs inside `unsafe`; never call `open_start` in tests).
- **Verify:** RS(start_key). **Commit:** `feat(native): open the Windows Start menu from the Start tile`

### T9.2 Recent files open by opaque ID [RUST]
- **Files:** `src-tauri/src/known_folders.rs`, `src-tauri/src/widget_data.rs`.
- **Spec:** every recent JSON entry gains `"id": recent_id(&file.path)`; `pub(crate) fn recent_id(path: &str) -> String { format!("recent-{:016x}", crate::pinned_apps::fnv1a64(path)) }`;
  `pub(crate) fn recent_path(id) -> Option<PathBuf>`: rescan with `snapshot()`, find `data.recent[*].id == id`, require `is_local_absolute_path` and
  `is_file()`; `pub(crate) fn opens_directly(path) -> bool`: `file_kind` is image, pdf, document, spreadsheet, presentation, video or audio.
  widget_data `action`, after `folder:open:`: `recent:open:<id>` -> `recent_path` (else `Recent file is no longer available`); open the file when
  `opens_directly`, otherwise its parent folder (code, scripts, archives, executables are revealed, never run); non-UTF-8 -> error.
- **Tests:** `recent_ids_are_opaque_and_case_insensitive`; `recent_paths_resolve_only_from_the_current_scan`; `executables_and_scripts_are_revealed_not_opened`
  (`a.pdf` true; `a.exe`, `a.js`, `a.ps1`, `a.zip` false); add `recent:open:`, `recent:open:../x`, `recent:open:recent-0000000000000000` to the rejected
  list in `folder_actions_are_allowlisted_and_malformed_volume_actions_are_rejected`.
- **Verify:** RS(recent), RS(folder_actions). **Commit:** `feat(native): open recent files by opaque ID`

### T9.3 Recent file IDs in the renderer [TS]
- **Files:** `src/widgets/final/types.ts` (`FileItem.id: string`), `useFinalWidgetData.ts` (`id: text(item.id) ?? ''`), `sample.ts` (ids `recent-0000000000000001`..`04`).
  Test: `useFinalWidgetData.test.ts` (known-folders test expects the id).
- **Verify:** TS. **Commit:** `feat(widgets): carry recent file IDs`

### T9.4 Open recent files from the card [TS]
- **Files:** `src/widgets/final/WidgetCard.tsx`, `src/components/dock/actions.ts`, `src/services/native.ts`. One-liner: `types.ts` `WidgetAction` += `` `recent:open:${string}` ``. Test: `final-widgets.test.tsx`.
- **Spec:** `WidgetActionId` += `` `recent:open:${string}` ``. File row `key={file.id || file.path}`, `onClick={() => onAction?.(\`recent:open:${file.id}\`)}`,
  `disabled={!onAction || !file.id}`. actions.ts routes `recent:open:` like `folder:open:` (native action, refresh `known-folders`).
- **Tests:** `recent file rows open by ID` (`recent:open:recent-0000000000000001`).
- **Verify:** TS. **Commit:** `fix(widgets): open the clicked recent file`

### T9.5 Safe Trash and honest Search card [TS]
- **Files:** `src/widgets/final/WidgetCard.tsx`. Test: `final-widgets.test.tsx`.
- **Spec:** component `TrashEmpty({ count, disabled, onConfirm })` with local `confirming`: `Empty Recycle Bin` sets it; then
  `<span role="status">Permanently delete {count} items?</span>` with `Empty permanently` (calls `onConfirm`, resets) and `Cancel`. Search card: with
  no `searchData` render no EmptyState and footer right `Local search`. Chips unchanged.
- **Tests:** the dispatch test clicks `Empty permanently` after `Empty Recycle Bin`; `trash needs confirmation before emptying`; `search card makes
  no index claims without data` (no `Local index is empty`, no `Index offline`).
- **Verify:** TS. **Commit:** `fix(widgets): confirm Trash emptying and drop false index status`

### T9.6 Running-app card activation and Start card copy [TS]
- **Files:** `src/widgets/final/WidgetCard.tsx`, `types.ts`, `useFinalWidgetData.ts`. Tests: `final-widgets.test.tsx`, `useFinalWidgetData.test.ts`.
- **Spec:** `WidgetAction` += `` `running-${number}` ``. Apps card tiles become `fw-app-launch` buttons (`Switch to ${title}`, `onAction?.(item.id as \`running-${number}\`)`).
  Start card copy: `Recent: ` -> `Open now: `, `Recent apps appear here` -> `Open windows appear here`. `mapWindows(snapshot, pinned?: Snapshot)` fills
  `pinnedApps` with `mapPinnedApps(pinned)` titles (null when absent); `views.windows` passes `pinnedApps`.
- **Tests:** `apps card switches to a window` (`running-12`); Start preview shows `Open now:`; mapWindows with pinned -> `['Chrome']`, without -> null.
- **Verify:** TS. **Commit:** `feat(widgets): activate windows from the Running Apps card`

### T9.7 Volume wheel on the tile [TS]
- **Files:** `src/components/dock/DockSurface.tsx`. Test: `DockShell.test.tsx`.
- **Spec:** volume slot `onWheel`: ignore events within 120ms of the last handled one (ref timestamp); `deltaY < 0` -> `volume:up`, `> 0` -> `volume:down` via `performWidgetAction`.
- **Tests:** `wheel over the volume tile steps the volume` (deltaY -100 -> `widget_action` `{ actionId: 'volume:up' }`; an immediate second wheel adds nothing).
- **Verify:** TS. **Commit:** `feat(dock): adjust volume with the wheel on its tile`

**IC9 (+native):** every table action maps to an allowlisted Rust action; `open_start` is never invoked by tests.

---

## Phase 10 - Documentation

Parallel-safe: T10.1, T10.2.

### T10.1 Design reference [DOCS]
- **Files:** `docs/design-reference.md`.
- **Spec:** keep source, baseline and comparison sections; replace "Geometry and states" with section 2 of this plan; list superseded rules (82px dock,
  66px tiles, "no magnification, bounce or spring", unsectioned search); cite the 2026-11-07 user decisions and `preview-tahoe-glass.html`.
- **Commit:** `docs(design): adopt the Tahoe tile dock reference`

### T10.2 Contract, widget README, status [DOCS]
- **Files:** `docs/widgets/CONTRACT.md`, `docs/widgets/README.md`, `docs/qa/STATUS.md`.
- **Spec:** CONTRACT: Pinned Apps (permissions, AppsFolder catalog, ID formula, layout-sourced IDs, `pinned_apps_*`, `pin_search_result` accepting only
  engine app results and emitting `dock:pin-app` to the dock), `recent:open:<id>` (rescan; documents open, others revealed), `fit_dock` edge/align
  validation, edge-aware card placement, Start via the Windows key with AppsFolder fallback. README: Pinned Apps entry (provider, permissions, 5s
  refresh, layout v3 migration, limits: UWP apps get no running dot; pinned and running squares are not merged). STATUS: section "macOS dock pass -
  2026-11-07" with landed tasks and IC command results, the no-scaffold decision, unused `.fw-dock*` CSS, backdrop blur not sampling the desktop in
  transparent WebView2 windows, the Windows-key UIPI limit, and every Phase 11 CU item marked **unverified** until a Computer Use run records evidence.
- **Commit:** `docs(qa): record the macOS dock pass and open native gates`

**IC10:** doc links resolve; no doc claims native verification.

---

## Phase 11 - Final verification

Integrator: full gate + debug build; back up `%APPDATA%\protonsearch\protondock-layout.json` before the first launch; launch a temp copy. Then a
Computer Use run (bundled skill; honour physical-Escape stops and locked desktops; never bypass them) checks the list at 100%, 125% and 150% DPI.
Record each result with evidence (`docs/qa/native/dock-tahoe-*.png`) or the exact blocker in docs/qa/STATUS.md. Nothing counts as verified unless observed.

- CU-1 Idle dock: 60px glass rail, 46px rounded tiles, live glyphs, separate shelf and Customize pill; no text pills (vs `dock-original-separate.png`).
- CU-2 Magnification: about 71px peak, neighbours spread, glass grows, nothing clipped, tooltip above the hovered tile, relaxes in about 150ms, shelf hides and returns.
- CU-3 Hit regions: clicks 20px above an idle rail (away from the shelf) reach the window behind; while hovered only the headroom is captured.
- CU-4 Magnification off in Customize: no scaling; tooltips remain.
- CU-5 Windows "Animation effects" off, relaunch: no magnification, card pop, Spotlight animation or equalizer motion.
- CU-6 Keyboard: Tab focus rings and tooltips; Enter (card/Start/Spotlight); Shift+F10 card; Alt+Arrow reorder keeps focus.
- CU-7 Running Apps: real icons and dots; click activates; right-click card; card tiles switch windows.
- CU-8 Pinned Apps: Ctrl+D in Spotlight pins (widget appears in the active profile); click launches; dot on running Win32 apps; Manage opens Customize; editor pin/unpin/reorder persists.
- CU-9 Cards: spring pop from the tile; close inside the top-right; Escape/close animate out; focus loss hides; card about 16px from the tile.
- CU-10 Edges: bottom/left/right x start/center/end placement; vertical rail; tooltips and dots on the correct sides; cards beside the dock; click-through outside; monitor move.
- CU-11 Drag: Start dragged to the middle shows the indicator and persists after relaunch; Escape cancels; no card opens after a drag.
- CU-12 Spotlight: Alt+Space shows the 94px bar with animation; Top Hit/Applications/Documents/Folders; real app icons; arrows, Enter, Escape, Tab scopes, blue selection.
- CU-13 Phase 9 actions: Start menu, volume wheel, Trash confirm, recent file open/reveal, folders, media controls.
- CU-14 Persistence: a v2 layout migrates on first save; edge/align/magnify/pins/order survive relaunch; unknown widget IDs remain in the file.
