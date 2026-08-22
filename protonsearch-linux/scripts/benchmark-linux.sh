#!/usr/bin/env bash
set -euo pipefail

# Lightweight, read-only release benchmark for a local Linux installation.
# It intentionally measures the same bounded CLI paths used by the launcher;
# GUI paint latency still needs a real desktop-session smoke test.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${1:-$ROOT_DIR/target/release/protonsearch-linux}"

if [[ ! -x "$BINARY" ]]; then
  printf 'Release binary not found: %s\nBuild it with: cargo build --release --locked\n' "$BINARY" >&2
  exit 1
fi

printf '%s\n' 'ProtonSearch Linux performance snapshot'
printf 'binary: %s\n' "$BINARY"
printf 'sha256: '; sha256sum "$BINARY" | awk '{print $1}'
printf 'cpus: %s\n' "$(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN || printf unknown)"
printf 'memory: '; free -h 2>/dev/null | awk '/^Mem:/ {print $2 " total, " $7 " available"}' || printf unknown
printf 'load: '; awk '{print $1 " " $2 " " $3}' /proc/loadavg 2>/dev/null || printf unknown
printf 'version: '; "$BINARY" version

measure() {
  local label="$1"
  shift
  printf '\n[%s]\n' "$label"
  if command -v /usr/bin/time >/dev/null 2>&1; then
    /usr/bin/time -f 'elapsed=%e sec max-rss=%M KiB user=%U sys=%S' "$@" >/dev/null
  else
    time "$@" >/dev/null
  fi
}

measure 'doctor and adaptive hardware detection' "$BINARY" doctor
measure 'file search' "$BINARY" search README
measure 'all-provider query' "$BINARY" search-all proton
measure 'image filename search' "$BINARY" search-all 'images:'

if command -v systemctl >/dev/null 2>&1 && systemctl --user is-active --quiet protonsearch.service 2>/dev/null; then
  pid="$(systemctl --user show protonsearch.service -p MainPID --value)"
  printf '\n[resident service]\n'
  systemctl --user show protonsearch.service \
    -p ActiveState -p MainPID -p MemoryCurrent -p CPUUsageNSec -p TasksCurrent
  ps -o pid,ppid,pcpu,rss,nlwp,comm,args -p "$pid"
fi
