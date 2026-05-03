#!/usr/bin/env bash
# Workflow drift detector — flags when .gitea/workflows/* diverges from
# .github/workflows/* in ways that aren't documented as intentional splits.
#
# Why: parkhub-rust ships BOTH .gitea/workflows (gitea actions runner) AND
# .github/workflows (GitHub Actions). Some files are intentionally split
# (e.g. ci.yml carries fop attestation pilot in gitea while ci.yaml mirrors
# the github source). Most should be kept in sync.
#
# What this checks:
#   1. Every file in .github/workflows/<NAME>.yml has a matching .gitea/
#      workflows/<NAME>.yml or .yaml (or is in the EXEMPT list).
#   2. For each matched pair: same `on:` triggers, same `jobs:` keys.
#      Step bodies are NOT compared (they intentionally differ —
#      gitea uses local action mirrors, github uses public).
#
# Usage:
#   scripts/local-workflow-drift.sh [--strict]
#
# --strict treats EXEMPT-listed files as still-required (no exemption).

set -euo pipefail

strict=0
[[ "${1:-}" == "--strict" ]] && strict=1

# Files that are documented as github-only (no gitea mirror needed) — these
# typically depend on github-specific infrastructure (Codespaces, Copilot bot,
# Dependabot, OSSF Scorecard, dependency-review API, etc).
EXEMPT_GITEA_MISSING=(
  copilot-setup-steps.yml         # Codespaces/Copilot setup
  dependabot-auto-merge.yml       # Dependabot bot
  dependency-review.yml           # GitHub Dependency Graph API
  scorecard.yml                   # OSSF Scorecard publishes to github API
  labeler.yml                     # GitHub PR labels
  changelog.yml                   # PR-merge driven, github webhook
  release.yml                     # Cosign keyless OIDC needs github OIDC
  release-rehearsal.yml           # mirrors release.yml constraints
  cosign-verify.yml               # post-publish verification on github registry
  devcontainer-publish.yml        # publishes to ghcr.io
  auto-merge.yml                  # github-only auto-merge of dependabot PRs
  deploy.yml                      # github-only Render/Fly/Koyeb dispatch
  tauri-mobile.yml                # macOS+Windows runners for Tauri builds
)

# Files that are documented as gitea-only (no github mirror needed).
EXEMPT_GITHUB_MISSING=(
  storybook.yaml                  # gitea-runner internal Storybook publish
)

contains() {
  local needle="$1"; shift
  for hay in "$@"; do
    [[ "$hay" == "$needle" ]] && return 0
  done
  return 1
}

# Collect file lists.
mapfile -t gh_files < <(find .github/workflows -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -exec basename {} \; 2>/dev/null | sort -u)
mapfile -t gt_files < <(find .gitea/workflows -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) -exec basename {} \; 2>/dev/null | sort -u)

# Normalize names to compare (drop extension).
norm() { printf '%s' "$1" | sed -E 's/\.(yml|yaml)$//'; }

declare -a missing_in_gitea=()
declare -a missing_in_github=()
declare -a trigger_drift=()
declare -a job_drift=()

for f in "${gh_files[@]}"; do
  base=$(norm "$f")
  found=0
  for g in "${gt_files[@]}"; do
    [[ "$(norm "$g")" == "$base" ]] && { found=1; break; }
  done
  if (( ! found )); then
    if (( strict )) || ! contains "$f" "${EXEMPT_GITEA_MISSING[@]}"; then
      missing_in_gitea+=("$f")
    fi
  fi
done

for f in "${gt_files[@]}"; do
  base=$(norm "$f")
  found=0
  for g in "${gh_files[@]}"; do
    [[ "$(norm "$g")" == "$base" ]] && { found=1; break; }
  done
  if (( ! found )); then
    if (( strict )) || ! contains "$f" "${EXEMPT_GITHUB_MISSING[@]}"; then
      missing_in_github+=("$f")
    fi
  fi
done

# Compare matched pairs.
for f in "${gh_files[@]}"; do
  base=$(norm "$f")
  gh_path=".github/workflows/$f"
  gt_path=""
  for g in "${gt_files[@]}"; do
    if [[ "$(norm "$g")" == "$base" ]]; then
      gt_path=".gitea/workflows/$g"
      break
    fi
  done
  [[ -z "$gt_path" ]] && continue

  if ! command -v python3 >/dev/null 2>&1; then
    continue
  fi

  drift=$(python3 - "$gh_path" "$gt_path" <<'PY' 2>/dev/null || true
import sys, yaml
gh, gt = sys.argv[1], sys.argv[2]
with open(gh) as f: gh_y = yaml.safe_load(f)
with open(gt) as f: gt_y = yaml.safe_load(f)
gh_on = sorted((gh_y.get('on') or gh_y.get(True) or {}).keys()) if isinstance(gh_y.get('on') or gh_y.get(True), dict) else []
gt_on = sorted((gt_y.get('on') or gt_y.get(True) or {}).keys()) if isinstance(gt_y.get('on') or gt_y.get(True), dict) else []
if gh_on != gt_on:
    print(f"on:gh={gh_on} on:gt={gt_on}")
gh_jobs = sorted((gh_y.get('jobs') or {}).keys())
gt_jobs = sorted((gt_y.get('jobs') or {}).keys())
gh_only = [j for j in gh_jobs if j not in gt_jobs]
gt_only = [j for j in gt_jobs if j not in gh_jobs]
if gh_only or gt_only:
    print(f"jobs:gh-only={gh_only} jobs:gt-only={gt_only}")
PY
)
  if [[ -n "$drift" ]]; then
    if [[ "$drift" == on:* ]]; then
      trigger_drift+=("$base: $drift")
    else
      job_drift+=("$base: $drift")
    fi
  fi
done

# Report.
exit_code=0
total_drift=$((${#missing_in_gitea[@]} + ${#missing_in_github[@]} + ${#trigger_drift[@]} + ${#job_drift[@]}))

if (( total_drift == 0 )); then
  echo "✓ no workflow drift detected (gh:${#gh_files[@]} files, gt:${#gt_files[@]} files)"
  exit 0
fi

if (( ${#missing_in_gitea[@]} > 0 )); then
  echo "✗ in .github/workflows but missing from .gitea/workflows:"
  for f in "${missing_in_gitea[@]}"; do echo "    $f"; done
  exit_code=1
fi

if (( ${#missing_in_github[@]} > 0 )); then
  echo "✗ in .gitea/workflows but missing from .github/workflows:"
  for f in "${missing_in_github[@]}"; do echo "    $f"; done
  exit_code=1
fi

if (( ${#trigger_drift[@]} > 0 )); then
  echo "⚠ trigger drift (different on: keys):"
  for d in "${trigger_drift[@]}"; do echo "    $d"; done
  # Trigger drift is advisory — gitea + github often diverge here intentionally.
fi

if (( ${#job_drift[@]} > 0 )); then
  echo "⚠ job drift (different jobs.* keys):"
  for d in "${job_drift[@]}"; do echo "    $d"; done
fi

echo
echo "Total drift: ${total_drift}"
echo "(missing files are gating; trigger/job drift is advisory)"
exit $exit_code
