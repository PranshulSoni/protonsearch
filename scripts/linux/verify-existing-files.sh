#!/usr/bin/env bash
set -u

if [[ $# -ne 1 ]]; then
    printf 'usage: %s BASELINE_SHA256\n' "$0" >&2
    exit 2
fi

# The baseline is created before implementation and contains paths relative to
# the repository root. sha256sum performs the byte-for-byte comparison.
sha256sum --quiet -c "$1"
