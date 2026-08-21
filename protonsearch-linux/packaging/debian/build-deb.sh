#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
VERSION=${PROTONSEARCH_VERSION:-1.0.0}
OUTPUT_DIR=${PROTONSEARCH_OUTPUT_DIR:-"${ROOT_DIR}/target/packages"}
ARCH=$(dpkg --print-architecture 2>/dev/null || echo amd64)
BUILD_DIR=$(mktemp -d "${TMPDIR:-/tmp}/protonsearch-deb.XXXXXX")
trap 'rm -rf -- "${BUILD_DIR}"' EXIT

if [[ ! -x "${ROOT_DIR}/target/release/protonsearch-linux" ]]; then
    cargo build --release --locked --manifest-path "${ROOT_DIR}/Cargo.toml"
fi

install -Dm755 "${ROOT_DIR}/target/release/protonsearch-linux" \
    "${BUILD_DIR}/usr/bin/protonsearch-linux"
install -Dm644 "${ROOT_DIR}/packaging/protonsearch-linux.desktop" \
    "${BUILD_DIR}/usr/share/applications/protonsearch-linux.desktop"
install -Dm644 "${ROOT_DIR}/assets/branding/protonsearch.png" \
    "${BUILD_DIR}/usr/share/icons/hicolor/256x256/apps/protonsearch.png"
install -Dm644 "${ROOT_DIR}/assets/branding/protonsearch-128.png" \
    "${BUILD_DIR}/usr/share/icons/hicolor/128x128/apps/protonsearch.png"
install -Dm644 "${ROOT_DIR}/packaging/protonsearch.service" \
    "${BUILD_DIR}/usr/lib/systemd/user/protonsearch.service"
install -Dm644 "${ROOT_DIR}/README.md" \
    "${BUILD_DIR}/usr/share/doc/protonsearch-linux/README.md"
install -Dm644 "${ROOT_DIR}/docs/SETTINGS_PARITY.md" \
    "${BUILD_DIR}/usr/share/doc/protonsearch-linux/SETTINGS_PARITY.md"
install -Dm644 "${ROOT_DIR}/docs/MULTI_DISTRO_SUPPORT.md" \
    "${BUILD_DIR}/usr/share/doc/protonsearch-linux/MULTI_DISTRO_SUPPORT.md"

install -d "${BUILD_DIR}/DEBIAN"
cat >"${BUILD_DIR}/DEBIAN/control" <<EOF
Package: protonsearch-linux
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: ProtonSearch contributors
Depends: libgtk-4-1, libgdk-pixbuf-2.0-0
Suggests: wl-clipboard, xclip, grim, slurp, poppler-utils, network-manager, bluez, wireplumber, brightnessctl, playerctl, power-profiles-daemon, upower
Description: Linux-native ProtonSearch launcher
 Fast launcher and search providers with runtime distribution and desktop detection.
EOF

mkdir -p "${OUTPUT_DIR}"
dpkg-deb --build --root-owner-group "${BUILD_DIR}" \
    "${OUTPUT_DIR}/protonsearch-linux_${VERSION}_${ARCH}.deb"
echo "Built ${OUTPUT_DIR}/protonsearch-linux_${VERSION}_${ARCH}.deb"
