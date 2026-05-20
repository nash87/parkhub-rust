#!/usr/bin/env bash
#
# Static guard for legal-readiness wording.
#
# ParkHub ships controls and templates that support compliant deployments, but
# live compliance depends on operator configuration, contracts, jurisdiction,
# and attorney review. Keep public docs from drifting back to absolute legal
# conclusions.
#
# Run: bash scripts/tests/test-legal-readiness-wording.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

scan_paths=(
    README.md
    docs
    legal
)

if [[ -f COMPLIANCE-REPORT.md ]]; then
    scan_paths+=(COMPLIANCE-REPORT.md)
fi

pattern="100% GDPR|GDPR compliant|DSGVO-konform|Compliance-Audited|Compliance Audited|complies with all GDPR|No DPA needed|no GDPR processor agreement needed|no mandatory data processor agreements|DPIA is NOT required|not required for typical deployments"

if rg --pcre2 -n "$pattern" "${scan_paths[@]}"; then
    echo "ERROR: legal-readiness docs contain absolute compliance wording." >&2
    echo "Use deployment-dependent wording and require operator/legal review." >&2
    exit 1
fi

echo "ParkHub legal-readiness wording contract OK."
