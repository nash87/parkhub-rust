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
fop --compact build --backend local . --preset custom -- cargo build --locked --release -p parkhub-server \
  --no-default-features --features 'full,headless,e2e-bypass'

echo "== start server on :${SERVER_PORT} =="
export DEMO_MODE=true
export PARKHUB_ADMIN_PASSWORD=demo
export PARKHUB_DISABLE_RATE_LIMITS=true
"${REPO_ROOT}/target/release/parkhub-server" \
  --headless --unattended --port "${SERVER_PORT}" > "${SERVER_LOG}" 2>&1 &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

# Wait for health — hard fail after 45s instead of silently proceeding to
# Playwright (Codex P2 on #348: without an explicit error path, every spec
# would then spend its timeout waiting for a backend that never came up,
# turning a 10-second failure into a multi-minute timeout storm).
READY=0
for _ in $(seq 1 45); do
  if curl -sf "http://localhost:${SERVER_PORT}/health" >/dev/null; then
    echo "   server ready"
    READY=1
    break
  fi
  sleep 1
done
if [[ "${READY}" -ne 1 ]]; then
  echo "   parkhub-server never became healthy on :${SERVER_PORT} within 45s" >&2
  echo "   last 50 log lines:" >&2
  tail -n 50 "${SERVER_LOG}" >&2 || true
  exit 1
fi

echo "== Playwright (hermetic local) =="
cd "${REPO_ROOT}/parkhub-web"
export E2E_LOCAL=1
export E2E_BASE_URL="http://localhost:${WEB_PORT}"
export PORT="${WEB_PORT}"
export API_ORIGIN="http://localhost:${SERVER_PORT}"

echo "== parkhub-web build =="
fop --compact build --backend local . --preset custom -- bash -lc "cd parkhub-web && VITE_API_URL= npm run build"

echo "== start SPA preview on :${WEB_PORT} =="
npm run preview:spa > "${REPO_ROOT}/target/e2e-web.log" 2>&1 &
WEB_PID=$!
trap 'kill $SERVER_PID $WEB_PID 2>/dev/null || true' EXIT

WEB_READY=0
for _ in $(seq 1 30); do
  if curl -sf "http://localhost:${WEB_PORT}/login" >/dev/null; then
    echo "   web ready"
    WEB_READY=1
    break
  fi
  sleep 1
done
if [[ "${WEB_READY}" -ne 1 ]]; then
  echo "   parkhub-web never became ready on :${WEB_PORT} within 30s" >&2
  echo "   last 50 web log lines:" >&2
  tail -n 50 "${REPO_ROOT}/target/e2e-web.log" >&2 || true
  exit 1
fi

if [[ "${1:-}" == "--ui" ]]; then
  shift
  npx playwright test --ui "$@"
elif [[ "${1:-}" == "--headed" ]]; then
  shift
  npx playwright test --headed "$@"
else
  npx playwright test "$@"
fi
