#!/usr/bin/env bash
# check-local-ci-report.sh — verify a local-CI success report exists for
# the current (or specified) commit SHA.
#
# Lookup order:
#   1. .nido/reports/local-ci-<profile>-<sha>.json  (canonical nido path)
#   2. .fop/reports/local-ci-<profile>-<sha>.json   (legacy fop compat path)
#
# Usage:
#   scripts/check-local-ci-report.sh [<profile>] [<sha>]
#
# Defaults:
#   profile  pr
#   sha      git rev-parse HEAD
#
# Exits 0 when a success report is found at either path, 1 otherwise.

set -euo pipefail

profile="${1:-pr}"
sha="${2:-}"

if [[ -z "$sha" ]]; then
  sha="$(git rev-parse HEAD 2>/dev/null || true)"
fi

if [[ -z "$sha" ]]; then
  echo "error: cannot determine HEAD SHA and none supplied" >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

nido_path="${repo_root}/.nido/reports/local-ci-${profile}-${sha}.json"
fop_path="${repo_root}/.fop/reports/local-ci-${profile}-${sha}.json"

check_report() {
  local path="$1"
  local label="$2"
  if [[ ! -f "$path" ]]; then
    return 1
  fi
  local state
  state="$(python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(d.get('state',''))" "$path" 2>/dev/null || true)"
  if [[ "$state" == "success" ]]; then
    echo "local-CI report OK (${label}): ${path}"
    return 0
  fi
  return 1
}

if check_report "$nido_path" "nido"; then
  exit 0
fi

if check_report "$fop_path" "fop-compat"; then
  exit 0
fi

echo "error: no local-CI success report found for '${profile}' profile, SHA ${sha:0:8}" >&2
echo "  looked for: ${nido_path}" >&2
echo "  looked for: ${fop_path}" >&2
echo "  run: .github/scripts/nido-local-ci.sh --profile ${profile} --post-status" >&2
exit 1
