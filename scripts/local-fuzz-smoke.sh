#!/usr/bin/env bash
# Local fuzz smoke — mirrors .github/workflows/fuzz-smoke.yml.
# Runs cargo-fuzz against parkhub's two fuzz targets (jwt_parse,
# webhook_hmac) for FUZZ_SECONDS each. Surfaces crashes as artifact
# files in the fuzz/artifacts/ directory.
#
# Usage:
#   scripts/local-fuzz-smoke.sh [--target jwt_parse|webhook_hmac|all] [--seconds N]
#
# Defaults: --target all --seconds 60 (matches GHA FUZZ_SECONDS=60).

set -euo pipefail

target="all"
seconds=60
for arg in "$@"; do
  case "$arg" in
    --target) shift; target="$1" ;;
    --target=*) target="${arg#--target=}" ;;
    --seconds) shift; seconds="$1" ;;
    --seconds=*) seconds="${arg#--seconds=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

# cargo-fuzz needs nightly. Skip if rustup nightly not installed (don't
# auto-install — that's a 600 MB download the user should consent to).
if ! command -v cargo-fuzz >/dev/null 2>&1 && ! cargo +nightly fuzz --version >/dev/null 2>&1; then
  echo "⊘ fuzz skipped — cargo-fuzz not installed"
  echo "  install: rustup install nightly && cargo +nightly install cargo-fuzz"
  exit 0
fi
if ! cargo +nightly --version >/dev/null 2>&1; then
  echo "⊘ fuzz skipped — rust nightly toolchain not installed"
  echo "  install: rustup install nightly"
  exit 0
fi

cd parkhub-server/fuzz 2>/dev/null || {
  echo "⊘ fuzz skipped — parkhub-server/fuzz/ missing"
  exit 0
}

case "$target" in
  jwt_parse|webhook_hmac)
    targets=("$target")
    ;;
  all)
    targets=(jwt_parse webhook_hmac)
    ;;
  *)
    echo "✗ unknown target: $target (allowed: jwt_parse, webhook_hmac, all)" >&2
    exit 2
    ;;
esac

for t in "${targets[@]}"; do
  echo "▶ cargo +nightly fuzz run $t — ${seconds}s"
  cargo +nightly fuzz run "$t" -- -max_total_time="$seconds" || {
    echo "⚠ fuzz target '$t' surfaced a crash (artifacts in $(pwd)/artifacts/$t/)"
  }
done

echo "✓ fuzz smoke complete (artifacts in parkhub-server/fuzz/artifacts/ if any)"
