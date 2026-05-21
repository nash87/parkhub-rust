#!/usr/bin/env bash
#
# check-openapi-drift.sh — local pre-push gate that mirrors
# `.github/workflows/openapi-drift.yml`. Fails if `docs/openapi/rust.json`
# is out of sync with the live utoipa-generated spec.
#
# Approach:
#   1. Build parkhub-server with `--features 'full,headless'` (cargo
#      cache makes incremental builds fast).
#   2. Start it on an ephemeral port, wait for /health.
#   3. Dump the live OpenAPI JSON to a temp file (jq -S sorted, same
#      shape as docs/openapi/rust.json).
#   4. `git diff --no-index --exit-code` against the committed snapshot.
#   5. Print the regen command if drift is detected.
#
# Local dev override — skip this gate when iterating on docs-only PRs:
#
#   SKIP_OPENAPI_DRIFT=1 git push
#
# (Lefthook will surface that the gate ran but exited 0.)

set -euo pipefail

if [[ "${SKIP_OPENAPI_DRIFT:-0}" == "1" ]]; then
    echo "openapi-drift: skipped (SKIP_OPENAPI_DRIFT=1)"
    exit 0
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

COMMITTED_SPEC="docs/openapi/rust.json"
PORT="${OPENAPI_DRIFT_PORT:-18181}"
LIVE_SPEC="$(mktemp -t parkhub-openapi-live-XXXXXX.json)"
DATA_DIR="$(mktemp -d -t parkhub-openapi-data-XXXXXX)"
LOG_FILE="$(mktemp -t parkhub-openapi-server-XXXXXX.log)"
PID_FILE="$(mktemp -t parkhub-openapi-pid-XXXXXX)"

cleanup() {
    if [[ -s "$PID_FILE" ]]; then
        local pid
        pid="$(cat "$PID_FILE")"
        kill "$pid" 2>/dev/null || true
        # SIGTERM polite, then SIGKILL after grace period
        for _ in 1 2 3; do
            kill -0 "$pid" 2>/dev/null || break
            sleep 1
        done
        kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$LIVE_SPEC" "$PID_FILE" "$LOG_FILE"
    rm -rf "$DATA_DIR"
}
trap cleanup EXIT

if [[ ! -f "$COMMITTED_SPEC" ]]; then
    echo "openapi-drift: missing committed spec at $COMMITTED_SPEC" >&2
    exit 1
fi

# Embed placeholder for rust_embed (release build needs it before compile).
mkdir -p parkhub-web/dist
[[ -f parkhub-web/dist/index.html ]] || \
    printf '%s' '<!doctype html><html><body></body></html>' \
    > parkhub-web/dist/index.html

echo "openapi-drift: building parkhub-server (debug, headless+full)..." >&2
TARGET_DIR="$(cargo metadata --locked --format-version 1 --no-deps | jq -er '.target_directory')"
# Debug build is faster than release for local checks; spec output is
# identical because utoipa derives are compile-time.
./scripts/fop-wrap.sh cargo build \
    --locked \
    -p parkhub-server \
    --no-default-features \
    --features 'full,headless' \
    >&2

BIN="$TARGET_DIR/debug/parkhub-server"
if command -v fop >/dev/null 2>&1 && [[ -d /var/tmp/fop-targets ]]; then
    FOP_BIN="$(
        find /var/tmp/fop-targets -path '*/debug/parkhub-server' -type f \
            -printf '%T@ %p\n' 2>/dev/null \
            | sort -nr \
            | awk 'NR == 1 {print $2}'
    )"
    if [[ -n "$FOP_BIN" && "$FOP_BIN" -nt "$BIN" ]]; then
        BIN="$FOP_BIN"
    fi
fi
if [[ ! -x "$BIN" ]]; then
    echo "openapi-drift: built binary not found at $BIN" >&2
    exit 1
fi

echo "openapi-drift: starting server on port $PORT..." >&2
PARKHUB_ADMIN_PASSWORD=demo DEMO_MODE=true \
    "$BIN" --headless --unattended --port "$PORT" --data-dir "$DATA_DIR" \
    > "$LOG_FILE" 2>&1 &
echo "$!" > "$PID_FILE"

# Wait for /health (max 45s, same as CI).
ready=0
for _ in $(seq 1 45); do
    if curl -fsS "http://localhost:$PORT/health" > /dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 1
done

if [[ "$ready" -ne 1 ]]; then
    echo "openapi-drift: server did not become healthy in time" >&2
    tail -n 50 "$LOG_FILE" >&2
    exit 1
fi

# Dump + normalise (matches scripts/dump-openapi.sh formatting).
if ! curl -fsS "http://localhost:$PORT/api-docs/openapi.json" \
        | jq -S '.' > "$LIVE_SPEC"; then
    echo "openapi-drift: failed to fetch live OpenAPI spec" >&2
    tail -n 50 "$LOG_FILE" >&2
    exit 1
fi

# Diff against committed snapshot. --no-index lets git diff arbitrary
# files; --exit-code returns 1 on any difference.
if ! git diff --no-index --exit-code "$COMMITTED_SPEC" "$LIVE_SPEC"; then
    cat >&2 <<EOF
::error:: docs/openapi/rust.json is out of sync with the code.

Regenerate locally:

    # In one terminal:
    ./scripts/fop-wrap.sh \\
        cargo run --no-default-features --features 'full,headless' \\
            -p parkhub-server -- --headless --port 18181

    # In another terminal:
    ./scripts/dump-openapi.sh 18181

Then commit the updated docs/openapi/rust.json.
EOF
    exit 1
fi

echo "openapi-drift: docs/openapi/rust.json matches the live spec."
