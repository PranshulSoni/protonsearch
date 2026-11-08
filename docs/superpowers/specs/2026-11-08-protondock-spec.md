# ProtonDock top-level specification

Date: 2026-11-08. Branch: `dock`. Status: authoritative for planning; nothing here is natively verified.
Supersedes: `antigravity/spec.md` and `antigravity/BACKEND_IMPLEMENTATION_PROMPT.md` (both unchanged, input only).
Authority: `CLAUDE.md`, `AGENTS.md`, `rules.md` > `docs/widgets/CONTRACT.md` > this spec > the two audits as detailed contracts (UI: `docs/superpowers/specs/2026-11-08-dock-design-system.md`, "DS"; backend: `docs/superpowers/specs/2026-11-08-dock-backend-spec.md`, "BS") > pinned HTML `docs/design/dock-editable.2126c49.html` (DS 1.4) > `.design.md` > antigravity inputs.
Plan: `docs/superpowers/plans/2026-11-08-protondock-plan.md`.

## 1. Product scope and non-goals

Scope: a native Windows dock (Tauri 2, Rust host, React UI) built only from widgets the user adds, orders and sizes; profiles (four built-ins plus custom); three themes (Dark, Light, Liquid Glass); Capsules and Lines styles; Edit mode; a Spotlight launcher (Kensaku UI) over the existing Rust ProtonSearch engine, which stays authoritative (no second index, no JS index); bottom edge first, then left and right edges with start, center and end alignment; AppBar space reservation and an optional "Hide Windows taskbar" switch.
Non-goals: replacing Explorer's shell (Start, tray flyout and toasts stay Explorer's); hosting the tray; remote calendar or mail accounts; calendar writes; Wi-Fi password entry; radio toggles; any listening socket; any administrator right or package identity; telemetry.
Rules carried everywhere: no fabricated value shown as live (missing is `--` or an honest state, DS 4.4); the renderer never supplies a path, command, URI, process id or window handle (BS 7.4); Rust validates provider ids, action ids and arguments; saved user data is migrated, never reset (unknown ids kept, other profiles untouched, v2 backup kept); every motion honours `prefers-reduced-motion`; every control has visible focus and a label.

## 2. Decision register

| Id | Decision | Status |
|---|---|---|
| D1 | Liquid Glass is the smoked recipe `rgba(22,24,28,.66)`; contrast never depends on blur | accepted default (vetoable) |
| D2 | Calendar glyph flat `#e11d48`, no gradient, no coloured shadow | accepted default (vetoable) |
| D3 | Widget `assistant` (alias `siri-ai`), neutral robot outline, no orb | accepted default (vetoable) |
| D4 | Glyphs: icons 24px, dials 28px, app icons 24px in a 32px box | accepted default (vetoable) |
| D5 | Bottom and centre only | OVERRIDDEN: user wants orientation and free placement; bottom first, left and right edges plus start, center, end alignment in a later phase; `edge` and `align` stay in `fit.ts` |
| D6 | Shelf keeps all five control groups; fades while the rail is hot | accepted default (vetoable) |
| D7 | `files`, `folders`, `trash`, `volume`, `agent` stay under "More widgets" with generic tile anatomy | accepted default (vetoable) |
| D8 | Width steps 48, 96, 140, 168, 220 with a per-widget allowed subset, no slider | accepted default (vetoable) |
| D9 | No toast HUD; visually hidden `aria-live` region | accepted default (vetoable) |
| D10 | No Widget Store tab | accepted default (vetoable) |
| V1 to V28 | Deviations from the HTML (DS 4.11) | accepted unless the user vetoes a row |
| BD-1 | `codex` canonical; `chatgpt` is a load-time alias (same Codex logs); `agent` stays separate | decided (orchestrator) |
| BD-2 | Layout stays JSON v3; quick notes are `.txt` files in the engine notes folder; no `protondock.db`, no second index, no new clipboard table | decided (user) |
| BD-3 | Wi-Fi: read, connect to saved profiles, disconnect, Settings handoff; no password entry, no radio toggle | default |
| BD-4 | One battery level per Bluetooth device; no AirPods left, right, case | default |
| BD-5 | `system-tray` out of default profiles; gated spike later (UI Automation flyout, else "Background apps") | decided (orchestrator) |
| BD-6 | Assistant: unconfigured tile only, no backend | default |
| BD-7 | Unknown widget ids kept in `unknown[]` | default |
| BD-8 | Weather default unconfigured after materializing old settings for existing items | default |
| BD-9 | Dock registers as an AppBar and reserves its space; "Hide Windows taskbar" switch in Customize, off by default, uses the Windows auto-hide state (never `SW_HIDE`), restored on exit and by `--restore-taskbar` | decided (user) |
| BD-10 | Native confirmation for `trash:empty` | default |
| BD-11 | ICS subset only; recurrence and time zones deferred, reported honestly | default |
| BD-12 | Clipboard widget deferred until ingestion is ported | default |
| BD-13 | Keyless AI usage: local logs only, no API-key flow, no fake quotas, never the OAuth usage endpoint | decided (orchestrator) |
| BD-14 | No toast-count badges on running apps | default |
| O-1 | Every simulated or fabricated control is removed: battery simulate toggle, AI working or ready test toggle, random CPU walk, connect buttons that ignore input, frozen analog clock, sample devices and networks (DS 3.2 B1, B2; BS 8) | decided (orchestrator) |
| O-2 | Native acceptance uses the Computer Use tool on the real app whenever it is present in the session; when absent the exact blocker goes to `docs/qa/STATUS.md`; nothing is verified without observation | decided (user) |

## 3. Resolution of disagreements between the audits

| # | Topic | Resolution |
|---|---|---|
| R1 (XD-1) | `unsupported` status | TS `WidgetStatus` adds `unsupported`; UI renders it as "unavailable" with the provider message as tooltip. Backend: `empty` means no hardware, `unsupported` means the API is missing. |
| R2 (XD-2) | Icon size request | One ladder for running, pinned and installed apps: `iconPx` = smallest of 32, 48, 64, 96, 128, 256 not below `ceil(24 * 1.22 * dpr)`; native validates and never upscales (DS 4.5, BS 4.5.4). |
| R3 (XD-3) | `chatgpt` versus `codex` | BD-1: alias. |
| R4 (XD-4) | Layout v3 | DS 5.4 shape; `isLayout` whitelist adds `profileNames`, `profileOrder`, `pinnedApps`, `layoutV2Backup` (and `dock` in the edges phase); native readers accept versions 1 to 3 and profile ids `[a-z0-9][a-z0-9-]{0,23}`; Rust keeps a previous-good `.bak` (BS 6.3). |
| R5 (XD-5) | Ahmedabad weather default | BD-8; materialize before flipping (BS 3.3 rule 6). |
| R6 (XD-6) | Search sub-label | `provider_snapshot("hotkey")` supplies registered and failed shortcuts. |
| R7 (XD-7) | Taskbar | BD-9 as decided by the user. |
| R8 (XD-8) | `system-tray` "always live" | False on this build; tile shows unavailable, absent from default profiles until BD-5's spike. |
| R9 (XD-9) | File overlaps | Hot files are serialized by the plan. |
| R10 (XD-10) | Toast HUD | D9. |
| R11 (XD-11) | Every provider polled for every profile | Subscriptions become demand driven before any new provider lands. |
| R12 | DS 5.2 moves the rail glass onto the rail; DS 4.8 animates glass extents during a morph | The glass stays a rail pseudo-element whose `left` and `right` animate (`--glass-l`, `--glass-r`); the DS 4.2 roles style that layer. |
| R13 | D5 says edge and align are not stored | Overridden: layout v3 gains an optional `dock: {edge, align}` (default bottom, center) in the edges phase, additive and whitelisted. |
| R14 | Widget defaults | Native code holds none; the UI copies the pinned HTML `DEFAULT_PROFILES` minus `system-tray` and `notifications`. |
| R15 | Claude and Codex "Connect account" | BD-13: the setup card states which local sessions were found; Claude shows tokens only, percentages `--`. |
| R16 | DS 7 says Computer Use is blocked (no `orca`) | Superseded by O-2. |

## 4. Widget catalogue

Tiers: system reads OS state without setup; user needs the user's content or setup. Widths: allowed (default). Profiles: d daily, w work, f founder, x focus.

| Id | Aliases | Tier | Default profiles | Widths | Source | Verdict | Honest empty state |
|---|---|---|---|---|---|---|---|
| start | windows | system | d w f x | 48 (48) | `open_start`: Win key `SendInput`, labelled Apps-folder fallback | feasible, caveat (UIPI) | always live |
| search | | system | d w f x | 48, 140 (48) | engine `search`, `execute_search_result` | feasible | always live; sub-label hotkey or none |
| running-apps | apps | system | d w f | 48 each | `running_apps_snapshot` (+`key`, `iconPx`) | feasible | renders nothing (edit mode: dashed placeholder) |
| pinned-apps | | user | w f | 48 each | `pinned_apps_snapshot`, `installed_apps_snapshot`, `app:launch:<key>` | feasible | edit mode placeholder; missing keys shown as missing |
| system-stats | system | system | d w f x | 48, 96, 220 (220) | `system` + `storage` providers | feasible | `--` per value; CPU null until second sample |
| calendar | | system | d w f x | 48, 140 (48) | browser date; card: ICS grant `calendar_snapshot` + week | tile feasible, events caveat | "Select a calendar" (disconnected) |
| peripherals | | system | d | 48, 140 (48) | `bluetooth` provider device battery | feasible, caveat | `--`, "No device reports battery" |
| music-player | spotify | system | d f x | 48, 168, 220 (168) | `media` provider + `media_artwork` | feasible | note glyph, "Nothing playing" |
| weather | | user | d w | 48, 96, 140, 220 (220) | `weather_snapshot`, `weather_geocode`, `weather_locate` | feasible | `--`, "Set location" |
| status-battery | | system | w | 48, 96, 140 (48) | `battery` provider | feasible | plug glyph, "No battery detected" |
| wifi | | system | w f | 48, 140 (48) | `network.wifi`, `wifi`, `wifi-networks`, `wifi:*` | feasible, caveat (location consent) | dimmed arcs; access needed: `settings:location` |
| bluetooth | | system | w | 48, 140 (48) | `bluetooth` provider | feasible, caveat | dimmed, "Bluetooth is off" |
| notifications | | system, opt-in | none | 48, 140 (48) | `notifications` count, list (spike first) | caveat (spike) | bell with slash, "Allow access in Windows settings" |
| quick-notes | | user | w x | 48, 140 (48) | `notes` provider, `note_save`, `notes:delete:<id>` | feasible | "No notes yet" |
| claude | | user | f | 48, 140 (48) | `ai_usage_snapshot.claude` (tokens, `working`) | feasible, caveat | "No local Claude sessions" |
| codex | chatgpt | user | f | 48, 140 (48) | `ai_usage_snapshot.chatgpt` + agent status | feasible | "No local Codex sessions" |
| assistant | siri-ai | user | none | 48 (48) | none (BD-6) | blocked | unconfigured, "Set up assistant" |
| system-tray | | system | none | 48 (48) | none in v1 (BD-5) | blocked as specified | unavailable |
| clock | | system | d w f x | 48, 96, 140 (48) | browser clock | feasible | always live |
| agent | | user | none | 48, 140 (48) | `ai_usage_snapshot.agent` | feasible | "No active session" |
| files | | system | none | 48, 140 (48) | `known_folders_snapshot` | feasible | "No recent files" |
| folders | | system | none | 48, 140 (48) | `known_folders_snapshot`, `folder:open:<id>` | feasible | always live |
| trash | | system | none | 48, 140 (48) | `trash` provider, `trash:empty` (confirmed) | feasible | "Recycle Bin is empty" |
| volume | | system | none | 48, 140 (48) | `volume` provider, `volume:*` | feasible | unavailable, "No output device" |

Deferred, never mapped or deleted: legacy SDK `clipboard` (BD-12), `focus`, and the `reference-widgets.json` ids (BS 3.2).

## 5. Exclusions

| Excluded | Reason |
|---|---|
| MCP server on `127.0.0.1:41782` | Listening socket reachable by every local process and by web pages via DNS rebinding; its tools take file paths (BS 4.5.3). |
| ANC and transparency control | Apple AACP over Classic L2CAP has no public user-mode Windows API (BS 4.4.7). |
| Tray enumeration via `Shell_TrayWnd` children | `SysPager` and `ToolbarWindow32` do not exist on build 26300 (BS 4.4.16). |
| Windows Calendar store | Needs the restricted `appointmentsSystem` capability (BS 4.4.6). |
| API-key quota percentages | No key-based API returns subscription quota; Admin APIs are org spend only; the OAuth endpoint is undocumented and a policy risk (BS 4.4.14, BD-13). |
| `protondock.db`, second search index, new clipboard table, `fastembed` | BD-2; engine removed embeddings on purpose (BS 4.5.5). |
| `ShowWindow(SW_HIDE)` on the taskbar | Breaks Start, the tray flyout and toasts (BS 4.6). |
| Wi-Fi password entry, radio toggles, calendar writes, `toggle_agent_execution` | Secrets through the renderer, unverified capability, contract violations (BS 2.7). |
| Toast HUD, Widget Store tab, simulate controls, brand look-alike icons | D9, D10, O-1, V28. |

## 6. Quality bar (each item has a named phase and native check in the plan)

1. No horizontal jitter idle, hovered, resizing or morphing (`CLAUDE.md` 1): verified in phase A, re-checked in B, J, K.
2. Crisp app icons at the display's device pixel ratio, never upscaled (`CLAUDE.md` 2): A, D, G.
3. Motion preserved (profile switch, hover, card open, widget transitions); reduced motion is the only off-switch (`CLAUDE.md` 3): B.
4. Restrained surfaces: black, white or smoked glass, no category tints; solid Windows logo (`CLAUDE.md` 4): C.
5. Glyphs read at 32 to 96px and match or beat `preview-tahoe-glass.html` (`CLAUDE.md` 5): D.
6. Honesty: no fabricated or simulated value anywhere on the native path (O-1): C and every widget phase.
7. Data preservation: v1 and v2 layouts migrate losslessly with `layoutV2Backup` and the `.bak` file (BS 6.10): A, D.
8. Security contract (CONTRACT.md, BS 7): every phase; guard test lands in phase B.

## 7. Pointers to the detailed contracts

- Tokens, colour roles, themes: DS 4.1, 4.2. Tile anatomy, glyph grid, widths: DS 4.3. States: DS 4.4. Icons at DPR: DS 4.5. Badges, dots, tooltip: DS 4.6. Cards: DS 4.7. Motion and reduced motion: DS 4.8. Focus and keyboard: DS 4.9. Shelf, Edit, new profile, Customizer: DS 4.10. Deviations: DS 4.11.
- Reconciliation with landed code (magnify, fit, sizing delete): DS 5.1, 5.2. Layout v3 and migration table: DS 5.4. Dock-level behaviour (fit, hit regions, overflow): DS 6. Native acceptance checks: DS 7.
- IPC contract (providers, commands, `widget_action` grammar, TS types, polling): BS 5. Persistence: BS 6 (6.3 native save, 6.4 notes, 6.10 migration tests). Security: BS 7. Honesty register: BS 8. Backend packages BE-01 to BE-71: BS 9. Feasibility per widget: BS 4.
