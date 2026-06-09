#!/usr/bin/env bash
# post-attestation-deferred.sh — Fire-and-forget poll-and-post for the
# fop/local-ci/pr commit status.
#
# Background: lefthook's pre-push runs BEFORE the actual `git push`, so
# the commit's SHA isn't on GitHub yet when this script wants to post a
# status against it. github.com would return HTTP 422 "No commit found
# for SHA". This script forks a detached subshell that polls GitHub
# every 5s (up to 2.5min) until the SHA shows up, then posts the status.
# The parent process returns immediately so lefthook isn't blocked.
#
# Usage:
#   bash scripts/post-attestation-deferred.sh [state] [description]
#   bash scripts/post-attestation-deferred.sh --sha <sha> [state] [description]
#
# By default the SHA comes from `git rev-parse HEAD` (the lefthook
# pre-push case). Use `--sha <sha>` to manually re-post against a
# different commit (e.g. when an earlier post failed and the PR is
# stuck on `fop/local-ci/pr` PENDING — see
# `feedback_parkhub_attestation_script_args_2026_05_03.md`).
#
# Skips cleanly if `gh` isn't on PATH or no GitHub remote is configured.

set -euo pipefail

sha=""
if [ "${1:-}" = "--sha" ]; then
  sha="${2:-}"
  if [ -z "$sha" ]; then
    echo "ERROR: --sha requires a SHA argument" >&2
    exit 2
  fi
  shift 2
fi

state="${1:-success}"
description="${2:-Local-first attestation: lefthook pre-push gates clean}"

# Validate state — GitHub statuses API rejects anything outside this set.
case "$state" in
  success|failure|pending|error) ;;
  *)
    echo "ERROR: state must be one of: success failure pending error (got: $state)" >&2
    exit 2
    ;;
esac

if ! command -v gh >/dev/null 2>&1; then
  exit 0
fi

if [ -z "$sha" ]; then
  sha="$(git rev-parse HEAD)"
else
  # Expand short / abbreviated SHAs to the full 40-char form.
  # GitHub's POST /repos/.../statuses/{sha} endpoint rejects anything
  # shorter with HTTP 422 "Sha must be a valid hex object ID", even
  # though GET /commits/{sha} accepts the prefix. Fail loudly if the
  # SHA isn't resolvable locally.
  expanded="$(git rev-parse --verify "${sha}^{commit}" 2>/dev/null || true)"
  if [ -z "$expanded" ]; then
    echo "ERROR: --sha '$sha' is not a valid commit in this repo" >&2
    exit 2
  fi
  sha="$expanded"
fi
repo=""
for remote in github upstream origin; do
  url="$(git remote get-url "$remote" 2>/dev/null || true)"
  if [[ "$url" =~ github.com[:/]([^/]+/[^/.]+)(\.git)?$ ]]; then
    repo="${BASH_REMATCH[1]}"
    break
  fi
done

if [ -z "$repo" ]; then
  exit 0
fi

# Fork the poll-and-post into the background so the caller (lefthook)
# returns immediately. nohup + disown detach from the controlling
# terminal so the process survives the pre-push hook completing.
log_dir="${HOME}/.cache/parkhub-attestation"
mkdir -p "$log_dir"
log_file="${log_dir}/post-${sha:0:8}.log"

nohup bash -c "
  set -uo pipefail
  for i in \$(seq 1 30); do
    sleep 5
    if gh api 'repos/${repo}/commits/${sha}' >/dev/null 2>&1; then
      # Post nido/local-ci/pr (canonical) first.
      gh api --method POST 'repos/${repo}/statuses/${sha}' \
        -f state='${state}' \
        -f context='nido/local-ci/pr' \
        -f description='${description}' >/dev/null 2>&1 \
        && echo \"\$(date -u +%FT%TZ) posted nido ${state} on ${sha:0:8}\" \
        || echo \"\$(date -u +%FT%TZ) nido post failed on ${sha:0:8}\"
      # Post fop/local-ci/pr (compat) so the existing required check keeps
      # passing until branch protection is flipped to nido/local-ci/pr.
      gh api --method POST 'repos/${repo}/statuses/${sha}' \
        -f state='${state}' \
        -f context='fop/local-ci/pr' \
        -f description='${description}' >/dev/null 2>&1 \
        && echo \"\$(date -u +%FT%TZ) posted fop-compat ${state} on ${sha:0:8}\" \
        || echo \"\$(date -u +%FT%TZ) fop-compat post failed on ${sha:0:8}\"
      exit 0
    fi
  done
  echo \"\$(date -u +%FT%TZ) timeout waiting for ${sha:0:8} on GitHub\"
" </dev/null >>"$log_file" 2>&1 &

disown
echo "Status post deferred to background poll (log: ${log_file})."
