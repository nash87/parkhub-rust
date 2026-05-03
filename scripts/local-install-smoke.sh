#!/usr/bin/env bash
# Local install smoke — mirrors .github/workflows/install-smoke.yml.
# Verifies the docker-compose path documented in INSTALLATION.md actually
# stands up a working ParkHub from a cold clone. Useful as the last step
# of `fop ci run --gate cd` before tagging a release, AND as a quick
# sanity check for any docker-compose.yml change.
#
# Usage:
#   scripts/local-install-smoke.sh [--keep]
#
# Flags:
#   --keep   Leave the smoke instance running on exit (default: teardown).
#
# Env:
#   FOP_INSTALL_SMOKE_PORT  Port to bind on the host (default: 18080)
#   FOP_INSTALL_SMOKE_DIR   Workdir for the cold clone (default: /tmp/parkhub-smoke)

set -euo pipefail

keep=0
for arg in "$@"; do
  case "$arg" in
    --keep) keep=1 ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
done

port="${FOP_INSTALL_SMOKE_PORT:-18080}"
workdir="${FOP_INSTALL_SMOKE_DIR:-/tmp/parkhub-smoke}"

# Container runtime — prefer podman (matches Bazzite policy).
runtime=""
compose=""
if command -v podman-compose >/dev/null 2>&1; then
  runtime=podman; compose="podman-compose"
elif command -v podman >/dev/null 2>&1 && podman compose version >/dev/null 2>&1; then
  runtime=podman; compose="podman compose"
elif command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  runtime=docker; compose="docker compose"
else
  echo "⊘ install-smoke skipped — neither podman-compose nor docker compose available"
  exit 0
fi

cleanup() {
  if (( ! keep )) && [[ -d "$workdir" ]]; then
    echo "▶ teardown"
    (cd "$workdir" && $compose down -v 2>/dev/null) || true
    rm -rf "$workdir"
  elif (( keep )); then
    echo "  (--keep set; instance left at $workdir, port $port)"
  fi
}
trap cleanup EXIT

# Step 0: cold clone (use the local working tree as the source — no network
# round-trip; mirrors what a contributor following INSTALLATION.md would do).
echo "▶ cold clone (local working tree → $workdir)"
rm -rf "$workdir"
git clone --depth 1 "$(pwd)" "$workdir" >/dev/null

cd "$workdir"

# Step 1: set admin password (matches the INSTALLATION.md step 2).
echo "▶ generate .env (PARKHUB_ADMIN_PASSWORD)"
echo "PARKHUB_ADMIN_PASSWORD=$(openssl rand -base64 24)" > .env

# Override the host port so we don't clash with anything running on 8080.
if grep -qE '^\s*-\s*"8080:' docker-compose.yml; then
  echo "▶ patching host port: 8080 → $port"
  sed -i.bak -E "s|^(\s*-\s*\")8080:|\1${port}:|" docker-compose.yml
fi

# Step 2: start
echo "▶ $compose up -d"
$compose up -d

# Step 3: wait for /health/ready
echo "▶ waiting for /health/ready on :$port"
for i in $(seq 1 60); do
  if curl -sf "http://localhost:${port}/health/ready" >/dev/null 2>&1; then
    echo "✓ READY after ${i}s ($runtime/$compose)"
    exit 0
  fi
  sleep 2
done

echo "✗ FAIL: /health/ready never green after 120s" >&2
$compose logs
exit 1
