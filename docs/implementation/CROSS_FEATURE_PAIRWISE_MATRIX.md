# ProtonSearch v2 — Cross-Feature Pairwise Test Matrix (v10 Gate 0)

Owner: L1-E (rel-recon). Status: Gate-0 planned matrix; execution belongs to Gate 8.
Source of required pairs: master prompt section 13 ("test every feature and every important pair").
This matrix records deliberate pairwise interactions; it does not claim n^2 exhaustive testing.
Every meaningful pair gets either a test or an explicit N/A rationale. All critical pairs must run
at least once on real Windows hardware.

## 1. Method

1. Enumerate factors and levels (section 2).
2. Use a pairwise covering array (not the full cross-product) for ordinary combinations.
3. Use exhaustive enumeration for high-risk/security pairs (section 4).
4. Record an explicit N/A rationale where a pair is impossible or meaningless.
5. Each executed cell carries evidence: command/log, screenshot/video, benchmark JSON, Windows
   environment (OS build, monitor, DPI, refresh, AC/DC, build hash).
6. A pair is PASS only if both features behave as specified when combined; otherwise it is a
   defect in BUG_LEDGER with owner and gate.

## 2. Factors and levels

| Factor | Levels |
|---|---|
| Launcher mode | Normal Launcher; Dynamic Island |
| Source | Files; Folders; Apps; Content; OCR; Bookmarks; History; Git; Clipboard (text/code/link/image/file/pinned); Agents; Actions; Settings catalog |
| Preview format | TXT; code; Markdown; HTML; JSON/CSV; PDF (text); PDF (scanned); DOCX; image; binary/unsupported/encrypted |
| Clipboard category | All; Text; Images; Links; Code; Files; Pinned |
| Island card | Search; Media; Clipboard; Agents; Quick Settings; Browser; Git; Notifications |
| Display setup | single 150% DPI (available); 100/125/200% scaling (available by scaling); dual / mixed DPI / disconnected (NOT AVAILABLE on this host - blocker) |
| Taskbar/edges | top; bottom; left; right; auto-hide; work-area clamp |
| Motion | full; reduced (Windows setting + app override) |
| Agent runtime | absent; Pi available read-only; Pi write-enabled (consent) |
| Clipboard privacy | capture off (default); capture on; paused; sensitive exclusion |
| Indexer state | idle; actively indexing; scanning network/removable path |
| Data lifecycle | warm; restart; update/migration |
| Notifications | 0; 1; many |

## 3. Required pair examples (mapped from master prompt section 13)

| ID | Pair | Combinations / strategy | Test cases | Win E2E required | Owner | Gate | Status |
|---|---|---|---|---|---|---|---|
| P-01 | Normal Launcher x every source | 2 modes x 12 source families; parity assertion | P01-01..24 | yes, both modes | L1-B + L2-A | G8 | PLANNED |
| P-02 | Dynamic Island x every source | same data set as P-01; parity with Normal | P02-01..24 | yes | L1-B | G8 | PLANNED |
| P-03 | Clipboard Image capture x Images category x restart x re-paste | exhaustive: 4 lifecycle points x 3 sources (Paint, Snipping Tool, browser image) x 2 modes | P03-01..18 | yes (real pixel copy) | L2-A | G2/G8 | PLANNED (P0-2) |
| P-04 | Clipboard Code x preview x long/unicode text | 3 lengths x 3 charsets x 2 modes | P04-01..12 | yes | L2-A/L1-B | G8 | PLANNED |
| P-05 | Pinned image x retention/clear x restart | exhaustive | P05-01..08 | yes | L2-A | G8 | PLANNED |
| P-06 | Clipboard paste x focused target window | Notepad, VS Code, Paint, browser, terminal; unsupported target error | P06-01..10 | yes | L2-A | G2/G8 | PLANNED |
| P-07 | Search result x preview x file type (TXT, code, MD, PDF, DOCX, image/OCR, binary/error) | 7 file classes x 2 modes | P07-01..14 | yes | L1-A/L1-B | G2/G8 | PLANNED (P0-1) |
| P-08 | Text preview x highlighted match x Unicode | ASCII, Hindi, CJK, emoji, mixed direction | P08-01..10 | yes | L1-A | G8 | PLANNED |
| P-09 | Renamed/deleted file x stale preview | rename, move, delete, lock while selected; cancellation tokens | P09-01..08 | yes | L1-A | G8 | PLANNED |
| P-10 | Card reorder x dots x scroll/swipe x default card x disabled card x restart | covering array (order x enabled x default x motion) + exhaustive restart persistence | P10-01..24 | yes | L1-B/L2-B | G6/G8 | PLANNED |
| P-11 | Card ordering x multi-monitor placement | order x monitor strategy x anchor x DPI | P11-01..16 | **blocked: single monitor**; partial 1-monitor + scaling only | L1-B | G6/G8 | BLOCKED (env) |
| P-12 | Island placement x primary/secondary x DPI x taskbar x edge clamp x tall expanded card x disconnect/reconnect | exhaustive on available: 1 monitor x 4 scaling x 4 edges x 8 card heights; dual/disconnect blocked | P12-01..40 | partial | L1-B | G6/G8 | PARTIAL/BLOCKED |
| P-13 | Settings Apply/Cancel x ghost preview x persistence x hotkey/show/hide | apply, cancel, revert, reset, crash before save, restart | P13-01..12 | yes | L1-B/L1-E | G6/G8 | PLANNED |
| P-14 | Fullscreen app x topmost island; hotkey conflict x restore previous focus | 3 window states x 2 policies | P14-01..08 | yes | L1-B/L1-E | G4/G5/G8 | PLANNED |
| P-15 | Media x output selection x Bluetooth device disconnect | session states x output states x device loss | P15-01..12 | yes | L1-A | G7/G8 | PLANNED |
| P-16 | Quick Settings Wi-Fi toggle x network list; airplane x Wi-Fi/Bluetooth availability; brightness x unsupported monitor | capability matrix x toggle/drilldown independence | P16-01..20 | yes | L1-A/L2-A | G7/G8 | PLANNED |
| P-17 | Agents x launch/pause/cancel x logs/history x permission denial x runtime crash | 5 lifecycle paths x 2 modes | P17-01..20 | yes | L1-D | G7/G8 | PLANNED |
| P-18 | Agent action x clipboard privacy (never send clipboard to model without explicit action) | exhaustive security pair | P18-01..06 | yes + code audit | L1-D/L1-E | G7/G8 | PLANNED |
| P-19 | Native search x Python runtime absent/crashed | 3 runtime states x cold/warm launcher | P19-01..08 | yes | L1-D/L1-E | G7/G8 | PLANNED |
| P-20 | Indexer active x typing/scrolling/animation smoothness x high CPU | indexer phases x motion x CPU load | P20-01..12 | yes + perf trace | L1-C/L1-B | G8 | PLANNED |
| P-21 | OCR indexing x battery saver/AC-DC | 2 power states x 3 indexer priorities | P21-01..06 | yes | L1-C/L2-A | G8 | PLANNED |
| P-22 | File events x index consistency (rename/delete/locked/network) | exhaustive event classes | P22-01..14 | yes | L1-A/L2-A | G8 | PLANNED |
| P-23 | Software update x DB/settings migration x rollback | old->new schema x crash during migration | P23-01..10 | yes | L1-E | G9 | PLANNED |
| P-24 | Browser/Git/Actions x permission failure x launcher fallback | source error x UI state x both modes | P24-01..18 | yes | L2-A/L1-B | G8 | PLANNED |
| P-25 | Notifications x privacy x source-specific clear | 0/1/many x dismiss/clear/read x disabled source | P25-01..10 | yes | L1-A/L1-B | G7/G8 | PLANNED |
| P-26 | Preview x both launchers (parity) | every preview format x normal/island | P26-01..20 | yes | L1-B/L1-A | G8 | PLANNED |

## 4. High-risk pairs requiring exhaustive coverage

- Clipboard image capture x persistence x re-paste x privacy pause/off x restart (P03/P05) - data
  loss and privacy risk.
- Preview of HTML/office/PDF x script/macro/external-resource safety x path validation (P07) -
  code execution and data-leak risk.
- Agent write-enabled run x per-run consent x settings change x restart (P17/P18) - arbitrary code
  execution boundary (see PI_AGENT_SECURITY RR-02).
- Destructive actions x confirmation handshake x launcher mode (P17) - no one-shot shutdown.
- Update/migration x rollback x backup (P23) - irreversible data loss.
- Placement/profile persistence x multiple displays x restart (P12) - unreachable launcher risk.
- Hotkey conflict x another app x restore previous (P14) - input trapping.
- Clipboard sensitive-content exclusions x capture on/off x logs (P05/P18) - secret leakage.

## 5. N/A rationales (examples, to be extended as factors settle)

| Pair | Why N/A |
|---|---|
| Island card x second physical monitor | No second monitor on this host at Gate 0; blocked environment, not an N/A - listed as gap in V10_GATE0_AUDIT. |
| Media card x clipboard image format | Independent surfaces; no shared state except agent transcript export (covered by P18). |
| Normal Launcher filters x Island dots | Different presentations of the same query; parity is covered by P-01/P-02, not a per-filter x per-dot matrix. |
| OCR extractor throughput x OCR indexed search p95 | Time-separated by design (extraction vs retrieval); covered by separate benchmark rows, not a runtime pair. |
| Settings Apply/Cancel x non-UI benchmark harness | No interaction path. |

## 6. Mandatory Windows E2E environments

Required by master prompt section 13: Windows 11 real hardware; 100/125/150/200% scaling; single and
dual monitors (mixed DPI, disconnect); 60 Hz and high refresh; light/dark; reduced motion; standard
user; optional admin only for specific tests; AC/DC; Win32 apps; browser; VS Code; Paint/Snipping
Tool. Current availability: Windows 11 build 26300, single 2880x1800 @ 120 Hz, effective 150%
(scaling variants available in-session), 100/125/200% can be set; dual/mixed-DPI/disconnect and
AC/DC battery states and a second physical survey are GAPS (see V10_GATE0_AUDIT). Virtualized/headless
Chromium is insufficient proof for shell, clipboard, media or devices.

## 7. Execution and evidence

- Gate 8 owns full execution; G2 owns P0 pairs (P03..P09, P17/P18 security); G4/G5 own launcher/card
  pairs; G6 owns placement/card-manager pairs; G7 owns native/agent pairs.
- Evidence per executed cell: test ID, build hash, OS build, display config, DPI/refresh, command and
  log, screenshot/video for UI, benchmark JSON for perf, defect ID if failed.
- L1-E maintains the executed-results appendix; L0 accepts each gate's pair evidence. A pair without
  evidence is NOT TESTED, not PASS.
