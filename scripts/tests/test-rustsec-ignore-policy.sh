#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

mapfile -t ignored < <(scripts/ci/cargo-audit-with-deny-ignores.sh --print-ignores)

for advisory in "${ignored[@]}"; do
  if [[ ! "${advisory}" =~ ^RUSTSEC-[0-9]{4}-[0-9]{4}$ ]]; then
    echo "parsed RustSec ignore ID is malformed: ${advisory}" >&2
    exit 1
  fi
done

tmp_deny="$(mktemp)"
trap 'rm -f "${tmp_deny}"' EXIT
cat >"${tmp_deny}" <<'TOML'
[advisories]
ignore = []
TOML
mapfile -t empty_ignores < <(CARGO_DENY_CONFIG="${tmp_deny}" scripts/ci/cargo-audit-with-deny-ignores.sh --print-ignores)
if [[ "${#empty_ignores[@]}" -ne 0 ]]; then
  echo "empty RustSec ignore lists must remain valid" >&2
  exit 1
fi

cat >"${tmp_deny}" <<'TOML'
[advisories]
ignore = [
  "RUSTSEC-2024-0001", # comment with ] must not close the array
  "RUSTSEC-2024-0002",
]
TOML
mapfile -t comment_bracket_ignores < <(CARGO_DENY_CONFIG="${tmp_deny}" scripts/ci/cargo-audit-with-deny-ignores.sh --print-ignores)
if [[ "${comment_bracket_ignores[*]}" != "RUSTSEC-2024-0001 RUSTSEC-2024-0002" ]]; then
  echo "RustSec ignore parser must only close on a TOML array closing line" >&2
  exit 1
fi

inline_matches="$(
  grep -R -n -E -- '--ignore([[:space:]]+|=)RUSTSEC-' \
    .github/workflows \
    .gitea/workflows \
    .github/scripts/nido-local-ci.sh \
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
  ".github/scripts/nido-local-ci.sh"
  "scripts/ci/local-security-audit.sh"
)

wrapper_pattern='(^|[^[:alnum:]_./-])(bash[[:space:]]+)?(\./)?scripts/ci/cargo-audit-with-deny-ignores\.sh([^[:alnum:]_./-]|$)'

for path in "${required_call_sites[@]}"; do
  if ! grep -Eq -- "${wrapper_pattern}" "${path}"; then
    echo "${path} does not use the centralized cargo-audit RustSec wrapper" >&2
    exit 1
  fi
done

echo "RustSec ignore policy centralized: ${#ignored[@]} advisories from deny.toml"
