# Planning artifact verification

Date: 20 October 2026, Asia/Kolkata.
Application baseline: `98d52ce`; portal reviewed through `f73971d`, with final
panel-status correction at `9b7391f`.

## Structural checks

- Ten HTML pages parsed with Python's HTMLParser.
- Local href/src targets and HTML fragments resolve; no duplicate IDs; one h1 per page.
- Twenty-five source entries are present.
- `node --check docs/2.0.0/concept.js` exits successfully.
- `tooling-lock.json` parses successfully.
- Planning files pass `git diff --check` against the application baseline.
  Vendored upstream skill whitespace is preserved; checking that third-party bundle
  may report upstream trailing-whitespace warnings.
- Baseline-to-planning diff contains only `docs/2.0.0/` and
  `.agents/skills/rust-skills/`; no Proton production sources/configuration changed.

## Browser checks performed

The site was served on loopback with Python's HTTP server, then checked in the
Codex in-app browser.

- Search preview opens and filters `notes` to the sample Widget ideas result.
- Escape closes the preview; focus returns to the triggering widget.
- Weather changes to compact and rich presentations; expanded sample forecast opens.
- Presentation profile hides sample agents/files/weather; Work restores them.
- Volume preview responds to keyboard ArrowRight, updating 64% to 65% in the preview.
- Clicking Start twice closes the panel; final status reads `Preview closed.`.
- Browser error log contained no JavaScript errors during the final interaction check.
- At 390 x 844 viewport, document client/scroll width both measured 375 CSS px
  (scrollbar excluded): no page-level horizontal overflow. Dock scroll is intentional.
- Architecture diagram and chapter navigation render at the narrow breakpoint.
- At 1280 x 600, sidebar uses `overflow-y: auto`, with client height 600 and content
  height 772; the written-plan link remains reachable. Temporary viewport reset afterward.

## Independent review

Three subagents supplied repository audit, Windows architecture research and product
contracts/roadmap input. A separate code-reviewer reviewed portal HTML/CSS/JavaScript,
plan consistency, security claims and skill provenance. One medium sidebar accessibility
finding was fixed and reviewed; the follow-up verdict reported no remaining actionable
findings. The final status-text change received an additional focused review request.

Pinned upstream SKILL.md, README.md and MIT LICENSE were independently compared against
the stated GitHub commit, normalizing newlines; they matched.

## Limits

This validates a planning portal and sample interactions. It does not validate Windows
AppBar/taskbar behavior, native accessibility, indexing performance, data migration,
agent containment or a production dock. No app build, unit suite, installer or imported
skill validation fixture was run. All 2.0 runtime budgets remain proposed targets.
The next implementation step is Stage 0 in implementation-plan.md.

## Glass mockup rebuild

The revised visual study replaces the overview's green illustration and is available
standalone at `glass-dock.html`. Production application files remain untouched.
Original SVG wallpaper and illustrative app artwork are documented in assets/README.md.

Verified in the in-app browser against the loopback preview:

- Overview iframe loads the study and Search filters `notes` to Widget ideas.
- Weather opens from the dock and, on narrow canvases, through More apps and widgets.
  The narrow panel stays inside its desktop canvas.
- Start's profile selector changes Presentation mode and returns keyboard focus to Start.
- Presentation Search excludes the sample private document/note records.
- Layout Cancel restores weather size, joined state and app order. Save retains the
  chosen rich weather size for the current preview.
- Sample volume responds to keyboard input (64% to 65%) without touching Windows volume.
- Panels reset scroll position when switching widgets and support Escape/retoggle close.
- Light, dark and solid material controls and Dawn/Midnight/Studio backgrounds are present.
- Rich compositions fit exact measured stage widths 820 and 920 CSS px, with both dock
  groups inside bounds. Also checked viewport widths 1280, 1440, 1920 and 2560.
- At viewport widths 320 and 375, dock/status remain inside the canvas and there is no
  page-level horizontal overflow. The compact layout preserves access through overflow.
- Temporary viewport override was reset after responsive checks.
- JavaScript syntax, local HTML asset targets and five SVG XML files pass validation.
- The browser reported one unattributed MutationObserver error on final navigation;
  the mockup source does not use MutationObserver. The interactions above succeeded.

Independent review initially identified responsive clipping, Presentation search privacy
and profile focus issues. All were fixed. The follow-up review through `52b0558` found
no remaining actionable issues. Universal group wrapping handles content-dependent fit;
joined capsules separate when the canvas is too narrow.

The visual study remains sample-only: no native taskbar control, file indexing, network
requests, real agents or OS volume changes. Native glass performance/accessibility and
the broader application acceptance matrix are not established by this mockup.
