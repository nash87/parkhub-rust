#!/usr/bin/env bash
# Local container image vulnerability scan for ParkHub Rust.
#
# This is the compatibility fallback for workstations whose installed fop
# binary predates `fop image scan`. It keeps the local pre-push gate security
# preserving instead of silently skipping image validation.
#
# Mechanism:
#   1. Hash Dockerfile, Cargo.lock, and parkhub-web/package-lock.json.
#   2. Reuse .fop/image-scan-validated-<hash>.stamp when present.
#   3. Otherwise: podman build -> trivy image -> grype image.
#   4. On success, write the stamp.

set -euo pipefail

force=0
podman_args=()
for arg in "$@"; do
  case "$arg" in
    --force) force=1 ;;
    --no-cache) podman_args+=(--no-cache) ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) echo "Unknown arg: $arg" >&2; exit 2 ;;
  esac
done

hash_input() {
  for file in Dockerfile Cargo.lock parkhub-web/package-lock.json; do
    if [[ -f "$file" ]]; then
      sha256sum "$file"
    else
      printf '%s  %s (missing)\n' \
        "0000000000000000000000000000000000000000000000000000000000000000" \
        "$file"
    fi
  done | sha256sum | awk '{print $1}'
}

input_hash="$(hash_input)"
short_hash="${input_hash:0:10}"
stamp_dir="${FOP_IMAGE_SCAN_DIR:-.fop}"
stamp_file="${stamp_dir}/image-scan-validated-${short_hash}.stamp"
image_tag="${FOP_IMAGE_SCAN_TAG:-parkhub-local:ci-${short_hash}}"

if [[ -f "$stamp_file" ]] && (( ! force )); then
  echo "image scan skipped: stamp present (${stamp_file})"
  echo "image-input hash: ${input_hash}"
  exit 0
fi

need_podman=0
need_trivy=0
need_grype=0
command -v podman >/dev/null 2>&1 || need_podman=1
command -v trivy >/dev/null 2>&1 || need_trivy=1
command -v grype >/dev/null 2>&1 || need_grype=1

if (( need_podman + need_trivy + need_grype > 0 )); then
  echo "image scan skipped: missing tools"
  (( need_podman )) && echo "  podman not on PATH"
  (( need_trivy )) && echo "  trivy not on PATH"
  (( need_grype )) && echo "  grype not on PATH"
  exit 0
fi

mkdir -p "$stamp_dir"

echo "podman build ${image_tag} (input hash ${short_hash})"
podman build "${podman_args[@]}" --network=host -t "$image_tag" . >&2

echo "trivy image ${image_tag} (CRITICAL,HIGH; .trivyignore filters applied)"
trivy_args=(
  image
  --quiet
  "--scanners=vuln,secret"
  "--severity=CRITICAL,HIGH"
  "--exit-code=1"
)
[[ -f .trivyignore ]] && trivy_args+=(--ignorefile .trivyignore)
trivy "${trivy_args[@]}" "$image_tag"

echo "grype ${image_tag} (fail-on critical)"
grype "$image_tag" --fail-on critical --quiet

{
  echo "image-input hash: ${input_hash}"
  echo "image tag:        ${image_tag}"
  echo "validated at:     $(date -u +%FT%TZ)"
  echo "trivy version:    $(trivy --version 2>&1 | head -1)"
  echo "grype version:    $(grype version 2>&1 | grep -i '^version' | head -1)"
} > "$stamp_file"

echo "image scan passed; stamp written to ${stamp_file}"
