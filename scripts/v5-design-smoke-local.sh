#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_PORT="${SERVER_PORT:-8081}"
SERVER_LOG="${SERVER_LOG:-/tmp/parkhub-v5-design-smoke-server.log}"

cargo_target_dir() {
  cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory
}

build_server() {
  if [[ "${FOP_LOCAL_CI_DIRECT:-0}" != "1" ]] && command -v fop >/dev/null 2>&1; then
    fop --compact build --backend local . --preset custom -- cargo build --locked --release -p parkhub-server \
      --no-default-features --features 'full,headless,e2e-bypass'
    return
  fi

  cargo build --locked --release -p parkhub-server \
    --no-default-features --features 'full,headless,e2e-bypass'
}

build_web() {
  if [[ "${FOP_LOCAL_CI_DIRECT:-0}" != "1" ]] && command -v fop >/dev/null 2>&1; then
    fop --compact build --backend local . --preset custom -- bash -euo pipefail -c "cd parkhub-web && VITE_API_URL= npm run build"
    return
  fi

  (cd parkhub-web && VITE_API_URL='' npm run build)
}

cd "$REPO_ROOT"

echo "== parkhub-web build =="
build_web

echo "== parkhub-server release build =="
build_server

echo "== start parkhub-server on :${SERVER_PORT} =="
export DEMO_MODE=true
export PARKHUB_ADMIN_PASSWORD=demo
export PARKHUB_DISABLE_RATE_LIMITS=true
SERVER_BIN="$(cargo_target_dir)/release/parkhub-server"
"${SERVER_BIN}" \
  --headless --unattended --port "${SERVER_PORT}" > "${SERVER_LOG}" 2>&1 &
SERVER_PID=$!
cleanup() {
  kill "${SERVER_PID}" 2>/dev/null || true
}
trap cleanup EXIT

READY=0
for _ in $(seq 1 45); do
  if curl -sf "http://localhost:${SERVER_PORT}/health" >/dev/null; then
    READY=1
    break
  fi
  sleep 1
done
if [[ "${READY}" -ne 1 ]]; then
  echo "parkhub-server never became healthy on :${SERVER_PORT}" >&2
  tail -n 80 "${SERVER_LOG}" >&2 || true
  exit 1
fi

echo "== v5 design smoke =="
cd "${REPO_ROOT}/parkhub-web"
export E2E_BASE_URL="http://localhost:${SERVER_PORT}"
if [[ $# -gt 0 ]]; then
  npx playwright test "$@"
else
  npx playwright test --project=chromium e2e/v5-design-smoke.spec.ts
fi
