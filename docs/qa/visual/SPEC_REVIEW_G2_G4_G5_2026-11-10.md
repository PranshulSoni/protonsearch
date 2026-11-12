# Visual/spec review - Dynamic Island renderer + Normal Launcher v2 (task-46)

Reviewer: L1-E (rel-recon). Date: 2026-11-10. Snapshot HEAD 93f84d1 + working tree. No code edits.
Scope: spec-level review of the landed UI against docs/design/ISLAND_V9_SPEC.md,
ISLAND_SETTINGS_SPEC.md and SCREENS.md; screenshots under docs/qa/visual/screenshots/.

## Evidence inspected

- Island: compact-1150.png (520x66), search-1150.png (550x397), clipboard-1150.png (510x391);
  audit docs/qa/visual/VISUAL_AUDIT_ISLAND.md; island tests and screenshot inventory.
- Launcher: launcher-task32-1180-results-dark.png (4 chips, grouped rows), launcher-task32-1180-preview-dark.png
  (pre-fix fallback panel, superseded by R2), task40-launcher-dark.png (Preferences primary-launcher page);
  audit docs/qa/visual/VISUAL_AUDIT_LAUNCHER.md incl. task-32 addendum C1/C2.

## Findings

1. Island geometry/motion/dots: matches the spec tables (islandCardDims 500/410/315 at stages
   520/430/335; expanded search 185 + 53*min(matches,5); 550/500/450/220 ms motion; reduced motion
   instant; 8 dots with aria-current). No spec regression found in the renderer geometry/token layer.
2. Island evidence limits: screenshots are Chromium mock; expanded cards lack 860/430 captures; no
   native Windows DPI/taskbar/fullscreen/High-Contrast/frame-pacing evidence. Spec section 8 requires
   1150/860/520/430/335 captures and real Windows verification - NOT MET yet.
3. Island card completeness vs spec: Pinned tab disabled (spec requires pinning), clipboard Images rows
   do not render the now-available thumbnailDataUrl, Agent card does not consume the now-available
   agent_tasks/agent_task_tree APIs, Media/Quick Settings/Notifications are honest disabled states
   pending task-35. These are functional gaps, not visual regressions; no fake data observed.
4. Normal Launcher: implementation follows the Lead G4 addendum C1/C2 (4 chips ALL/FILES/FOLDERS/APPS;
   quick destinations Browser Bookmarks/Browser History/Clipboard History/Agents + Git history/Agent
   History; source prefixes stay reachable; row anatomy, group order, footer and height maths match
   inspected screenshots). The binding design docs however still specify 7 chips (SCREENS 2.11) and a
   recents-only empty state (SCREENS 2.4); the task-32 addendum states the spec is "frozen until
   design-spec's Normal Launcher micro-spec lands". This is a pending design-spec sync, not an
   unrecorded code regression, but it blocks G4 sign-off until the micro-spec is updated.
5. Preview spec: SCREENS 2.6 still describes the alpha metadata/snippet-only preview; after R2 the
   engine/command/bridge provide real text/image/OCR previews with status/truncated/error. The design
   spec must be updated as part of G2/G4 documentation (task-38 acceptance).
6. Environment evidence gap: launcher screenshots are browser mock; native Windows captures at
   100/125/150/200% DPI, dark/light and 560 narrow are required before G4/G8 sign-off.

## Recommendation

- G5: PARTIAL / not signed - complete card wiring (thumbnails, agent hierarchy, pinning or formal CUT),
  task-35 adapters, native DPI + frame-pacing evidence, and the missing 860/430 captures.
- G4: CONDITIONAL - design-spec micro-spec update (4 chips + quick destinations + preview section) and
  native Windows screenshots.
- G2: preview UI (task-38) acceptance still requires fresh native screenshots showing real preview
  bodies (text/image/OCR/unsupported) in the launcher, and both launcher presentations once G5 lands.
