#!/usr/bin/env bash
# Local build timeline — wraps `cargo build --timings=html` and copies the
# HTML report to .fop/reports/build-trace-<sha>/. Lets you spot the slow
# crate in the workspace dependency graph (often a transitive proc-macro
# or a heavyweight build script).
#
# Why local + on-demand: cargo --timings is a built-in feature that most
# projects don't use because the report has to be manually located in
# target/cargo-timings/. This script puts it next to the other .fop/reports/
# artifacts so it's discoverable.
#
# Usage:
#   scripts/local-build-trace.sh [--package PKG] [--features F]
#   scripts/local-build-trace.sh --release   # release-mode timing
#
# Defaults: workspace, dev profile, headless features.

set -euo pipefail

package=""
features="headless"
profile_flag=""
for arg in "$@"; do
  case "$arg" in
    --package) shift; package="$1" ;;
    --package=*) package="${arg#--package=}" ;;
    --features) shift; features="$1" ;;
    --features=*) features="${arg#--features=}" ;;
    --release) profile_flag="--release" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "⊘ build-trace skipped — cargo not on PATH"
  exit 0
fi

sha=$(git rev-parse --short HEAD)
out=".fop/reports/build-trace-$sha"
mkdir -p "$out"

# Build with --timings=html. cargo writes target/cargo-timings/cargo-timing-<ts>.html
# (and a -<crate>.html per-crate breakdown). We collect the latest.
build_args=(build --locked --timings=html "$@")
if [[ -n "$package" ]]; then
  build_args+=(-p "$package")
fi
if [[ -n "$profile_flag" ]]; then
  build_args+=("$profile_flag")
fi
build_args+=(--no-default-features --features "$features")

# Clean target/cargo-timings so we collect only this run's files.
rm -rf target/cargo-timings 2>/dev/null || true

echo "▶ cargo ${build_args[*]}"
cargo "${build_args[@]}" 2>&1 | tail -20

# Collect the HTML reports.
if [[ -d target/cargo-timings ]]; then
  cp target/cargo-timings/cargo-timing-*.html "$out/" 2>/dev/null || true
  # Latest report is the canonical entry point.
  latest=$(ls -t "$out"/cargo-timing-*.html 2>/dev/null | head -1)
  if [[ -n "$latest" ]]; then
    ln -sf "$(basename "$latest")" "$out/index.html"
    echo
    echo "✓ build trace report: file://$(realpath "$out/index.html")"
    echo "  $(ls -1 "$out"/*.html | wc -l) HTML file(s) in $out/"
  fi
else
  echo "⚠ no target/cargo-timings/ directory after build — check log above"
fi
