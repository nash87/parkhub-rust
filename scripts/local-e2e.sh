#!/usr/bin/env bash
# Local Playwright E2E runner — mirrors .github/workflows/e2e.yml.
# Builds the Rust server with feature-flag e2e-bypass + builds the frontend,
# starts the server on port 8081, runs the full Playwright suite (not just
# chromium — also firefox + webkit + mobile-chrome).
#
# Already covered by Stage 6 of fop-local-ci.sh in `full` profile (chromium
# only). This script lets you run the FULL multi-browser suite on demand —
# useful before tagging a release.
#
# Usage:
#   scripts/local-e2e.sh [--project chromium|firefox|webkit|mobile-chrome]
#                        [--grep PATTERN]
#                        [--ui]
#
# Skips silently if cargo/playwright not installed.

set -euo pipefail

project=""
grep=""
ui=0
for arg in "$@"; do
  case "$arg" in
    --project) shift; project="$1" ;;
    --project=*) project="${arg#--project=}" ;;
    --grep) shift; grep="$1" ;;
    --grep=*) grep="${arg#--grep=}" ;;
    --ui) ui=1 ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "⊘ E2E skipped — cargo not on PATH"
  exit 0
fi
if [[ ! -d parkhub-web ]]; then
  echo "⊘ E2E skipped — parkhub-web/ missing"
  exit 0
fi
if [[ ! -d parkhub-web/node_modules/@playwright ]]; then
  echo "⊘ E2E skipped — @playwright not in parkhub-web/node_modules (run npm ci first)"
  exit 0
fi

echo "▶ build frontend (parkhub-web)"
(cd parkhub-web && npm run build)

echo "▶ build server (release, full+headless+e2e-bypass)"
cargo build --locked --release -p parkhub-server \
  --no-default-features --features 'full,headless,e2e-bypass'

target_dir=$(cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory)
server_bin="$target_dir/release/parkhub-server"
data_dir=$(mktemp -d -t parkhub-e2e-data-XXXXXX)
log_file=$(mktemp -t parkhub-e2e-XXXXXX.log)

cleanup() {
  if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$data_dir" "$log_file"
}
trap cleanup EXIT

echo "▶ start server on :8081 (data: $data_dir, log: $log_file)"
DEMO_MODE=true \
PARKHUB_ADMIN_PASSWORD=demo \
PARKHUB_DISABLE_RATE_LIMITS=true \
"$server_bin" --headless --unattended --port 8081 --data-dir "$data_dir" \
  >"$log_file" 2>&1 &
pid=$!

echo -n "  waiting for /health"
for i in $(seq 1 60); do
  if curl -sf http://localhost:8081/health >/dev/null 2>&1; then
    echo " — ready ($i tries)"
    break
  fi
  echo -n "."
  sleep 1
done

if ! curl -sf http://localhost:8081/health >/dev/null 2>&1; then
  echo " — TIMEOUT" >&2
  tail -50 "$log_file" >&2
  exit 1
fi

cd parkhub-web

# Build playwright args.
pw_args=(test)
[[ -n "$project" ]] && pw_args+=(--project="$project")
[[ -n "$grep" ]] && pw_args+=(--grep="$grep")
(( ui )) && pw_args+=(--ui)

echo "▶ npx playwright ${pw_args[*]}"
npx playwright "${pw_args[@]}"
