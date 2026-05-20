#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

mapfile -t ignored < <(scripts/ci/cargo-audit-with-deny-ignores.sh --print-ignores)

if [[ "${#ignored[@]}" -eq 0 ]]; then
  echo "no RustSec ignores parsed from deny.toml" >&2
  exit 1
fi

for advisory in "${ignored[@]}"; do
  if [[ ! "${advisory}" =~ ^RUSTSEC-[0-9]{4}-[0-9]{4}$ ]]; then
    echo "parsed RustSec ignore ID is malformed: ${advisory}" >&2
    exit 1
  fi
done

inline_matches="$(
  rg -n -- '--ignore[[:space:]]+RUSTSEC-' \
    .github/workflows \
    .gitea/workflows \
    .github/scripts/fop-local-ci.sh \
    scripts/ci/local-security-audit.sh || true
)"

if [[ -n "${inline_matches}" ]]; then
  echo "inline cargo-audit RustSec ignore lists must use scripts/ci/cargo-audit-with-deny-ignores.sh" >&2
  echo "${inline_matches}" >&2
  exit 1
fi

required_call_sites=(
  ".github/workflows/security.yml"
  ".github/workflows/nightly.yml"
  ".github/workflows/dependabot-local-ci-bridge.yml"
  ".gitea/workflows/security.yaml"
  ".gitea/workflows/nightly.yaml"
  ".gitea/workflows/dependabot-local-ci-bridge.yaml"
  ".github/scripts/fop-local-ci.sh"
  "scripts/ci/local-security-audit.sh"
)

for path in "${required_call_sites[@]}"; do
  if ! rg -q "scripts/ci/cargo-audit-with-deny-ignores.sh" "${path}"; then
    echo "${path} does not use the centralized cargo-audit RustSec wrapper" >&2
    exit 1
  fi
done

echo "RustSec ignore policy centralized: ${#ignored[@]} advisories from deny.toml"
