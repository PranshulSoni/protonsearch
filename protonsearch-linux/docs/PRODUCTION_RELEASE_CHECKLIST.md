# ProtonSearch Arch Linux production checklist

This is the release gate for `arch-linux-1.0.0`. A feature is only marked
complete when its behavior is verified in the release build or explicitly
marked Linux-dependent/deferred.

## Pipeline

- [x] Repository and architecture inspection started
- [ ] Windows-versus-Linux feature matrix completed
- [ ] Release performance profile recorded
- [ ] Settings dependency audit completed
- [ ] Linux Settings implementation verified page by page
- [ ] Hermes detection, setup, and health check verified
- [ ] Agent prompt round trip verified through ProtonSearch
- [ ] Agent History persistence and reopen behavior verified
- [ ] Search UI, tags, keyboard navigation, and loading states verified
- [ ] Dark/light themes and icon states visually audited
- [ ] System tray icon and menu actions verified
- [ ] Required versus optional dependency flow verified
- [ ] CPU, memory, thread, and subprocess behavior profiled
- [ ] Error mapping and production logging audited
- [ ] Automated regression tests pass
- [ ] Manual application test matrix completed
- [ ] Clean-environment installation test completed
- [ ] Arch package/release artifact tested
- [ ] README and Linux release documentation updated
- [ ] Reddit launch shortlist and posting restrictions reviewed
- [ ] Final diff hygiene, commit, and push completed

## Current verified baseline

- Linux release binary builds with `cargo build --release --locked`.
- Automated suite currently passes 28 tests.
- `protonsearch.service` is enabled and resident on the development machine.
- File/folder separation, image thumbnails, clipboard image handling, light
  theme contrast, bounded search providers, action worker isolation, and tray
  registration have existing implementation coverage.
- OCR is intentionally deferred and must show a Coming Soon result.
- Hermes discovery and session listing exist, but the real ProtonSearch Agent
  prompt round trip is not yet accepted as verified by this checklist.

## Release evidence to record

| Area | Evidence | Result |
| --- | --- | --- |
| Cold startup | Release measurement and method | Pending |
| Warm startup | Release measurement and method | Pending |
| Search latency | Representative query measurements | Pending |
| Settings open | Release measurement | Pending |
| Idle CPU | Sample duration and process | Pending |
| Memory stability | Repeated interaction sample | Pending |
| Threads/processes | Resident service snapshot | Pending |
| Hermes prompt | Prompt, response, and logs without secrets | Pending |
| Clean install | Isolated Arch-like environment result | Pending |
| Package artifact | Exact artifact and verification command | Pending |

## Status rules

`PASS` means tested in the optimized release build. `PARTIAL` means the Linux
implementation is intentionally desktop- or dependency-dependent and has a
clear user-facing explanation. `DEFERRED` means the UI explicitly says the
feature is coming soon. No visible setting may remain silently non-functional.
