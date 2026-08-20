# ProtonSearch multi-distribution Linux support

ProtonSearch has one Linux implementation. Distribution-specific behavior is
centralized in `src/platform.rs`; providers consume capabilities and do not
select package commands themselves.

## Runtime platform model

At startup and in `protonsearch-linux doctor`, ProtonSearch detects:

- `/etc/os-release`: `ID`, `ID_LIKE`, `NAME`, `VERSION_ID`, and `PRETTY_NAME`.
- Distribution family: Arch, Debian/Ubuntu, Fedora/RHEL, or Other.
- Package manager: `pacman`, `apt-get`, `dnf`, or Unknown.
- Desktop: GNOME, KDE Plasma, Cinnamon, XFCE, Hyprland, Sway, i3, or Other.
- Display server: Wayland, X11, or Unknown.
- Optional provider capabilities and the package name for that family.

Unknown distributions use generic capability detection and are reported as
community-compatible. They are never silently labeled officially tested.

## Package-manager boundary

`platform::PackageManager` exposes package detection, version lookup, search,
install, remove, and metadata refresh using argument arrays. Elevation is only
requested by an explicit package operation through `sudo`; ProtonSearch never
runs the application as root and never stores credentials.

The source installer mirrors the same family mapping and supports `pacman`,
`apt-get`, and `dnf`. Missing optional dependencies are displayed first and
installed only after explicit consent or `--install-optional`.

## Support levels

| Level | Meaning |
| --- | --- |
| Officially tested | The complete release matrix ran on that distribution and desktop environment. |
| Compatible / community tested | Shared Linux backend and dependency mapping apply, but the full desktop matrix is not yet complete. |
| Unsupported | A required Linux runtime or package manager is unavailable; the app explains the missing capability. |

Current evidence is tracked in
[`DISTRIBUTION_TEST_MATRIX.md`](DISTRIBUTION_TEST_MATRIX.md). The current
developer machine provides Arch + Hyprland evidence only. Ubuntu, Mint,
Debian, and Fedora must not be marked PASS until clean desktop environments
run the actual installer and end-to-end tests.

## Desktop and display limitations

There is no universal Linux global-hotkey API. Hyprland and Sway installers can
write a managed binding when their configuration is discoverable. GNOME, KDE,
Cinnamon, XFCE, i3, and other desktops require assigning the installed
launcher command in the desktop shortcut UI. Wayland clipboard and tray
behavior remains capability-dependent; X11 uses the same core providers but
does not receive a fake Wayland capability.

## Validation commands

```sh
cargo test --all-targets
cargo build --release --locked
PROTONSEARCH_SKIP_SERVICE=1 PROTONSEARCH_SKIP_HOTKEY=1 \
  ./packaging/install-linux.sh --no-install-optional
protonsearch-linux doctor
```

For a real distribution release, run the installer from a clean VM or
installation, then verify launch, settings, search, themes, tray/startup,
clipboard/files, Hermes Agent, Agent History, restart behavior, idle CPU, and
memory. Containers are valid for parser, package mapping, build, and CLI tests
only; they are not proof of desktop integration.
