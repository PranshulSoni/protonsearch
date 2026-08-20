#!/usr/bin/env bash
set -euo pipefail

BIN_DIR=${PROTONSEARCH_BIN_DIR:-"${HOME}/.local/bin"}
DATA_HOME=${XDG_DATA_HOME:-"${HOME}/.local/share"}
CONFIG_HOME=${XDG_CONFIG_HOME:-"${HOME}/.config"}

remove_hotkey_block() {
    local config="$1"
    [[ -f "${config}" ]] || return 0
    grep -Fq "ProtonSearch default launcher hotkey; managed by ProtonSearch installer." "${config}" || return 0
    cp -p "${config}" "${config}.protonsearch-uninstall-backup-$(date +%Y%m%d-%H%M%S)"
    local temporary
    temporary=$(mktemp "${config}.protonsearch.XXXXXX")
    if [[ "${config}" == *.lua ]]; then
        sed '/^-- ProtonSearch default launcher hotkey; managed by ProtonSearch installer\.$/,/^hl\.bind("ALT + SPACE"/d' "${config}" > "${temporary}"
    else
        sed '/^# ProtonSearch default launcher hotkey; managed by ProtonSearch installer\.$/,/^bind = ALT, SPACE, exec,/d' "${config}" > "${temporary}"
    fi
    chmod --reference="${config}" "${temporary}"
    mv "${temporary}" "${config}"
}

for config in \
    "${HYPRLAND_CONFIG:-}" \
    "${CONFIG_HOME}/hypr/hyprland.lua" \
    "${CONFIG_HOME}/hypr/hyprland.conf" \
    "${CONFIG_HOME}/sway/config"; do
    [[ -n "${config}" ]] && remove_hotkey_block "${config}"
done

if command -v hyprctl >/dev/null 2>&1; then
    hyprctl reload >/dev/null 2>&1 || true
elif command -v swaymsg >/dev/null 2>&1; then
    swaymsg reload >/dev/null 2>&1 || true
fi

if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
    systemctl --user disable --now protonsearch.service >/dev/null 2>&1 || true
    systemctl --user daemon-reload >/dev/null 2>&1 || true
fi

rm -f "${CONFIG_HOME}/systemd/user/protonsearch.service"
rm -f "${DATA_HOME}/applications/protonsearch-linux.desktop"
rm -f "${DATA_HOME}/icons/hicolor/256x256/apps/protonsearch.png"
rm -f "${DATA_HOME}/icons/hicolor/128x128/apps/protonsearch.png"
rm -f "${BIN_DIR}/protonsearch-linux"

echo "ProtonSearch binaries, desktop integration, service, and icons were removed."
echo "User settings and search data were kept at ${CONFIG_HOME}/protonsearch and ${DATA_HOME}/protonsearch."
