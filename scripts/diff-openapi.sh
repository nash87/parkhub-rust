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
    # Normalise two things:
    #   (1) `{id}` / `{uuid}` / `{slug}` → `{id}` so routes that differ only
    #       in parameter name don't show up as drift.
    #   (2) Scramble drops either the whole `/api/v1` prefix (when the
    #       Scramble `api_path` points at the `v1` group) or just the
    #       `/api` half (when it points at the outer `api` group). Utoipa
    #       on the Rust side always keeps the full `/api/v1/…` path.
    #       Collapse both PHP variants to the Rust form: leading `/v1/…`
    #       becomes `/api/v1/…`, and leading `/foo/…` (i.e. no `/api` and
    #       no `/v1`) becomes `/api/v1/foo/…`.
    jq -r '.paths | keys[]' \
        | sed -E 's/\{[a-zA-Z_]+\}/{id}/g' \
        | awk '
            /^\/api\/v1\// { print; next }
            /^\/v1\//       { print "/api" $0; next }
            /^\/api\//      { sub(/^\/api/, "/api/v1"); print; next }
            { print "/api/v1" $0 }
          ' \
        | sort -u
}

rust_tmp=$(mktemp)
php_tmp=$(mktemp)
trap 'rm -f "$rust_tmp" "$php_tmp"' EXIT

fetch "$rust_url" | extract_paths > "$rust_tmp"
fetch "$php_url" | extract_paths > "$php_tmp"

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
