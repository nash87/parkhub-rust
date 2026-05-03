#!/usr/bin/env bash
# Local container image vulnerability scan — mirrors the GHA security.yml
# trivy-image + grype-image jobs (which only run post-publish on main).
#
# Mechanism:
#   1. Compute a stable input hash from Dockerfile + Cargo.lock +
#      parkhub-web/package-lock.json.
#   2. If `.fop/image-scan-validated-<hash>.stamp` exists, skip (cached).
#   3. Otherwise: podman build → trivy image → grype image, fail on critical.
#   4. On success, write the stamp.
#
# Usage:
#   scripts/local-image-scan.sh [--force] [--no-cache]
#
# Env:
#   FOP_IMAGE_SCAN_TAG     image tag (default: parkhub-local:ci-<hash[0:10]>)
#   FOP_IMAGE_SCAN_DIR     stamp dir (default: .fop)
#
# Skips gracefully if podman, trivy, or grype is missing on PATH.

set -euo pipefail

force=0
no_cache=""
for arg in "$@"; do
  case "$arg" in
    --force) force=1 ;;
    --no-cache) no_cache="--no-cache" ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) echo "Unknown arg: $arg" >&2; exit 2 ;;
  esac
done

# Compute input hash deterministically across the 3 files that drive image
# content. If any is missing, the missing component is hashed as empty so the
# stamp varies when a file appears later.
hash_input() {
  for f in Dockerfile Cargo.lock parkhub-web/package-lock.json; do
    if [[ -f "$f" ]]; then
      sha256sum "$f"
    else
      echo "0000000000000000000000000000000000000000000000000000000000000000  $f (missing)"
    fi
  done | sha256sum | awk '{print $1}'
}

input_hash=$(hash_input)
short_hash=${input_hash:0:10}
stamp_dir="${FOP_IMAGE_SCAN_DIR:-.fop}"
stamp_file="$stamp_dir/image-scan-validated-$short_hash.stamp"
image_tag="${FOP_IMAGE_SCAN_TAG:-parkhub-local:ci-$short_hash}"

if [[ -f "$stamp_file" ]] && (( ! force )); then
  echo "✓ image scan skipped (stamp present: $stamp_file)"
  echo "  image-input hash: $input_hash"
  exit 0
fi

# Tool availability — skip stage entirely if podman missing (most common case
# in CI runners that already validated images upstream).
need_podman=0
need_trivy=0
need_grype=0
command -v podman >/dev/null 2>&1 || need_podman=1
command -v trivy >/dev/null 2>&1 || need_trivy=1
command -v grype >/dev/null 2>&1 || need_grype=1

if (( need_podman + need_trivy + need_grype > 0 )); then
  echo "⊘ image scan skipped — missing tools:"
  (( need_podman )) && echo "    podman not on PATH (install: dnf install podman)"
  (( need_trivy )) && echo "    trivy not on PATH (install: https://aquasecurity.github.io/trivy/)"
  (( need_grype )) && echo "    grype not on PATH (install: https://github.com/anchore/grype#installation)"
  exit 0
fi

mkdir -p "$stamp_dir"

# Build the image. --network=host is required under Bazzite Flatpak sandbox
# (Podman default rootless networking conflicts with the sandbox).
echo "▶ podman build $image_tag (input hash $short_hash)"
podman build $no_cache --network=host -t "$image_tag" . >&2

# Trivy image scan — same severity + ignorefile as the GHA workflow.
echo "▶ trivy image $image_tag (CRITICAL,HIGH; .trivyignore filters applied)"
trivy_args=(
  image --quiet
  --scanners=vuln,secret
  --severity=CRITICAL,HIGH
  --exit-code=1
)
[[ -f .trivyignore ]] && trivy_args+=(--ignorefile .trivyignore)
trivy "${trivy_args[@]}" "$image_tag"

# Grype image scan — defense-in-depth, fail-on critical.
echo "▶ grype $image_tag (defense-in-depth, fail-on critical)"
grype "$image_tag" --fail-on critical --quiet

# All checks passed — write stamp.
{
  echo "image-input hash: $input_hash"
  echo "image tag:        $image_tag"
  echo "validated at:     $(date -u +%FT%TZ)"
  echo "trivy version:    $(trivy --version 2>&1 | head -1)"
  echo "grype version:    $(grype version 2>&1 | grep -i ^version | head -1)"
} > "$stamp_file"

echo "✓ image scan passed; stamp written to $stamp_file"
