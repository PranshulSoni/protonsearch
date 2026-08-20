# ProtonSearch Linux

Linux-native launcher and search providers for ProtonSearch.

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

The installer builds and installs the optimized release binary. It asks before
installing missing optional Arch providers; it never silently installs packages
or overwrites an existing `Alt + Space` binding. To install all detected
optional providers explicitly:

```sh
./packaging/install-linux.sh --install-optional
```

To install ProtonSearch without changing packages:

```sh
./packaging/install-linux.sh --no-install-optional
```

After installation, run `protonsearch-linux doctor` to see which providers are
available on the current desktop. Wi-Fi, Bluetooth, audio, brightness, power,
display, screenshot, OCR, PDF, clipboard, and browser providers are capability
checked at runtime. A missing provider produces a readable explanation rather
than a blank result or raw diagnostic object. Some providers also require the
desktop service or hardware permissions to be active (for example NetworkManager
for Wi-Fi and a backlight device for brightness).

Hermes Agent is an optional external application. If its `hermes` command is
already installed, the Agents and Agent History sources use it automatically.
The ProtonSearch installer does not download third-party AI software or handle
its credentials; install Hermes through its own trusted distribution method.

## Images and clipboard previews

Choose `Images` or type `images:` to browse supported images found in the
user's home/XDG folders and configured search roots. Results use bounded
thumbnails; select an image and hold `Alt` for the inline preview. The same
preview behavior is available for image entries in `clipboard:` history when
`cliphist` and `wl-clipboard` are installed. Pressing `Enter` on a clipboard
image restores the binary image to the Wayland clipboard. Screenshot actions
also copy the generated PNG to the clipboard, so the new screenshot is added
to clipboard history automatically. Image history rows use thumbnails rather
than exposing cliphist's binary-data metadata; clicking one opens its preview.

If a category has no matches, ProtonSearch shows a centered explanation such
as `No image files found`, `No clipboard history found`, or `No Git commits
found` instead of leaving the launcher blank. OCR text search remains optional
and uses `tesseract` when it is available.

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
./packaging/uninstall-linux.sh
```

Linux does not provide one universal global-shortcut API across every desktop
environment. Hyprland and Sway are configured automatically; on other desktop
environments the installer leaves existing shortcuts untouched and prints the
exact executable to assign to `Alt + Space` in the desktop keyboard settings.

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
