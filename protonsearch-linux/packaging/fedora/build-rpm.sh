#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
OUTPUT_DIR=${PROTONSEARCH_OUTPUT_DIR:-"${ROOT_DIR}/target/packages"}
RPM_TOP=$(mktemp -d "${TMPDIR:-/tmp}/protonsearch-rpm.XXXXXX")
trap 'rm -rf -- "${RPM_TOP}"' EXIT

command -v rpmbuild >/dev/null 2>&1 || {
    echo "rpmbuild is required to build the Fedora package" >&2
    exit 1
}

if [[ ! -x "${ROOT_DIR}/target/release/protonsearch-linux" ]]; then
    cargo build --release --locked --manifest-path "${ROOT_DIR}/Cargo.toml"
fi

mkdir -p "${RPM_TOP}/SOURCES" "${RPM_TOP}/SPECS" "${OUTPUT_DIR}"
install -Dm755 "${ROOT_DIR}/target/release/protonsearch-linux" \
    "${RPM_TOP}/SOURCES/protonsearch-linux"
install -Dm644 "${ROOT_DIR}/packaging/fedora/protonsearch-linux.spec" \
    "${RPM_TOP}/SPECS/protonsearch-linux.spec"
install -Dm644 "${ROOT_DIR}/packaging/protonsearch-linux.desktop" \
    "${RPM_TOP}/SOURCES/protonsearch-linux.desktop"
install -Dm644 "${ROOT_DIR}/assets/branding/protonsearch.png" \
    "${RPM_TOP}/SOURCES/protonsearch.png"
install -Dm644 "${ROOT_DIR}/assets/branding/protonsearch-128.png" \
    "${RPM_TOP}/SOURCES/protonsearch-128.png"
install -Dm644 "${ROOT_DIR}/packaging/protonsearch.service" \
    "${RPM_TOP}/SOURCES/protonsearch.service"

rpmbuild --define "_topdir ${RPM_TOP}" --define "_rpmdir ${OUTPUT_DIR}" \
    -bb "${RPM_TOP}/SPECS/protonsearch-linux.spec"
echo "Built Fedora RPM(s) under ${OUTPUT_DIR}"
