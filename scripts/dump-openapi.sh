#!/usr/bin/env bash
#
# Dump the current Rust OpenAPI spec into docs/openapi/rust.json.
#
# Intended for developer use before committing an API change: run this,
# commit the updated docs/openapi/rust.json alongside the code, and the
# PR diff will show the contract change.
#
# Requires: the server to be running on $PORT (default 8081) with the
# `mod-api-docs` feature compiled in.

set -euo pipefail

PORT="${1:-8081}"
URL="http://localhost:${PORT}/api-docs/openapi.json"
OUT="docs/openapi/rust.json"

mkdir -p "$(dirname "$OUT")"

if ! curl -fsS -o "$OUT" "$URL"; then
    echo "!! Could not fetch $URL — start the server first:" >&2
    echo "   cargo run --no-default-features --features 'full,headless' -- --headless --port $PORT" >&2
    exit 1
fi

# Pretty-print + normalise ordering so diffs stay readable
jq -S '.' "$OUT" > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"

echo "wrote $OUT ($(wc -c < "$OUT") bytes, $(jq '.paths | length' "$OUT") paths)"
