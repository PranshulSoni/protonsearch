# ProtonSearch v2 — G8 Verification Round 2 (RC 4e9f589, 2026-11-10)

Owner: L1-E (rel-recon). Task: task-53. RC source commit: 4e9f589. Artifact commit: a85e7be.
Reviewer of record: L1-E; acceptance authority: L0.

## 1. Verdict and final recommendation

**G8 ROUND 2: PASS for local/internal-alpha deployment; NOT SIGNED OFF for public release.**

Final L0 recommendation: **DEPLOYMENT-READY LOCALLY as an UNSIGNED INTERNAL ALPHA** (can be
installed/run on this machine for internal evaluation, with the caveats in section 7).
**NOT signed off for public distribution** - unsigned artifacts, installer upgrade/uninstall drill
not run, single-monitor only (no mixed-DPI/High-Contrast evidence), idle memory and debug input
latency over budget, native media/Wi-Fi/Bluetooth adapters deferred to post-alpha milestone 2, no
live Pi provider credentials, and one G2 preview condition (matched-term highlight) remains open.

## 2. RC artifacts (UNSIGNED)

Build metadata: dist-installer/BUILD_INFO.txt, built_utc=2026-11-09T21:47:50Z, git_rev=4e9f589,
rustc/cargo 1.99.0, node v26.7.0, bundle_target=nsis, installer_mode=currentUser, SIGNED=false.

| Artifact | Size | SHA-256 |
|---|---:|---|
| dist-installer/ProtonSearch.exe | 20,750,336 B | 3010FBEED2AE5BB9EA0FC91960F0D8325BBF5BC589D9F3596B631BEFB6FE15EF |
| dist-installer/ProtonSearch_2.0.0-alpha.1_x64-setup.exe | 5,567,529 B | F5D15C2D89CA2857C6DDB71B9289645A693B37C958AAA462E7B662CEBD6FBF8D |
| dist-installer/BUILD_INFO.txt | 518 B | 61F871B94B844547F05038E43B463BC9CDB49F2BF6DE180EE124B5071126EB21 |
| dist-installer/SHA256SUMS.txt | 192 B | 3726F703BB755B07431B6196BC14E52A17FB57B15254BA804BC3E3580B71CE06 |

Hashes independently re-computed during this review and match SHA256SUMS.txt. Release app was
launched against the portable profile and created its v2 data dir successfully; the canonical
release-app Home render is docs/qa/visual/live-shots/release-app-final.png. For CDP-driven UI checks a
debug build of the same tree (cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml, exit 0)
was used; release builds intentionally disable remote debugging (V2-05).

## 3. Suites (green tree, app stopped for clipboard tests)

| Suite | Result | Evidence |
|---|---|---|
| cargo test --workspace | PASS - exit 0; 32 test binaries, 306 passed, 0 failed, 1 pre-existing ignored (sum running 307) | .agents/notes/g8r2-cargo.log |
| Frontend unit tests | PASS - 18 files / 241 tests passed | .agents/notes/g8r2-frontend-tests.log |
| Frontend build | PASS - vite build exit 0 | .agents/notes/g8r2-frontend-build.log |
| Clipboard contention | all Rust tests ran with no ProtonSearch process; app started only after suites | process log |

## 4. Live P0-1 previews (14 formats, highlight/Unicode/stale)

Method: debug CDP build + portable profile docs/qa/g8-verify-round2/appdata (fresh full scan of
docs/qa/preview-audit/fixtures incl added JSON/CSV/Unicode fixtures; engine ready, 14 filename
entries, content FTS ready). Evidence: p0-1-round2-evidence.json, p0-1-token-evidence.json and
screenshots p0-1-{docx,image,json,highlight,unicode}.png.

| Fixture | Result |
|---|---|
| notes.txt | PASS ok/txt, text 191 chars |
| sample.rs | PASS ok/rs, text 232 chars |
| readme.md | PASS ok/md, text 202 chars |
| sample.json | PASS ok/json, text 70 chars (JSON preview + UI screenshot) |
| sample.csv | PASS ok/csv, text 38 chars |
| report.pdf | PASS ok/pdf, text 83 chars |
| scanned.pdf | PASS honest unsupported: "no text layer; OCR preview is not available yet" |
| report.docx | PASS ok/docx, text 132 chars; UI screenshot shows real DOCX preview |
| sheet.xlsx | PASS ok/xlsx, text 55 chars |
| diagram.png | PASS ok/png, imageDataUrl 26,746 chars, OCR 45 chars; UI screenshot with image |
| blob.bin | PASS explicit unsupported message |
| unknown.xyz | PASS explicit unsupported message |
| large.txt | PASS ok/txt, truncated=true at 1,499 chars |
| unicode-note.txt | PASS ok/txt, text "नमस्ते KOALA-UNICODE-2026 你好 😀" intact (Unicode/emoji preserved) |
| stale file (diagram.png hidden after selection) | PASS honest stale handling: status=error, explicit "Image could not be decoded: ... cannot find the file specified"; no blank panel |
| matched-term highlight | **NOT DELIVERED (condition open)** - content FTS token searches return the right content_match rows and preview text contains the token (DOCX/JSON/CSV/Unicode), but PreviewPayload.highlightRanges is empty and the live UI rendered 0 mark elements. The text/snippet path works; the highlight-range contract is not implemented in preview. |

## 5. Live P0-2 clipboard images

With the app running and clipboard history enabled, a separate STA PowerShell process copied a real
System.Windows.Forms.Clipboard.SetImage 320x200 bitmap.

| Check | Result |
|---|---|
| Live capture | PASS - clipboard count 2; row clip:2 "Image 320 x 200", full PNG in profile clipboard-images dir |
| IPC contract | PASS - meta.thumbnailDataUrl 4,918 chars, imageWidth 320, imageHeight 200, mime image/png |
| Island Images tab | PASS - live screenshot p0-2-island-images.png shows Image 320 x 200 "just now" with a real 34x34 data:image/png thumbnail |
| Restart persistence | PASS - after app restart the same clip:2 row/thumbnail is still present and rendered (fresh screenshot produced by the same script) |
| UI paste-back | NOT VERIFIED this round - harness-level paste-back is verified (R3 evidence + clipboard_image_metadata_survives_restart_and_copies_back) |

## 6. Pairwise execution (round 2, testable pairs)

| Pair | Result |
|---|---|
| P-03 clipboard image x Images category x restart x re-paste | PARTIAL PASS - capture/category/thumbnail/restart pass live; UI re-paste and Paint/Snipping/browser GUI matrix remain |
| P-07 search result x preview x file type | PARTIAL PASS - 14 formats executed live; matched-term highlight fails (condition); both-launcher parity not executed |
| P-09 stale/renamed deleted file x preview | PASS - stale preview returns explicit error, no blank/crash |
| P-10 card order x dots x scroll x default x disabled x restart | PARTIAL - component tests + live card screenshots; reorder/default/disabled combination not executed |
| P-13 settings apply/cancel x persistence | PARTIAL - implemented/screenshot evidence; full apply/cancel/restart E2E not executed |
| P-26 preview x both launchers | PARTIAL - normal launcher verified live; island search card has no preview hookup |
| P-06 paste into focused target, P-17 agent lifecycle, P-18 agent x clipboard | NOT EXECUTED/BLOCKED (GUI target matrix, no Pi credentials) |
| P-11/P-12/P-14 placement/fullscreen/multi-monitor | BLOCKED - single-monitor host |
| P-23 migration/update/rollback | NOT RUN (installer upgrade/uninstall drill not executed) |

## 7. Honest blockers and known gaps (as of RC)

1. **Unsigned**: no Authenticode, no signed update manifest, no updater; artifacts must not be
   distributed publicly. Public release sign-off requires signing plus signtool /pa verification.
2. **Installer upgrade/uninstall drill not run**: currentUser NSIS installer built; install,
   upgrade-over, uninstall, data-retention and rollback have NOT been executed on a clean VM.
3. **Single monitor only**: mixed-DPI, disconnect/reconnect, dual-monitor, taskbar-edge and
   High-Contrast evidence missing; not tested at 100/125/200% beyond default 150%.
4. **Performance**: content retrieval p95 8.984 ms MET; release activation p95 14.289 ms MET;
   selection 6 ms MET; **input-to-visible p95 38.2 ms NOT MET** (target 30 ms, debug build);
   **idle memory 786.9 MiB NOT MET** (target ~150 MB budget; 70.9 app + 716.0 WebView2 across 8 procs).
5. **Native adapters**: Media/Quick Settings(media)/Wi-Fi/Bluetooth are honest-disabled with reasons;
   native milestone 1 (volume/brightness/audio output/notifications) is the alpha scope, media/
   Wi-Fi/Bluetooth are post-alpha milestone 2. No fake toggles.
6. **Agent**: no live Pi provider credentials on this host; read-only alpha gate remains
   (allow_writes=false); Pi live inference, approvals and write path unverified end-to-end.
7. **Preview**: matched-term highlight contract not delivered (G2 condition); island search card
   preview hookup missing; both-launcher preview parity not executed.
8. **Installer packaging**: lockfiles/CI reproducibility not re-audited in this round; no public
   release/tag/push performed.
9. **Environment**: single machine, no clean VM, no signing certificate, no non-Pi provider keys.

## 8. Gate verdicts

| Gate | Verdict |
|---|---|
| G1 | CLOSED (recommended) |
| G2 | **PASS (conditional)** - P0-1/P0-2 live evidence on RC; condition: matched-term highlight + island preview hookup/both-launcher parity |
| G3 | **MET** - content p95 8.984 ms (official quiet, n=960, 13,296 docs) vs 30 ms target; OCR FTS/UI E2E still partial |
| G4 | CONDITIONAL - live launcher + 4 filters/quick destinations work; design-spec micro-spec sync + native DPI/HC evidence pending |
| G5 | PARTIAL - island renderer/cards/dots live and thumbnail/agent wiring improved; native adapters post-alpha; native DPI/frame pacing pending |
| G6 | CONDITIONAL - settings/placement/card manager implemented; apply/cancel/restart native E2E and DPI evidence pending |
| G7 | PARTIAL - read-only Pi gate closed; live provider unverified; native adapters milestone-1 partial |
| G8 | **PASS for local/internal alpha** with open conditions above; NOT signed for public release |
| G9 | **NOT SIGNED OFF** - unsigned, installer drill not run, no public release |

## 9. Final L0 recommendation

- **Deployment-ready locally: YES** - as an UNSIGNED INTERNAL ALPHA. The RC binary/installer are
  hash-verified, the app launches and renders, full Rust/frontend suites are green, P0-1 previews
  and P0-2 clipboard images work live (thumbnails + restart persistence), and the highest-value
  search/content performance targets are met. Suitable for local evaluation on this machine.
- **Public release signed off: NO** - do not distribute or claim production readiness. Blocking for
  public release: code signing + updater, installer upgrade/uninstall/rollback drill on a clean VM,
  mixed-DPI/High-Contrast/multi-monitor evidence, idle memory and input-latency budget misses,
  native media/Wi-Fi/Bluetooth milestone 2, live Pi credentials/approval verification, and the
  matched-term highlight condition.
- Next actions if a public release is desired: sign artifacts, run installer drill, execute the
  remaining G8 pair/list items on hardware, fix highlight + memory/input budgets, then L0 acceptance.

## 10. Evidence index

- docs/qa/g8-verify-round2/: p0-1-round2-evidence.json, p0-1-token-evidence.json, p0-2-evidence.json,
  screenshots p0-1-{docx,image,json,highlight,unicode}.png, p0-2-island-images.png, raw scripts.
- docs/qa/visual/live-shots/release-app-final.png (release app Home render).
- .agents/notes/g8r2-cargo.log, g8r2-frontend-tests.log, g8r2-frontend-build.log,
  g8r2-debug-build.log.
- dist-installer/{ProtonSearch.exe,setup.exe,BUILD_INFO.txt,SHA256SUMS.txt}.
- Perf: docs/performance/BENCHMARKS.md sections 14-15 and results JSONs.
