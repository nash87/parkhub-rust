#!/usr/bin/env bash
#
# Static guard for visible route polish regressions.
#
# Run: bash scripts/tests/test-ui-polish-contract.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

scan_paths=(
    parkhub-web/src/views
    parkhub-web/src/components
)

if [[ -d resources/js/src/views ]]; then
    scan_paths+=(resources/js/src/views resources/js/src/components)
fi

pattern="rounded-\\[[^]]+\\]|tracking-(tight|tighter|wide|wider|widest)|letterSpacing:\\s*['\\\"]-(?!0)|letterSpacing:\\s*-[0-9]|Booking studio|MARMOR GOVERNANCE STUDIO|OPERATIVER FOKUS"

if rg --pcre2 -n --glob '!**/*.test.*' "$pattern" "${scan_paths[@]}"; then
    echo "ERROR: route polish contract found arbitrary radius/tracking or blocked copy." >&2
    exit 1
fi

for phrase in "Coming soon" "Booking studio" "MARMOR GOVERNANCE STUDIO" "OPERATIVER FOKUS"; do
    if ! grep -Fq "$phrase" e2e/pages.spec.ts; then
        echo "ERROR: e2e/pages.spec.ts must block '$phrase'." >&2
        exit 1
    fi
done

echo "ParkHub UI polish contract OK."
