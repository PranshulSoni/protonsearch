# ProtonSearch Arch Linux production checklist

This is the release gate for `arch-linux-1.0.0`. A feature is only marked
complete when its behavior is verified in the release build or explicitly
marked Linux-dependent/deferred.

## Pipeline

- [x] Repository and architecture inspection started
- [ ] Windows-versus-Linux feature matrix completed
- [x] Release performance profile recorded
- [ ] Settings dependency audit completed
- [ ] Linux Settings implementation verified page by page
- [x] Hermes detection, setup, and health check verified
- [x] Agent prompt round trip verified through ProtonSearch
- [ ] Agent History persistence and reopen behavior verified
- [ ] Search UI, tags, keyboard navigation, and loading states verified
- [x] Dark/light themes and icon states visually audited
- [ ] System tray icon and menu actions verified
- [ ] Required versus optional dependency flow verified
- [x] CPU, memory, thread, and subprocess behavior profiled
- [x] Error mapping and production logging audited
- [x] Automated regression tests pass
- [ ] Manual application test matrix completed
- [ ] Clean-environment installation test completed
- [x] Arch package/release artifact tested
- [x] README and Linux release documentation updated
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
- Hermes v0.19.1 is detected and session listing is available. The release
  CLI prompt path returned `PROTONSEARCH_APP_HERMES_E2E_OK`; the internal Agent
  window is also wired to the same bounded Hermes worker.

## Release evidence to record

| Area | Evidence | Result |
| --- | --- | --- |
| Cold startup | Release measurement and method | Pending |
| Warm startup | Release measurement and method | Pending |
| Search latency | Release CLI: empty 0.016s, `git:Proton` 0.154s, `file:Cargo` 0.756s, doctor 0.033s | PASS |
| Settings open | Release measurement | Pending |
| Idle CPU | After warmup, CPU usage increased about 59ms over 26s with the launcher resident and no active command | PASS |
| Memory stability | Resident service settled around 58 MiB cgroup memory; repeated snapshots decreased after provider startup | PASS |
| Threads/processes | 18 resident tasks after warmup; no child processes remained idle; subprocesses use bounded workers | PASS |
| Hermes prompt | `protonsearch-linux agent 'Reply with exactly: PROTONSEARCH_APP_HERMES_E2E_OK'` returned the expected response | PASS |
| Clean install | Isolated Arch-like environment result | Pending |
| Package artifact | `makepkg --nodeps --noconfirm` produced `protonsearch-linux-1.0.0-1-x86_64.pkg.tar.zst`; `pacman -Qp --info` and `--list` verified metadata and installed paths | PASS |

## Status rules

`PASS` means tested in the optimized release build. `PARTIAL` means the Linux
implementation is intentionally desktop- or dependency-dependent and has a
clear user-facing explanation. `DEFERRED` means the UI explicitly says the
feature is coming soon. No visible setting may remain silently non-functional.
