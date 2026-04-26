#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: .github/scripts/fop-local-ci.sh [--profile pr|full|cd] [--dry-run] [--post-status]

Runs ParkHub's local-first CI through fop's build queue. The optional
--post-status flag publishes the commit status context for the selected
profile. The GitHub PR attestation gate expects this exact command:

  .github/scripts/fop-local-ci.sh --profile pr --post-status

Profiles:
  pr    Fast PR gate: format, Rust headless checks, frontend tests/build,
        TypeScript typecheck, generated type drift.
  full  PR gate plus OpenAPI drift and Playwright smoke/e2e.
  cd    Release-oriented build and supply-chain preflight.
EOF
}

profile="pr"
dry_run=0
post_status=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      profile="${2:?missing profile}"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --post-status)
      post_status=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

case "$profile" in
  pr|full|cd) ;;
  *)
    echo "invalid profile: $profile" >&2
    exit 2
    ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

sha="$(git rev-parse HEAD)"
context="fop/local-ci/${profile}"
report_dir="$repo_root/.fop/reports"
report_path="$report_dir/local-ci-${profile}-${sha}.json"
started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

status_repo() {
  if [[ -n "${FOP_LOCAL_CI_STATUS_REPO:-}" ]]; then
    printf '%s\n' "$FOP_LOCAL_CI_STATUS_REPO"
    return 0
  fi
  for remote in github upstream origin; do
    url="$(git remote get-url "$remote" 2>/dev/null || true)"
    if [[ "$url" =~ github.com[:/]([^/]+/[^/.]+)(\.git)?$ ]]; then
      printf '%s\n' "${BASH_REMATCH[1]}"
      return 0
    fi
  done
  echo "unable to derive GitHub owner/repo; set FOP_LOCAL_CI_STATUS_REPO" >&2
  return 1
}

post_commit_status() {
  local state="$1"
  local description="$2"
  if [[ "$post_status" -ne 1 || "$dry_run" -eq 1 ]]; then
    return 0
  fi
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh is required for --post-status" >&2
    return 1
  fi

  gh api \
    --method POST \
    "repos/$(status_repo)/statuses/${sha}" \
    -f state="$state" \
    -f context="$context" \
    -f description="$description" >/dev/null
}

write_report() {
  local state="$1"
  local failed_step="${2:-}"
  mkdir -p "$report_dir"
  cat > "$report_path" <<EOF
{
  "schema": "parkhub.local-ci.v1",
  "profile": "$profile",
  "state": "$state",
  "commit": "$sha",
  "started_at": "$started_at",
  "finished_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "failed_step": "$failed_step",
  "context": "$context"
}
EOF
}

run_step() {
  local name="$1"
  local command="$2"
  printf '\n==> %s\n' "$name"
  if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY-RUN: %s\n' "$command"
    return 0
  fi
  fop build --backend local . --preset custom -- bash -euo pipefail -c "$command"
}

run_direct() {
  local name="$1"
  local command="$2"
  printf '\n==> %s\n' "$name"
  if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY-RUN: %s\n' "$command"
    return 0
  fi
  bash -euo pipefail -c "$command"
}

mark_failure() {
  local line="$1"
  write_report "failure" "line:${line}"
  post_commit_status "failure" "fop local ${profile} failed"
}
trap 'mark_failure "$LINENO"' ERR

post_commit_status "pending" "fop local ${profile} running"

run_direct "working tree whitespace" "git diff --check"

run_step "cargo fmt" "cargo fmt --all -- --check"

run_step "cargo check headless" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo check --locked --package parkhub-common --all-targets && cargo check --locked --package parkhub-server --no-default-features --features headless --all-targets"

run_step "cargo clippy headless" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo clippy --locked --package parkhub-common --all-targets -- -D warnings && cargo clippy --locked --package parkhub-server --no-default-features --features headless --all-targets -- -D warnings -A clippy::cognitive_complexity -A clippy::assigning_clones"

run_step "frontend typecheck" "cd parkhub-web && ./node_modules/.bin/tsc --noEmit"

run_step "frontend test and build" "cd parkhub-web && npm test && npm run build"

run_step "typescript bindings drift" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo test --locked --features gen-types -p parkhub-server --test ts_export -- --nocapture && git diff --exit-code parkhub-web/src/generated/ && test -z \"\$(git status --porcelain parkhub-web/src/generated/)\""

if [[ "$profile" == "full" ]]; then
  run_step "openapi drift" "cd parkhub-web && npm run build && cd .. && cargo build --locked --release -p parkhub-server --no-default-features --features 'full,headless' && pid=''; cleanup() { if [[ -n \"\${pid:-}\" ]]; then kill \"\$pid\" 2>/dev/null || true; fi; }; trap cleanup EXIT; mkdir -p /tmp/parkhub-drift-db && { ./target/release/parkhub-server --headless --unattended --port 18181 --data-dir /tmp/parkhub-drift-db >/tmp/parkhub-drift.log 2>&1 & pid=\$!; }; for i in \$(seq 1 45); do curl -sf http://localhost:18181/health >/dev/null 2>&1 && break; sleep 1; done; ./scripts/dump-openapi.sh 18181; git diff --exit-code docs/openapi/rust.json"
  run_step "playwright chromium" "cd parkhub-web && npm run build && cd .. && cargo build --locked --release -p parkhub-server --no-default-features --features 'full,headless,e2e-bypass' && pid=''; cleanup() { if [[ -n \"\${pid:-}\" ]]; then kill \"\$pid\" 2>/dev/null || true; fi; }; trap cleanup EXIT; { DEMO_MODE=true PARKHUB_ADMIN_PASSWORD=demo PARKHUB_DISABLE_RATE_LIMITS=true ./target/release/parkhub-server --headless --unattended --port 8081 >/tmp/parkhub-e2e.log 2>&1 & pid=\$!; }; for i in \$(seq 1 45); do curl -sf http://localhost:8081/health >/dev/null 2>&1 && break; sleep 1; done; npx playwright test --project=chromium"
fi

if [[ "$profile" == "cd" ]]; then
  run_step "release image preflight" "cargo test --locked --package parkhub-common --all-targets && cargo test --locked --package parkhub-server --no-default-features --features headless --all-targets"
fi

write_report "success"
post_commit_status "success" "fop local ${profile} passed"

printf '\nlocal CI passed: %s\n' "$report_path"
