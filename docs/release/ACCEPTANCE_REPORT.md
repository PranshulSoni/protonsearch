# Acceptance Report — ProtonSearch v2.0.0-alpha.1 (RC 4e9f589, 2026-11-10)

Owner: L1-E (rel-recon). Acceptance authority: L0. Scope: release-candidate verification and final
gate recommendation for the v2 "Project Phoenix" alpha.

## 1. Verdict

Deployment-ready locally: YES — as an UNSIGNED INTERNAL ALPHA.
Public release signed off: NO — not signed, not distributable.

The release candidate can be installed/run on this machine for internal evaluation. It must not be
published: artifacts are unsigned, the installer upgrade/uninstall/rollback drill has not been run,
mixed-DPI/High-Contrast/multi-monitor evidence is missing, idle memory and debug input latency exceed
budgets, native media/Wi-Fi/Bluetooth adapters are post-alpha milestone 2, no live Pi provider
credentials were verified, and one preview condition (matched-term highlight) remains open.

## 2. Artifacts (UNSIGNED)

Source commit 4e9f589; build metadata dist-installer/BUILD_INFO.txt (built_utc=2026-11-09T21:47:50Z,
rustc/cargo 1.99.0, node v26.7.0, bundle_target=nsis, installer_mode=currentUser, SIGNED=false).
Hashes independently recomputed and matched to SHA256SUMS.txt:

| Artifact | Size | SHA-256 |
|---|---:|---|
| dist-installer/ProtonSearch.exe | 20,750,336 B | 3010FBEED2AE5BB9EA0FC91960F0D8325BBF5BC589D9F3596B631BEFB6FE15EF |
| dist-installer/ProtonSearch_2.0.0-alpha.1_x64-setup.exe | 5,567,529 B | F5D15C2D89CA2857C6DDB71B9289645A693B37C958AAA462E7B662CEBD6FBF8D |
| dist-installer/BUILD_INFO.txt | 518 B | 61F871B94B844547F05038E43B463BC9CDB49F2BF6DE180EE124B5071126EB21 |
| dist-installer/SHA256SUMS.txt | 192 B | 3726F703BB755B07431B6196BC14E52A17FB57B15254BA804BC3E3580B71CE06 |

No Authenticode signature, no signed updater, no public release/tag/push.

## 3. Verification evidence

- Suites (green tree, app stopped for clipboard): cargo test --workspace exit 0 — 32 binaries,
  306 passed / 0 failed / 1 pre-existing ignored; frontend 18 files / 241 tests passed; production
  build exit 0. Logs: .agents/notes/g8r2-cargo.log, g8r2-frontend-tests.log, g8r2-frontend-build.log.
- Live P0-1 previews (RC tree via CDP debug build, portable profile): 14 formats executed —
  TXT/code/MD/JSON/CSV/PDF text/scanned-PDF honest unsupported/DOCX/XLSX/PNG+OCR/binary/unknown/large
  truncation/Unicode, plus stale-file error handling; matched-term highlightRanges empty and UI marks
  0 => BUG-G8-01 (P1) open.
- Live P0-2 clipboard images: real .NET Clipboard.SetImage 320x200 captured live; row clip:2
  "Image 320 x 200", PNG + thumbnailDataUrl 4,918 chars, imageWidth/Height 320/200, mime image/png;
  island Images tab shows real thumbnail; restart persistence verified. UI paste-back path remains
  a sign-off item (harness-level paste-back verified).
- Release app smoke: launched against portable profile, created v2 data dir; canonical Home render
  docs/qa/visual/live-shots/release-app-final.png.
- Performance (official/nominal): content v2 p95 8.984 ms MET (30 ms target, n=960); release
  activation p95 14.289 ms MET; selection 6 ms MET; debug input->visible p95 38.2 ms NOT MET (30 ms
  target); idle tree working set 786.9 MiB NOT MET (~150 MB budget: 70.9 app + 716.0 WebView2 across
  8 processes).

## 4. Gate status

G1 CLOSED; G2 PASS (conditional — highlight); G3 MET (content p95); G4 CONDITIONAL (design-spec sync
+ native DPI/HC); G5 PARTIAL (native adapters post-alpha milestone 2); G6 CONDITIONAL (settings E2E);
G7 PARTIAL (no live Pi); G8 PASS for local/internal alpha; G9 NOT SIGNED OFF.

## 5. Honest blockers before public release

1. Unsigned artifacts; no Authenticode, no signed update manifest, no updater.
2. Installer install/upgrade/uninstall/rollback drill NOT run on a clean VM.
3. Single monitor only; no mixed-DPI/disconnect/multi-monitor/High-Contrast evidence.
4. Idle memory 786.9 MiB over budget; debug input->visible p95 38.2 ms over 30 ms target.
5. Native media/Wi-Fi/Bluetooth deferred to post-alpha milestone 2 (honest disabled states).
6. No live Pi provider credentials/approval verification; write path remains closed (RR-02).
7. Preview matched-term highlight (BUG-G8-01) and island preview hookup/both-launcher parity open.
8. CI reproducibility/lockfile and packaging re-audit not part of this round.

## 6. Recommendation to L0

Accept the RC as an unsigned internal alpha for local deployment/evaluation; do NOT sign off a
public release. Public-release path: sign + verify, run installer drill, close highlight/UI parity,
fix memory/input budgets, verify agent approvals with real credentials, then re-run G8 and G9
acceptance.
