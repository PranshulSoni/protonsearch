# G8 Verification Round 3 — HEAD acf74ce (release-native), 2026-11-10

Verifier: L1-E (rel-recon). Target HEAD: acf74ce87661cf86d25767cd4dea67279c401c32 (mode switch no
longer destroys the inactive window; delayed WebView2 LOW trim), on top of 5142a25 and e1b2992.
Verification was done on the RELEASE build/HEAD; the earlier debug/CDP about:blank island observation is
recorded as a devUrl/CDP harness artefact and is not release behaviour. No app was launched for this
docs wrap-up; the app is not part of the current documentation update.

## 1. Verdict

PARTIAL — release-native Dynamic Island render and enter/exit animation are now CONFIRMED on HEAD
acf74ce. Mode switch, clean opacity-over-text capture, notifications, Quick Settings and island search
parity remain PARTIAL/UNVERIFIED in this round and are carried as honest open items. No release sign-off
on this round.

## 2. Release-native evidence (HEAD acf74ce)

- Release binary rebuilt from HEAD acf74ce. App launched island-primary with a portable profile; the
  Primary Island window exists as "ProtonSearch Island".
- Hotkey show works natively: the island window became visible and focused
  (visibleMs=15.955, focusedMs=26.352; island rect left=573, width=774, height=409 in that run).
  Desktop capture release-island-shown.png shows the island pill rendered over the desktop.
- The earlier debug/CDP island about:blank (BUG-R3-01) does NOT reproduce in the release-native app:
  the island renders. This is recorded as a devUrl/CDP harness artefact, not a release defect.
- Enter/exit animation frames captured natively from the release app (desktop frame sequence, ~35 ms
  cadence):
  - exit: 12 frames, 5 distinct SHA-256 frame hashes;
  - enter: 12 frames, 4 distinct SHA-256 frame hashes.
  Frames: docs/qa/g8-verify-round3/release-exit-00..11.png and release-enter-00..11.png.
- Opacity-over-text: a controlled capture with a white text window behind the island was attempted but
  the captured frame missed the island (island visibility/focus flipped during the capture script);
  the release-island-shown.png capture shows an opaque island over the desktop, but the controlled
  white-text opacity proof is NOT yet cleanly captured. Mark PARTIAL/unverified; recapture required.
- Note on the previous round: the debug/CDP about:blank island blocked island UI checks; that artefact
  is now superseded by the release-native render above.

## 3. Suites and memory

- Green suites inherited from HEAD 5142a25 (the 5142a25 to acf74ce diff is shell/lazy-window and trim
  behaviour only): cargo test --workspace exit 0, 32 binaries, 311 passed / 0 failed / 1 pre-existing
  ignored (312 running); frontend 18 files / 242 tests passed. Suites were not re-run on acf74ce in this
  docs-only wrap.
- Memory (measured on 5142a25 release build, memory-only harness, .devdata):
  - idle launcher-only tree: 431.7 MiB WS / 203.4 MiB private across 7 processes;
  - before (4e9f589 baseline): 783.2 MiB WS / 490.3 MiB private (821,182,464 B);
  - improvement: -44.9% WS; hidden trim 238.2 -> 215.5 MiB (-22.8 MiB, 2 processes trimmed).
  Raw: docs/qa/g8-verify-round3/memory-after-5142a25.json.
  PLACEHOLDER: perf-bench acf74ce-specific memory numbers (delayed LOW trim) will land shortly and must
  replace/extend this row when available.
- Idle memory remains above the ~150 MB budget: improved but NOT MET.

## 4. Still partial / unverified after round 3

1. Clean opacity-over-text proof (controlled white text window behind the island) — recapture needed;
   release-island-shown.png only shows the island rendered over the desktop.
2. Mode switch Normal -> Dynamic Island -> Normal on the release app without exit, and hotkey with the
   alternate window absent. Earlier evidence on 5142a25 was partial (app stayed alive mid-switch, full
   two-way not automated).
3. Notifications 0/1/many layout, dismiss and clear — not executed live on release; design evidence and
   code review only (ps-sources notification store + island NotificationsCard reviewed).
4. Quick Settings live volume/brightness/Wi-Fi/Bluetooth states, drill-downs and Open Windows settings
   fallbacks — not executed live on release; IPC handlers and fallback copy code-reviewed, design
   evidence exists.
5. Island search parity with the Normal Launcher, real app icons and filters — not executed on release
   this round; shared result-rendering/search-parity work is in the commit but no new live parity capture.
6. Perf-bench acf74ce frame pacing / latency numbers: pending.
7. User-reported island card-switch lag (BUG-R3-02): not yet measured; dedicated card-switch frame-
   pacing sweep and smoothing work are open (perf-bench + design/UI). See BUG_LEDGER section 8.

## 5. Bug status change

- BUG-R3-01 (island lazy WebView about:blank): CLOSED as a debug/CDP harness artefact. Release-native
  HEAD acf74ce renders the island and the hotkey show/focus path works. See BUG_LEDGER section 7.

## 6. Remaining release blockers / open items

1. Recapture controlled opacity-over-text proof on release.
2. Native release mode-switch two-way and hotkey-with-alternate-absent confirmation.
3. Notifications and Quick Settings live checks (or explicit acceptance of design+code-review evidence).
4. Island search parity/icons/filter live evidence.
5. Perf-bench acf74ce memory/latency/frame-pacing numbers.
6. Idle memory still over budget (431.7 MiB vs ~150 MB target); input latency and native
   media/Wi-Fi/Bluetooth milestone-2, single-monitor/DPI/HC, signing/installer drill remain from prior
   rounds.

## 7. Status

Round 3: PARTIAL, release-native island render + enter/exit animation CONFIRMED on HEAD acf74ce; debug
about:blank closed as harness artefact; remaining live checks listed above. No release sign-off on this
round. App was not launched for this docs wrap-up.
