# Island reference capture manifest

**Source:** `docs/reference/v10-handoff/protonsearch_dynamic_island_cards_v9_polished.html` (V9 binding) and
protonsearch_island_settings_placement_concept.html (settings concept). **Generated:** 2026-11-09T20:32:54.399Z
**Capture:** Playwright/Chromium 1.65 headless, deviceScaleFactor 1, reference stage at the viewport width;
element screenshots of `#island` (states) / `#stage` (hidden) and `#placement-app` (settings).
**Reproduce:** `node docs/qa/island-reference/capture.mjs` (repo-local Playwright at tools/visual-qa).

## Counts

| Group | Files |
|---|---|
| V9 island states | 130 |
| Settings concept | 6 |
| **Total PNG** | **136** |

Per width: 335 = 26, 430 = 26, 520 = 26, 860 = 27, 1150 = 26, 1240 = 5.
Every PNG has a sibling `.metrics.json` with computed boxes, font sizes, dot metrics and counts.

## V9 states (identical set at each width)

- `00-hidden-stage.png`
- `01-compact-search.png`
- `02-expanded-search.png`
- `03-zero-results.png`
- `04-card-00-search.png`
- `04-card-01-media.png`
- `04-card-01b-media-output.png`
- `04-card-02-clipboard.png`
- `04-card-02-clipboard-all.png`
- `04-card-02-clipboard-text.png`
- `04-card-02-clipboard-images.png`
- `04-card-02-clipboard-links.png`
- `04-card-02-clipboard-code.png`
- `04-card-02-clipboard-files.png`
- `04-card-02-clipboard-pinned.png`
- `04-card-03-agent.png`
- `04-card-03b-agent-hierarchy.png`
- `04-card-04-controls.png`
- `04-card-04b-controls-wifi.png`
- `04-card-04c-controls-bluetooth.png`
- `04-card-05-browser.png`
- `04-card-06-git.png`
- `04-card-07-notifications.png`
- `04-card-07b-notifications-1.png`
- `04-card-07c-notifications-0.png`
- `05-reduced-motion-compact.png`

State meaning: 00 hidden stage; 01 compact search; 02 expanded search (3 sample results); 03 zero results;
04 cards (search, media, clipboard All/Text/Images/Links/Code/Files/Pinned, agent, agent hierarchy with two
spawned subagents, controls, controls Wi-Fi drilldown, controls Bluetooth drilldown, browser, git, notifications
many/1/0, media output drilldown); 05 reduced motion compact.

## Settings states

- settings/01-placement-default.png (1240 px)
- settings/02-cards-order.png (1240 px)
- settings/03-behavior.png (1240 px)
- settings/04-placement-bottom-right.png (1240 px)
- settings/05-placement-external.png (1240 px)
- settings/06-placement-860.png (860 px)

## Card dimensions measured from the live prototype (actual captured boxes)

| Card | 1150 (nominal) | 520 | 430 | 335 |
|---|---|---|---|---|
| 00-search | 520×66 | 500×66 | 410×66 | 315×66 |
| 01-media | 490×286 | 490×286 | 410×286 | 315×286 |
| 01b-media-output | 490×342 | 490×365 | 410×365 | 315×365 |
| 02-clipboard | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-all | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-code | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-files | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-images | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-links | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-pinned | 510×391 | 500×416 | 410×416 | 315×446 |
| 02-clipboard-text | 510×391 | 500×416 | 410×416 | 315×446 |
| 03-agent | 520×411 | 500×445 | 410×445 | 315×445 |
| 03b-agent-hierarchy | 520×411 | 500×445 | 410×445 | 315×445 |
| 04-controls | 515×376 | 500×400 | 410×400 | 315×483 |
| 04b-controls-wifi | 515×366 | 500×418 | 410×418 | 315×418 |
| 04c-controls-bluetooth | 515×366 | 500×418 | 410×418 | 315×418 |
| 05-browser | 500×361 | 500×408 | 410×408 | 315×408 |
| 06-git | 505×330 | 500×355 | 410×355 | 315×355 |
| 07-notifications | 500×345 | 500×385 | 410×385 | 315×385 |
| 07b-notifications-1 | 500×345 | 500×385 | 410×385 | 315×385 |
| 07c-notifications-0 | 500×345 | 500×385 | 410×385 | 315×385 |

The nominal formulas and breakpoint rules are in `docs/design/ISLAND_V9_SPEC.md` §2; measured values here are
the authoritative screenshot evidence.
