# PROTONSEARCH V2 — PROJECT PHOENIX
## Final master implementation prompt: Rust engine • V9 Dynamic Island • Centered launcher • Native Windows integrations • Agent hierarchy • Complete functional audit

**ROLE:** You are the Level-0 Program Director, Principal Windows Systems Engineer, Product Design Director, and Integration Orchestrator responsible for delivering a real, installable, reliable, exceptionally fast ProtonSearch v2. **Do not produce just a plan, screenshots, scaffolding, or pretty UI. Investigate, implement, test, review, integrate, benchmark, and ship an application whose features truly work.**

**Product repository:** https://github.com/PranshulSoni/protonsearch  
**Benchmark/comparison reference:** https://github.com/yjid/kensaku  
**Authoritative Dynamic Island reference:** `protonsearch_dynamic_island_cards_v9_polished.html` (user-provided, attached).  
**Additional historical product/UI references (if supplied):** original ProtonSearch codebase, earlier `protonsearch-v2-ui-reference.html`, brand/logo imagery, original app screens, and V9 UI audit. Historical references may help with features, desktop app and onboarding; **V9 alone controls the Dynamic Island's visual design and interaction behavior**. Do not replace V9 with V4, V5, V6, V7, a radial wheel, a horizontal navigation strip, or an invented dashboard.  
**New supplemental settings reference:** `protonsearch_island_settings_placement_concept.html` (this handoff includes a separate interactive placement/card-management demonstration; it supplements, but does not supersede, V9).  

If a file is missing, identify it in your initial audit. Never pretend to have inspected a file you cannot access. If the tool environment can launch subagents, use them. If it cannot, fulfill the same named roles in deliberate sequential passes and disclose the limitation. Preserve user's existing app data, functionality, and Git history. Make a safe backup/rollback plan before schema or architecture changes.

---

# 0. THE NON-NEGOTIABLE PRODUCT CONTRACT

You are building **one product, two alternative primary launchers, one shared Rust core, and one full settings/management application**:

1. **Normal Launcher**: centered floating desktop launcher, polished and keyboard-first; user explicitly chooses it in Settings/onboarding. Search box and visible, tasteful filters **All, Files, Folders, Apps** (additional sources available through search syntax, quick destinations, and contextual actions without cluttering the top row). Empty state exposes the original ProtonSearch quick destinations in a well-composed design: Browser Bookmarks, Browser History, Clipboard History, Agents; Git and Agent History remain accessible, not erased. Preview support must work. It must open at the CENTER of the relevant display, respect the work area, and not accidentally occupy the entire screen.
2. **Dynamic Island**: optional alternate launcher, **V9 VISUAL REFERENCE IS BINDING**. By default completely hidden until invoked. Alt+Space summons a compact floating top-positioned Search capsule with the original P logo. Typing reveals results. Switching between cards changes the SAME surface's shape and dimensions with smooth, coherent motion. **Exactly preserve V9's layout language, SVG icon sizing, media controls, Wi-Fi/Bluetooth split buttons, clipboard formatting, eight clickable pagination dots, wheel/swipe switching, slide/fade choreography, and clipping fixes**, subject only to changes demonstrably required by production platform behavior, accessibility, or bug fixes. Do NOT add a radial menu, permanent scrollable category bar, giant dashboard, arbitrary arrows, green online dots, gratuitous blur, purple gradients, or unrelated aesthetic redesigns.
3. **Full desktop settings/management app**: distinct from the two quick launchers. Include welcoming home, personalized greeting, indexing/source management, appearance, performance diagnostics, clipboard management, agent management, permissions and privacy, hotkeys, updates, troubleshooting, and critically **Dynamic Island placement and card customization**. Visually polished and consistent with ProtonSearch brand, but not forced into the tiny island dimensions.
4. **Existing source parity**: every previously functioning ProtonSearch source/action must remain available: local files/folders, app search/launch, full-text document/code search, OCR/screenshot search, Browser Bookmarks, Browser History, Git Commits, clipboard history, actions/commands, agents and agent history, quicklinks/snippets/calculator/window controls if actually present, plus original settings. Produce a verified **before/after parity matrix**; never silently remove a feature.
5. **Critical existing bugs explicitly reported by the user:**
   - **File/text previews do not show currently.** Repair the entire preview pipeline—not simply a decorative preview panel. Search results must reveal actual extracted text, accurate snippets with matched-term highlighting, and real previews of supported documents, code, images, OCR content and metadata, with safe, explicit fallbacks for unsupported formats.
   - **Clipboard does not capture images currently.** Implement a real Windows clipboard image listener, decoding, persistence, thumbnails, categorized history, and reliable re-copy/paste/preview. Handle image contents, not just file paths or text strings.
6. **Everything must be end-to-end real.** UI actions must call actual backend services (or show a clearly honest unsupported state). Prototype sample data, mock device lists, fake agents, fabricated search counts, fake music titles, and canned notifications are forbidden in production outside a distinctly labeled demo/test fixture.
7. **Performance goal:** aggressively optimize filename, folder, full-text, OCR-index lookup, UI activation, scroll, indexing efficiency, memory and power. “World's fastest” is a desired challenge, not an unverified claim. Publish reproducible benchmarks versus relevant comparable tools and clearly distinguish OCR extraction time from indexed OCR search time.
8. **Visual goal:** V9's smoothness is a measurable acceptance target, not an excuse for vague “Apple-like” descriptions. Use the exact reference’s morphing dimensions, timings, curves, easing, corner behavior, clip boundaries, icon alignment, visual density, card states and pagination behavior as your baseline. Browser preview smoothness can differ by hardware; measure and close perceptual gaps on real Windows machines rather than declaring identical fidelity without evidence.

### Precedence when requirements conflict
1. Latest explicit user requests in this brief.
2. User-attached **V9 Dynamic Island** for island visuals, card interactions and motion.
3. User-supplied settings-placement concept for the placement editor only.
4. Original ProtonSearch code, functional parity and existing data as the source of behavioral truth.
5. Earlier ProtonSearch design references for normal launcher/onboarding/full app only.
6. Competitor patterns as inspiration, not requirements or permission to copy assets.

---

# 1. MANDATORY HIERARCHY OF SUB-AGENTS

**You are Level 0, the Program Director/Final Integrator.** Your top-level tasks: maintain a real work breakdown, enforce design fidelity, resolve interface contracts, obtain cross-functional reviews, block bad merges, validate every feature and important combination, own the installer and deliver the final product. Do not delegate away accountability. Create `docs/implementation/WORK_BREAKDOWN.md`, `FEATURE_PARITY_MATRIX.md`, `FEATURE_TEST_MATRIX.md`, `CROSS_FEATURE_PAIRWISE_MATRIX.md`, `ARCHITECTURE_DECISIONS.md`, `BENCHMARK_PLAN.md`, `BUG_LEDGER.md`, and `RELEASE_GATES.md` before large changes.

## Level 1 — five subordinate team leads

**L1-A: Rust Core / Windows Platform Lead** — search-core architecture, Windows OS access, IPC, sources, persistence, app launching, process/window management, clipboard, media, Wi-Fi/Bluetooth and supported native controls, preview backend.

**L1-B: UI / Motion / Product Design Lead** — V9 island fidelity, centered launcher, desktop settings, real-time event presentation, accessibility, high-DPI, responsive handling, icons, motion, clipping and visual QA.

**L1-C: Search Quality / Performance Lead** — filesystem indexing, corpus architecture, full-text/OCR retrieval, semantic optionality, rank merging, latency measurements, thread contention, energy and memory budgets.

**L1-D: Agent Runtime / Extensibility Lead** — evaluate and integrate an EXISTING Python agent framework/CLI runtime as an optional sidecar, MCP tool contracts, agent hierarchy/status/streams, user approval gates, activity history, extensibility APIs.

**L1-E: Reliability / Security / Integration / Release Lead** — security boundaries, test harness, verification across views and connected devices, crash recovery, data migration, installers, updates, code signing plan, CI, regression, and release evidence.

## Level 2 — specialist agents, delegated by the appropriate lead

- **A1:** Windows hotkeys, focus, Tauri/WebView2 window lifecycle, multi-monitor/DPI/placement, taskbar/fullscreen behavior.
- **A2:** Windows filesystem enumeration, USN change journal where supported, watchers/fallbacks, file IDs, rename tracking.
- **A3:** Clipboard **text + bitmap/image formats**, persistence, type classification, dedup, privacy and clipboard operations.
- **A4:** Preview/document extraction service: actual content snippets, formatting, image/PDF/doc/code/OCR preview, error recovery.
- **A5:** Media/session controls and OS device services: GSMTC, sound output, Wi-Fi, Bluetooth, brightness/volume/etc. with capability gates.
- **B1:** **V9 Dynamic Island renderer** and eight-card componentization with pixel/motion comparisons.
- **B2:** **Centered Normal Launcher** and noncluttered filters/quick destinations.
- **B3:** Full desktop app, home, onboarding, theme/settings system.
- **B4:** **Visual island placement preview** with screens, draggable placements, X/Y offsets, edge constraints, presets and persist/preview/cancel.
- **B5:** **Card manager**: enabled/hidden/pinned/default, reordering, keyboard accessibility, compact card behavior, pagination order sync.
- **B6:** Animation performance, icons, accessibility, responsiveness, visual regression screenshot/video testing.
- **C1:** Filename/folder candidate and rank retrieval.
- **C2:** Full-text/code/content indexing and query planner.
- **C3:** OCR extractor/thumbnail and indexed OCR search pipeline.
- **C4:** Benchmark, profiling, relevance tests and regression thresholds.
- **D1:** Python runtime/sidecar evaluation and packaging.
- **D2:** Agent status/controls and protocol, orchestration tree and task history.
- **D3:** MCP/ACP/permissioned tools and approval UX.
- **E1:** E2E native Windows test automation including keyboard/multi-monitor/DPI.
- **E2:** Security/privacy, clipboard and agent threat model.
- **E3:** Pairwise feature-combination test matrix and failure recovery.
- **E4:** Installer, migration/updater/CI/documentation/release.

**Level 3**: permitted narrow temporary workers, e.g., testing CF_DIBV5 screenshot history, fixing one PDF snippet extractor, measuring one p95 regression, auditing one V9 card at 125% DPI. They must have task ownership, evidence and scoped edits. Don’t spawn agents merely to increase agent count.

### Strict delegation and integration protocol

Each assigned work item requires an OWNER, independently designated REVIEWER, file ownership, prerequisites, expected outputs, interfaces/contracts, acceptance tests, risk, rollback, and a reproducible verification command. Use parallel worktrees/branches only if available, prevent simultaneous writes to shared files, use narrowly defined interfaces. Prefer small incremental PRs with CI. Cross-team review on Rust-to-TypeScript IPC, clipboard image handling, preview extraction, card ordering, OS privilege boundaries and agent execution. L0 integrates and independently validates. **Do not mark work complete because an agent says “done.”** Require executable evidence. If subagent execution tooling is unavailable, use the hierarchy as structured sequential responsibility passes and document that fact.

Required agent handoff format:
```
TASK ID / OWNER / REVIEWER / DEPENDENCIES
Existing behavior verified (with evidence)
Files, schemas, APIs and interfaces changed
Tests executed + results + environment
Visual evidence (screenshots/video) if UI-related
Latency/resource measurements if performance-related
Security/permission/migration impact
Known limitations or blockers
Reviewer verdict / Integration verdict / Rollback strategy
```

---

# 2. FIRST: FORENSIC AUDIT OF THE ORIGINAL PRODUCT

Before any major rewrite:

- Clone original ProtonSearch and preserve a `pre-v2-baseline` tag/branch. Inspect licenses, README feature claims, issues, source tree, Rust crates, Win32 code, data schemas, browser/history integration, existing UI, existing OCR, Git and agents, installer and updater.
- Build and run **on Windows**, on an ordinary user account, without changing global system configuration without approval. Record logs, screenshots, memory and process-tree startup behavior. Reproduce the user's **broken text/file preview** and **clipboard image not recorded** bugs with exact test cases.
- Inventory every existing feature and every supported launcher pathway. For each entry: `Exists?`, `Runs?`, `Works?`, `Correct output?`, `Error handling?`, `Native rights needed?`, `Regression coverage?`.
- Verify storage locations and migration logic. Back up clipboard DB/history, source-index DB, preferences, hotkeys, exclusions, browser connector permissions and any agent data. Never wipe history on upgrade by default. Include no telemetry without opt-in.
- Run a baseline performance suite with actual filename datasets, mixed PDF/DOCX/code/image corpora, cold/warm runs, antivirus enabled, and constrained memory/high CPU tests.
- Inspect and render `protonsearch_dynamic_island_cards_v9_polished.html` using a browser. Capture each of its eight card states, each drill-down, search empty/results, clipboard Code/Images tabs, notifications with/without items, agents with deep hierarchy and Wi-Fi/Bluetooth chevron subpanels. Record at least 100%, 125%, 150%, 200% scaling targets and narrow widths.
- Inspect the code for unsafe subprocess execution, clipboard leakage, browser DB locking, SQL injection, path traversal, malformed UTF-8, unbounded memory, indexing loops, and privilege escalation.
- Study Kensaku and Everything objectively; never use star count or repo size as performance evidence.

Outputs: verified functional audit, failing-bug reproductions, baseline metrics, index schema/migration plan, architectural ADRs, work breakdown and dependencies. Do not proceed blindly.

---

# 3. PRODUCTION ARCHITECTURE: ONE CORE / TWO LAUNCHERS / OPTIONAL AGENTS

Suggested shape; refine after audit and profiling:

```text
                Alt+Space / native Windows hotkeys
                              │
                 Native Window Coordinator (Rust)
                  ┌───────────┴──────────┐
          Normal Launcher          Dynamic Island V9
          centered window          top floating island
                  └───────────┬──────────┘
                       Shared UI model
                       Typed IPC/Event bus
                              │
                      Rust Query Coordinator
             ┌──────────┬──────────┬──────────┐
          Hot in-RAM    Content    Source adapters/Actions
          filename     FTS+OCR    Apps/Browser/Git/Clipboard
          directory    previews   OS/MCP/Agent history
             └──────────┴──────────┴──────────┘
             Persistent indexes + bounded async workers
                              │
                Optional Python agent sidecar
                (on-demand, independently killable)
```

**Hard boundaries**: Rust search, metadata and hotkey handling can operate without Python, a model, network, OCR engine, or WebView. UI never blocks on disk extraction or agent initialization. Search requests get sequence IDs/cancellation tokens; late results must not replace newer ones. IPC validates inputs and maintains authorization. Features use one source registry and query/ranking engine; two launchers are presentations, not two independent implementations.

Evaluate Tauri 2 + WebView2 + React/TypeScript against existing native UI. If migrating, do it incrementally with working feature parity. WebView2 overhead is real; measure cold/warm activation, hidden process tree, renderer memory and focus stability. Keep preinitialized hidden launcher if measured beneficial, minimize unnecessary JS bundle/IPC chatter, virtualize result lists, stream updates and preload only necessary icons. The full settings application can initialize separately from the fast search path.

---

# 4. THE DYNAMIC ISLAND — V9 DESIGN & MOTION ARE BINDING

### 4.1 Preserve V9, do NOT invent another interface

The eight **logical card IDs and default order** as inspected in the V9 reference are:

1. `search` — fast search capsule and expandable results.
2. `media` — compact Now Playing, shuffle, previous, play/pause, next, repeat, output-device drill-down.
3. `clipboard` — category tabs All / Text / Images / Links / Code / Files / Pinned, real thumbnails and compact text/code previews.
4. `agent` — running agents, parent/child hierarchy, status, launch/pause/resume/stop, logs and permissions.
5. `controls` — Wi-Fi, Bluetooth, airplane, energy saver, hotspot, casting, night light, brightness and volume, customize controls.
6. `browser` — bookmarks and browser history, with source/browser filtering and real open actions.
7. `git` — git commits and repository information; real navigation to commit/repo.
8. `bell` — notifications/alerts: actual supported sources, dismiss/clear/read state.

**Exact V9 card fidelity checkpoints:** tiny initially hidden capsule; V9 P logo; dark graphite surfaces and restrained navy; correctly sized icons/chevrons; proper card heights and corner radii; no clipping at narrower widths; dedicated click targets in Quick Settings; code thumbnails never explode to giant images; search content is keyboard-first; clickable pagination dots reflect active card order; scroll/swipe switches cards when appropriate; inner list scroll does NOT accidentally switch cards; deep views (Wi-Fi, Bluetooth, audio output, etc.) morph within the island; `Escape` behavior is predictable; animation interruptions cancel cleanly. Preserve existing media source-device iconography and arrow positions from V9.

### 4.2 Smoothness and actual native behavior

- Capture V9 reference animation using an automated browser at 60 Hz; derive width/height/corner/easing/opacity/color/slide timings and relative choreography. Implement with CSS transforms/opacities and judicious width/height animation only where unavoidable; profile layout thrashing. Favor one composited content transition per card change. Never queue multiple stale transitions; latest user input wins. Respect `prefers-reduced-motion` and Windows animation accessibility settings.
- Define measurable interaction goals: on supported hardware stable 60 fps and 120 Hz target, p95 input-to-highlight under one rendered frame when feasible; track long tasks, dropped frames, p50/p95/p99 frame time, layout shifts, hotkey activation, focus time and rapid-scroll responsiveness. These are stretch goals, not achieved figures.
- With V9 HTML and production running side by side, record screen captures at identical resolutions and compare each state. Fix icon glyph size, padding, line height, scroll region, selection, backdrop, shadows, border color, corner profile and animation direction. Perform manual real-feel testing on 60 Hz and high-refresh displays.
- Detect scroll context. Outer island wheel/swipe selects cards; user scrolling inner clipboard/search/agent/notification content stays INSIDE that card. Dragging sliders, seek bars or changing card order must never inadvertently switch cards. Keyboard Tab/Shift+Tab, arrows, Escape, Enter, Space, screen readers and focus restoration must function. Dots include accessible labels and reflect user-custom ordering.
- When launching/hiding: no white flash, no input focus loss, no abrupt jump, no accidental taskbar icon unless selected, no phantom click-through when closed. Handle topmost policies and fullscreen/exclusive apps responsibly.

### 4.3 Cards are functional, not demo scenery

**Search** — real app/file/folder/action sources, typed filters and relevance; no fake counts. Supports file preview, context actions, keyboard results; empty input stays compact exactly as V9 unless a user-enabled idle suggestion option is configured. Default search is all sources. Nested categories are presented only when useful, not a horizontal rail of seven huge tabs.

**Media** — use Windows-supported Global System Media Transport Controls/session-manager APIs or suitable platform adapters for active sessions, metadata/artwork, seek/position if supported, pause/play, previous/next, shuffle/repeat only where exposed, and output-device selection using appropriate Windows audio endpoint APIs. Display disabled capability accurately. Never fabricate song or device state.

**Clipboard** — real Win32-backed image/text/html/RTF/file list formats and categories, real previews, pin/delete/copy/paste/restore, controlled history retention. Image thumbnails never disappear or turn into a text placeholder after restart. Support content filtering within the card. Never expose clipboard passwords unintentionally; include privacy exclusions and pause.

**Agents** — real agent process/task state, persisted IDs and parent-child relationships, permission requests, logs, launch/pause/resume/cancel (with honest semantics when runtime cannot pause), queued/running/completed/failed/error/offline. No pretend percentage without real progress data; use indeterminate progress otherwise.

**Quick Settings** — split tile behavior: clicking Wi-Fi/Bluetooth primary toggle differs from clicking right-hand chevron, which drills down to available networks/devices. Appropriate permission and supported-API checks. Nonavailable capabilities show disabled/redirect, never lie. Volume, brightness and output route respect hardware limitations. Toggles don't silently change unrelated OS preferences.

**Browser / Git** — real source adapters, permission/availability messaging, correct dedup/timestamps and working open actions.

**Notifications** — user-authorized, feasible app notifications and system-source integrations; unsupported third-party Windows notifications must not be represented as universally accessible. Include local ProtonSearch notifications by default; implement Windows integration only as technically supported and user-approved.

### 4.4 Card configuration requirements

Users may reorder, hide/show, pin, choose default card, reset to original V9 order, and set per-card visibility while closed/while typing/while media playing (where appropriate). Cards have stable IDs; order changes update pagination dots and scroll/swipe navigation without changing IDs or erasing card state. Search must remain accessible (cannot disable all paths into it). Preserve V9 dot styling and spacing. Avoid showing dots for hidden cards. Support optional 'skip disabled/unavailable cards' and 'return to Search on next invocation.' Persist across restarts and migrations.

---

# 5. NORMAL CENTERED LAUNCHER — AN EQUALLY REAL FIRST-CLASS OPTION

Provide Settings > Launcher > **Primary launch mode**: `Normal Launcher` or `Dynamic Island`; default is user choice in first-run onboarding. Both are included in one install and users can change at any time without rebuilding indexes or restarting the agent runtime.

**Normal Launcher:** open centered relative to CURRENT monitor/work area, correct when focus is on secondary monitor, handle mixed DPI, taskbar and window focus, borderless appropriate shape, consistent P branding, smooth but restrained entrance/exit and selected-row transitions. Visible search filters: **All, Files, Folders, Apps**. Contextual options can access Actions, Git, Clipboard, Agents, Browser and others through the search box, autocomplete/aliases, appropriate sidebar or compact quick shortcuts—**do not clutter with eight rows of tabs**. Empty search should offer useful quick destinations: Browser Bookmarks, Browser History, Clipboard History and Agents, with access to Git and Agent History, along with user-customizable recent items. Users must not lose the old quick sources. Keyboard-first selection, hit highlights, file preview, right-click/secondary actions and accessibility.

**Normal and island feature parity**: same source adapters, ranking, indexing, clipboard categories and history, agent task model, preview data and permissions; difference only in presentation/density. Verify every feature via both paths. Normal launcher is NOT merely the island resized to 800px.

Global shortcuts: primary Alt+Space (unless conflicting with existing OS shortcut; provide conflict detection/remapping). Provide optional separate hotkeys for opening the alternative mode and specific functions, user-controlled, no collisions, release hotkeys on exit. Restore focus to previous window on dismiss correctly. Window background indexing and search-core startup remain independent.

---

# 6. FULL SETTINGS / MANAGEMENT APP — REQUIRED SECTIONS

Provide a coherent desktop app with sidebar or clear navigation. At minimum:

**Home:** personalized greeting (user-provided name, stored locally), current index health, useful recent activity if opted in, quick launcher action and actionable alerts. No decorative fake analytics.

**Search & Indexing:** watched folders, exclusions, filename vs full-text vs OCR coverage, source status, per-source health, crawler backlog, index rebuild/repair, CPU/power scheduling, search ranking and hotkey diagnostics.

**Appearance & Motion:** light/dark/system mode, typography density, animations/reduced-motion, transparency/accessibility, selection appearance, V9 default preservation, preview sizing.

**Launcher:** choose Normal vs Dynamic Island, shortcuts, on-focus monitor selection, normal-launcher width/height/density/centering, search defaults, quick destinations, optional start minimized and reopen behavior.

**Dynamic Island:** **rich graphical placement editor**, position, size, monitor selection, display rules, active card order, hide/reorder and live preview. Entire settings interaction must be just as polished and responsive as V9; it is a full-size app, not a dark island stretched to settings dimensions.

**Clipboard:** enable/disable history, collect images, capture limits/file formats, classifications, categories, pinned entries, dedup controls, privacy exclusion patterns, storage size, encryption capabilities, clear/export, monitor health.

**Agent Runtime:** runtime choice and install/health verification, connected model/provider configuration (secret storage), allowed tools, permissions, workflows, agent logs/hierarchy/history, persistent agent behavior and CPU/memory limits.

**Windows Integrations:** media sessions, output devices, Wi-Fi, Bluetooth, audio/brightness hardware compatibility, notifications, browser sources, Git, windows/actions; show permissions and unsupported states honestly.

**Storage/Privacy/Updates/Diagnostics:** database location, usage and integrity, migration/recovery, logs with sensitive values redacted, opt-in telemetry, updates, rollback, settings export/import, version and troubleshooting.

## 6.1 VISUAL ISLAND PLACEMENT EDITOR — CRITICAL

Implement a user-friendly *graphical monitor-layout preview* (not merely numbers). Display correct aspect ratio of each enumerated monitor and its work area; selectable monitor tabs; taskbar, window edge and safe areas. Render a scaled V9 island ghost at the actual chosen anchor and offsets. Drag/drop the island ghost inside the monitor illustration to reposition; snap to anchors and update offset values. Include selectable presets:

- Top center (V9 default)
- Top left / top right
- Bottom center / bottom left / bottom right
- Optional custom coordinate placement with X/Y offsets
- An optional keep-on-current-monitor / follow-active-window-monitor behavior

**Live preview** must display a noninteractive ghost first; support explicit `Test placement` to show a temporary actual overlay with a visible `Revert` control and automatic safe recovery on lost monitor / out-of-bounds settings. **Apply / Cancel / Reset** must work; changes persist reliably. Keep all positions within usable display bounds, respect DPI scaling, safe screen edges and taskbar. Never trap users with an unreachable floating island. Provide both mouse drag and keyboard arrow adjustments, exact numeric input, and obvious focus. If multi-monitor configuration changes while running, relocate safely and preserve user-intended monitor preference if it reappears.

Show preview for ALL eight card shapes (idle search, populated search, media, clipboard, agents, controls, browser, Git, notifications) to make sure a placement works for tall expanded cards. Prevent expanded cards from extending past edges: auto-flip expansion direction or clamp intelligently with stable animation, without teleporting the input/focus point. Handle screen readers and reduced motion.

## 6.2 CARD ORDER / VISIBILITY MANAGER

Display an interactive ordered list of the eight V9 cards. Each row: consistent icon, card name, enable/disable, pin, keyboard-friendly move up/down, drag handle, and optional per-card settings. Reorder updates a LIVE PAGINATION DOT PREVIEW and wheel/swipe sequence immediately. Offer `Reset to V9 order`, `Select default card`, `Show only enabled`, `Restore defaults` and undo before applying.

Card list reordering must NOT mean reordering unrelated search sources. Cards have stable `card_id` keys; persist ordered ID array separately from enabled flags; reject duplicate/unknown IDs; migrate older preferences safely; default card fallback if disabled. Search must remain accessible in all configurations.

### Suggested settings state contract (revise based on existing config):
```ts
type LauncherMode = 'normal' | 'dynamic_island';
type CardId = 'search' | 'media' | 'clipboard' | 'agent' | 'controls' | 'browser' | 'git' | 'bell';
type Anchor = 'top_left' | 'top_center' | 'top_right' | 'bottom_left' | 'bottom_center' | 'bottom_right' | 'custom';
interface IslandPreferences {
  enabled: boolean;
  monitor: { strategy: 'primary'|'cursor'|'foreground'|'specific'; deviceId?: string };
  anchor: Anchor;
  offsetDips: { x: number; y: number }; // monitor-relative, DIP units, not raw physical pixels
  expandedBoundsPolicy: 'clamp'|'flip_then_clamp';
  orderedCards: CardId[];
  hiddenCards: CardId[];
  defaultCard: CardId;
  returnToSearchOnShow: boolean;
  animations: 'system'|'full'|'reduced';
}
interface ProductSettings { primaryLauncher: LauncherMode; island: IslandPreferences; }
```
Source of truth is the typed Rust settings schema with a versioned migration; TS types are generated or checked against that schema. Atomic writes, validation, migration tests, and rollback for bad config are mandatory.

---

# 7. URGENT BUG P0: FILE / TEXT / DOCUMENT PREVIEW IS BROKEN

Treat this as a top-priority repair in an existing shipped behavior. Diagnose the end-to-end flow:

```
Search hit -> stable file ID/path -> extraction/index record -> snippet/preview service
  -> typed Rust IPC payload -> UI state -> selected result preview render
```

Check each link. Don't fix only the layout and call it done. Test preview independently of indexing and via both launchers.

**Required behavior**:
- Plain-text files: actual lines, Unicode/BOM/encoding detection or safe fallback, accurate snippet surrounding matched words, line numbers where appropriate; no fabricated content.
- Source code: common languages, syntax highlighting, match highlight and exact line/column jump where supported; long files, tabs, CRLF, malformed encoding and binary detection.
- Markdown/HTML/JSON/CSV: safe preview with suitable formatting, text/metadata and sanitized HTML; never execute file script, macro or embedded HTML.
- PDF: text extraction from regular PDF, page number/snippet; scanned PDF via queued OCR once; clear loading/progress; handle encrypted PDFs with explicit status.
- DOCX and supported Office formats: safe parse and text excerpt; no macro execution, temp-file leaks or unsafe conversion. Other formats must show explicit unsupported/metadata fallback rather than blank.
- Images/screenshots: real thumbnail and metadata; optional indexed OCR snippet highlighted with confidence/position if available. No repeated OCR on every keystroke.
- Preview loading: selected-result debouncing/cancellation, async loading, bounded memory/caches, stale response guards, skeleton/error/empty states, metadata correctness, stale-cache invalidation on file change/rename/delete.
- Zero leakage: previews only from allowed indexed paths, path validation, avoid automatic external-resource fetching from HTML/doc previews, respect exclusions, do not load huge files into the UI thread.

**Required test corpus**: ASCII, Hindi/CJK/emoji filenames and content, 0-byte file, 1 MB/100 MB text, CRLF/LF, UTF-8/UTF-16 BOM, invalid bytes, PDF with selectable text, scanned PDF, DOCX with tables, Markdown and code with matches, deleted/moved file, locked/encrypted file, unsupported binary. Tests verify **actual expected excerpts**, not only that a component is visible.

**Acceptance**: selecting a result yields either a verified correct preview/snippet or clear documented unsupported/error behavior; no silent blank panel. Screenshots and automated tests for each supported file type and both launcher modes.

---

# 8. URGENT BUG P0: CLIPBOARD IMAGES NOT RECORDED

Implement real native Windows clipboard capture. Search the existing code for text-only assumptions, missed clipboard update events, format filtering and decode/storage failures.

- Subscribe to clipboard updates using appropriate Win32 mechanisms (such as `AddClipboardFormatListener` / `WM_CLIPBOARDUPDATE`) or a tested compatible library. Respect clipboard ownership and retry briefly when another process temporarily locks the clipboard; never busy-spin or block hotkey/search input.
- Correctly identify and decode actual image formats including **CF_DIB, CF_DIBV5, CF_BITMAP when exposed, registered PNG or equivalent format,** and relevant alternatives; handle clipboard data coming from Snipping Tool (`Win+Shift+S`), browsers, Paint, screenshot apps, image editors, and Office applications. Platform format negotiation matters; test real apps.
- Distinguish **image bitmap contents** from **CF_HDROP file references**, HTML, text, RTF and other clipboard formats. Some clipboard entries expose multiple formats for one underlying copy event: choose best representation, deduplicate formats into ONE history item, preserve useful alternate representations where feasible.
- Convert into a stable internal representation (e.g., encoded PNG + content hash + dimensions + thumbnail) using bounded safe image decoding, alpha/orientation/large-image protection, optional background downsampling and explicit storage quota. Do not block UI or indexer while decoding. Store metadata and binary payload atomically; verify persistence across restarts.
- Show thumbnails in both V9 clipboard card and full history view. Make image re-copy/paste work in compatible target apps; on unsupported targets show a clear error rather than fake success. Add pinned image entries, deletion, cross-category search and correct sort order.
- Classes: `All`, `Text`, `Images`, `Links`, `Code`, `Files`, `Pinned`. Each item has appropriate native classification and user-overridable category; a code snippet should not render as an enormous SVG. Mixed format/HTML image cases should be tested.
- Privacy: optional exclude sensitive apps/processes where reliably detectable, ignore identical updates, pause history, retention policies, secure wipe behavior consistent with storage architecture, clear-on-exit option, redact clipboard content in logs, never forward to agents/cloud models without explicit request and confirmation. Do not pretend automatic password detection is perfect.

**Mandatory real-world tests**:
1. Copy text from Notepad and recopy from history.
2. Copy code from VS Code, verify Code category and exact multiline content.
3. Copy an actual PNG bitmap from Paint (NOT an image file path), inspect thumbnail, restart app, paste back into Paint.
4. Snip with `Win+Shift+S`, paste via ProtonSearch and verify pixels/dimensions.
5. Copy browser image via “Copy image” and compare formats.
6. Copy URL, confirm Links category.
7. Copy file(s) via Explorer, confirm Files rather than image bitmap.
8. Copy HTML-rich content, verify classification/dedup and safe rendering.
9. Large/high-DPI transparent images, rapid sequential copying, empty clipboard, clipboard locked by another process, restart during capture, pinned item retention, delete/corrupt record recovery.
10. Verify no duplicates on multiple clipboard notifications for one copy.

**Acceptance**: real image pixel history, real thumb, restorable/pasteable bitmap, correct categories, persisted across restart, and both launcher presentations work. Unit mock tests do not replace actual Windows copy/paste tests.

---

# 9. ULTRA-FAST SEARCH / INDEX / OCR ENGINE

Research, prototype, measure and keep the best measured engineering option—not an ideological rewrite.

**Filename/folder engine**: high-performance Windows/NTFS enumeration where appropriate, stable file IDs, USN journal change tracking with permissions and journal wrap handling, watcher/crawler fallback for non-NTFS and network volumes, compact hot in-memory file metadata, prefix/exact candidates before fuzzy, top-k ranking, cancellation, bounded allocations, Unicode normalization and case rules, correct folder/app discrimination. Protect from deleted/symlink loops and unstable network volumes.

**Content**: compare existing SQLite FTS5 against Tantivy and other options on representative mixed datasets, indexing update cost, phrase/boolean queries, highlights/snippets, relevance; prefer existing working indexes if measurements show adequate performance. Index extracted content incrementally with content hashing, skip unchanged files, safely parse DOCX/PDF/code/Markdown and other supported formats. Support index health/repair and incremental version migration.

**OCR**: separate **OCR extraction throughput/accuracy** from **searching already extracted OCR text**. Evaluate native Windows OCR and optional appropriate OCR engines using English + representative multilingual datasets; measure screenshot-size images, scanned PDFs, mixed layouts, CPU/GPU/NPU capability only where actually available. OCR in bounded background queue, prioritize recent screenshot entries appropriately, hash/dedup, backpressure, thumbnails and indexing; search from inverted index without re-running OCR per keystroke. Make results return file name, snippet and page/image region where available.

**Query coordination**: instant tier-0 app/filename results, later full-text/OCR/semantic enrichments, stable selection, rate limiting of expensive work, late result rejection, ranking quality fixtures; avoid visually reordered rows under rapid keyboard navigation. Selection state must survive asynchronous completions if logically possible. Use task priorities so indexing/embeddings/agents cannot starve hotkey/search/render paths.

**Optional semantic**: local embeddings and vector index only when explicitly enabled; measure disk/RAM/CPU overhead, isolate models from instant search; avoid network egress by default.

**Background**: bounded worker pools, I/O concurrency limits, battery saver awareness, checkpoints, WAL durability, crash recovery, throttling under game/fullscreen scenarios, no index rebuild on every restart unless necessary. Present index readiness per source, not fake global percentages.

### Benchmark goals (stretch objectives, not facts)

- Warm filename lookup @ 100k files: target p95 <= 5 ms (measure exact query/matching mode).
- Warm filename lookup @ 1M files: target p95 <= 15 ms.
- Warm indexed text and indexed OCR retrieval: target p95 <= 30 ms on defined corpora.
- Warm launcher visible after global hotkey: target p95 <= 50 ms on test machine.
- Input-to-first-visible-result on warm cache: target p95 <= 30 ms on defined machine.
- 60 FPS on ordinary monitors, pursue 120 FPS on supported displays; measure p95/p99 frame pacing and visible animation defects.
- Near-zero CPU while truly idle, optional Python runtime not running when unused, no active busy loops.

If targets cannot be met on measured hardware, report truthful numbers, scope, bottlenecks and next steps. Run competitor comparisons using same data, OS, hardware, antivirus and power state; never invent success.

---

# 10. WINDOWS OS INTEGRATION — IMPLEMENT CAPABILITY-AWARE ADAPTERS

Implement and verify supported native OS functions with Rust/WinRT/Win32 wrappers or maintained libraries. **Web UI is not an operating-system device driver.** Use explicit typed IPC and correct thread/apartment handling. Detect unsupported devices/permissions and provide a clear fallback; do not silently simulate:

| Function | Real backend requirements | Fallback/limits |
|---|---|---|
| Global launcher hotkey | Register global hotkey reliably; conflict detection and remapping | Explain conflicts; never trap key combinations |
| Media | Windows supported media session APIs, metadata, transport controls | Player may not support shuffle/repeat/seek/artwork |
| Output device | Discover default/render audio endpoints and available switches using supported APIs | Disable unsupported switching; open Windows sound picker/settings |
| Wi-Fi | Enumerate adapters/networks and connect with supported WLAN APIs/permissions | Network setup may need OS authorization; offer system settings |
| Bluetooth | Enumerate paired/discoverable devices via supported Windows APIs, connect where exposed | Pairing dialogs/rights and device-specific behavior; honest unsupported states |
| Airplane/energy saver/hotspot/cast/night light | Identify actual supported user-level API path and permissions for installed Windows build | Show disabled/redirect; do not promise undocumented OS toggles |
| Brightness | Internal laptop displays via supported WMI/WinRT where appropriate; external via DDC/CI if supported | Per-monitor capability; graceful disable |
| Notifications | ProtonSearch local notifications and authorized available sources | Windows does not universally expose all other apps' notifications to normal desktop apps |
| Browser/history | Browser-specific adapters, safe DB snapshots/locking, opt-in paths | Browser missing/in-use/private-mode restrictions |
| Git | Local repo discovery, safe subprocess/libgit2 calls, correct timestamps/path quoting | Non-repo/status error states |
| Agent actions | Permissioned Python runtime and tool protocols | No arbitrary shell execution on untrusted text without review |

Implement API-availability probing in Settings, card-specific capability labels, and tests with both supported and unsupported hardware. Unknown must never be rendered as “On.” Avoid privileges/admin where user-mode solutions suffice.

---

# 11. OPTIONAL PYTHON AGENT RUNTIME — DO NOT REINVENT THE HARNESS

Evaluate an existing maintained Python agent runtime (e.g., Nanobot or a Pydantic AI-based provider/agent adapter); document license, Windows installation, startup/memory, provider support, tools, events, RPC, streaming, sessions, cancellation, persistence, and security. Choose based on integration evidence, not popularity. The Rust UI/engine launches it **on demand** with a local versioned protocol (stdio JSON-RPC/NDJSON or secured local IPC). No always-on agent or Python prerequisite for basic search.

Support multiple model providers through existing integrations, explicit consent for file/system access, per-tool policy, redacted logs, keychain/credential-manager storage, no secret hardcoding, user approval before destructive actions or network communication, replay-safe IDs, cancellation/stop, crash isolation, graceful restart, resource quotas, per-workspace policies. Persist task tree, stable agent ID, parent ID, state, timestamps and logs. Display *real* status and nested subagents inside V9 Agents card and full desktop app. If pause/resume isn't supported by selected agent runtime, do not fake it: provide safe task cancellation/queue control and label accurately. Agent history remains searchable.

---

# 12. FULL UI / UX QUALITY SYSTEM

Set a coherent visual system: original geometric P logo, graphite/dark navy, precise fonts/icons, consistent strokes, realistic content density and hierarchy, correct hover/focus/pressed/disabled/empty/error/busy states, intentional motion without bouncing/glowing/purple gradients/large generic card grids.

For every component and view, deliver: state list, screenshot at expected dimensions, keyboard interaction, focus restoration, semantic controls and aria labels where relevant, high contrast/reduced-motion test, and native DPI test. When rebuilding from V9, treat it as a *motion and layout specification*, not just a screenshot to approximate. No gigantic icons, clipped media controls, Wi-Fi arrows misaligned, 0px hit targets, invisible footer, hidden overflow, or accidental scroll chaining. V9 visual fixes are regression tests.

**V9 state coverage includes:** hidden island; compact Search idle; expanded Search results and zero matches; each of eight card modes; clickable dots and reordered dots; all clipboard tabs including images and code; audio output selector; Wi-Fi/Bluetooth subpanels; settings disabled hardware; deep agent list; notifications 0/1/many; rapid tab switching; reduced motion; scaling 100–200%; monitor moves; long titles/non-English filenames; WebView2 transient focus loss; slow data source; offline agent.

**Desktop settings placement concept** must also be beautiful and functional. Preserve the original V9 island itself as the ghost preview; don't replace it with a generic rectangular placeholder in final implementation.

---

# 13. TEST EVERY FEATURE AND EVERY IMPORTANT PAIR OF FEATURES

The user explicitly demands complete verification—**not a smoke test with three clicks**. Build and continuously maintain two matrices:

1. `FEATURE_TEST_MATRIX.md`: every feature, every supported OS operation, every card, every normal-launcher destination, every settings toggle, every preview format, and every clipboard type. Columns: status before, implemented behavior, test cases, unit, integration, Windows E2E, screenshots, permissions, regression state, reviewer, release gate.
2. `CROSS_FEATURE_PAIRWISE_MATRIX.md`: deliberate *pairwise interactions* between features, platform conditions and modes; generate covering-array combinations where possible, plus exhaustive test cases for security/high-risk pairs. Do not assert n² exhaustive tests of impossible/irrelevant combinations; for all meaningful pairs record a test or an explicit rationale why not applicable. Run all critical pairs at least once on actual Windows hardware.

Required pair examples:
- Normal Launcher × every source; Dynamic Island × every source (parity).
- Clipboard Image capture × Clipboard category Images × restart × re-paste; Clipboard Code × preview × long/unicode text; image pinned × retention/clear; clipboard paste × focused target window.
- Search result × preview × file type (TXT, code, MD, PDF, DOCX, image/OCR, binary/error); text preview × highlighted match × Unicode; file renamed/deleted × stale preview.
- Island card reorder × dots × scroll/swipe × default card × disabled card × restart; card ordering × multi-monitor placement.
- Island placement × primary/secondary monitor × DPI × taskbar × edge clamp × tall expanded card × disconnect/reconnect monitor.
- Settings Apply/Cancel × visual ghost preview × persistence × hotkey/show/hide; fullscreen app × topmost island; conflict hotkey × restore previous focus.
- Media × output selection × Bluetooth device disconnect; quick settings Wi-Fi toggle × network list; airplane mode × Wi-Fi/Bluetooth availability; brightness × unsupported monitor.
- Agents × launch/pause/cancel × logs/history × permission denial × runtime crash; agent action × clipboard privacy; native search × Python runtime absent/crashed.
- Indexer active × scrolling/typing/animation smoothness × high CPU; OCR indexing × battery saver; file events × index consistency; software update × DB migration.
- Browser/Git/Actions × permission failure × launcher fallback; notifications × privacy × source-specific clear.

### Mandatory Windows E2E environments

Test Win11 on actual hardware, Windows scale 100%,125%,150%,200%, single + dual monitors (mixed DPI and disconnected), 60 Hz and high refresh if available, light/dark, reduced motion, user-standard permissions, optional admin only for specific tests, battery AC/DC, Win32 apps, browser, VS Code, Paint/Snipping Tool. Treat virtualization/headless Chromium as useful but **insufficient proof** of Windows shell, image clipboard, media or device integrations. If any environment unavailable, mark gap honestly and block the corresponding fully verified claim.

### Automated suite

Rust unit and property tests, IPC contract tests, schema migration tests, query relevance golden tests, preview extraction fixtures, OCR accuracy corpus, clipboard format decoding/dedup tests, React component tests, E2E Playwright/Windows UI automation, visual screenshot comparisons, motion/frame-timing captures, installer upgrade/uninstall recovery, fault injection (crash service, locked clipboard/DB, OCR panic, network loss, agent crash, device removal, monitor hotplug). Add deterministic fixtures while ensuring real Windows end-to-end tests exercise actual OS features.

A feature is **DONE** only if the implementer and independent reviewer both verify the actual result under expected conditions, relevant pairwise tests pass, and L0 accepts its release-gate evidence. No checked box solely based on static code review.

---

# 14. EXECUTION MILESTONES / GATES (DO NOT STOP AFTER PLANNING)

**G0 — Audit and baseline.** Clone/build current product; inspect supplied V9 HTML; reproduce preview and clipboard-image failures; publish feature/status matrix, benchmark baseline, architecture map, screenshots and data backup.

**G1 — Architecture/contracts/hierarchy.** Instantiate L0/L1/L2 roles, assign file ownership and test reviewer; define Rust crates, TS types, IPC, Windows adapters, settings schema and data migration. Get ADR approval.

**G2 — Repair the two P0 bugs early.** Correct real text/document/content previews and real clipboard bitmap capture/persistence; test using Notepad, VS Code, Paint, Snipping Tool, PDF and DOCX samples. These are required before adding glamorous features.

**G3 — Rust search & retrieval.** Preserve working search while modularizing proven bottlenecks; indexing, files, FTS, OCR, previews, ranking, browser, Git, clipboard and actions fully wired. Baseline vs optimized results recorded.

**G4 — Normal Launcher production integration.** Centered Win32/Tauri window, all four visible filters, original quick destinations, working previews and keyboard actions. Verify desktop/native tests and hotkey semantics.

**G5 — V9 Dynamic Island implementation.** All eight cards from exact HTML reference, authentic transitions, proper dots, scroll/swipe, real data sources, native controls and no clipping. Run exhaustive screenshot/motion/interaction audit.

**G6 — Full desktop settings and onboarding.** Personalized home, settings, permissions; graphical monitor placement editor, draggable preview, card reordering, visibility/default card, persisted per-monitor placement, onboarding choice of primary launcher, migration compatibility.

**G7 — Optional agents and advanced integrations.** Existing Python runtime sidecar, model config, MCP permissions, agents/subagents, honest control semantics, logs/history; feature isolation when runtime absent. Native device support capability-gated.

**G8 — Cross-feature verification.** Run every feature and pairwise matrix, fix bugs, optimize CPU/memory/fps, test Windows monitor/DPI, real clipboard pixels, actual preview excerpts and graceful failure. Security audit and performance comparative report.

**G9 — Release.** Build verified Windows installer/updater, data migration/rollback, uninstall cleanup, accessibility, docs, README, changelog, recording of all eight island cards, screenshots/short videos of placement editor and both launchers, benchmark methodology and reproducible tests. Produce signed binaries if appropriate credentials are available, otherwise clearly state not signed.

No gate is complete until L0 accepts its evidence. Keep moving to the next gate autonomously when safe and environment permits. If blocked by a required secret, irreversible action or unavailable Windows hardware, complete every independent deliverable and report precise gaps, not imaginary verification.

---

# 15. FINAL DELIVERABLES REQUIRED

- Complete working source in original repository with clean commit history and documented build steps.
- Native Windows installer, launcher and settings binary, or explicit blocker if environment cannot build/test.
- Original user data migrated correctly (database, settings, clipboard including image entries, exclusions and history).
- V9 comparison screenshots/videos for all 8 cards and settings state transitions, plus real native app footage.
- Full source parity matrix with individual features **verified in both Normal and Island modes**.
- Real preview format report and before/after repair evidence; real clipboard image capture/re-paste results.
- Visual placement editor / card reorder screens, settings migration and multi-monitor tests.
- Full agent hierarchy/task history and independent reviewer evidence for every specialty workstream.
- Unit/integration/Windows E2E results, full feature matrix, risk-based pairwise matrix, security audit, benchmark reproducibility package and signed-off release gates.
- Honest list of Windows API limitations and unsupported controls, performance caveats, known defects, hardware not tested and follow-on tasks.

## FINAL EXECUTION COMMAND

**Start now at Gate 0. Read the attached V9 HTML and the existing repository first. Build your full hierarchical sub-agent team; reproduce preview and clipboard-image failures; define exact evidence-backed work breakdown; and proceed through implementation, independent reviews, Windows-native testing, integration, benchmarks, settings design and release. Do not stop with a plan or a visual mockup. The criterion of success is a fast, polished, secure, fully connected ProtonSearch that truly works—in both launcher modes—while preserving V9 Dynamic Island's visual and interaction quality.**
