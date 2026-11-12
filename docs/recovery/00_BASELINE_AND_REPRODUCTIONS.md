# Recovery baseline — 10 November 2026

Gate G0 is incomplete. No release acceptance is claimed.

User request: pasted recovery directive. Specification: Downloads/PROTONSEARCH_RECOVERY_MASTER_PROMPT.md. Binding visual reference: supplied V9 polished HTML. The twelve named November 10 screenshots were not found among the supplied attachments or directly in Downloads; screenshot-specific comparisons remain untested.

Baseline HEAD: 533fda3c0244715cd15d7d058a83d72e105c417f. Existing main.rs memory-target no-op and .agents/hang_test.json predate this work and are preserved.

- Frontend baseline: 19 files, 246 tests passed. Several tests require click-to-expand empty Search and fixed 397px height, contradicting recovery acceptance.
- Existing target/release/ProtonSearch.exe (10:41:40 UTC) is not a valid packaged UI baseline: direct fresh-profile launch rendered localhost ERR_CONNECTION_REFUSED. A release optimization profile does not prove embedded assets/custom protocol.
- Existing Release transition harness: blocked; island not visible/focused within five seconds. Raw report: ../performance/results/recovery-before.json.
- Fresh-profile startup log: setup completed; Alt+Space registration failed because the shortcut is already registered. Process Responding=True is insufficient to establish application usability.
- Native read-only probe exited successfully; no hardware control roundtrip is established.

Fresh profiles .recovery-baseline and .recovery-fresh isolate testing from personal settings and indexes. No installed application has been replaced.

## Scoped repair verification

- `npm test -- --reporter=dot`: 19 files / 251 tests passed after recovery fixes.
- `npm run tauri build -- --no-bundle`: embedded Windows Release application built successfully. Unlike the old executable, it loads http://tauri.localhost with actual application content.
- `cargo check -p protonsearch-v2` and the lazy-window structural regression test passed. This test checks routes, not native behavior.
- Independent Sol Medium review approved current scoped changes after correcting canceled-transition phases, stale queued selections, canceled native geometry and misleading feedback. Terra was unavailable.
- Actual Windows test using .recovery-packaged: fresh setup → Skip setup → Home → Preferences rendered without a persistent hang. This tests the skip path, not all seven onboarding steps.
- Returning profile: second-instance activation → normal launcher → Open ProtonSearch created the lazy Home window successfully. Normal launcher had useful idle destinations rather than a blank canvas.
- Default Alt+Space registration still fails on this host. A test-profile-only Alt+Shift+Space successfully summoned a compact native island. Subsequent UI automation lost the window during focus/blur, so query entry, native query collapse and full card navigation are NOT accepted from this run.
- No installed app was upgraded. No ten-relaunch, fresh Windows installation, existing personal-data upgrade, full hardware matrix or installer validation was performed.
