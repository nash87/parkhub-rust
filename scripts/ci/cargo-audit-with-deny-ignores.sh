#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"
deny_file="${CARGO_DENY_CONFIG:-${repo_root}/deny.toml}"

if [[ ! -f "${deny_file}" ]]; then
  echo "deny.toml not found at ${deny_file}" >&2
  exit 1
fi

mapfile -t rustsec_ignores < <(
  awk '
    /^\[advisories\][[:space:]]*$/ {
      in_advisories = 1
      next
    }
    /^\[/ {
      in_advisories = 0
      in_ignore = 0
    }
    in_advisories && /^[[:space:]]*ignore[[:space:]]*=/ {
      in_ignore = 1
    }
    in_advisories && in_ignore {
      line = $0
      while (match(line, /RUSTSEC-[0-9]{4}-[0-9]{4}/)) {
        id = substr(line, RSTART, RLENGTH)
        if (!seen[id]++) {
          print id
        }
        line = substr(line, RSTART + RLENGTH)
      }
      if ($0 ~ /\]/) {
        in_ignore = 0
      }
    }
  ' "${deny_file}"
)

if [[ "${1:-}" == "--print-ignores" ]]; then
  printf '%s\n' "${rustsec_ignores[@]}"
  exit 0
fi

if [[ "${1:-}" == "--" ]]; then
  shift
fi

args=()
for advisory in "${rustsec_ignores[@]}"; do
  args+=(--ignore "${advisory}")
done

exec cargo audit "${args[@]}" "$@"
