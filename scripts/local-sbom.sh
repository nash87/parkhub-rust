#!/usr/bin/env bash
# Local SBOM generation — mirrors the syft step in
# .github/workflows/docker-publish.yml (which currently runs only on the
# release path). Locally generates an SPDX-JSON SBOM for the Rust + npm
# dependency tree so contributors can see what ships with a given commit
# without waiting for the release pipeline.
#
# Optional: if FOP_LOCAL_SBOM_SIGN_KEY is set to a cosign keypair path,
# signs the SBOM with `cosign sign-blob`. CI keyless OIDC is intentionally
# NOT available locally (no Sigstore OIDC issuer for workstations) so the
# signing step here is opt-in via a developer-key path.
#
# Usage:
#   scripts/local-sbom.sh [--out PATH]
#
# Env:
#   FOP_LOCAL_SBOM_SIGN_KEY  Path to cosign private key (optional, opt-in)
#   FOP_LOCAL_SBOM_SIGN_PWD  Password for that key (passed via env to cosign)

set -euo pipefail

out="${PWD}/.fop/reports/sbom-spdx-$(git rev-parse --short HEAD).json"
for arg in "$@"; do
  case "$arg" in
    --out) shift; out="$1" ;;
    --out=*) out="${arg#--out=}" ;;
    -h|--help) sed -n '2,/^$/p' "$0" | sed 's/^# \?//'; exit 0 ;;
  esac
  shift 2>/dev/null || true
done

if ! command -v syft >/dev/null 2>&1; then
  echo "⊘ SBOM skipped — syft not on PATH (install: https://github.com/anchore/syft#installation)"
  exit 0
fi

mkdir -p "$(dirname "$out")"

echo "▶ syft scan (Cargo.lock + parkhub-web/package-lock.json + filesystem)"
syft scan dir:. \
  -o "spdx-json=$out" \
  --exclude='./target/**' \
  --exclude='./node_modules/**' \
  --exclude='./parkhub-web/node_modules/**' \
  --exclude='./.fop/**' \
  --exclude='./.claude/worktrees/**' \
  --quiet

echo "✓ SBOM written: $out"
echo "  components: $(jq '.packages | length' < "$out" 2>/dev/null || echo '?')"

# Optional sign step.
if [[ -n "${FOP_LOCAL_SBOM_SIGN_KEY:-}" ]]; then
  if ! command -v cosign >/dev/null 2>&1; then
    echo "⊘ SBOM sign skipped — cosign not on PATH"
    exit 0
  fi
  if [[ ! -f "${FOP_LOCAL_SBOM_SIGN_KEY}" ]]; then
    echo "✗ SBOM sign FAILED — key not found: ${FOP_LOCAL_SBOM_SIGN_KEY}" >&2
    exit 1
  fi
  echo "▶ cosign sign-blob (key: ${FOP_LOCAL_SBOM_SIGN_KEY})"
  cosign sign-blob \
    --key "${FOP_LOCAL_SBOM_SIGN_KEY}" \
    --output-signature "${out}.sig" \
    --output-certificate "${out}.cert" \
    --yes \
    "$out"
  echo "✓ SBOM signed: ${out}.sig + ${out}.cert"
fi
