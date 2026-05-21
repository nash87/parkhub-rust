#!/usr/bin/env bash
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
SERVER_LOG="${SERVER_LOG:-/tmp/parkhub-v5-design-smoke-server.log}"
SERVER_DATA_DIR="${SERVER_DATA_DIR:-$(mktemp -d "${TMPDIR:-/var/tmp/florian-offload/tmp}/parkhub-v5-design-smoke.${SERVER_PORT}.XXXXXX")}"
SERVER_TARGET_DIR="${SERVER_DATA_DIR}/cargo-target"
export CI="${CI:-true}"

build_server() {
  if [[ "${FOP_LOCAL_CI_DIRECT:-0}" != "1" ]] && command -v fop >/dev/null 2>&1; then
    CARGO_TARGET_DIR="${SERVER_TARGET_DIR}" fop --compact build --backend local . --preset custom -- cargo build --locked --release -p parkhub-server \
      --no-default-features --features 'full,headless,e2e-bypass'
    return
  fi

  CARGO_TARGET_DIR="${SERVER_TARGET_DIR}" cargo build --locked --release -p parkhub-server \
    --no-default-features --features 'full,headless,e2e-bypass'
}

build_web() {
  if [[ "${FOP_LOCAL_CI_DIRECT:-0}" != "1" ]] && command -v fop >/dev/null 2>&1; then
    fop --compact build --backend local . --preset custom -- bash -euo pipefail -c "cd parkhub-web && CI=true VITE_API_URL= npm run build"
    return
  fi

  (cd parkhub-web && CI=true VITE_API_URL='' npm run build)
}

server_binary_path() {
  find "${SERVER_TARGET_DIR}" -type f -path '*/release/parkhub-server' -print -quit
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
SERVER_BIN="$(server_binary_path)"
if [[ -z "${SERVER_BIN}" ]]; then
  echo "parkhub-server release binary not found under ${SERVER_TARGET_DIR}" >&2
  exit 1
fi
SERVER_RUN_BIN="${SERVER_DATA_DIR}/parkhub-server-e2e-bypass"
# The default release target path is shared across worktrees. Build under this
# smoke run's temp dir and copy before launch so sibling CI cannot overwrite it.
cp "${SERVER_BIN}" "${SERVER_RUN_BIN}"
chmod 0755 "${SERVER_RUN_BIN}"
"${SERVER_RUN_BIN}" \
  --headless --unattended --port "${SERVER_PORT}" --data-dir "${SERVER_DATA_DIR}" > "${SERVER_LOG}" 2>&1 &
SERVER_PID=$!
cleanup() {
  kill "${SERVER_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
  echo "parkhub-server exited before the health check became reachable on :${SERVER_PORT}" >&2
  tail -n 80 "${SERVER_LOG}" >&2 || true
  exit 1
fi

READY=0
for _ in $(seq 1 45); do
  if curl -sf "http://127.0.0.1:${SERVER_PORT}/health" >/dev/null; then
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

echo "== route + v5 design smoke =="
export E2E_BASE_URL="http://127.0.0.1:${SERVER_PORT}"
if [[ $# -gt 0 ]]; then
  cd "${REPO_ROOT}/parkhub-web"
  npx playwright test "$@"
else
  export NODE_PATH="${REPO_ROOT}/parkhub-web/node_modules${NODE_PATH:+:${NODE_PATH}}"
  npx --prefix parkhub-web playwright test --config playwright.config.ts --project=chromium --project=mobile-chrome e2e/pages.spec.ts
  cd "${REPO_ROOT}/parkhub-web"
  npx playwright test --project=chromium --project=mobile-chrome e2e/v5-design-smoke.spec.ts e2e/v5-happy-paths.spec.ts
fi
