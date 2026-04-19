#!/usr/bin/env bash
# One-command hermetic e2e: build release server, start it in demo mode,
# spin up Astro dev, run Playwright against the full local stack. Supports
# AI-driven vibe-coding flows (Playwright MCP, Claude Code /e2e loops).
#
# Usage:
#   ./scripts/e2e-local.sh                       # run full suite headless
#   ./scripts/e2e-local.sh --headed              # headed run
#   ./scripts/e2e-local.sh --ui                  # Playwright UI mode (time-travel debug)
#   ./scripts/e2e-local.sh e2e/design-*.spec.ts  # run specific specs
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_PORT="${SERVER_PORT:-8081}"
WEB_PORT="${WEB_PORT:-4321}"
SERVER_LOG="${REPO_ROOT}/target/e2e-server.log"

echo "== parkhub-server (release + e2e-bypass) =="
cd "$REPO_ROOT"
cargo build --locked --release -p parkhub-server \
  --no-default-features --features 'full,headless,e2e-bypass'

echo "== start server on :${SERVER_PORT} =="
export DEMO_MODE=true
export PARKHUB_ADMIN_PASSWORD=demo
export PARKHUB_DISABLE_RATE_LIMITS=true
"${REPO_ROOT}/target/release/parkhub-server" \
  --headless --unattended --port "${SERVER_PORT}" > "${SERVER_LOG}" 2>&1 &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

# Wait for health
for _ in $(seq 1 45); do
  if curl -sf "http://localhost:${SERVER_PORT}/health" >/dev/null; then
    echo "   server ready"
    break
  fi
  sleep 1
done

echo "== Playwright (hermetic local) =="
cd "${REPO_ROOT}/parkhub-web"
export E2E_LOCAL=1
export E2E_BASE_URL="http://localhost:${WEB_PORT}"
export PARKHUB_API_URL="http://localhost:${SERVER_PORT}"

if [[ "${1:-}" == "--ui" ]]; then
  shift
  npx playwright test --ui "$@"
elif [[ "${1:-}" == "--headed" ]]; then
  shift
  npx playwright test --headed "$@"
else
  npx playwright test "$@"
fi
