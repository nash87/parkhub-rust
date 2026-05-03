#!/usr/bin/env bash
# Local visual regression — mirrors visual-regression.yml. Runs Playwright
# snapshot tests against the local server, compares against
# parkhub-web/test-results/baselines/. Catches chrome drift on demand
# (the GHA workflow is nightly + advisory because GHA-runner antialiasing
# differs from local; LOCAL runs are the canonical reference).
#
# Usage:
#   scripts/local-visual-regression.sh [--update-snapshots]
#
# --update-snapshots rebases ALL snapshots (use only after intentional
# UI changes — review the diff carefully before committing).

set -euo pipefail

update=0
for arg in "$@"; do
  case "$arg" in
    --update-snapshots) update=1 ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "⊘ visual-regression skipped — cargo not on PATH"
  exit 0
fi
if [[ ! -d parkhub-web/node_modules/@playwright ]]; then
  echo "⊘ visual-regression skipped — @playwright not installed (run npm ci in parkhub-web)"
  exit 0
fi

# Build server with e2e-bypass + frontend.
echo "▶ build frontend"
(cd parkhub-web && npm run build)

echo "▶ build server (release, full+headless+e2e-bypass)"
cargo build --locked --release -p parkhub-server \
  --no-default-features --features 'full,headless,e2e-bypass'

target_dir=$(cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory)
server_bin="$target_dir/release/parkhub-server"
data_dir=$(mktemp -d -t parkhub-vr-data-XXXXXX)
log_file=$(mktemp -t parkhub-vr-XXXXXX.log)

cleanup() {
  if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$data_dir" "$log_file"
}
trap cleanup EXIT

echo "▶ start server on :18181 (data: $data_dir)"
DEMO_MODE=true \
PARKHUB_ADMIN_PASSWORD=demo \
PARKHUB_DISABLE_RATE_LIMITS=true \
"$server_bin" --headless --unattended --port 18181 --data-dir "$data_dir" \
  >"$log_file" 2>&1 &
pid=$!

echo -n "  waiting for /health"
for i in $(seq 1 60); do
  if curl -sf http://127.0.0.1:18181/health >/dev/null 2>&1; then
    echo " — ready ($i tries)"
    break
  fi
  echo -n "."
  sleep 1
done

if ! curl -sf http://127.0.0.1:18181/health >/dev/null 2>&1; then
  echo " — TIMEOUT" >&2
  tail -50 "$log_file" >&2
  exit 1
fi

cd parkhub-web

# Discover the visual spec — same scope as visual-regression.yml.
spec_glob="e2e/v5-visual.spec.ts"
[[ ! -f "$spec_glob" ]] && spec_glob="e2e/visual.spec.ts"
[[ ! -f "$spec_glob" ]] && {
  echo "⊘ no visual spec found at e2e/v5-visual.spec.ts or e2e/visual.spec.ts"
  exit 0
}

E2E_BASE_URL="http://127.0.0.1:18181"
export E2E_BASE_URL

if (( update )); then
  echo "▶ npx playwright test --update-snapshots $spec_glob"
  npx playwright test --update-snapshots "$spec_glob" --project=chromium
  echo "✓ snapshots updated. Review with: git diff parkhub-web/e2e/__snapshots__/ && commit"
else
  echo "▶ npx playwright test $spec_glob (chromium)"
  npx playwright test "$spec_glob" --project=chromium
fi
