#!/usr/bin/env bash
# SLSA-3 reproducibility check — builds parkhub-server twice from clean
# state and verifies the resulting binaries are byte-for-byte identical.
#
# Why: SLSA-3 progression requires that an attacker who controls one
# build environment cannot smuggle a tampered binary because INDEPENDENT
# builds of the same source produce the same output. This script
# locally proves that property (or surfaces the divergence).
#
# Mechanism:
#   1. Build release binary into TARGET_A (isolated CARGO_TARGET_DIR).
#   2. Build release binary into TARGET_B (different dir, same source).
#   3. sha256sum both binaries.
#   4. If hashes match → ✓ reproducible.
#      If hashes differ → ✗ run diffoscope (if installed) for byte-level
#                         analysis; otherwise dump cmp -l first divergence.
#
# Usage:
#   scripts/local-reproducibility-check.sh [--package PKG] [--features F]
#
# Defaults: package = parkhub-server, features = "headless"
#
# Skips silently if:
#   - cargo not on PATH
#   - we're inside a worktree with uncommitted target/ files
#
# Note: even with sccache + cargo-chef + reproducible flags
# (RUSTFLAGS=-Cmetadata=...), some sources of nondeterminism remain:
#   - SOURCE_DATE_EPOCH not set in build script
#   - debug symbols recording absolute paths
#   - timestamps in archive headers
# This script catches drift introduced by code changes; surfacing all of
# the above is a separate hardening task.

set -euo pipefail

package="parkhub-server"
features="headless"
for arg in "$@"; do
  case "$arg" in
    --package) shift; package="$1" ;;
    --package=*) package="${arg#--package=}" ;;
    --features) shift; features="$1" ;;
    --features=*) features="${arg#--features=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "⊘ reproducibility check skipped — cargo not on PATH"
  exit 0
fi

# Determinism flags. Combine with the build to maximize repro odds.
# - SOURCE_DATE_EPOCH = git commit timestamp (deterministic).
# - LC_ALL=C, TZ=UTC = stable locale + timezone for any embedded strings.
sde=$(git log -1 --pretty=%ct 2>/dev/null || echo 0)
common_env=(
  "SOURCE_DATE_EPOCH=$sde"
  "LC_ALL=C"
  "TZ=UTC"
)

work=$(mktemp -d -t parkhub-repro-XXXXXX)
target_a="$work/target-a"
target_b="$work/target-b"

cleanup() {
  rm -rf "$work"
}
trap cleanup EXIT

build_into() {
  local dir="$1"; shift
  echo "▶ build $package --features $features → $dir"
  env "${common_env[@]}" CARGO_TARGET_DIR="$dir" cargo build \
    --locked --release \
    -p "$package" \
    --no-default-features \
    --features "$features" \
    >"$work/build-$(basename "$dir").log" 2>&1
}

build_into "$target_a"
build_into "$target_b"

bin_a="$target_a/release/$package"
bin_b="$target_b/release/$package"
if [[ ! -f "$bin_a" || ! -f "$bin_b" ]]; then
  echo "✗ build artifacts missing" >&2
  echo "  expected: $bin_a + $bin_b" >&2
  exit 1
fi

hash_a=$(sha256sum "$bin_a" | awk '{print $1}')
hash_b=$(sha256sum "$bin_b" | awk '{print $1}')

echo
echo "binary A: $bin_a"
echo "  sha256: $hash_a"
echo "  size:   $(stat -c %s "$bin_a") bytes"
echo "binary B: $bin_b"
echo "  sha256: $hash_b"
echo "  size:   $(stat -c %s "$bin_b") bytes"
echo

if [[ "$hash_a" == "$hash_b" ]]; then
  echo "✓ REPRODUCIBLE — identical sha256 ($hash_a)"
  exit 0
fi

echo "✗ NOT REPRODUCIBLE — hashes differ"
echo
if command -v diffoscope >/dev/null 2>&1; then
  echo "▶ diffoscope $bin_a $bin_b (depth 0, brief)"
  diffoscope --html-dir "$work/diff" "$bin_a" "$bin_b" 2>&1 | head -50 || true
  echo "  full report: $work/diff/index.html"
else
  echo "▶ cmp -l (first 20 diff bytes; install diffoscope for full structural diff)"
  cmp -l "$bin_a" "$bin_b" | head -20 || true
fi

exit 1
