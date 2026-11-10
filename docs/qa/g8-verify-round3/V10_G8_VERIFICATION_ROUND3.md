# G8 Verification Round 3 — HEAD 5142a25 (mode-switch fix), 2026-11-10

Verifier: L1-E (rel-recon). Target: HEAD 5142a25db3ad8b708d82752c9ce0ae325ca68128 (fix: launcher mode
switch no longer exits the app; lazy create missing windows), on top of e1b2992 / 8857bc7. Tree clean
except tools/benchmark/island-perf.mjs (perf-bench work in progress; not touched). Debug and release
binaries were rebuilt from HEAD.

## 1. Verdict

PARTIAL. Suites and memory are verified; mode-switch no-exit is partially observed. The island-rendered
checks (opacity over text, enter/exit animation, notifications, Quick Settings, island search parity and
icons) are BLOCKED by a new observation: after lazy island creation the island WebView target stays
about:blank and no island surface is rendered (desktop capture corroborates no island pixels).

Do not sign off an installer on this run. Hand the island lazy-window observation to perf-bench/L0 for
native confirmation while the app is rebuilt; it may be a real lazy-window regression on 5142a25 or an
automation artefact, but it is reproducible here and blocks visual sign-off.

## 2. Suites on HEAD (green)

- cargo test --workspace: exit 0. 32 test binaries, 311 passed, 0 failed, 1 pre-existing ignored
  (312 running). Log: .agents/notes/g8r3b-cargo.log.
- Frontend: 18 files / 242 tests passed. Log: .agents/notes/g8r3b-frontend-tests.log.
- All suites ran with no ProtonSearch process running (clipboard-test contention rule).

## 3. Memory after (release build, memory-only harness, .devdata profile)

Raw: docs/qa/g8-verify-round3/memory-after-5142a25.json.

| Metric | Before (4e9f589) | After HEAD 5142a25 | Delta |
|---|---:|---:|---:|
| Idle launcher-only tree Working Set | 783.2 MiB (821,182,464 B) | 431.7 MiB | -351.5 MiB (-44.9%) |
| Idle launcher-only private bytes | 490.3 MiB | 203.4 MiB | -286.9 MiB |
| Process count idle | 9 | 7 | -2 |
| Visible to hidden+trim tree WS | not measured | 238.2 to 215.5 MiB | -22.8 MiB, 2 procs trimmed |
| Launcher activation (single run) | not measured | visible p50 raw 22.6 ms / send-to-paint 53 ms | - |

Lead independently measured 428 MiB WS / 211.6 MiB private, corroborating the improvement. Improvement
vs 783 MiB is material; idle memory is still above the ~150 MB budget, so the budget row remains NOT MET
(improved, not closed).

## 4. Mode-switch fix (partial)

Attempted a live update_settings switch Normal to Dynamic Island on 5142a25 while connected over CDP.
The app process stayed alive (no exit) and the CDP target set was recreated (old launcher page closed, a
new target appeared), consistent with the intended fix. NOT conclusively verified: the invoking page was
destroyed mid-call, so the full Normal-Island-Normal sequence, persistence, and hotkey-with-alternate-
window-absent path could not be automated. Settings file reverted to normal after the interrupted call.
Mark PARTIAL PASS (no app exit observed); needs a native/manual two-way check.

## 5. Island UI checks — BLOCKED in this run

Observed: with the app running, invoking show_island and lazy island creation produced a CDP target
titled/URL about:blank; repeated waits (5 s) did not navigate. The /json/list output was
[about:blank, http://localhost:5173/] with no island app page. A desktop capture with a text window behind
the island region (opacity-island-over-text.png) shows no island surface, consistent with a blank/absent
island window rather than an opacity bug.

Because the island page never rendered the app UI, these minimum checks could NOT be executed:
1. Island opaque over a real text window (no bleed) - not verified; captured desktop shows no island.
2. Enter/exit animation frames distinct - not verified.
3. Notifications 0/1/many layout/dismiss/clear - not verified.
4. Quick Settings real volume/brightness/Wi-Fi/Bluetooth toggles/drilldowns/fallbacks - not verified live
   (IPC handlers exist and were code-reviewed earlier; no live UI evidence this round).
5. Island search parity + real app icons + filters - not verified this round.
6. Hotkey with alternate window absent - not verified (lazy island window did not render).

Evidence files: opacity-island-over-text.png; docs/qa/g8-verify-round3/ (raw scripts launch.mjs,
e2e-round3.mjs, show3.mjs kept for reproduction). App was killed after the run and left stopped.

## 6. Remaining blockers

1. BLOCKER TO TRIAGE: island WebView about:blank after lazy window creation on 5142a25 (debug/CDP
   environment); confirm natively, fix if real, then re-run island opacity/animation/notifications/QS/
   search checks.
2. Mode-switch full two-way + hotkey lazy-window path still needs a native confirmation.
3. Idle memory improved but still over budget (431.7 MiB vs ~150 MB target).
4. Perf-bench task-57 report is still holding; formal frame-pacing/latency numbers pending their run.
5. Native media/Wi-Fi/Bluetooth milestone-2 and single-monitor/DPI/HC gaps remain from prior rounds; no
   live Pi credentials.
6. Installer rebuild/signing/upgrade-uninstall drill still pending.

## 7. Status

Round 3: PARTIAL. Green suites and material memory improvement verified on HEAD 5142a25; mode-switch
no-exit partially observed; island UI checks blocked by about:blank lazy island webview. No release
sign-off on this round; app left stopped.
