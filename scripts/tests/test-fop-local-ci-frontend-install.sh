#!/usr/bin/env bash
set -euo pipefail

script=".github/scripts/nido-local-ci.sh"

line_of() {
  local pattern="$1"
  grep -nF "$pattern" "$script" | head -n 1 | cut -d: -f1 || true
}

install_line="$(line_of 'run_step "frontend npm install" "cd parkhub-web && npm ci"')"
astro_line="$(line_of '  ensure_astro_types')"
typecheck_line="$(line_of 'run_step "frontend typecheck"')"
test_build_line="$(line_of 'run_step "frontend test and build"')"

if [[ -z "$install_line" || -z "$astro_line" || -z "$typecheck_line" || -z "$test_build_line" ]]; then
  echo "missing frontend local CI install/typecheck/build contract" >&2
  exit 1
fi

if (( install_line >= astro_line || install_line >= typecheck_line || install_line >= test_build_line )); then
  echo "frontend local CI must run npm ci before astro sync, tsc, and build" >&2
  exit 1
fi

echo "ParkHub Rust local CI frontend dependency contract OK."
