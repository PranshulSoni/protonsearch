# Dynamic Island V10 redesign (2026-11-12)

Owner brief for the implementers. Goal: the island looks deliberate and calm, every card is wired to
real data, and nothing grows past a fixed height. Search keeps its current UI (colors + animated
highlight only). Read this whole file before editing.

## Stack (installed by the foundation agent)
- `motion` (import from `motion/react`), `lucide-react` (16 px, `strokeWidth={1.75}`), `@tanstack/react-virtual` (lists > 40 rows).
- Tailwind v4 via `@tailwindcss/vite`; shadcn/ui (new-york, dark only) in `src/components/ui/*`, `cn()` in `src/lib/utils.ts`, alias `@/` -> `src/`.
- shadcn components: command, scroll-area, tabs, context-menu, tooltip (+ whatever a referenced block pulls in).
- Tailwind entry `src/island/island-tw.css` is imported ONLY by `IslandView`, and `App.tsx` lazy-loads `IslandView`, so Tailwind preflight never reaches the launcher or main window. Verify the launcher/main CSS chunk has no preflight after `npm run build`.

## Tokens (dark only; define once as CSS vars + Tailwind `@theme`)
| token | value | use |
|---|---|---|
| surface | `#0d1012` | island panel (opaque, no blur, no transparency) |
| raised | `#151a1d` | tiles, inputs, tab track |
| hover | `#1b2226` | row hover |
| active | `#22303a` | selected row / highlight pill |
| border | `#262d31` | 1 px hairlines |
| text | `#eef1f3` / `#a3adb4` / `#6f7a82` | primary / secondary / tertiary |
| accent | `#9cc3dc` | ONE accent: focus ring, active tab, slider fill, toggle on |
| ok / warn / danger | `#7fc8a0` / `#e2b86b` / `#e5847a` | status only, never decoration |
Radius: surface 28 (compact pill 999), tile 14, row 10, chip 999. Spacing 4/8/12/16. Card padding 16.
Type: Inter Variable; base 13 px; titles 15 px/600; meta 12 px; `tabular-nums` for times/percent.

## Hard rules
- Max surface height **460 px** (`MAX_ISLAND_HEIGHT` in islandDims.ts). Card bodies scroll inside a ScrollArea; no card, list, drill-down or empty state grows the surface beyond 460.
- No gradients, glow, glassmorphism, neon, purple, emoji, sparkles, fake data, lorem, or decorative badges. Honest empty/error/unsupported states with one useful action.
- Copy is short and concrete ("5 pages", "Copied 2 min ago"); no instructional footers ("Select a page to open", "Enter pastes here" are removed).
- Motion only on transform/opacity (layout animations via motion `layout`); `prefers-reduced-motion` -> instant.
- Invoke the `antislop:antislop-ui` skill before styling and follow it.

## Motion
- Island morph (apex dynamic-island): surface uses motion `layout` with `{ type: "spring", duration: 0.5, bounce: 0.12 }`; radius animates with it; card content swaps with `AnimatePresence mode="popLayout"` (out: opacity 0 + y -6, 140 ms; in: opacity 1 from y 6, 220 ms). The morph must settle within `MORPH_SETTLE_MS` (600) in IslandView.
- Highlight: one shared `layoutId` pill behind the active row (search results, clipboard, browser, agents), spring `{ stiffness: 520, damping: 42 }`. Keyboard and hover move it smoothly; it never jumps.
- Gooey navigation (apex gooey nav) replaces the page dots: SVG goo filter, active blob stretches between positions, same click/keyboard behaviour as `IslandPageDots`.
- Animated tabs (apex animated tabs): shared-pill tabs for clipboard kinds, browser All/Today/Pinned, search filters.
- Sliders (apex slider): volume, brightness, media seek. Stepper (apex stepper): agent task progress.

## Contracts (do not break)
- Every card keeps its exported name and props type (drop-in swap). Keep the existing data wiring: `api.*`, `islandData.ts`, `islandNativeCards.ts`, `lib/todoStore.ts`, `lib/results.ts`. Read the old card first and keep every action it supports.
- Window sizing goes only through IslandView `fitWindow`/`armSettle` + `islandCardDims`. Cards never call `setIslandGeometry`.
- Keyboard: Esc chain, Ctrl+Up/Down switches cards, arrows move the highlight, Enter activates, Tab order sane, visible focus ring (accent, 2 px).
- Perf: rows memoized, long lists virtualized, no per-frame React state, no layout reads in render.

## Card mapping (slot -> component -> reference)
| slot | component | reference / notes |
|---|---|---|
| search | SearchCard (IslandCards.tsx) | keep layout; new colors; animated highlight + animated filter tabs |
| media | MediaCard | https://21st.dev/@chetanverma16/components/audio-player ; real GSMTC session from `ControlsStatus.media` (title, artist, artwork, position/duration, play/pause/next/prev, output picker) |
| clipboard | ClipboardCard | https://www.shadcn.io/blocks/radix/command-menu-clipboard ; remove the "Enter pastes here" hint; images + text + pin + copy back |
| agent | AgentsCard | https://ui.ballmac.com/components/agent-roster ; Pi-only rules stay; stepper for task progress |
| controls | ControlsCard | https://ui.ballmac.com/components/control-center (fallback https://webberui.com/docs/control-center-panel) ; real volume/mute/brightness/Wi-Fi/Bluetooth/output + settings links |
| browser | BrowserCard | screenshot spec: header (icon, "Browser History", count), search field, tabs All/Today/Pinned, day groups, favicon + title + domain . browser + relative time, pin + hide actions; NO footer hint; pins/hides persist in localStorage |
| git | GitCard | restyle only, same data |
| bell | TodoCard | `npx shadcn@latest add https://www.shadcn.io/r/todo-list-compact.json` look; keep todoStore persistence + date/time picker |
| shell | IslandView + GooeyNav | https://apex-ui.in/components/dynamic-island , apex gooey navigation, animated tabs, slider, stepper (https://apex-ui.in/components) |
If a reference is paid or unreachable: screenshot it (firecrawl `screenshot` format) and replicate the look with shadcn + Tailwind + motion.

## File ownership (parallel agents must not collide)
- Foundation/shell agent: package.json/lock, vite.config.ts, tsconfig*.json, components.json, `src/components/ui/*`, `src/lib/utils.ts`, `src/island/island-tw.css`, App.tsx (lazy), IslandView.tsx, IslandPageDots.tsx -> GooeyNav, SearchCard in IslandCards.tsx, islandDims.ts (cap), shared motion primitives in `src/island/ui/*` (AnimatedTabs, HighlightList, Slider, Stepper).
- Card agents: NEW files only, `src/island/cards/<Name>Card.tsx` (+ `<Name>Card.test.tsx`). Never edit IslandView/IslandCards/NativeCards/island.css. Report the card's target height (<= 460).
- Integrator: swaps imports in IslandView, deletes the replaced code and dead CSS, updates islandDims + tests, builds and screenshots.

## Verify
- `cd apps/desktop && npx tsc --noEmit && npx vitest run && npm run build` (all green; fix tests you break).
- Release (integrator only): `cd apps/desktop && npm run tauri build -- --no-bundle` (custom-protocol; a plain `cargo build --release` loads the dev URL and shows a WebView2 error page). Never run the exe inside the repo (Low integrity label): use `scripts/run-release.ps1`.
- After edits: `git status --short --untracked-files=all` and delete untracked 0-byte junk files the cmd hook creates from `>` in tool input.
