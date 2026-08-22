# ProtonSearch Linux

Linux-native launcher and search providers for ProtonSearch.

## Distribution support

ProtonSearch uses one shared Linux backend with runtime detection for the
distribution family, package manager, desktop environment, display server,
and available capabilities.

| Distribution | Family | Validation status |
| --- | --- | --- |
| Arch Linux | Arch | Officially tested on Arch/Hyprland |
| Ubuntu | Debian | Compatibility mapping added; desktop validation pending |
| Linux Mint | Debian | Compatibility mapping added; Cinnamon validation pending |
| Debian | Debian | Compatibility mapping added; desktop validation pending |
| Fedora | Fedora/RHEL | Compatibility mapping added; GNOME validation pending |

Other Arch, Debian/Ubuntu, Fedora/RHEL, and unknown distributions use generic
Linux capability detection and are community-compatible until they complete
the release matrix. A successful build alone never marks a distribution
officially tested.

## Install from a source checkout

```sh
./packaging/install-linux.sh
```

The installer builds the release binary when needed, installs it to
`~/.local/bin`, installs the desktop entry and ProtonSearch icon into the
user's XDG data directories, enables the resident systemd user service, and
configures `Alt + Space` for Hyprland or Sway when the compositor config is
discoverable. Existing non-ProtonSearch `Alt + Space` bindings are never
overwritten; the installer reports the conflict instead.

For a build-only or CI environment:

```sh
PROTONSEARCH_SKIP_SERVICE=1 PROTONSEARCH_SKIP_HOTKEY=1 ./packaging/install-linux.sh
```

The app stores user settings under `$XDG_CONFIG_HOME/protonsearch` and data
under `$XDG_DATA_HOME/protonsearch`, so every user gets their own folders and
configuration.

The installer builds and installs the optimized release binary. It detects
`pacman`, `apt-get`, or `dnf`, maps optional providers to distribution-specific
package names, asks before installing missing packages, and never silently
overwrites an existing `Alt + Space` binding. To install all detected optional
providers explicitly:

```sh
./packaging/install-linux.sh --install-optional
```

To install ProtonSearch without changing packages:

```sh
./packaging/install-linux.sh --no-install-optional
```

After installation, run `protonsearch-linux doctor` to see the detected
distribution, family, package manager, desktop, display server, and providers.
Wi-Fi, Bluetooth, audio, brightness, power, display, screenshot, PDF,
clipboard, and browser providers are capability checked at runtime. A missing
provider produces a readable explanation rather than a blank result or raw
diagnostic object. Some providers also require a desktop service or hardware
permissions to be active (for example NetworkManager for Wi-Fi and a backlight
device for brightness).

Hermes Agent is an optional dependency. If its `hermes` or legacy
`hermes-agent` command is already installed, the Agents and Agent History
sources open an Agent workspace inside the ProtonSearch launcher. The
conversation list and messages are stored per user at
`$XDG_STATE_HOME/protonsearch/agent-history.json`.

When Hermes' authenticated local gateway is reachable at `127.0.0.1:8642`,
ProtonSearch uses its Runs API and consumes progress events in the launcher.
Otherwise it uses the installed Hermes CLI as a compatibility fallback. Agent
work always runs outside the GTK thread, and missing installation/provider
configuration is shown as an actionable in-app status instead of freezing the
launcher. Use `protonsearch-linux doctor` to inspect the secret-free Hermes
installation and gateway status.

The ProtonSearch installer does not download third-party AI software or copy
its credentials. Install Hermes through its own trusted distribution method;
the installer detects it and explains the next step without silently enabling
messaging integrations.

## Images and clipboard previews

Choose `Images` or type `images:` to browse supported images found in the
user's home/XDG folders and configured search roots. Results use bounded
thumbnails; select an image and hold `Alt` for a separate preview window. The
same preview behavior is available for image entries in `clipboard:` history.
ProtonSearch monitors the GTK/GDK clipboard directly on Wayland and X11, stores
a bounded history in the user's XDG state directory, and does not require
`cliphist`. `wl-clipboard` and `xclip` are optional compatibility fallbacks
for desktops where GDK cannot expose a clipboard payload directly. Pressing `Enter` on a clipboard image restores the binary image to
the desktop clipboard. Screenshot actions also copy the generated PNG to the
clipboard, so the new screenshot is added to history automatically. Image
history rows use thumbnails rather than exposing binary metadata; clicking one
opens its preview.

If a category has no matches, ProtonSearch shows a centered explanation such
as `No image files found`, `Clipboard history is empty`, or `No Git commits
found` instead of leaving the launcher blank. If the graphical clipboard
backend is unavailable, the launcher reports that separately from an empty
history. OCR text search is intentionally
deferred; use the Images category for image filename search until OCR ships.

Git search uses `git` directly, discovers repositories in the home/XDG folders
and common mounted user project folders, and shows recent commit messages and
hashes under `git:`. Set `PROTONSEARCH_GIT_ROOTS` to a colon-separated list when
projects live somewhere outside those roots. In clipboard results, use
`Ctrl+Space` or Ctrl-click to select multiple rows; text entries are combined
with newlines. A single selected image becomes the active clipboard image; when
multiple images are selected, ProtonSearch creates one bounded contact-sheet
PNG so all selected images can be pasted in a single operation.

## Useful commands

```sh
protonsearch-linux doctor
protonsearch-linux search "filename"
protonsearch-linux settings
protonsearch-linux agent "Ask Hermes a question"
./packaging/uninstall-linux.sh
```

Linux does not provide one universal global-shortcut API across every desktop
environment. Hyprland and Sway are configured automatically; on GNOME, KDE,
Cinnamon, XFCE, i3, and other desktops the installer leaves existing shortcuts
untouched and prints the exact executable to assign to `Alt + Space` in the
desktop keyboard settings. Wayland and X11 are detected independently.

## Release checklist

```sh
cargo test --all-targets
cargo build --release --locked
./packaging/install-linux.sh --no-install-optional
protonsearch-linux doctor
systemctl --user status protonsearch.service
```

The Linux settings window keeps its own dark theme so it remains readable while
the launcher changes between system, dark, and light mode. The Indexing &
Database page documents the current bounded on-demand search model; it does not
claim to maintain a continuously rebuilt database.
