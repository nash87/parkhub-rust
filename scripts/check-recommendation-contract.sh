#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fixture="docs/recommendation-engine-fixtures/weighted_v1.basic.json"
expected_fixture_sha="fe8ffc6a8cdb645f48ded1bebcaf3f48eb4d8576c95520a75378e2f4394b4bfa"
exact_cover_fixtures=(
  "6dfcb84cd4eb61339135552ac82be5c2bb5d2f20682fac78b8ae4d10d9dad116 docs/recommendation-engine-fixtures/exact_cover_v1.batch_basic.json"
  "b81532d1e4be7cab0e909701aee355a45a52d183981f7523b877d1dd9b5628da docs/recommendation-engine-fixtures/exact_cover_v1.empty.json"
  "ded9af5e6b86cb6657a19c6d27a04b317da44d6f6e0f212d581039a89e1e6dfb docs/recommendation-engine-fixtures/exact_cover_v1.fairness_tiebreak.json"
  "971ba4478425b038464de9ab7e3c411d631ab9f8eef9b738f8df90b0c237c378 docs/recommendation-engine-fixtures/exact_cover_v1.no_solution.json"
)

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

for entry in "${exact_cover_fixtures[@]}"; do
  expected_exact_cover_fixture_sha="${entry%% *}"
  exact_cover_fixture="${entry#* }"
  require_file "$exact_cover_fixture"
  actual_exact_cover_fixture_sha="$(sha256sum "$exact_cover_fixture" | awk '{print $1}')"
  if [[ "$actual_exact_cover_fixture_sha" != "$expected_exact_cover_fixture_sha" ]]; then
    echo "ERROR: $exact_cover_fixture hash drifted: $actual_exact_cover_fixture_sha" >&2
    echo "       expected: $expected_exact_cover_fixture_sha" >&2
    echo "       Update both Rust/PHP exact-cover fixtures, tests, and this gate together." >&2
    exit 1
  fi
  require_grep '"algorithm": "exact_cover_v1"' "$exact_cover_fixture"
  require_grep '"legal_review_required": true' "$exact_cover_fixture"
  require_grep '"attorney_review_status": "required_before_customer_wording"' "$exact_cover_fixture"
  require_grep '"execution_allowed": false' "$exact_cover_fixture"
  require_grep 'attorney review, citation verification, client authorization, and final legal judgment remain required' "$exact_cover_fixture"
done

require_grep '"selected_option_ids": ["slot-a", "slot-b"]' docs/recommendation-engine-fixtures/exact_cover_v1.batch_basic.json
require_grep '"status": "fallback_no_solution"' docs/recommendation-engine-fixtures/exact_cover_v1.no_solution.json
require_grep 'deterministic fairness tie-break' docs/recommendation-engine-fixtures/exact_cover_v1.fairness_tiebreak.json
require_grep_each 'exact_cover_v1' \
  parkhub-server/src/api/recommendation_allocation.rs \
  docs/recommendation-engine-contract.md
require_grep 'allocation trace' docs/recommendation-engine-contract.md
require_grep 'allocation_trace_id' docs/recommendation-engine-contract.md parkhub-server/src/api/recommendation_allocation.rs
require_grep 'ExactCoverAllocationServed' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'constraint_set_hash' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'candidate_set_hash' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'tenant_id' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'tenant ID' docs/recommendation-engine-contract.md
require_grep 'resolve_tenant_id' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'retention_deletion_class' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'pseudonymous IDs only' docs/recommendation-engine-contract.md
require_grep 'eligibility constraints' docs/recommendation-engine-contract.md
require_grep 'legal-review flag' docs/recommendation-engine-contract.md
require_grep 'solve_exact_cover_v1' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'solve_exact_cover_allocation' parkhub-server/src/api/recommendation_allocation.rs parkhub-server/src/api/mod.rs
require_grep '/api/v1/recommendations/allocation/exact-cover' parkhub-server/src/api/mod.rs docs/recommendation-engine-contract.md
require_grep 'pub mod recommendation_allocation' parkhub-server/src/api/mod.rs
require_grep 'exact_cover_v1_shared_fixtures_match_contract' parkhub-server/src/api/recommendation_allocation.rs
require_grep 'allocation_strategy' parkhub-server/src/api/modules/schemas.rs parkhub-server/src/api/recommendations.rs
require_grep 'exact_cover_max_search_nodes' parkhub-server/src/api/modules/schemas.rs parkhub-server/src/api/recommendations.rs

echo "ParkHub Rust recommendation contract gate OK."
