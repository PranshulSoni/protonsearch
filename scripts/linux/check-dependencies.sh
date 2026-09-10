#!/usr/bin/env bash
set -u

missing=0
check() {
    local command_name="$1"
    local package_name="$2"
    if command -v "$command_name" >/dev/null 2>&1; then
        printf 'available %-18s (%s)\n' "$command_name" "$package_name"
    else
        printf 'missing   %-18s install package: %s\n' "$command_name" "$package_name"
        missing=1
    fi
}

printf '%s\n' 'ProtonSearch Linux optional dependency report'
check xdg-open xdg-utils
check nmcli networkmanager
check bluetoothctl bluez-utils
check wpctl wireplumber
check brightnessctl brightnessctl
check playerctl playerctl
check loginctl systemd
check powerprofilesctl power-profiles-daemon
check hyprctl hyprland

printf '%s\n' 'Content, clipboard, and screenshot providers (optional)'
check wl-paste wl-clipboard
check wl-copy wl-clipboard
check cliphist cliphist
check sqlite3 sqlite
check pdftotext poppler
check tesseract tesseract
check grim grim
check slurp slurp

if [[ "$missing" -eq 0 ]]; then
    printf '%s\n' 'All checked providers are available.'
else
    printf '%s\n' 'Some optional providers are missing; core search remains usable.'
fi
exit 0
