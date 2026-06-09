#!/usr/bin/env bash
# test-check-local-ci-report.sh — TDD tests for scripts/check-local-ci-report.sh
#
# Tests:
#   (a) nido report accepted   — .nido/reports/local-ci-pr-<sha>.json success
#   (b) fop-only accepted      — only .fop/reports/... present, nido absent
#   (c) neither → fail         — no report at either path

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
script="${repo_root}/scripts/check-local-ci-report.sh"

pass_count=0
fail_count=0

pass() { echo "PASS: $1"; (( pass_count++ )) || true; }
fail() { echo "FAIL: $1"; (( fail_count++ )) || true; }

# Isolated temp workdir so tests don't pollute the real repo.
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

# Minimal git repo in tmpdir so git rev-parse works.
git -C "$tmpdir" init -q
git -C "$tmpdir" commit --allow-empty -m "init" -q

fake_sha="$(git -C "$tmpdir" rev-parse HEAD)"

# Helper: write a JSON report at a given path.
write_report() {
  local path="$1"
  local state="$2"
  mkdir -p "$(dirname "$path")"
  python3 - "$path" "$state" <<'EOF'
import json, sys
path, state = sys.argv[1], sys.argv[2]
with open(path, "w") as f:
    json.dump({"schema": "parkhub.local-ci.v2", "profile": "pr",
               "state": state, "commit": "abc", "context": "nido/local-ci/pr"}, f)
EOF
}

# ─── (a) nido report present and state=success → exit 0 ────────────────────
nido_path="${tmpdir}/.nido/reports/local-ci-pr-${fake_sha}.json"
write_report "$nido_path" "success"

if (cd "$tmpdir" && bash "$script" pr "$fake_sha" >/dev/null 2>&1); then
  pass "(a) nido report accepted"
else
  fail "(a) nido report accepted — script returned non-zero"
fi
rm -f "$nido_path"

# ─── (a2) nido report present but state=failure → exit 1 ───────────────────
write_report "$nido_path" "failure"
if ! (cd "$tmpdir" && bash "$script" pr "$fake_sha" >/dev/null 2>&1); then
  pass "(a2) nido failure report not accepted"
else
  fail "(a2) nido failure report must NOT be accepted"
fi
rm -f "$nido_path"

# ─── (b) fop-only report present → exit 0 ──────────────────────────────────
fop_path="${tmpdir}/.fop/reports/local-ci-pr-${fake_sha}.json"
write_report "$fop_path" "success"

if (cd "$tmpdir" && bash "$script" pr "$fake_sha" >/dev/null 2>&1); then
  pass "(b) fop-only report accepted"
else
  fail "(b) fop-only report accepted — script returned non-zero"
fi
rm -f "$fop_path"

# ─── (c) neither path present → exit 1 ─────────────────────────────────────
if ! (cd "$tmpdir" && bash "$script" pr "$fake_sha" >/dev/null 2>&1); then
  pass "(c) no report → fail"
else
  fail "(c) no report must return non-zero"
fi

# ─── (d) nido takes precedence over fop when both present ───────────────────
write_report "$nido_path" "success"
write_report "$fop_path" "failure"
output="$(cd "$tmpdir" && bash "$script" pr "$fake_sha" 2>&1)"
if echo "$output" | grep -q "nido" && (cd "$tmpdir" && bash "$script" pr "$fake_sha" >/dev/null 2>&1); then
  pass "(d) nido path wins over fop when both present"
else
  fail "(d) nido path must be tried first"
fi
rm -f "$nido_path" "$fop_path"

# ─── summary ────────────────────────────────────────────────────────────────
echo ""
echo "check-local-ci-report tests: ${pass_count} passed, ${fail_count} failed"

if (( fail_count > 0 )); then
  exit 1
fi
