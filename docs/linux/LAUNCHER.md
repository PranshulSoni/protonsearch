# ProtonSearch graphical launcher

The Linux build includes a GTK4 launcher window. It searches visible `.desktop` applications, bounded XDG file roots, settings/actions, recent files, and calculator expressions. Optional providers add browser bookmarks/history, Git commits, PDF/text content, OCR, and clipboard history. The process stays resident after the first launch, so running the shortcut again toggles the same popup instead of creating duplicates.

Useful prefixes are `app:`, `file:`, `folder:`, `settings:`, `commands:`, `recent:`, `browser:`, `bookmarks:`, `history:`, `git:`, `content:`, `ocr:`, `images:`, and `clip:`.

The `settings:` results open an editable GTK settings window. It controls hidden-file policy, optional system/Hyprland providers, additional search roots, and ignored directory names.

## Run from the checkout

```bash
./protonsearch-linux/target/release/protonsearch-linux gui
```

If the release binary is not built:

```bash
cargo build --release --locked --manifest-path protonsearch-linux/Cargo.toml
```

## Hyprland shortcut

Wayland does not provide one universal global-hotkey API. Hyprland owns global shortcuts, so add this line to the user's Hyprland config:

```ini
bind = SUPER, SPACE, exec, protonsearch-linux gui
```

To make it behave as a centered floating popup, add these window rules as well:

```ini
windowrulev2 = float, class:^(com.protonsearch.Linux)$
windowrulev2 = center, class:^(com.protonsearch.Linux)$
```

Reload Hyprland after changing the config. The keybind starts the launcher; after an application or file is opened, the popup hides while the resident process remains ready for the next toggle. The packaged desktop entry can also be assigned a shortcut through another desktop environment's keyboard settings.

The program never edits Hyprland or other desktop configuration files automatically.
