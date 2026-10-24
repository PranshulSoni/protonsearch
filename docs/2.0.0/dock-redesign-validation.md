# Native dock redesign validation — 24 October 2026

Validated native redesign through commit `0cc114f` on `2.0.0`. The executable is
`protonsearch/target/debug/ProtonSearch.exe`; this report concerns the Windows app,
not the HTML mockup. The integration agent did not launch the application.

## Automated results

| Check | Result |
|---|---|
| `cargo check --locked --manifest-path protonsearch/Cargo.toml --all-targets` | Passed before the final small integration fixes; final build and tests compiled the final native sources. |
| `cargo build --locked --manifest-path protonsearch/Cargo.toml --bin ProtonSearch` | Passed on final native sources; 32.12 seconds, 17 warnings. Fresh executable: 64,763,904 bytes. |
| `cargo test --locked --manifest-path protonsearch/Cargo.toml --bin ProtonSearch -- --skip tests::test_antigravity_icons` | 94 passed, zero failed, one filtered out. |
| `cargo test --locked --manifest-path protonsearch/Cargo.toml --lib` | 10 passed, zero failed. |
| Focused `rustfmt --check --edition 2021 --config skip_children=true` on the six `src/dock/{appbar,platform,taskbar,supervisor,model,mod}.rs` files | Passed. |
| `cargo clippy --locked --manifest-path protonsearch/Cargo.toml --all-targets` | Failed on two existing denied `mistyped_literal_suffixes` diagnostics in `src/main.rs:261` and `src/main.rs:8387`; neither line belongs to the redesign diff. Other warnings remain. |

The excluded Antigravity icon test was already dependent on the locally installed
application before this redesign. Its exclusion is explicit; this is not an
unqualified full-suite pass.

The first integration check found an unsupported Slint `Window::is_active()`
call, replaced with a native foreground-window query. Final native build also
found a missing unsafe block on the recovery process lookup; fixed in `674591e`.

## Integration review

Reviewed the compact Slint dock, model migration, runtime, Shell artwork copying,
AppBar work-area reservation, taskbar recovery lease, and supervisor exit paths.

Two concrete animation/placement issues found during review were corrected:

- One-second refresh replaced the entire widget model. Stable row IDs now retain
  delegates, with notifications limited to changed rows (`252474e`).
- Popup anchors ignored horizontal dock scrolling. Anchors now incorporate the
  actual viewport offset (`e68465b`).

The taskbar option defaults off, validates Explorer window identities, persists
the restoration lease before hiding, and restores from the dock's normal drop or
the controller's child-exit/stop handling. Lease parsing is bounded to 64 KiB and
64 windows. Stale-owner validation checks the executable identity. These are
code-review observations; live crash/recovery behavior still needs manual tests.

## Manual validation required

1. Launch the freshly built native app and inspect the compact 82-logical-pixel
   dock at the actual display scale. Capture dock and expanded widget panels.
2. Verify pinned/running app launch, activation, window picker, close, pin/unpin,
   Search, Files, focus controls, volume/mute, media keys, and profile persistence.
3. Verify popup anchoring after scrolling, focus-loss dismissal, transitions,
   fullscreen suppression, negative-coordinate monitors, and DPI changes.
4. Toggle **Hide the Windows taskbar**, restore it, quit normally, and test child
   crash recovery while the controller remains alive. Confirm Explorer restart
   and secondary taskbar behavior before relying on replacement mode.
5. Validate no visible startup jump or work-area feedback loop on this machine.

## Product limits retained

- Hiding Explorer's taskbar window does not remove Explorer's own work-area
  reservation. An empty strip may remain; this is not yet a complete Windows
  shell replacement.
- If both controller and renderer are forcibly terminated, durable lease recovery
  occurs on the next startup; there is no independent recovery service.
- Weather has no live provider. It explicitly shows **No forecast**, and the dead
  location input was removed. Music controls send media keys; media metadata is
  not implemented. Native system tray hosting is not implemented.
- High-resolution Shell artwork is requested, but applications which provide only
  small source artwork may still look soft. Drawer artwork loads incrementally.
- Smoothness, visual quality, and native taskbar restoration are not proven by
  compilation or these unit tests; the live inspection above is still required.

## Review Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 outstanding code findings | two corrected during integration |
| LOW | 0 reported | baseline lint warnings recorded separately |

Verdict: APPROVE for native visual inspection; manual recovery and interaction
validation remain necessary before claiming a finished taskbar replacement.
