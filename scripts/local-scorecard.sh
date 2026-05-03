#!/usr/bin/env bash
# Local OSSF Scorecard runner — mirrors .github/workflows/scorecard.yml.
# Uses the public scorecard container so contributors can see the same
# repository-security score that GHA reports without waiting for the weekly
# scheduled run.
#
# Requires: podman (or docker), GH_TOKEN env var with read:repo + read:org
# permissions on the parkhub-rust repo. Without GH_TOKEN, scorecard's GitHub
# probes (branch protection, dependency-review, etc.) return reduced detail.
#
# Usage:
#   GH_TOKEN=$(gh auth token) scripts/local-scorecard.sh
#   scripts/local-scorecard.sh --repo nash87/parkhub-rust  # explicit repo

set -euo pipefail

repo=""
for arg in "$@"; do
  case "$arg" in
    --repo) shift; repo="$1" ;;
    --repo=*) repo="${arg#--repo=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

# Default to the canonical github remote if not provided.
if [[ -z "$repo" ]]; then
  if git remote get-url github >/dev/null 2>&1; then
    url=$(git remote get-url github)
    # Convert git@github.com:nash87/parkhub-rust.git → nash87/parkhub-rust
    repo=$(printf '%s' "$url" | sed -E 's#(git@github\.com:|https://github\.com/)##; s#\.git$##')
  else
    echo "✗ no --repo and no github remote configured" >&2
    exit 2
  fi
fi

# Container runtime — prefer podman.
runtime=""
if command -v podman >/dev/null 2>&1; then
  runtime=podman
elif command -v docker >/dev/null 2>&1; then
  runtime=docker
else
  echo "⊘ scorecard skipped — neither podman nor docker on PATH"
  exit 0
fi

# GH_TOKEN passed through; scorecard reads it from env.
if [[ -z "${GH_TOKEN:-}" ]]; then
  if command -v gh >/dev/null 2>&1; then
    GH_TOKEN=$(gh auth token 2>/dev/null || true)
  fi
fi
if [[ -z "${GH_TOKEN:-}" ]]; then
  echo "⊘ scorecard skipped — GH_TOKEN not set (run: GH_TOKEN=\$(gh auth token) $0)"
  exit 0
fi

echo "▶ scorecard for github.com/$repo (container: $runtime)"
"$runtime" run --rm \
  -e GITHUB_AUTH_TOKEN="$GH_TOKEN" \
  --network=host \
  gcr.io/openssf/scorecard:stable \
  --repo="github.com/$repo" \
  --format=json \
  > ".fop/reports/scorecard-$(git rev-parse --short HEAD).json"

# Pretty-print the score summary.
jq -r '.scorecard.commit as $sha
       | "score: \(.score) (\($sha[0:8]))"
       , (.checks[] | "\(.score|tostring|.[0:5]) \(.name) — \(.reason // "ok")")' \
   < ".fop/reports/scorecard-$(git rev-parse --short HEAD).json" 2>/dev/null \
   || echo "(scorecard output written; jq not available for pretty-print)"
