#!/usr/bin/env bash
# Local test coverage — runs cargo-llvm-cov for Rust + vitest --coverage
# for the frontend, writes a unified report to .fop/reports/coverage-<sha>/.
# Surfaces line/branch coverage on demand without needing CI artifacts.
#
# Why local + on-demand: per-PR coverage gating is noisy (one new test
# can move the needle by 0.5%); contributors want to see where the gaps
# are when they're working on a slice, not in a delayed CI artifact.
#
# Usage:
#   scripts/local-coverage.sh                # both Rust + frontend
#   scripts/local-coverage.sh --rust-only
#   scripts/local-coverage.sh --frontend-only
#   scripts/local-coverage.sh --html         # also emit HTML report
#
# Skips silently if cargo-llvm-cov / vitest absent.

set -euo pipefail

run_rust=1
run_frontend=1
emit_html=0
for arg in "$@"; do
  case "$arg" in
    --rust-only) run_frontend=0 ;;
    --frontend-only) run_rust=0 ;;
    --html) emit_html=1 ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
done

sha=$(git rev-parse --short HEAD)
out=".fop/reports/coverage-$sha"
mkdir -p "$out"

echo "▶ coverage report → $out/"
echo

# ── Rust coverage via cargo-llvm-cov ─────────────────────────────────
if (( run_rust )); then
  if command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "▶ cargo llvm-cov (parkhub-common + parkhub-server headless)"
    cov_args=(
      llvm-cov --workspace --locked
      --no-default-features --features headless
      --ignore-filename-regex='(parkhub-client|parkhub-desktop|fuzz|tests/common)'
      --json --output-path "$out/rust-llvm-cov.json"
    )
    if (( emit_html )); then
      cov_args+=(--html --output-dir "$out/rust-html")
    fi
    cargo "${cov_args[@]}" 2>&1 | tail -10 || echo "⚠ cargo-llvm-cov surfaced errors"

    if [[ -f "$out/rust-llvm-cov.json" ]]; then
      lines=$(jq '.data[0].totals.lines.percent' < "$out/rust-llvm-cov.json" 2>/dev/null || echo "?")
      branches=$(jq '.data[0].totals.branches.percent // 0' < "$out/rust-llvm-cov.json" 2>/dev/null || echo "?")
      echo "  Rust coverage: lines=${lines}%  branches=${branches}%"
    fi
  else
    echo "⊘ Rust coverage skipped — cargo-llvm-cov not installed"
    echo "  install: cargo install cargo-llvm-cov && rustup component add llvm-tools-preview"
  fi
  echo
fi

# ── Frontend coverage via vitest --coverage ─────────────────────────
if (( run_frontend )); then
  if [[ -d parkhub-web/node_modules/vitest ]]; then
    echo "▶ vitest --coverage (parkhub-web)"
    cov_html_args=()
    (( emit_html )) && cov_html_args+=(--coverage.reporter=html)
    (
      cd parkhub-web
      npx vitest run --coverage \
        --coverage.reporter=json \
        "${cov_html_args[@]}" \
        --coverage.reportsDirectory="../$out/frontend"
    ) 2>&1 | tail -15 || echo "⚠ vitest surfaced errors"

    if [[ -f "$out/frontend/coverage-final.json" ]]; then
      total=$(jq '[.[] | .lines.total] | add' < "$out/frontend/coverage-final.json" 2>/dev/null || echo "?")
      covered=$(jq '[.[] | .lines.covered] | add' < "$out/frontend/coverage-final.json" 2>/dev/null || echo "?")
      if [[ "$total" =~ ^[0-9]+$ ]] && [[ "$total" -gt 0 ]]; then
        pct=$(awk "BEGIN { printf \"%.1f\", $covered * 100 / $total }")
        echo "  Frontend coverage: $covered/$total lines (${pct}%)"
      fi
    fi
  else
    echo "⊘ Frontend coverage skipped — vitest not in parkhub-web/node_modules"
  fi
fi

echo
echo "✓ coverage report written to $out/"
(( emit_html )) && echo "  HTML: $out/{rust-html,frontend}/index.html"
