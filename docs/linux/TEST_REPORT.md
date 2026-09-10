# Linux test report

## Passed

All commands below ran on the current Arch Linux host (`x86_64`, Wayland, Hyprland):

```text
cargo fmt --manifest-path protonsearch-linux/Cargo.toml --all -- --check
cargo clippy --manifest-path protonsearch-linux/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path protonsearch-linux/Cargo.toml --release
cargo test --manifest-path protonsearch-linux/Cargo.toml --all-targets
scripts/linux/check-dependencies.sh
cargo run --quiet --manifest-path protonsearch-linux/Cargo.toml -- doctor
timeout 4s ./protonsearch-linux/target/release/protonsearch-linux gui
```

Result: formatting, Clippy with warnings denied, release compilation, and all 10 unit tests passed. The dependency checker completed without installing or enabling anything. `doctor` returned Linux-only capability data and confirmed the local Arch/Wayland/Hyprland session. The graphical launcher stayed running for the smoke-test window, repeated invocation toggled the resident instance through its Unix socket, and the settings window stayed open until the bounded smoke timeout.

## Unit-tested

* XDG settings/data path shape.
* Mandatory system-root exclusion.
* Case-insensitive file search and hidden-file default.
* Desktop-entry type/hidden filtering and keyword parsing.
* Desktop Exec tokenization without a shell.
* Executable lookup rejecting shell-shaped strings.
* Hyprland query allowlist rejecting mutation commands.

## Mock-tested / safe validation

* Provider actions use fixed argv and reject invalid volume/brightness percentages before spawning a provider.
* Power actions return a confirmation-required result without executing when `--confirm` is absent.
* Missing-provider behavior is represented by capability states; no package installation path exists.
* No Wi-Fi, Bluetooth, audio, brightness, media, power, or session state was changed during testing.

## Not runtime-tested

* Real Bluetooth pairing/discovery, Wi-Fi connection/password flows, and monitor writes are intentionally not implemented.
* Hardware-specific brightness permissions, battery-only systems, absent audio servers, and non-Hyprland compositor IPC were not all available in one host session.
* The existing Windows crate was not built on Linux because it is intentionally Windows-only and was not modified.
* Global Wayland shortcuts are compositor-owned; the Hyprland binding is documented but was not written into the user's config.
* Browser/content/OCR/clipboard/Git providers are runtime-optional and were verified by provider availability checks; exhaustive data-source correctness depends on each user's installed browser and provider packages.
* Tray integration, screenshot portals, and browser upload-based visual search remain deferred.

## Rollback

Rollback is additive and does not require touching the Windows implementation. Remove only the exact new paths listed in the migration plan: `protonsearch-linux/`, `docs/linux/`, `scripts/linux/`, and `packaging/arch/`. Preserve all pre-existing paths and review Git status before any removal. The existing Windows crate, lockfile, assets, and dotfiles are not rollback targets.
