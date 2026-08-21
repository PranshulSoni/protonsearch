#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
VERSION=${PROTONSEARCH_VERSION:-1.0.0}
OUTPUT_DIR=${PROTONSEARCH_OUTPUT_DIR:-"${ROOT_DIR}/target/packages"}

command -v makepkg >/dev/null 2>&1 || {
    echo "makepkg is required to build the Arch package" >&2
    exit 1
}

if [[ ! -x "${ROOT_DIR}/target/release/protonsearch-linux" ]]; then
    cargo build --release --locked --manifest-path "${ROOT_DIR}/Cargo.toml"
fi

mkdir -p "${OUTPUT_DIR}"
pushd "${ROOT_DIR}/packaging/arch" >/dev/null
PROTONSEARCH_VERSION="${VERSION}" makepkg --syncdeps --noconfirm --cleanbuild --clean
shopt -s nullglob
packages=(./*.pkg.tar.*)
if ((${#packages[@]} == 0)); then
    echo "makepkg did not produce an Arch package" >&2
    exit 1
fi
cp -- "${packages[@]}" "${OUTPUT_DIR}/"
popd >/dev/null
echo "Built Arch package(s) under ${OUTPUT_DIR}"
