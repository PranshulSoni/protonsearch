# Implementation and acceptance status

This is a working evidence ledger, not a release certification.

## Preview dock and widget manager - 2026-11-07

HTML preview only (`preview-tahoe-glass.html`); the React/native port was not
started. The dock is an 82px graphite rail with nine default widgets, inline SVG
icons, and Pill/Tile/Card variants. All 14 widgets appear in the manager, with
preview-first installed/store grids, per-widget settings, per-profile in-memory
state, and a full-card detail preview. Optional widgets install into the dock.

Chrome headless checks at 1600x1000 found 14 installed cards, five store cards,
nine dock widgets, and no page errors. The launcher still has five scope pills
and twelve default result rows. Every settings field was exercised and changed
its detail preview; `node --check` and `git diff --check` pass. Evidence:
`dock-final.png`, `widgets-manager.png`, `widget-spotify-card.png`,
`widget-system-card.png`, `widgets-store.png`, `widgets-popover.png`,
`widgets-dock-1280.png`.

Open gates: all provider values in the preview are marked SAMPLE; live native
providers, real account usage, settings persistence across reloads, packaged-app
rendering, and physical 125%/DPI visual checks remain unverified. The preview
does not claim live metrics.

## Launch recovery — 2026-11-06

The project-tree debug executable fails WebView2 initialization. Isolated tests
showed the identical binary starts from Documents or TEMP, including when its
working directory remains the project. Root cause beyond executable location is
unconfirmed; fresh profiles alone do not fix the project-tree executable.

A copy was placed at `C:\Users\Pranshul\Documents\ProtonDock\protondock.exe`.
Computer Use launched this copy without an environment override and observed the
dock and real running-application icons. Existing ProtonSearch data remains in
its normal directory. This is a working launch location, not a source-level fix;
the copy must be refreshed after rebuilding. Window switching, packaged installer,
and the underlying location-dependent WebView2 failure remain unverified/open.

## Running applications widget

The existing Apps widget now displays up to 12 running windows with native Windows
icons and server-issued switch IDs. Native code revalidates window identity before
activation; unavailable icons have a neutral fallback. Sizes, scrolling, keyboard
navigation and stale/error states are covered by frontend tests. Seven native icon
and identity tests passed; the embedded debug build passed. Scoped security review
found no remaining actionable issue. User layout and search result IDs are retained.

Computer Use could not open the final build: the launch timed out twice. Direct
diagnostics showed WebView2 startup failure HRESULT 0x800700AA; an isolated test
WebView profile also failed with HRESULT 0x8000FFFF. Therefore actual widget icon
rendering, pointer/keyboard activation and native visual comparisons remain open.
No existing WebView profile was deleted and no other application was terminated.

## Widget theme refinement

Dock widget surfaces, typography, icon treatments, controls and focus states now
share existing Light/Dark/Glass tokens while retaining Lines mode and real data.
Frontend production build and validation of 29 widget IDs passed. The canonical
reference capture was inspected; current rendered browser/native comparisons,
all widget sizes/states and per-theme contrast remain unverified. This is a shared
styling pass, not a completed redesign of every widget.

## Dock motion refinement

Widget reorder and addition use short position/entrance animations; the size menu
and widget gallery use restrained entrances and control feedback. Motion respects
reduced-motion preferences. Frontend build and widget validation passed; focused
dock tests cover reorder with and without reduced motion. Native visual timing,
drag interruption, settings-window reopening and screenshot comparison remain
unverified. No new widget, provider, or persisted layout format was introduced.

## Scheduled continuation — 2026-11-05, 14:25 IST

Completed integration: local one-off ICS calendar now has native settings-only
picker/unlink commands, profile-bound opaque grants, bounded no-reparse local
reads, supported-subset parsing and a settings-aware capsule. Native review
corrected malformed-grant preservation, duplicate event fields, post-read profile
validation, DOS path ambiguity, ancestor rename protection and old-token
preservation when selection results are discarded or layout persistence fails.
Final reviewed calendar gates passed: Cargo check, strict scoped Clippy, formatting,
33 native unit tests and 10 integration tests. Lead combined frontend verification
passed 142 tests across 12 files and 29 widget IDs; final embedded build passed.

Computer Use opened the actual settings calendar picker, observed the .ics filter,
cancelled with Escape, then confirmed the Choose button re-enabled and calendar
remained unlinked. No user file was selected, no calendar grant created, and no
location or playback settings changed. Live calendar event binding, profile source
relaunch and actual picker success remain unverified. Native right-click clock
size menu also opened with compact/standard/wide choices and Escape dismissed it
without changing size. Dark theme and non-editing dock were restored.

Saved snippet/quicklink dispatch completed: exact snippet suffix copies through
existing clipboard helper and keeps search open; parsed HTTP(S)-only quicklinks
use plain shell opening on a COM-initialized worker. Cached IDs remain required,
with no new raw action IPC. Two regressions passed in the final native suite;
independent source review found no issue. Actual clipboard/browser action execution
was not performed to avoid altering user state.

Account budget had reset (9% used initially). No active implementation agents
were found; started separate calendar implementation, Rust/security review and
search-parity audit owners. Preserved the existing checkout and user data.

Bundled Computer Use launched the latest embedded binary, opened search from
the dock and entered `1+1` through the observed editable field. Completed native
panel displayed `30 RESULTS`; accessibility exposed exactly 30 options and the
calculator remained first. No action was executed or copied. Native test host
then stopped to release the artifact for calendar fixture compilation.

Saved snippets and HTTP(S) quicklinks are being restored through existing cached
result IDs; exact snippet payload and plain browser launching preserve legacy
behavior. No new raw command IPC or duplicate indexer introduced. Calendar native
drafts are undergoing fixture compilation and trust-boundary review before UI
enablement. Do not interpret in-progress source as completed acceptance.

## Latest resumed integration — 2026-11-05

Lead combined verification: 130 frontend tests across 11 files passed, and 29
widget IDs validated. Embedded debug build rerun with the reviewed search result
cap; it preserves authoritative order and bounds both published rows and action
IDs. Native owner recorded 21 host tests passing; weather author recorded four
Rust tests and 51 focused frontend checks passing. Weather source review fixed
profile-specific consent/editor reuse and persisted-coordinate response races.
No location was configured or fetched during these checks.

Computer Use on the preceding reviewed build demonstrated: native dock opening;
real saved-clipboard count with previews hidden; opaque search at 638 by 500;
`notepad` query returned 17 real engine results; Down/Up visibly moved selection;
Enter activated Notepad and dismissed search; Alt+Space reopened the same search
HWND. The calculator query briefly shrank to 174px loading height and then grew
back to 500px while origin stayed at (881,289). It exposed 921 rows despite limit
30, prompting the subsequently reviewed adapter cap. Final capped binary still
needs a runtime result-count check. No clipboard-copy action was executed.

One browser F11 fullscreen cycle hid the dock after the bounded sampling delay;
visible inventory confirmed absence. F11 exit restored the same visible dock
geometry. Browser returned to normal mode and its newly opened test window was
closed. Notepad was not edited or closed because it restored existing user tabs.
These checks do not certify all applications, DPI values, monitors or sleep paths.

Calendar source work remains disabled/unregistered pending review of no-reparse
local-file access. A reviewer identified possible remote traversal before final
path validation; do not expose calendar IPC or represent it as connected until
the guard is reviewed and tested. Existing calendar remains explicitly unlinked.
See the latest usage-resume handoff before continuing after the 14:25 IST schedule.

## Resumed Computer Use checks — 2026-11-05

User resumed work and requested deleting the old schedule. Old 06:38 continuation
deleted; replacement `continue-protondock-after-next-reset` is active once at
14:25 IST, after the newly observed 14:23:48 account reset. Initial usage was 0%.

Bundled Computer Use launched the latest embedded binary. Native dock screenshot
shows the reduced 132px host and no previous large outer rectangle. This does not
certify transparent-margin click-through or physical 3K scaling. Accessibility-
indexed clicks sometimes had stale bounds after movement (one Customize action
reported an out-of-window negative Y); screenshot-coordinate clicking successfully
enabled Customize and opened the separate native settings window.

Native settings Light, Liquid Glass and Dark selections were clicked and visibly
updated settings. Light also visibly propagated to the dock in a separate capture.
Original Dark was restored. No profiles were reset, no widgets removed, and no
playback/hardware state changed. Search appeared in native window inventory after
a dock action but was absent before the next capture; native owner is investigating
transient focus-loss handling. Async inventory timing and stale accessibility
bounds limit attribution. Repeated stable opening is not yet verified.

Substantive Rust source review completed with scoped check, strict Clippy,
formatting and 12 tests passing. No high/critical source issue found. Three medium
findings: fallback clipping reset errors were ignored, adapter index reuse could
misattribute network rates, and unexpected media errors became empty. Owners are
correcting these with regression checks; new lifecycle/provider work requires
fresh review and embedded build before acceptance.

Later screenshot-coordinate search activation captured the corrected opaque
638px by 500px results surface with no previous blank lower region. This improves
the geometry evidence but does not establish repeated activation reliability.
Keyboard focus was reported as the root document even after input clicks/Tab;
an attempted direct editable-value operation then failed because the window was
no longer usable. Native owner identified missing hosted-WebView focus in the
installed Tauri implementation and is correcting that separately from transient
focus-loss handling. No real search query was entered or executed in these checks.
Original Dark and non-editing dock view were restored before stopping the old
test binary for rebuild.

A five-second process-tree resource sample observed 10 processes, 523.4 MB summed
private bytes and 78.89% of one CPU core. Indexing activity was not established,
so this is not an idle baseline or a performance acceptance result. A failed
earlier sample returned invalid zero values and was discarded.

## Latest integration and usage handoff — 2026-11-05

Implemented real quick controls for fixed Windows Sound/Network/Bluetooth/
Notifications settings routes, with no arbitrary command/URI arguments. Reviewed
launch-worker correction adds SEE_MASK_NOASYNC. Optional persisted Light/Dark/
Glass themes now apply to dock/settings/search with read-only search settings
permission. Theme-load errors use a separate nonblocking warning rather than
hiding valid engine results.

Lead combined frontend check: 112 tests across nine files and 29 widget IDs
passed. Final theme-error correction subsequently passed 14 focused search tests.
Embedded debug build rerun after that correction; no desktop testing followed
the Computer Use physical-Escape stop. Native owner recorded 12 host tests and
strict Clippy passing. Geometry, controls and themes received focused reviews;
new provider/FFI code still needs a completed substantive Rust/security review.
Existing dependency warnings and whole-repository formatting limitations remain.

Account use reached 96% on the latest check, crossing the requested stop boundary
between checks. New work stopped; all subagents idle/completed. One-time thread
continuation scheduled for 06:38 IST on 5 November 2026, after observed reset at
06:36:42. Read `usage-resume-handoff-2026-11-05.md` first and recheck actual usage.
The full rebuild and corrected native visual/runtime acceptance remain incomplete.

## Window geometry and real local providers — 2026-11-05

Reproduced dock-click search opening before the geometry changes: the native
panel occupied a fixed 654 by 640 pixels with unused lower space and desktop text
visible through it. One opening was observed; an intermittent crash/glitch was
not reproduced. Source confirmed near-work-area-width dock hosting and fixed
search geometry. Current fixes use measured dock content and rounded Win32 hit
regions, disable native shadows, and request a 638px search width with adaptive
174–500px height. Search top position stays fixed while results change. Dock fit
failures show a compact fallback and unsuccessful frontend measurements can retry.

Real local widget implementations now include Windows media metadata/capability-
gated transport, physical adapter byte rates, persisted focus timer controls,
system-drive storage and battery. All 29 widget IDs are mapped in
`widget-provider-map-2026-11-05.md`; account-backed placeholders remain explicitly
unconnected. Read-only native probes returned live media and network snapshots;
first network sample was correctly loading. No playback action was injected.

Lead verification: 98 frontend tests across eight files passed and 29 IDs passed
widget validation. Embedded debug build passed after the standard media stale-
label selector correction. Native owner recorded 11 host tests, including real
GDI region checks at 1x and 1.5x and read-only provider probes. Geometry review
found and corrected hidden-on-fit-failure and query-driven input movement;
provider review corrected a hidden standard-size stale label. Detailed evidence
is in `geometry-2026-11-05.md` and `native-window-geometry-2026-11-05.md`.

The final corrected binary has not been visually accepted: Computer Use reported
a physical-Escape stop when launching it, so further desktop input stopped.
Corrected native click-through, repeated search opens, displayed provider layouts,
interactive media controls, physical 3K/DPI geometry and settings relaunch remain
unverified. No taskbar replacement, automatic crash watchdog or Windows-key fix
is claimed. Later controls/theme work must add its own verification evidence.

## Parallel implementation batch — 2026-11-05

Three independent owners implemented widget sizing, search interactions and native
recovery. Existing layouts, providers and authoritative Rust search remain in use.
Detailed evidence: `widget-sizing-2026-11-05.md`,
`2026-11-05-search-interaction-evidence.md`, and `native-recovery-2026-11-05.md`.

Combined verification by lead: 87 frontend tests across six files passed; widget
validation accepted 29 IDs; `npm run tauri -- build --debug --no-bundle` passed,
including the frontend production build. Native owner recorded 39 library tests,
four host tests and four isolated emergency-restoration CLI probes passing.
Search and native recovery received focused independent source review. Rust
specialist review stopped at existing repository-wide formatting differences in
unrelated files; this is not a clean whole-repository formatting certification.
The native build reports five existing library warnings.
Widget review found compact views hid connection-state text; corrected with visible
Offline/Unlinked markers and three regression checks. Targeted re-review passed.
The widget owner rebuilt the frontend after that correction. The embedded native
binary and observed native dock precede this final marker correction; a further
native rebuild and visual check remain required.

Lead launched the embedded debug application using bundled Windows Computer Use.
The dock rendered with clock, actual CPU/RAM values and explicit disconnected
weather/media/calendar states. No taskbar hiding or production startup changes
were performed. Computer Use was then stopped by the physical Escape key; no
further desktop input was issued. Native search actions, size-menu interaction,
relaunch persistence and emergency tray restoration were not verified this run.
The native capture is not evidence of the requested physical 3K monitor baseline.

Taskbar replacement and an automatic UI-independent crash watchdog remain absent.
The Windows-key defect has not been reproduced or certified fixed. Native floating
widget panels, the full theme matrix, legacy search feature parity, performance
budgets and shell/display lifecycle checks remain open. Do not retire the legacy
launcher or describe this batch as completion of the full rebuild.

## Appearance update — 2026-11-05

Implemented in the existing React dock: accessible Capsules/Lines selection in the
widget gallery, saved independently for Daily/Founder in an optional `appearances`
map. Legacy version-1 layouts remain valid and default to Capsules. Widget order,
sizes, settings and unknown IDs remain preserved. Lines removes tile fills/borders/
shadows/radii, including decorative inner surfaces; separators are drawn inside
the ordered visible widget slots, with none before the first. Calendar text keeps
theme contrast. Windows/Search logo proportions follow the supplied fixed HTML.

Verified this run: 73 frontend tests pass; production frontend build passes;
widget validator accepts 29 IDs. Source review found and corrected the calendar
contrast issue. Chromium at 3072 × 1920, DPR 1: actual gallery buttons selected
Lines, selected state updated, and reload restored Lines. Computed styles on all
nine Daily tiles showed transparent backgrounds/borders and zero radius. Divider
content was absent on the first slot and present on the remaining eight.

Browser captures in `appearance/`: `react-capsules-3072.png`,
`react-lines-3072.png`, `react-lines-1440.png`, `reference-capsules-3072.png`,
and `reference-lines-3072.png`. Reference captures use the actual supplied HTML
in Dark theme, same 3072 × 1920 viewport and DPR 1. Visual inspection confirms
the appearance distinction. Existing differences remain: React fills nearly the
whole screen width; reference centers its content; React has disconnected/native
provider presentations rather than the reference's sample metrics and app icons.
These captures establish appearance behavior, not full visual parity.

Still unverified for this update: native Windows 11 rendering, settings-window
synchronization and native relaunch persistence; native search/Start actions;
Light/Glass theme matrix (the React dock currently lacks theme selection); and
monitor/DPI changes. 3072 × 1920 is an assumed 3K browser viewport, not a measured
physical monitor resolution. No native Rust or shell code changed; native tests
were not rerun. Prior native observations below are historical.

## Phase 0

Implemented: package extraction, document/source audit, ownership and IPC boundaries.
Verified: canonical HTML opens in Chromium at 1440 × 900; Daily, Founder, search and
customize screenshots captured under `reference/`.

## Observed Windows execution

2026-11-04, local Windows/WebView2, embedded debug executable (no development server).
Actual native screenshots are in `native/`; `daily-comparison.png` pairs the canonical
HTML and native dock at 1624px capture width. Desktop backdrops differ; this is not a
pixel-difference certification. Original baseline viewport was 1440 × 900, DPR 1.

* Dock tile opened the separate native Spotlight window.
* Alt+Space opened the same window. Typed `notepad`; 14 actual engine results returned,
  with the engine reporting 50ms for this query. Enter opened Windows Notepad (new
  process observed at 19:37:13 local time) and search disappeared.
* Daily and Founder switched in the actual native dock. Canonical default order is
  restored. Founder screenshot captured before adding the proof widget.
* Customize enabled editing; Widgets opened the separate native gallery.
* Added automatically discovered Storage Health through gallery; dock updated across
  windows and showed actual system-drive 470 GB free / 757 GB total. Screenshot saved.
* Relaunched final embedded build: Founder selection and added Storage Health restored
  with real values. `native/restored-layout.png` records restoration. Final Spotlight
  filled the window without the previous blank bottom area; Escape hid it, verified
  by the remaining native window inventory. `native/search-final.png` records bounds.
* Actual system CPU, RAM and battery values observed updating. CPU starts unavailable
  until a second native sampling point exists.
* CSS fixes after visual inspection: standalone Spotlight opacity, full-window panel
  to remove the unused transparent bottom area, single scrolling list, sample labels
  in gallery, and selected-row scrolling.

## Automated checks

Frontend build passed. 68 tests across five files passed; widget validation accepted
29 IDs. Native owner recorded four host tests and 35 existing library tests passing.
Source review found and fixed extraction subprocess dispatch, old-data-directory
migration, stale backend result publication, unsupported action false successes,
malformed-layout overwrites and Enter bubbling. See the separate review documents.

## Acceptance still open — not a production release

* Visual parity is incomplete: generic Apps/Founder/service presentations, gallery,
  analog clock, responsive widths and installed application icons need further porting.
  The paired image explicitly exposes differences. No pixel-perfect claim.
* Existing clipboard/timeline ingestion, AI/agent/OCR panels, action menus, snippet
  expansion and other legacy UI-owned behavior are not fully migrated. The legacy
  binary and source remain intact and are still required for those workflows.
* Notes currently searches existing notes and opens native text files; no quick-note
  creation editor. Focus timer, network, media, quick controls and external providers
  remain disconnected. Weather/calendar/account setup is not implemented.
* Transparency is visually observed, but transparent-margin desktop hit testing and
  full-screen suppression are not established. No all-monitor/all-DPI claim.
* Layout add/refresh and relaunch restoration are observed; full remove/reorder/resize matrix,
  startup registration, installer execution/upgrade and cross-host shortcut behavior
  still require validation.
* The SDK has build-time discovery and shared polling, but complete migration hooks,
  permission enforcement per widget, event/manual provider lifecycles and all requested
  accessibility/performance contracts are not finished.
* One 50ms query is not a latency benchmark. Cold/warm open, contention, full WebView
  memory, idle CPU/frame budgets and original-launcher comparison remain unmeasured.

Installer build results are recorded in `docs/HANDOFF.md`. Do not retire the old
launcher, change production startup, or claim full acceptance from these checks.

## Native provider verification (2026-11-07)

Scope: `src-tauri/src/widget_data.rs`, `src-tauri/src/main.rs`,
`src-tauri/src/known_folders.rs`, `src-tauri/Cargo.toml`. No React, CSS or
`preview-tahoe-glass.html` edits.

Implemented and compiled:

* `provider_snapshot("network")` keeps `downloadBytesPerSecond`,
  `uploadBytesPerSecond` and `interfaces`, and adds `connectionType`, `wifi`,
  `ethernet`, `vpn` and `bluetooth`. Wi-Fi state and SSID come from the WLAN API;
  Bluetooth comes from `Windows.Devices.Radios`; no `netsh` process is spawned.
* `provider_snapshot("system")` keeps `cpu`, `ram` and `battery`, and adds
  `batteryCharging`, `temperatureC` (null until a real source exists) and
  `uptimeSeconds`.
* `known_folders_snapshot` is registered and returns known folders plus up to 12
  recent files. It is not wired into the React UI yet.
* `docs/widgets/NATIVE_PROVIDERS.md` records the exact shapes, statuses,
  permissions and backing Windows APIs.

Observed on this Windows 11 host:

* Network smoke: first sample `loading`, second `live`, `connectionType=vpn`,
  `wifi.state=disconnected`, `ethernet.connected=false`, `vpn.connected=true`,
  `bluetooth.available=true`, `bluetooth.enabled=false`; media `empty`.
* The active VPN was `PANGP Virtual Ethernet Adapter Secure` (GlobalProtect). The
  enabled Wi-Fi adapter was up in NDIS but not associated, so it was correctly not
  treated as the active connection.
* System: `cpu=null` on the first sample, `ram=61`, `battery=85`,
  `batteryCharging=false`, `uptimeSeconds=39976` (one run).
* Known folders: `status=live`, 6 folders, 12 recent files.

Verification commands and exact outcomes:

* `cargo fmt --manifest-path src-tauri/Cargo.toml` completed clean.
* `cargo check --manifest-path src-tauri/Cargo.toml` passed. Only pre-existing
  ProtonSearch warnings were emitted; protondock emitted none.
* `cargo test --manifest-path src-tauri/Cargo.toml --bin protondock -- --skip
  local_calendar::` -> 42 passed, 0 failed, 10 filtered.
* Full `cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast` -> bin
  target 47 passed, 5 failed; integration target 5 passed, 5 failed. All failures
  are in the unmodified `local_calendar` fixture: `std::fs::create_dir` at
  `src-tauri/src/local_calendar.rs:465` returns Windows error 5
  (`PermissionDenied`, "Access is denied") for its temp directory. The full suite
  is therefore not green on this host; this is not claimed as a native-provider
  regression or a passing run.

Not verified / still open:

* Live Wi-Fi association (SSID and signal percent) was not observed because this
  host's WLAN interface was disconnected. Decoding and bounds are unit-tested.
* Physical Ethernet and a connected Bluetooth radio were not observed.
* Only the PANGP/GlobalProtect VPN name heuristic was observed live; other VPN
  vendors are unit-tested name matches only.
* The known-folder `permission-denied` path, a missing Screenshots folder, and
  reparse-point rejection were not exercised on live data.
* No Computer Use or visual UI verification was performed for these provider
  changes; only native Rust tests were run.
* `known_folders_snapshot` has no frontend consumer yet.
