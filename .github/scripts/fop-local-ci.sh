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
        TypeScript typecheck, generated type drift. Diff-aware: skips
        Rust steps if no .rs/Cargo.{toml,lock} touched, skips frontend
        if no parkhub-web/ touched. Set FOP_LOCAL_CI_NO_DIFF_AWARE=1
        to force every step.
  full  PR gate plus OpenAPI drift and Playwright smoke/e2e (always full).
  cd    Release-oriented build and supply-chain preflight (always full).

Environment:
  FOP_LOCAL_CI_STATUS_REPO    override owner/repo for status post
  FOP_LOCAL_CI_NO_DIFF_AWARE=1 disable diff-aware skipping (pr profile)
  FOP_LOCAL_CI_REUSE_PREPUSH=1 skip Rust steps already validated by lefthook
                              pre-push hook (.fop/pre-push-validated-<sha>.json)
  FOP_LOCAL_CI_NO_AUTO_HEAL=1 disable astro sync auto-run on missing types
  FOP_CAPACITY_WAIT_MAX_SECS  fop build queue capacity-wait timeout (default
                              1800s when MemAvailable < 6GB, else fop default)
  FOP_LOCAL_CI_RUN_LINTERS=1  run actionlint + zizmor (if installed) on workflows
  FOP_LOCAL_CI_DIRECT=1       bypass fop build queue entirely (kernel + earlyoom
                              handle memory). Use only when fop queue is
                              unreachable (bootstrap chicken-and-egg) or when
                              queueing behind cross-tab cargo builds would
                              starve a frontend-only run for >10 min.
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

# ─── capacity-aware: extend fop's queue timeout when other tabs dominate RAM ───
mem_avail_gb="$(awk '/MemAvailable/ {print int($2/1024/1024)}' /proc/meminfo 2>/dev/null || echo 8)"
if (( mem_avail_gb < 6 )) && [[ -z "${FOP_CAPACITY_WAIT_MAX_SECS:-}" ]]; then
  export FOP_CAPACITY_WAIT_MAX_SECS=1800
  printf 'ℹ capacity-aware: MemAvailable=%dGB tight; extending FOP_CAPACITY_WAIT_MAX_SECS=%ss\n' "$mem_avail_gb" "$FOP_CAPACITY_WAIT_MAX_SECS"
fi

# ─── diff-aware step gating (pr profile only) ───
diff_paths=""
diff_touch_rust=0
diff_touch_frontend=0
diff_touch_ts_export=0
diff_touch_workflows=0
diff_touch_php=0

compute_diff_paths() {
  if [[ "${FOP_LOCAL_CI_NO_DIFF_AWARE:-}" == "1" ]] || [[ "$profile" != "pr" ]]; then
    diff_touch_rust=1
    diff_touch_frontend=1
    diff_touch_ts_export=1
    diff_touch_workflows=1
    diff_touch_php=1
    return 0
  fi

  local base_ref
  for candidate in github/main upstream/main origin/main main; do
    if git rev-parse --verify --quiet "$candidate" >/dev/null; then
      base_ref="$candidate"
      break
    fi
  done

  if [[ -z "${base_ref:-}" ]]; then
    printf 'ℹ diff-aware: no base ref resolvable; running full pr profile\n'
    diff_touch_rust=1
    diff_touch_frontend=1
    diff_touch_ts_export=1
    diff_touch_workflows=1
    diff_touch_php=1
    return 0
  fi

  local merge_base
  merge_base="$(git merge-base "$base_ref" HEAD 2>/dev/null || echo "$base_ref")"
  diff_paths="$(git diff --name-only "${merge_base}..HEAD" 2>/dev/null || true)"

  if [[ -z "$diff_paths" ]]; then
    printf 'ℹ diff-aware: empty diff vs %s; running full pr profile\n' "$base_ref"
    diff_touch_rust=1
    diff_touch_frontend=1
    diff_touch_ts_export=1
    diff_touch_workflows=1
    diff_touch_php=1
    return 0
  fi

  if grep -qE '\.rs$|^Cargo\.(toml|lock)$|^.+/Cargo\.toml$|^rust-toolchain' <<<"$diff_paths"; then
    diff_touch_rust=1
  fi
  if grep -qE '^parkhub-web/' <<<"$diff_paths"; then
    diff_touch_frontend=1
  fi
  # ts_export drift can be triggered by Rust changes (server types) OR generated/ overrides
  if (( diff_touch_rust )) || grep -qE '^parkhub-web/src/generated/' <<<"$diff_paths"; then
    diff_touch_ts_export=1
  fi
  if grep -qE '^\.github/(workflows|scripts|actions)/' <<<"$diff_paths"; then
    diff_touch_workflows=1
  fi
  if grep -qE '\.php$|^composer\.(json|lock)$' <<<"$diff_paths"; then
    diff_touch_php=1
  fi

  printf 'ℹ diff-aware (vs %s): rust=%d frontend=%d ts_export=%d workflows=%d php=%d (%d files)\n' \
    "$base_ref" "$diff_touch_rust" "$diff_touch_frontend" "$diff_touch_ts_export" \
    "$diff_touch_workflows" "$diff_touch_php" "$(wc -l <<<"$diff_paths")"
}

# ─── pre-push hook result re-use (opt-in via FOP_LOCAL_CI_REUSE_PREPUSH=1) ───
prepush_marker="$repo_root/.fop/pre-push-validated-${sha}.json"
prepush_validated=0
if [[ "${FOP_LOCAL_CI_REUSE_PREPUSH:-}" == "1" ]] && [[ -f "$prepush_marker" ]]; then
  # Marker must be < 1 hour old to be trustworthy
  if (( $(date +%s) - $(stat -c %Y "$prepush_marker") < 3600 )); then
    prepush_validated=1
    printf 'ℹ pre-push reuse: %s validated; skipping cargo fmt/check/clippy\n' "$prepush_marker"
  fi
fi

# ─── astro auto-heal: regenerate .astro/types.d.ts if missing or stale ───
ensure_astro_types() {
  if [[ "${FOP_LOCAL_CI_NO_AUTO_HEAL:-}" == "1" ]]; then return 0; fi
  if [[ ! -d parkhub-web ]]; then return 0; fi
  local types_file="parkhub-web/.astro/types.d.ts"
  local needs_sync=0
  if [[ ! -f "$types_file" ]]; then
    needs_sync=1
  else
    # Stale if any source/config file is newer than types.d.ts (skips dist/, node_modules/)
    local newest_src
    newest_src="$(find parkhub-web/src parkhub-web/astro.config.* parkhub-web/tsconfig.json -type f -newer "$types_file" 2>/dev/null | head -1)"
    if [[ -n "$newest_src" ]]; then needs_sync=1; fi
  fi
  if (( needs_sync )); then
    if [[ -x parkhub-web/node_modules/.bin/astro ]]; then
      printf '\n==> astro sync (auto-heal: %s missing or stale)\n' "$types_file"
      if [[ "$dry_run" -eq 0 ]]; then
        ( cd parkhub-web && ./node_modules/.bin/astro sync )
      else
        printf 'DRY-RUN: cd parkhub-web && ./node_modules/.bin/astro sync\n'
      fi
    else
      printf 'ℹ astro auto-heal skipped: parkhub-web/node_modules/.bin/astro not found (run npm ci first)\n'
    fi
  fi
}

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
  "schema": "parkhub.local-ci.v2",
  "profile": "$profile",
  "state": "$state",
  "commit": "$sha",
  "started_at": "$started_at",
  "finished_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "failed_step": "$failed_step",
  "context": "$context",
  "diff_aware": {
    "enabled": $([[ "${FOP_LOCAL_CI_NO_DIFF_AWARE:-}" == "1" ]] && echo false || echo true),
    "rust": $([[ $diff_touch_rust == 1 ]] && echo true || echo false),
    "frontend": $([[ $diff_touch_frontend == 1 ]] && echo true || echo false),
    "ts_export": $([[ $diff_touch_ts_export == 1 ]] && echo true || echo false),
    "workflows": $([[ $diff_touch_workflows == 1 ]] && echo true || echo false),
    "php": $([[ $diff_touch_php == 1 ]] && echo true || echo false)
  },
  "prepush_reused": $([[ $prepush_validated == 1 ]] && echo true || echo false),
  "memory_available_gb": $mem_avail_gb
}
EOF
}

# run_step: light fop queue allocation (~2 GB) — frontend/tsc/vitest/astro/npm
# run_step_heavy: heavy allocation (~6 GB) — cargo {fmt,check,clippy,test,build}
# Backports parkhub-php's --resource-profile pattern (PR #385) to fix the
# multi-tab capacity contention where parkhub-php's CI port was starved by
# blanket 6 GB requests for npm-class steps.
#
# FOP_LOCAL_CI_DIRECT=1 bypasses the fop queue entirely (kernel handles memory).
# Use only when the queue is unreachable (bootstrap chicken-and-egg, fop service
# down, or for short frontend-only runs where the kernel + earlyoom are safer
# than queueing behind a 1+ hour cargo build in another tab).
run_step() {
  local name="$1"
  local command="$2"
  printf '\n==> %s\n' "$name"
  if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY-RUN: %s\n' "$command"
    return 0
  fi
  if [[ "${FOP_LOCAL_CI_DIRECT:-}" == "1" ]]; then
    bash -euo pipefail -c "$command"
    return $?
  fi
  fop build --backend local --resource-profile interactive-small . --preset custom -- bash -euo pipefail -c "$command"
}

run_step_heavy() {
  local name="$1"
  local command="$2"
  printf '\n==> %s\n' "$name"
  if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY-RUN: %s\n' "$command"
    return 0
  fi
  if [[ "${FOP_LOCAL_CI_DIRECT:-}" == "1" ]]; then
    bash -euo pipefail -c "$command"
    return $?
  fi
  fop build --backend local --resource-profile batch-medium . --preset custom -- bash -euo pipefail -c "$command"
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

skip_step() {
  printf '\n==> %s [SKIP: %s]\n' "$1" "$2"
}

mark_failure() {
  local line="$1"
  write_report "failure" "line:${line}"
  post_commit_status "failure" "fop local ${profile} failed"
}
trap 'mark_failure "$LINENO"' ERR

compute_diff_paths

post_commit_status "pending" "fop local ${profile} running"

# ─── Stage 1: working tree hygiene (always) ─────────────────────────────────
run_direct "working tree whitespace" "git diff --check"

# ─── Stage 2: workflow + GHA security (when workflows touched) ──────────────
if (( diff_touch_workflows )) || [[ "${FOP_LOCAL_CI_RUN_LINTERS:-}" == "1" ]]; then
  if command -v actionlint >/dev/null 2>&1; then
    run_direct "actionlint" "actionlint -color"
  fi
  if command -v zizmor >/dev/null 2>&1; then
    # Audit-mode: surface findings, don't fail the gate yet
    run_direct "zizmor (audit)" "zizmor --no-progress --persona auditor .github/workflows/ || true"
  fi
fi

# ─── Stage 3: Rust headless checks (skip if no .rs touched OR pre-push reused) ───
if (( diff_touch_rust )) && (( ! prepush_validated )); then
  run_step_heavy "cargo fmt" "cargo fmt --all -- --check"
  run_step_heavy "cargo check headless" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo check --locked --package parkhub-common --all-targets && cargo check --locked --package parkhub-server --no-default-features --features headless --all-targets"
  run_step_heavy "cargo clippy headless" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo clippy --locked --package parkhub-common --all-targets -- -D warnings && cargo clippy --locked --package parkhub-server --no-default-features --features headless --all-targets -- -D warnings -A clippy::cognitive_complexity -A clippy::assigning_clones"
elif (( prepush_validated )); then
  skip_step "cargo fmt" "validated by lefthook pre-push"
  skip_step "cargo check headless" "validated by lefthook pre-push"
  skip_step "cargo clippy headless" "validated by lefthook pre-push"
else
  skip_step "cargo fmt" "diff-aware: no Rust files touched"
  skip_step "cargo check headless" "diff-aware: no Rust files touched"
  skip_step "cargo clippy headless" "diff-aware: no Rust files touched"
fi

# ─── Stage 4: Frontend (skip if parkhub-web/ untouched) ──────────────────────
if (( diff_touch_frontend )); then
  ensure_astro_types
  run_step "frontend typecheck" "cd parkhub-web && ./node_modules/.bin/tsc --noEmit"
  run_step "frontend test and build" "cd parkhub-web && npm test && npm run build"
else
  skip_step "frontend typecheck" "diff-aware: parkhub-web/ untouched"
  skip_step "frontend test and build" "diff-aware: parkhub-web/ untouched"
fi

# ─── Stage 5: TypeScript bindings drift (Rust→TS contract; skip if both untouched) ───
if (( diff_touch_ts_export )); then
  run_step_heavy "typescript bindings drift" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo test --locked --features gen-types -p parkhub-server --test ts_export -- --nocapture && git diff --exit-code parkhub-web/src/generated/ && test -z \"\$(git status --porcelain parkhub-web/src/generated/)\""
else
  skip_step "typescript bindings drift" "diff-aware: no Rust + no parkhub-web/src/generated/ touched"
fi

# ─── Stage 6: full profile extras (always full when invoked) ─────────────────
if [[ "$profile" == "full" ]]; then
  run_step_heavy "openapi drift" "cd parkhub-web && npm run build && cd .. && cargo build --locked --release -p parkhub-server --no-default-features --features 'full,headless' && pid=''; cleanup() { if [[ -n \"\${pid:-}\" ]]; then kill \"\$pid\" 2>/dev/null || true; fi; }; trap cleanup EXIT; mkdir -p /tmp/parkhub-drift-db && { ./target/release/parkhub-server --headless --unattended --port 18181 --data-dir /tmp/parkhub-drift-db >/tmp/parkhub-drift.log 2>&1 & pid=\$!; }; for i in \$(seq 1 45); do curl -sf http://localhost:18181/health >/dev/null 2>&1 && break; sleep 1; done; ./scripts/dump-openapi.sh 18181; git diff --exit-code docs/openapi/rust.json"
  run_step_heavy "playwright chromium" "cd parkhub-web && npm run build && cd .. && cargo build --locked --release -p parkhub-server --no-default-features --features 'full,headless,e2e-bypass' && pid=''; cleanup() { if [[ -n \"\${pid:-}\" ]]; then kill \"\$pid\" 2>/dev/null || true; fi; }; trap cleanup EXIT; { DEMO_MODE=true PARKHUB_ADMIN_PASSWORD=demo PARKHUB_DISABLE_RATE_LIMITS=true ./target/release/parkhub-server --headless --unattended --port 8081 >/tmp/parkhub-e2e.log 2>&1 & pid=\$!; }; for i in \$(seq 1 45); do curl -sf http://localhost:8081/health >/dev/null 2>&1 && break; sleep 1; done; npx playwright test --project=chromium"
fi

# ─── Stage 7: cd profile extras ──────────────────────────────────────────────
if [[ "$profile" == "cd" ]]; then
  run_step_heavy "release image preflight" "cargo test --locked --package parkhub-common --all-targets && cargo test --locked --package parkhub-server --no-default-features --features headless --all-targets"
fi

# ─── Stage 8: trivy filesystem scan ─────────────────────────────────────────
# Mirrors .github/workflows/security.yml trivy-fs job. Apache-2.0 license.
# Skips gracefully on `pr` profile if trivy isn't on PATH (so contributors
# without trivy installed can still pass the local gate); always required on
# `cd`/`full` profiles. Findings under .trivyignore (with justification
# comments) are filtered. Severity matches the workflow: CRITICAL,HIGH only.
trivy_required=0
[[ "$profile" == "cd" || "$profile" == "full" ]] && trivy_required=1
if command -v trivy >/dev/null 2>&1; then
  run_step "trivy filesystem scan" "trivy fs --quiet --exit-code 1 --scanners=vuln,misconfig --severity=CRITICAL,HIGH --ignorefile .trivyignore --skip-dirs=node_modules,target,parkhub-web/node_modules,.claude/worktrees ."
elif [[ $trivy_required -eq 1 ]]; then
  echo "✗ trivy filesystem scan FAILED: trivy not on PATH (required for ${profile} profile)" >&2
  write_report "failure" "trivy filesystem scan"
  post_commit_status "failure" "fop local ${profile} failed: trivy not installed"
  exit 1
else
  skip_step "trivy filesystem scan" "trivy not on PATH (install: https://aquasecurity.github.io/trivy/)"
fi

# ─── Stage 9: zizmor (GitHub Actions SAST, advisory) ────────────────────────
# Mirrors .github/workflows/security.yml zizmor job. MIT license. Replaces
# CodeQL's `actions/missing-workflow-permissions` coverage and adds 30+ rules
# for CI/CD hardening (template injection, cache poisoning, persist-credentials,
# excessive-permissions). Uses --persona=auditor to match the workflow.
#
# Advisory mode: matches workflow's `continue-on-error: true` — zizmor surfaces
# findings as informational but does NOT fail the gate. Promote to a hard
# failure (drop the `|| true`) once the open-finding inventory is at zero.
# Suppressions live in zizmor.yml with per-rule justification.
if command -v zizmor >/dev/null 2>&1; then
  run_step "zizmor (GHA SAST, advisory)" "zizmor --persona=auditor --min-severity=high --no-online-audits .github/workflows/ .gitea/workflows/ || echo 'zizmor returned non-zero (advisory — see findings above)'"
else
  skip_step "zizmor (GHA SAST)" "zizmor not on PATH (install: cargo install zizmor or https://docs.zizmor.sh)"
fi

write_report "success"
post_commit_status "success" "fop local ${profile} passed"

printf '\nlocal CI passed: %s\n' "$report_path"
