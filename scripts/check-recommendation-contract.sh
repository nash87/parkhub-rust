#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fixture="docs/recommendation-engine-fixtures/weighted_v1.basic.json"
expected_fixture_sha="fe8ffc6a8cdb645f48ded1bebcaf3f48eb4d8576c95520a75378e2f4394b4bfa"

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "ERROR: missing $path" >&2
    exit 1
  fi
}

require_grep() {
  local pattern="$1"
  shift
  if ! grep -R -n --fixed-strings "$pattern" "$@" >/dev/null; then
    echo "ERROR: missing recommendation contract pattern: $pattern" >&2
    echo "       in: $*" >&2
    exit 1
  fi
}

require_grep_each() {
  local pattern="$1"
  shift
  local path
  for path in "$@"; do
    require_grep "$pattern" "$path"
  done
}

require_file "$fixture"
actual_fixture_sha="$(sha256sum "$fixture" | awk '{print $1}')"
if [[ "$actual_fixture_sha" != "$expected_fixture_sha" ]]; then
  echo "ERROR: $fixture hash drifted: $actual_fixture_sha" >&2
  echo "       expected: $expected_fixture_sha" >&2
  echo "       Update both Rust/PHP fixtures, tests, and this gate together." >&2
  exit 1
fi

require_grep '"algorithm": "weighted_v1"' "$fixture"
require_grep '"slot_id": "slot-usual"' "$fixture"
require_grep '"score": 69' "$fixture"

require_grep_each 'fop_pipeline_v1' \
  parkhub-server/src/api/recommendations.rs \
  parkhub-server/src/api/modules/schemas.rs \
  docs/recommendation-engine-contract.md
require_grep 'fallback_algorithm=weighted_v1' docs/recommendation-engine-contract.md
require_grep_each 'RecommendationServed' parkhub-server/src/api/recommendations.rs docs/recommendation-engine-contract.md
require_grep '"weighted_v1", "fop_pipeline_v1"' parkhub-server/src/api/modules/schemas.rs
require_grep 'execution_allowed=false' docs/recommendation-engine-contract.md

echo "ParkHub Rust recommendation contract gate OK."
