# Interactive Dock Widgets Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Make the HTML study demonstrate a polished, interactive Proton widget dock.

**Architecture:** Keep the existing single-shell layout editor. Introduce a widget
definition library and separate local interaction runtime, coordinated through DOM
events, with a shared detail panel. Retain existing layout validation and persistence.

**Tech Stack:** Local HTML, CSS, JavaScript and original inline SVG; no new dependencies.

---

### Task 1: Coordinated widget library and runtime

Files: create `docs/2.0.0/widget-library.js` and `widget-runtime.js`; modify
`docs/2.0.0/glass-dock.html` and `concept.js`.

- [x] Define original SVG icons and semantic widget bodies. Populate existing profile
  sections before layout initialization; keep `data-widget-id/name/default-w/min-w/max-w`
  and `.widget-drag`/`.widget-resize` as the layout contract.
- [x] Load deferred scripts in this order:
  `<script src="widget-library.js" defer></script>` → `concept.js` → `widget-runtime.js`.
- [x] Create reusable detail dialog, sample fixtures, event delegation, independent
  playback/focus/agent timers. Use `textContent` for task/query/metric values.
- [x] Implement all interactions specified in the approved design. Add profile/edit
  lifecycle notifications; runtime closes panels on those events. Replace legacy timer
  code rather than running duplicate intervals against newly composed widgets.
- [x] Render gallery previews from the new widget library and preserve existing layout
  storage, custom metric creation, focus, scroll and keyboard editor behavior.
- [x] Run `node --check` on all three scripts, inspect local asset references, commit/push.

### Task 2: Visual polish

File: create `docs/2.0.0/widget-polish.css` (loaded after `unified-dock.css`).

- [x] Give `.widget-face` a consistent height, spacing, text hierarchy and SVG scale.
  Use container queries to hide `.widget-secondary` at narrow widths; never hide the
  only accessible control or temperature. Keep one continuous outer shell.
- [x] Style search, apps, weather, music, controls, focus, clock, agent and metric views.
  Use CSS art for music cover, original SVG weather glyphs and clean app monograms.
- [x] Style detail dialog, gallery previews, responsive limits, editing handles and
  focus states. Include `@media(prefers-reduced-motion:reduce)` overrides.
- [x] Confirm layout at 1440, 375 and 320px, commit/push.

### Task 3: Verify, review and document

Files: update `docs/2.0.0/verification.md` and `glass-dock-design.md`.

- [x] Use the loopback preview to exercise sample search, app activation, weather tabs
  and units, music play/next/seek, volume/mute/output/Wi-Fi, focus start/reset and agent
  run/cancel/result. Confirm keyboard, same-trigger/Escape/outside closing and focus.
- [x] Check profile switching, independent layouts, custom metrics and editing. Inspect
  narrow detail-panel bounds and dock reachability. Reset temporary viewport overrides.
- [x] Run `node --check docs/2.0.0/concept.js` and the same for the two new JS files.
  Parse all HTML; reject duplicate IDs or broken local href/src paths.
- [x] Request independent code review, fix actionable findings, repeat relevant checks.
- [x] Record exact performed checks and fixture limitations, save screenshot, commit/push,
  confirm `git status --short` is empty and HEAD equals origin/2.0.0.

Execution uses the existing user-selected `2.0.0` checkout and the previously authorized
subagent workflow. No extra worktree or execution-choice prompt is needed for this
continuing docs-only rebuild. Repository rules require killing the app before edits and
committing/pushing every edit batch.
