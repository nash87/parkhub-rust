#!/usr/bin/env bash
# Local Lighthouse CI runner — mirrors .github/workflows/lighthouse.yml.
#
# Builds parkhub-web, starts astro preview on port 18182, runs `npx lhci
# autorun` against parkhub-web/lighthouserc.json (which now asserts the INP
# threshold post-#510), tears the preview down. Skips if @lhci/cli isn't on
# PATH OR if MemAvailable < 6 GiB (Lighthouse + headless Chromium need ~3 GB
# resident).
#
# Usage:
#   scripts/local-lighthouse.sh [--port N]
#
# Env:
#   FOP_LIGHTHOUSE_MIN_MEM_GIB  bypass the 6 GiB memory floor (default: 6)

set -euo pipefail

port=18182
for arg in "$@"; do
  case "$arg" in
    --port) shift; port="$1" ;;
    --port=*) port="${arg#--port=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

# Pre-flight: memory check
min_mem_gib="${FOP_LIGHTHOUSE_MIN_MEM_GIB:-6}"
mem_avail_kib=$(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
mem_avail_gib=$((mem_avail_kib / 1024 / 1024))
if (( mem_avail_gib < min_mem_gib )); then
  echo "⊘ lighthouse skipped — MemAvailable ${mem_avail_gib} GiB < ${min_mem_gib} GiB floor"
  echo "  (set FOP_LIGHTHOUSE_MIN_MEM_GIB=<n> to override)"
  exit 0
fi

# Pre-flight: tools
if ! command -v node >/dev/null 2>&1; then
  echo "⊘ lighthouse skipped — node not on PATH"
  exit 0
fi
if [[ ! -f parkhub-web/lighthouserc.json ]]; then
  echo "⊘ lighthouse skipped — parkhub-web/lighthouserc.json missing"
  exit 0
fi

cd parkhub-web

# Ensure dist exists. astro preview serves dist/.
if [[ ! -d dist ]] || [[ -z "$(ls -A dist 2>/dev/null)" ]]; then
  echo "▶ npm run build (dist/ empty)"
  npm run build >&2
fi

# Start astro preview in background; capture PID for cleanup.
preview_log=$(mktemp -t lh-preview-XXXXXX.log)
echo "▶ astro preview --port $port (log: $preview_log)"
npx astro preview --port "$port" --host 127.0.0.1 > "$preview_log" 2>&1 &
preview_pid=$!

cleanup() {
  if [[ -n "${preview_pid:-}" ]] && kill -0 "$preview_pid" 2>/dev/null; then
    kill "$preview_pid" 2>/dev/null || true
    wait "$preview_pid" 2>/dev/null || true
  fi
  rm -f "$preview_log"
}
trap cleanup EXIT

# Wait for preview to be listening (up to 30s)
echo -n "  waiting for preview"
for i in $(seq 1 60); do
  if curl -sf "http://127.0.0.1:${port}/" >/dev/null 2>&1; then
    echo " — ready ($i tries)"
    break
  fi
  echo -n "."
  sleep 0.5
done
if ! curl -sf "http://127.0.0.1:${port}/" >/dev/null 2>&1; then
  echo " — TIMEOUT" >&2
  echo "preview log:" >&2
  tail -30 "$preview_log" >&2
  exit 1
fi

# Override the URL in lighthouserc.json on the fly so we don't depend on the
# baked startServerCommand (we already have the server running). LHCI accepts
# CLI flags that override the JSON.
echo "▶ npx lhci autorun (asserts categories + INP threshold from lighthouserc.json)"
LHCI_BUILD_CONTEXT__CURRENT_BRANCH=$(git -C .. rev-parse --abbrev-ref HEAD) \
  npx --yes @lhci/cli@^0.13 autorun \
    --config=lighthouserc.json \
    --collect.url="http://127.0.0.1:${port}/" \
    --collect.startServerCommand= \
    --collect.numberOfRuns=1
