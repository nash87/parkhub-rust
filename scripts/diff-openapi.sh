#!/usr/bin/env bash
#
# Diff the path list of the live Rust OpenAPI spec against the PHP
# Scramble spec so operators and CI can spot endpoint drift.
#
# Usage:
#   scripts/diff-openapi.sh \
#     http://localhost:8081/api-docs/openapi.json \
#     http://localhost:8000/docs/api.json
#
# With no arguments, falls back to the committed dumps in docs/openapi/.
#
# Depends on: curl, jq, sort, comm. Uses no network if both URLs are
# file:// or relative paths.

set -euo pipefail

rust_url="${1:-docs/openapi/rust.json}"
php_url="${2:-docs/openapi/php.json}"

fetch() {
    local src="$1"
    if [[ "$src" == http* ]]; then
        curl -fsSL "$src"
    else
        cat "$src"
    fi
}

extract_paths() {
    local runtime="$1"

    # Normalise parameter-name noise on both specs:
    #   `{uuid}` / `{slug}` / `{bookingId}` → `{id}`
    #
    # Only the PHP Scramble spec needs prefix repair. Rust's utoipa output
    # already reflects the real mounted route path, including top-level
    # public endpoints like `/status` and `/health/detailed`; rewriting those
    # into `/api/v1/...` creates false parity drift.
    jq -r '.paths | keys[]' \
        | sed -E 's/\{[a-zA-Z_]+\}/{id}/g' \
        | awk -v runtime="$runtime" '
            runtime != "php" { print; next }
            /^\/api\/v1\// { print; next }
            /^\/v1\//      { print "/api" $0; next }
            /^\/api\//     { sub(/^\/api/, "/api/v1"); print; next }
            { print "/api/v1" $0 }
          ' \
        | sort -u
}

rust_tmp=$(mktemp)
php_tmp=$(mktemp)
trap 'rm -f "$rust_tmp" "$php_tmp"' EXIT

fetch "$rust_url" | extract_paths rust > "$rust_tmp"
fetch "$php_url" | extract_paths php > "$php_tmp"

rust_total=$(wc -l < "$rust_tmp")
php_total=$(wc -l < "$php_tmp")
shared=$(comm -12 "$rust_tmp" "$php_tmp" | wc -l)
rust_only=$(comm -23 "$rust_tmp" "$php_tmp" | wc -l)
php_only=$(comm -13 "$rust_tmp" "$php_tmp" | wc -l)

printf '== ParkHub OpenAPI parity ==\n'
printf 'Rust paths      : %s\n' "$rust_total"
printf 'PHP paths       : %s\n' "$php_total"
printf 'Shared          : %s\n' "$shared"
printf 'Rust only       : %s\n' "$rust_only"
printf 'PHP only        : %s\n' "$php_only"
printf '\n'

printf '== Endpoints only in Rust ==\n'
comm -23 "$rust_tmp" "$php_tmp"
printf '\n'

printf '== Endpoints only in PHP ==\n'
comm -13 "$rust_tmp" "$php_tmp"

# Non-zero exit if anything drifted so CI can gate on it.
if [[ "$rust_only" -gt 0 || "$php_only" -gt 0 ]]; then
    exit 1
fi
