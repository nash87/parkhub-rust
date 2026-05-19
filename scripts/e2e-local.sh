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

# Allocate a parkhub-server port that is unlikely to collide with a sibling
# fop-local-ci run for a different PR worktree on the same desktop.
#
# Order of preference:
#   1. FOP_LOCAL_CI_SERVER_PORT  — explicit operator override
#   2. SERVER_PORT               — caller already has one in env
#   3. 8081 if free              — preserves docs + muscle memory
#   4. random free port in ephemeral range (49152-65535) via ss
#   5. fallback: 8082 + small random offset 0-199 (best-effort)
allocate_parkhub_server_port() {
  if [[ -n "${FOP_LOCAL_CI_SERVER_PORT:-}" ]]; then
    printf '%s' "${FOP_LOCAL_CI_SERVER_PORT}"
    return 0
  fi
  if [[ -n "${SERVER_PORT:-}" ]]; then
    printf '%s' "${SERVER_PORT}"
    return 0
  fi
  if command -v ss >/dev/null 2>&1; then
    local in_use
    in_use="$(ss -ltn 2>/dev/null | awk 'NR>1 {sub(/.*:/,"",$4); print $4}' | sort -un)"
    if ! grep -qx '8081' <<<"$in_use"; then
      printf '%s' '8081'
      return 0
    fi
    if command -v shuf >/dev/null 2>&1; then
      local picked
      picked="$(comm -23 <(seq 49152 65535) <(printf '%s\n' "$in_use") 2>/dev/null | shuf -n 1)"
      if [[ -n "$picked" ]]; then
        printf '%s' "$picked"
        return 0
      fi
    fi
  fi
  printf '%s' "$((8082 + RANDOM % 200))"
}

SERVER_PORT="$(allocate_parkhub_server_port)"
WEB_PORT="${WEB_PORT:-4321}"
SERVER_LOG="${SERVER_LOG:-/tmp/parkhub-e2e-server.log}"
export CI="${CI:-true}"

cargo_target_dir() {
  cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory
}

build_server() {
  if command -v fop >/dev/null 2>&1; then
    fop --compact build --backend local . --preset custom -- cargo build --locked --release -p parkhub-server \
      --no-default-features --features 'full,headless,e2e-bypass'
    return
  fi

  echo "   fop not found; falling back to cargo build for local e2e" >&2
  cargo build --locked --release -p parkhub-server \
    --no-default-features --features 'full,headless,e2e-bypass'
}

build_web() {
  if command -v fop >/dev/null 2>&1; then
    fop --compact build --backend local . --preset custom -- bash -lc "cd parkhub-web && CI=true VITE_API_URL= npm run build"
    return
  fi

  echo "   fop not found; falling back to npm build for local e2e" >&2
  (cd parkhub-web && CI=true VITE_API_URL='' npm run build)
}

echo "== parkhub-server (release + e2e-bypass) =="
cd "$REPO_ROOT"
build_server

echo "== start server on :${SERVER_PORT} =="
export DEMO_MODE=true
export PARKHUB_ADMIN_PASSWORD=demo
export PARKHUB_DISABLE_RATE_LIMITS=true
SERVER_BIN="$(cargo_target_dir)/release/parkhub-server"
"${SERVER_BIN}" \
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
build_web

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
