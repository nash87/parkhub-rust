#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: .github/scripts/fop-local-ci.sh [--profile pr|full|cd] [--dry-run] [--post-status] [--background]

Runs ParkHub's local-first CI through fop's build queue. The optional
--background runs the gate in a detached subshell, logs to .fop/reports/
local-ci-<profile>-<sha>-bg.log, returns immediately. Combine with
--post-status for fire-and-forget background "full" runs that publish
their own commit status context (fop/local-ci/full) when complete.

--post-status flag publishes the commit status context for the selected
profile. The GitHub PR attestation gate expects this exact command:

  .github/scripts/fop-local-ci.sh --profile pr --post-status

Profiles:
  pr    Fast PR gate: format, Rust headless checks, frontend tests/build,
        TypeScript typecheck, generated type drift, and Playwright spec
        compile when E2E files change. Diff-aware: skips Rust steps if no
        .rs/Cargo.{toml,lock} touched, skips frontend if no parkhub-web/
        touched. Set FOP_LOCAL_CI_NO_DIFF_AWARE=1 to force every step.
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
  FOP_LOCAL_CI_RUN_LINTERS=1  run actionlint + yamllint + zizmor + helm-validate
                              even if no workflow / helm chart files touched
  FOP_LOCAL_CI_AUDIT_STRICT=1 fail the gate on any cargo-audit finding (default:
                              advisory; CI enforces strict)
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
background=0

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
    --background)
      background=1
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

# ─── --background: re-exec self in detached subshell, log to file ───────────
# When set, the rest of the run happens in a background subshell so the
# developer's terminal is freed immediately. The post_commit_status mechanism
# fires when the background run completes, posting the result via PAT.
# Caller sees: PID + log path + immediate exit 0.
if (( background )); then
  background=0  # avoid recursion in the re-exec'd child
  repo_root_for_bg="$(git rev-parse --show-toplevel)"
  bg_log_dir="$repo_root_for_bg/.fop/reports"
  mkdir -p "$bg_log_dir"
  bg_sha="$(git rev-parse HEAD)"
  bg_log="$bg_log_dir/local-ci-${profile}-${bg_sha:0:8}-bg.log"
  # Re-build args without --background.
  bg_args=("--profile" "$profile")
  (( dry_run )) && bg_args+=("--dry-run")
  (( post_status )) && bg_args+=("--post-status")
  echo "▶ fop-local-ci backgrounded: profile=$profile log=$bg_log"
  nohup "$0" "${bg_args[@]}" >"$bg_log" 2>&1 < /dev/null &
  bg_pid=$!
  disown 2>/dev/null || true
  echo "  PID=$bg_pid sha=${bg_sha:0:8}"
  echo "  watch: tail -f $bg_log"
  exit 0
fi

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
diff_touch_e2e=0
diff_touch_image=0
diff_touch_design_smoke=0

compute_diff_paths() {
  if [[ "${FOP_LOCAL_CI_NO_DIFF_AWARE:-}" == "1" ]] || [[ "$profile" != "pr" ]]; then
    diff_touch_rust=1
    diff_touch_frontend=1
    diff_touch_ts_export=1
    diff_touch_workflows=1
    diff_touch_php=1
    diff_touch_e2e=1
    diff_touch_design_smoke=1
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
    diff_touch_e2e=1
    diff_touch_design_smoke=1
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
  if grep -qE '^e2e/|^playwright\.config\.(ts|js|mjs|cjs)$' <<<"$diff_paths"; then
    diff_touch_e2e=1
  fi
  if grep -qE '^(parkhub-web/(src/(design-v5|views|components|context|api|lib|styles)/|src/(App|main)\.tsx|e2e/|package(-lock)?\.json|astro\.config\.mjs|playwright\.config\.ts)|e2e/|playwright\.config\.ts)$' <<<"$diff_paths"; then
    diff_touch_design_smoke=1
  fi
  if grep -qE '^(Dockerfile|Containerfile.*|Cargo\.lock|parkhub-web/package-lock\.json)$' <<<"$diff_paths"; then
    diff_touch_image=1
  fi

  printf 'ℹ diff-aware (vs %s): rust=%d frontend=%d ts_export=%d workflows=%d php=%d e2e=%d design_smoke=%d image=%d (%d files)\n' \
    "$base_ref" "$diff_touch_rust" "$diff_touch_frontend" "$diff_touch_ts_export" \
    "$diff_touch_workflows" "$diff_touch_php" "$diff_touch_e2e" "$diff_touch_design_smoke" \
    "$diff_touch_image" "$(wc -l <<<"$diff_paths")"
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
    newest_src="$(find parkhub-web/src parkhub-web/astro.config.* parkhub-web/tsconfig.json -type f -newer "$types_file" 2>/dev/null | head -1 || true)"
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

  # Tolerate "No commit found for SHA" (HTTP 422) — happens when this
  # script runs from the pre-push hook BEFORE the commit has reached
  # GitHub. The local-ci-attestation gate's extended polling window then
  # handles the missing status once the SHA appears on GitHub.
  # gh emits both the JSON body and the "(HTTP 422)" line on stdout
  # (not stderr) so we capture combined stdout for the match.
  local out
  if ! out="$(gh api \
    --method POST \
    "repos/$(status_repo)/statuses/${sha}" \
    -f state="$state" \
    -f context="$context" \
    -f description="$description" 2>&1)"; then
    if echo "$out" | grep -qE "No commit found for SHA|HTTP 422"; then
      echo "Skipping status post — commit ${sha:0:8} not yet on GitHub (will land after push; gate falls back to timeout)." >&2
      return 0
    fi
    echo "$out" >&2
    return 1
  fi
}

write_report() {
  local state="$1"
  local failed_step="${2:-}"
  if [[ "$dry_run" -eq 1 ]]; then
    echo "DRY-RUN: not writing local-ci ${state} report for ${sha:0:8}"
    return 0
  fi
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
    "php": $([[ $diff_touch_php == 1 ]] && echo true || echo false),
    "e2e": $([[ $diff_touch_e2e == 1 ]] && echo true || echo false)
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
run_fop_step() {
  local resource_profile="$1"
  local command="$2"
  local marker="__PARKHUB_FOP_STEP_OK_${RANDOM}_${RANDOM}__"
  local log_file
  local wrapped_command

  log_file="$(mktemp -t parkhub-fop-step.XXXXXX.log)"
  printf -v wrapped_command '%s\nprintf "%%s\\n" "$PARKHUB_FOP_STEP_MARKER"' "$command"

  set +e
  PARKHUB_FOP_STEP_MARKER="$marker" \
    fop build --backend local --resource-profile "$resource_profile" . --preset custom -- \
      bash -euo pipefail -c "$wrapped_command" 2>&1 | tee "$log_file"
  local status=${PIPESTATUS[0]}
  set -e

  if [[ "$status" -ne 0 ]]; then
    rm -f "$log_file"
    return "$status"
  fi

  if ! grep -Fq "$marker" "$log_file"; then
    echo "ERROR: fop build reported success but the inner step completion marker was missing." >&2
    echo "This usually means the wrapped command exited before completion or fop masked its status." >&2
    rm -f "$log_file"
    return 1
  fi

  rm -f "$log_file"
}

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
  if [[ "${FOP_LOCAL_CI_DIRECT:-}" == "1" ]] || ! command -v fop >/dev/null 2>&1; then
    # Direct mode: explicit opt-in OR fop binary not on PATH (GitHub
    # Actions runners, fresh contributor boxes). Kernel + earlyoom
    # handle resource pressure when fop queue is unavailable.
    bash -euo pipefail -c "$command"
    return $?
  fi
  run_fop_step interactive-small "$command"
}

run_step_heavy() {
  local name="$1"
  local command="$2"
  printf '\n==> %s\n' "$name"
  if [[ "$dry_run" -eq 1 ]]; then
    printf 'DRY-RUN: %s\n' "$command"
    return 0
  fi
  if [[ "${FOP_LOCAL_CI_DIRECT:-}" == "1" ]] || ! command -v fop >/dev/null 2>&1; then
    bash -euo pipefail -c "$command"
    return $?
  fi
  run_fop_step batch-medium "$command"
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
run_direct "ui polish contract" "scripts/tests/test-ui-polish-contract.sh"

# ─── Stage 2: workflow + GHA security (when workflows touched) ──────────────
if (( diff_touch_workflows )) || [[ "${FOP_LOCAL_CI_RUN_LINTERS:-}" == "1" ]]; then
  if command -v actionlint >/dev/null 2>&1; then
    run_direct "actionlint" "actionlint -color"
  fi
  if command -v zizmor >/dev/null 2>&1; then
    # Audit-mode: surface findings, don't fail the gate yet
    run_direct "zizmor (audit)" "zizmor --no-progress --persona auditor .github/workflows/ || true"
  fi
  if command -v yamllint >/dev/null 2>&1; then
    # Match GHA ci.yml yamllint scope: docker-compose.yml, render.yaml, koyeb.yaml
    # only check files that exist (silent skip otherwise).
    run_direct "yamllint (compose+deploy manifests)" \
      "for f in docker-compose.yml docker-compose.test.yml render.yaml koyeb.yaml; do \
        [[ -f \$f ]] && yamllint -d 'rules: {line-length: disable, document-start: disable, truthy: disable}' \"\$f\"; \
      done"
  fi
  # fop ci-audit: gitea+github workflow audit (missing --add-host, hardcoded
  # localhost, etc). Advisory — surfaces findings but doesn't fail the gate.
  if command -v fop >/dev/null 2>&1; then
    run_direct "fop ci-audit (advisory)" \
      "fop ci-audit . 2>&1 | grep -E '\\[(WARN|ERROR|FAIL)\\]' | head -30 || echo '✓ fop ci-audit: no findings'"
  fi
  # workflow drift detector — gitea ↔ github workflow file pairing.
  # Gating: missing files fail the gate; trigger/job drift is advisory.
  if [[ -x scripts/local-workflow-drift.sh ]]; then
    run_direct "workflow drift (gitea ↔ github)" "./scripts/local-workflow-drift.sh"
  fi
fi

# ─── Stage 2b: spell-check via typos (advisory, all profiles) ───────────────
# Mirrors security.yml typos job (crate-ci/typos action). MIT licensed.
# Advisory: surfaces likely typos but doesn't fail the gate. Common in commit
# messages, comments, README; less common in code identifiers (those are caught
# by clippy/tsc).
if command -v typos >/dev/null 2>&1; then
  run_direct "typos (spell-check, advisory)" \
    "typos --color=always 2>&1 | head -50 || echo 'typos found likely typos (advisory — see above)'"
else
  skip_step "typos (spell-check)" "typos not on PATH (install: cargo install typos-cli)"
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
  run_step "frontend test and build" "cd parkhub-web && CI=true npm test && CI=true npm run build"
else
  skip_step "frontend typecheck" "diff-aware: parkhub-web/ untouched"
  skip_step "frontend test and build" "diff-aware: parkhub-web/ untouched"
fi

# ─── Stage 4b: Playwright specs (compile-only when E2E harness changed) ─────
if (( diff_touch_e2e )); then
  run_step "playwright spec list" "CI=true npx playwright test --list"
else
  skip_step "playwright spec list" "diff-aware: e2e/ untouched"
fi

# ─── Stage 4c: Route + design-system smoke (blocking when app UI changed) ───
if (( diff_touch_design_smoke )); then
  run_step_heavy "frontend route + v5 design smoke" "FOP_LOCAL_CI_DIRECT=1 ./scripts/v5-design-smoke-local.sh"
else
  skip_step "frontend route + v5 design smoke" "diff-aware: no route/design/e2e files touched"
fi

# ─── Stage 5: TypeScript bindings drift (Rust→TS contract; skip if both untouched) ───
if (( diff_touch_ts_export )); then
  run_step_heavy "typescript bindings drift" "mkdir -p parkhub-web/dist && printf '%s' '<!doctype html><html><body></body></html>' > parkhub-web/dist/index.html && cargo test --locked --features gen-types -p parkhub-server --test ts_export -- --nocapture && git diff --exit-code parkhub-web/src/generated/ && test -z \"\$(git status --porcelain parkhub-web/src/generated/)\""
else
  skip_step "typescript bindings drift" "diff-aware: no Rust + no parkhub-web/src/generated/ touched"
fi

# ─── Stage 6: full profile extras (always full when invoked) ─────────────────
if [[ "$profile" == "full" ]]; then
  run_step_heavy "openapi drift" "cd parkhub-web && CI=true npm run build && cd .. && cargo build --locked --release -p parkhub-server --no-default-features --features 'full,headless' && target_dir=\"\$(cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory)\" && server_bin=\"\$target_dir/release/parkhub-server\" && pid=''; cleanup() { if [[ -n \"\${pid:-}\" ]]; then kill \"\$pid\" 2>/dev/null || true; fi; }; trap cleanup EXIT; mkdir -p /tmp/parkhub-drift-db && { \"\$server_bin\" --headless --unattended --port 18181 --data-dir /tmp/parkhub-drift-db >/tmp/parkhub-drift.log 2>&1 & pid=\$!; }; for i in \$(seq 1 45); do curl -sf http://localhost:18181/health >/dev/null 2>&1 && break; sleep 1; done; ./scripts/dump-openapi.sh 18181; git diff --exit-code docs/openapi/rust.json"
  run_step_heavy "playwright chromium" "cd parkhub-web && CI=true npm run build && cd .. && cargo build --locked --release -p parkhub-server --no-default-features --features 'full,headless,e2e-bypass' && target_dir=\"\$(cargo metadata --locked --no-deps --format-version 1 | jq -r .target_directory)\" && server_bin=\"\$target_dir/release/parkhub-server\" && pid=''; cleanup() { if [[ -n \"\${pid:-}\" ]]; then kill \"\$pid\" 2>/dev/null || true; fi; }; trap cleanup EXIT; { DEMO_MODE=true PARKHUB_ADMIN_PASSWORD=demo PARKHUB_DISABLE_RATE_LIMITS=true \"\$server_bin\" --headless --unattended --port 8081 >/tmp/parkhub-e2e.log 2>&1 & pid=\$!; }; for i in \$(seq 1 45); do curl -sf http://localhost:8081/health >/dev/null 2>&1 && break; sleep 1; done; npx playwright test --project=chromium"
fi

# ─── Stage 6b: Helm chart validation (full+cd profiles or helm/ touched) ────
# Mirrors .github/workflows/ci.yml `helm-validate` job lines 45-70: lint +
# template renders for the 4 chart-value variants, each piped to a YAML parser
# to catch silent rendering bugs. Skips silently if helm not on PATH (per the
# `pr` profile contributor-friendly pattern).
helm_required=0
[[ "$profile" == "cd" || "$profile" == "full" ]] && helm_required=1
helm_should_run=0
[[ $helm_required -eq 1 ]] && helm_should_run=1
if (( diff_touch_workflows )) || [[ "${FOP_LOCAL_CI_RUN_LINTERS:-}" == "1" ]]; then
  helm_should_run=1
fi
if [[ -d helm/parkhub ]] && (( helm_should_run )); then
  if command -v helm >/dev/null 2>&1; then
    run_step "helm lint (parkhub chart)" "helm lint ./helm/parkhub"
    # 4 template variants matching ci.yml: default + grafana + ha + servicemonitor
    run_step "helm template (default)" \
      "helm template parkhub ./helm/parkhub | python3 -c 'import sys,yaml; list(yaml.safe_load_all(sys.stdin))'"
    if [[ -f helm/parkhub/values-grafana.yaml ]]; then
      run_step "helm template (grafana)" \
        "helm template parkhub ./helm/parkhub -f helm/parkhub/values-grafana.yaml | python3 -c 'import sys,yaml; list(yaml.safe_load_all(sys.stdin))'"
    fi
    if [[ -f helm/parkhub/values-ha.yaml ]]; then
      run_step "helm template (ha)" \
        "helm template parkhub ./helm/parkhub -f helm/parkhub/values-ha.yaml | python3 -c 'import sys,yaml; list(yaml.safe_load_all(sys.stdin))'"
    fi
    if [[ -f helm/parkhub/values-servicemonitor.yaml ]]; then
      run_step "helm template (servicemonitor)" \
        "helm template parkhub ./helm/parkhub -f helm/parkhub/values-servicemonitor.yaml | python3 -c 'import sys,yaml; list(yaml.safe_load_all(sys.stdin))'"
    fi
  elif (( helm_required )); then
    echo "✗ helm chart validation FAILED: helm not on PATH (required for ${profile} profile)" >&2
    write_report "failure" "helm chart validation"
    post_commit_status "failure" "fop local ${profile} failed: helm not installed"
    exit 1
  else
    skip_step "helm chart validation" "helm not on PATH (install: https://helm.sh/docs/intro/install/)"
  fi
fi

# ─── Stage 6c: cargo audit (RustSec) + cargo-geiger (unsafe SAST) ───────────
# Mirrors .github/workflows/security.yml cargo-audit + cargo-geiger jobs.
# cargo audit: gating on full+cd profiles; same RUSTSEC ignore list as the
# workflow (kept in sync via deny.toml — cargo-audit reads its own DB but
# respects --ignore CLI flags).
# cargo-geiger: advisory only (unsafe-block trend tracking, not a hard gate).
if [[ "$profile" == "cd" || "$profile" == "full" ]]; then
  if command -v cargo-audit >/dev/null 2>&1; then
    # --deny warnings matches GHA. The transitive Slint/Tauri RUSTSEC ignores
    # live in deny.toml; cargo-audit respects those via --ignore CLI on each
    # crate ID. For local runs we accept warnings as advisory unless
    # FOP_LOCAL_CI_AUDIT_STRICT=1 is set (CI enforces strictly).
    if [[ "${FOP_LOCAL_CI_AUDIT_STRICT:-}" == "1" ]]; then
      run_step "cargo audit (RustSec, strict)" "cargo audit --deny warnings"
    else
      run_step "cargo audit (RustSec, advisory)" "cargo audit || echo 'cargo-audit found advisories (advisory — see above)'"
    fi
  else
    skip_step "cargo audit" "cargo-audit not installed (cargo install cargo-audit)"
  fi
  if command -v cargo-geiger >/dev/null 2>&1; then
    # Advisory: scans for `unsafe` blocks across the dep tree. Use parkhub-server
    # as the entry point (workspace root has multiple targets).
    run_step "cargo-geiger (unsafe SAST, advisory)" \
      "cargo geiger --quiet --manifest-path parkhub-server/Cargo.toml --no-default-features --features headless 2>&1 | tail -30 || echo 'cargo-geiger advisory output above'"
  else
    skip_step "cargo-geiger" "cargo-geiger not installed (cargo install cargo-geiger)"
  fi
fi

# ─── Stage 6d: dep hygiene (cargo machete + cargo sort, full+cd profiles) ───
# cargo-machete: detects unused dependencies in Cargo.toml (catches deps
# added during a PR but not actually `use`d, reducing build time + supply-
# chain surface). MIT licensed.
# cargo-sort: keeps Cargo.toml dep tables sorted alphabetically (catches
# cosmetic drift; advisory).
if [[ "$profile" == "cd" || "$profile" == "full" ]]; then
  if command -v cargo-machete >/dev/null 2>&1; then
    # Gating — unused deps are real bloat. --skip-target-dir avoids false
    # positives from the build cache.
    run_step "cargo-machete (unused deps)" \
      "cargo machete --skip-target-dir 2>&1 | tail -30"
  else
    skip_step "cargo-machete" "cargo-machete not installed (cargo install cargo-machete)"
  fi
  if command -v cargo-sort >/dev/null 2>&1; then
    # Advisory: cosmetic — dep tables alphabetized.
    run_step "cargo-sort (Cargo.toml hygiene, advisory)" \
      "cargo sort --check --workspace 2>&1 | tail -20 || echo 'cargo-sort suggested re-sorting Cargo.toml (advisory — run: cargo sort --workspace)'"
  else
    skip_step "cargo-sort" "cargo-sort not installed (cargo install cargo-sort)"
  fi
fi

# ─── Stage 7: cd profile extras ──────────────────────────────────────────────
if [[ "$profile" == "cd" ]]; then
  run_step_heavy "release image preflight" "cargo test --locked --package parkhub-common --all-targets && cargo test --locked --package parkhub-server --no-default-features --features headless --all-targets"
fi

# ─── Stage 7b: container image vulnerability scan (cd, or Dockerfile touched on full) ───
# Mirrors .github/workflows/security.yml trivy-image + grype-image jobs (which
# only run post-publish on main). Locally we build a fresh image and scan it.
# Stamp-cached on Dockerfile + Cargo.lock + parkhub-web/package-lock.json SHA
# so we don't rebuild across pushes that don't touch image inputs.
image_scan_should_run=0
[[ "$profile" == "cd" ]] && image_scan_should_run=1
[[ "$profile" == "full" ]] && (( diff_touch_image )) && image_scan_should_run=1
if (( image_scan_should_run )); then
  if [[ -x scripts/local-image-scan.sh ]]; then
    # The script gracefully skips if podman/trivy/grype is missing, so we
    # don't need to gate on tool presence here.
    run_step_heavy "container image scan (build + trivy image + grype)" "./scripts/local-image-scan.sh"
  else
    skip_step "container image scan" "scripts/local-image-scan.sh missing"
  fi
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

# ─── Stage 10: OSV-Scanner (supply-chain via OSV database) ──────────────────
# OSV-Scanner v2 (Apache-2.0, Google) reads Cargo.lock + package-lock.json
# directly and matches against the OSV database (broader than RUSTSEC alone:
# also catches GHSA + CVE entries). Complements cargo-deny advisories on the
# Rust side and npm audit on the frontend.
# Advisory mode: known transitive advisories are documented in deny.toml;
# OSV-Scanner findings are surfaced informationally and do NOT fail the gate.
# Promote to gating once an osv-scanner.toml ignore list mirrors deny.toml.
if command -v osv-scanner >/dev/null 2>&1; then
  # osv-scanner.toml at repo root mirrors the deny.toml advisory ignore list,
  # so this step is now gating (failure = real vuln, not a documented one).
  run_step "osv-scanner (supply-chain)" "osv-scanner scan source --recursive --config=osv-scanner.toml ."
else
  skip_step "osv-scanner" "osv-scanner not on PATH (install: https://google.github.io/osv-scanner/installation/)"
fi

# ─── Stage 7c: SBOM generation (cd profile only) ────────────────────────────
# Mirrors the syft step in .github/workflows/docker-publish.yml. Generates
# SPDX-JSON SBOM into .fop/reports/. Optional cosign sign via FOP_LOCAL_SBOM_SIGN_KEY.
if [[ "$profile" == "cd" ]]; then
  if [[ -x scripts/local-sbom.sh ]]; then
    run_step "SBOM generation (syft → SPDX-JSON)" "./scripts/local-sbom.sh"
  else
    skip_step "SBOM generation" "scripts/local-sbom.sh missing"
  fi
fi

# ─── Stage 10b: Lighthouse CI (perf + a11y; full+cd profiles, frontend touched) ───
# Mirrors .github/workflows/lighthouse.yml. Skipped if MemAvailable < 6 GiB
# (Lighthouse + headless Chromium need ~3 GB; the script enforces the floor).
# The lighthouserc.json now asserts INP threshold (#510), so this catches
# perf regressions before they ship.
lh_should_run=0
[[ "$profile" == "cd" ]] && lh_should_run=1
[[ "$profile" == "full" ]] && (( diff_touch_frontend )) && lh_should_run=1
if (( lh_should_run )); then
  if [[ -x scripts/local-lighthouse.sh ]]; then
    # Heavy step — queue through fop build batch-medium.
    run_step_heavy "lighthouse CI (perf + a11y + INP threshold)" "./scripts/local-lighthouse.sh"
  else
    skip_step "lighthouse CI" "scripts/local-lighthouse.sh missing"
  fi
fi

# ─── Stage 11: Grype (vuln scanner, defense-in-depth) ───────────────────────
# Grype (Apache-2.0, Anchore) is a complementary vuln scanner to Trivy.
# Different DB sources catch different findings — defense-in-depth on the
# supply chain. Advisory only on `cd` profile (release path); skipped on `pr`.
if [[ "$profile" == "cd" ]] && command -v grype >/dev/null 2>&1; then
  run_step "grype (defense-in-depth, advisory)" "grype dir:. --fail-on critical --quiet 2>&1 | tail -20 || echo 'grype found vulns (advisory)'"
elif [[ "$profile" == "cd" ]]; then
  skip_step "grype" "grype not on PATH (install: https://github.com/anchore/grype#installation)"
fi

if [[ "$dry_run" -eq 1 ]]; then
  printf '\ndry-run local CI completed; no success report or commit status was written.\n'
else
  write_report "success"
  post_commit_status "success" "fop local ${profile} passed"

  printf '\nlocal CI passed: %s\n' "$report_path"
fi
