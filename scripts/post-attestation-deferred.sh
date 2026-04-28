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
# Usage: bash scripts/post-attestation-deferred.sh [success|failure] [description]
#
# Skips cleanly if `gh` isn't on PATH or no GitHub remote is configured.

set -euo pipefail

state="${1:-success}"
description="${2:-Local-first attestation: lefthook pre-push gates clean}"

if ! command -v gh >/dev/null 2>&1; then
  exit 0
fi

sha="$(git rev-parse HEAD)"
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
      gh api --method POST 'repos/${repo}/statuses/${sha}' \
        -f state='${state}' \
        -f context='fop/local-ci/pr' \
        -f description='${description}' >/dev/null 2>&1 \
        && echo \"\$(date -u +%FT%TZ) posted ${state} on ${sha:0:8}\" \
        || echo \"\$(date -u +%FT%TZ) post failed on ${sha:0:8}\"
      exit 0
    fi
  done
  echo \"\$(date -u +%FT%TZ) timeout waiting for ${sha:0:8} on GitHub\"
" </dev/null >>"$log_file" 2>&1 &

disown
echo "Status post deferred to background poll (log: ${log_file})."
