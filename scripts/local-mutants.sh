#!/usr/bin/env bash
# Local mutation testing — mirrors .github/workflows/mutants.yml.
# Runs cargo-mutants against parkhub-common and writes results to
# .fop/reports/mutants-<sha>/. Not a gating step; surfaces escaped
# mutants (mutations that pass the test suite — i.e., coverage gaps).
#
# v1 scope matches the workflow: parkhub-common only. parkhub-server
# is too big for a single 3-hour run; expand once baseline kill rate
# is known.
#
# Usage:
#   scripts/local-mutants.sh [--package PKG] [--timeout N]
#
# Defaults match the GHA workflow: --package parkhub-common --timeout 300.

set -euo pipefail

package="parkhub-common"
timeout=300
for arg in "$@"; do
  case "$arg" in
    --package) shift; package="$1" ;;
    --package=*) package="${arg#--package=}" ;;
    --timeout) shift; timeout="$1" ;;
    --timeout=*) timeout="${arg#--timeout=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

if ! command -v cargo-mutants >/dev/null 2>&1; then
  echo "⊘ mutants skipped — cargo-mutants not installed (cargo install cargo-mutants)"
  exit 0
fi

sha=$(git rev-parse --short HEAD)
out=".fop/reports/mutants-$sha"
mkdir -p "$out"

echo "▶ cargo mutants --json --timeout $timeout --test-package $package"
cargo mutants \
  --json \
  --timeout "$timeout" \
  --test-package "$package" \
  --output "$out" \
  || true   # mutants exit non-zero on missed mutants — surface, don't gate

if [[ -f "$out/outcomes.json" ]]; then
  total=$(jq '[.outcomes[] | .summary] | length' < "$out/outcomes.json" 2>/dev/null || echo "?")
  caught=$(jq '[.outcomes[] | select(.summary == "CAUGHT")] | length' < "$out/outcomes.json" 2>/dev/null || echo "?")
  missed=$(jq '[.outcomes[] | select(.summary == "MISSED")] | length' < "$out/outcomes.json" 2>/dev/null || echo "?")
  echo "✓ mutants done: $total total, $caught caught, $missed escaped"
  echo "  full report: $out/"
else
  echo "⚠ mutants ran but no outcomes.json — check $out/ for raw output"
fi
