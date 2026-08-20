#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
BIN_DIR=${PROTONSEARCH_BIN_DIR:-"${HOME}/.local/bin"}
DATA_HOME=${XDG_DATA_HOME:-"${HOME}/.local/share"}
CONFIG_HOME=${XDG_CONFIG_HOME:-"${HOME}/.config"}
BIN_PATH="${BIN_DIR}/protonsearch-linux"
DATA_DIR="${DATA_HOME}/protonsearch"
DESKTOP_DIR="${DATA_HOME}/applications"
ICON_DIR="${DATA_HOME}/icons/hicolor/256x256/apps"
ICON_DIR_128="${DATA_HOME}/icons/hicolor/128x128/apps"
SERVICE_DIR="${CONFIG_HOME}/systemd/user"
SKIP_SERVICE=${PROTONSEARCH_SKIP_SERVICE:-0}
SKIP_HOTKEY=${PROTONSEARCH_SKIP_HOTKEY:-0}
INSTALL_OPTIONAL=${PROTONSEARCH_INSTALL_OPTIONAL:-ask}

usage() {
    cat <<'EOF'
Usage: packaging/install-linux.sh [--skip-service] [--skip-hotkey]
       [--install-optional] [--no-install-optional]

Installs ProtonSearch for the current user, enables its resident service,
installs the desktop entry and branded icon, and configures Alt+Space when
the current compositor has a supported configuration format. Missing optional
Arch providers are detected and offered for installation; no package or
service is changed without confirmation.
EOF
}

for argument in "$@"; do
    case "$argument" in
        --skip-service) SKIP_SERVICE=1 ;;
        --skip-hotkey) SKIP_HOTKEY=1 ;;
        --install-optional) INSTALL_OPTIONAL=1 ;;
        --no-install-optional) INSTALL_OPTIONAL=0 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: ${argument}" >&2; usage >&2; exit 2 ;;
    esac
done

offer_optional_packages() {
    command -v pacman >/dev/null 2>&1 || {
        echo "ProtonSearch: pacman is unavailable; optional providers were not installed." >&2
        return 0
    }

    local missing=()
    local package command_name
    local providers=(
        "xdg-utils:gio"
        "wl-clipboard:wl-paste"
        "cliphist:cliphist"
        "sqlite:sqlite3"
        "tesseract:tesseract"
        "poppler:pdftotext"
        "grim:grim"
        "slurp:slurp"
        "networkmanager:nmcli"
        "bluez-utils:bluetoothctl"
        "wireplumber:wpctl"
        "brightnessctl:brightnessctl"
        "playerctl:playerctl"
        "power-profiles-daemon:powerprofilesctl"
        "upower:upower"
    )
    for provider in "${providers[@]}"; do
        package=${provider%%:*}
        command_name=${provider#*:}
        if ! command -v "${command_name}" >/dev/null 2>&1; then
            missing+=("${package}")
        fi
    done
    if [[ ${#missing[@]} -eq 0 ]]; then
        echo "ProtonSearch: all optional Linux providers are already available."
        return 0
    fi

    local unique_missing=()
    local candidate already_seen
    for candidate in "${missing[@]}"; do
        already_seen=0
        for package in "${unique_missing[@]}"; do
            [[ "${package}" == "${candidate}" ]] && already_seen=1 && break
        done
        [[ ${already_seen} -eq 0 ]] && unique_missing+=("${candidate}")
    done
    echo "ProtonSearch: optional providers missing: ${unique_missing[*]}"
    if [[ "${INSTALL_OPTIONAL}" == 0 ]]; then
        echo "ProtonSearch: optional installation disabled; install packages later with pacman if needed."
        return 0
    fi
    if [[ "${INSTALL_OPTIONAL}" != 1 && ! -t 0 ]]; then
        echo "ProtonSearch: non-interactive install; no optional packages were changed." >&2
        echo "ProtonSearch: rerun with --install-optional to opt in explicitly." >&2
        return 0
    fi
    if [[ "${INSTALL_OPTIONAL}" != 1 ]]; then
        read -r -p "Install missing optional ProtonSearch providers with pacman? [y/N] " answer
        [[ "${answer}" =~ ^[Yy]$ ]] || {
            echo "ProtonSearch: optional providers were not installed."
            return 0
        }
    fi
    sudo pacman -S --needed "${unique_missing[@]}"
}

offer_optional_packages

if [[ ! -x "${ROOT_DIR}/target/release/protonsearch-linux" ]]; then
    command -v cargo >/dev/null 2>&1 || {
        echo "cargo is required when target/release/protonsearch-linux is absent" >&2
        exit 1
    }
    cargo build --release --locked --manifest-path "${ROOT_DIR}/Cargo.toml"
fi

install -Dm755 "${ROOT_DIR}/target/release/protonsearch-linux" "${BIN_PATH}"
install -d "${DATA_DIR}" "${DESKTOP_DIR}" "${ICON_DIR}" "${ICON_DIR_128}" "${SERVICE_DIR}"
install -Dm644 "${ROOT_DIR}/assets/branding/protonsearch.png" "${ICON_DIR}/protonsearch.png"
install -Dm644 "${ROOT_DIR}/assets/branding/protonsearch-128.png" "${ICON_DIR_128}/protonsearch.png"

sed \
    -e "s|@BINDIR@|${BIN_DIR}|g" \
    -e "s|@DATADIR@|${DATA_HOME}|g" \
    "${ROOT_DIR}/packaging/protonsearch-linux.desktop.in" \
    > "${DESKTOP_DIR}/protonsearch-linux.desktop"
sed \
    -e "s|@BINDIR@|${BIN_DIR}|g" \
    -e "s|@DATADIR@|${DATA_HOME}|g" \
    "${ROOT_DIR}/packaging/protonsearch.service.in" \
    > "${SERVICE_DIR}/protonsearch.service"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${DATA_HOME}/icons/hicolor" >/dev/null 2>&1 || true
fi

configure_hyprland_hotkey() {
    local config=""
    if [[ -n "${HYPRLAND_CONFIG:-}" && -f "${HYPRLAND_CONFIG}" ]]; then
        config=${HYPRLAND_CONFIG}
    elif [[ -f "${CONFIG_HOME}/hypr/hyprland.lua" ]]; then
        config="${CONFIG_HOME}/hypr/hyprland.lua"
    elif [[ -f "${CONFIG_HOME}/hypr/hyprland.conf" ]]; then
        config="${CONFIG_HOME}/hypr/hyprland.conf"
    fi
    if [[ -z "${config}" ]]; then
        echo "ProtonSearch: Hyprland detected but no config file was found; Alt+Space was not changed." >&2
        return 1
    fi

    if grep -Eq 'ALT[[:space:]]*\+[[:space:]]*SPACE' "${config}" && grep -Eq 'protonsearch-linux' "${config}"; then
        echo "ProtonSearch: Alt+Space already points to ${BIN_PATH}."
    elif grep -Eq 'ALT[[:space:]]*\+[[:space:]]*SPACE|bind[[:space:]]*=[[:space:]]*ALT,[[:space:]]*SPACE' "${config}"; then
        echo "ProtonSearch: Alt+Space is already assigned in ${config}; refusing to overwrite it." >&2
        echo "ProtonSearch: assign Alt+Space to ${BIN_PATH} or run with --skip-hotkey." >&2
        return 1
    else
        cp -p "${config}" "${config}.protonsearch-backup-$(date +%Y%m%d-%H%M%S)"
        if [[ "${config}" == *.lua ]]; then
            local lua_path=${BIN_PATH//\\/\\\\}
            lua_path=${lua_path//\"/\\\"}
            printf '\n-- ProtonSearch default launcher hotkey; managed by ProtonSearch installer.\n' >> "${config}"
            printf 'hl.bind("ALT + SPACE", hl.dsp.exec_cmd("%s"))\n' "${lua_path}" >> "${config}"
        else
            printf '\n# ProtonSearch default launcher hotkey; managed by ProtonSearch installer.\n' >> "${config}"
            local conf_path=${BIN_PATH//\\/\\\\}
            conf_path=${conf_path//\"/\\\"}
            printf 'bind = ALT, SPACE, exec, "%s"\n' "${conf_path}" >> "${config}"
        fi
        echo "ProtonSearch: configured Alt+Space in ${config}."
    fi

    if command -v hyprctl >/dev/null 2>&1; then
        hyprctl reload >/dev/null 2>&1 || echo "ProtonSearch: restart Hyprland to activate the new binding." >&2
    fi
}

configure_sway_hotkey() {
    local config="${CONFIG_HOME}/sway/config"
    [[ -f "${config}" ]] || {
        echo "ProtonSearch: Sway detected but ${config} was not found." >&2
        return 1
    }
    if grep -Eq 'Mod1\+space' "${config}"; then
        echo "ProtonSearch: Alt+Space is already assigned in ${config}; refusing to overwrite it." >&2
        return 1
    fi
    cp -p "${config}" "${config}.protonsearch-backup-$(date +%Y%m%d-%H%M%S)"
    printf '\n# ProtonSearch default launcher hotkey; managed by ProtonSearch installer.\n' >> "${config}"
    local sway_path=${BIN_PATH//\\/\\\\}
    sway_path=${sway_path//\"/\\\"}
    printf 'bindsym Mod1+space exec --no-startup-id "%s"\n' "${sway_path}" >> "${config}"
    command -v swaymsg >/dev/null 2>&1 && swaymsg reload >/dev/null 2>&1 || true
    echo "ProtonSearch: configured Alt+Space in ${config}."
}

if [[ "${SKIP_HOTKEY}" != 1 ]]; then
    if [[ "${XDG_CURRENT_DESKTOP:-}" == *Hyprland* || -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
        configure_hyprland_hotkey || true
    elif [[ "${XDG_CURRENT_DESKTOP:-}" == *Sway* || -n "${SWAYSOCK:-}" ]]; then
        configure_sway_hotkey || true
    else
        echo "ProtonSearch: automatic Alt+Space setup is not available for this desktop." >&2
        echo "ProtonSearch: the app is installed; assign ${BIN_PATH} to Alt+Space in desktop keyboard settings." >&2
    fi
fi

if [[ "${SKIP_SERVICE}" != 1 ]] && command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
    if command -v dbus-update-activation-environment >/dev/null 2>&1; then
        dbus-update-activation-environment --systemd WAYLAND_DISPLAY DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XDG_RUNTIME_DIR HYPRLAND_INSTANCE_SIGNATURE >/dev/null 2>&1 || true
    fi
    systemctl --user daemon-reload
    systemctl --user enable --now protonsearch.service
else
    echo "ProtonSearch: systemd user services are unavailable; start ${BIN_PATH} gui manually." >&2
fi

echo "ProtonSearch installed for ${USER:-the current user}."
echo "Binary: ${BIN_PATH}"
echo "Settings: ${CONFIG_HOME}/protonsearch/settings.json"
echo "Check providers with: ${BIN_PATH} doctor"
echo "Hermes Agent is detected when its 'hermes' command is on PATH; install it separately if you want AI providers."
