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

require_text() {
    local file="$1"
    local text="$2"

    if ! grep -Fq "$text" "$file"; then
        echo "ERROR: $file is missing required legal-readiness text: $text" >&2
        exit 1
    fi
}

require_text docs/release-checklist.md "scripts/tests/test-legal-readiness-wording.sh"
require_text docs/release-checklist.md "scripts/tests/test-legal-openapi-contract.sh"
require_text docs/release-checklist.md "fop legal catalog"
require_text docs/release-checklist.md "attorney review"
require_text docs/release-checklist.md "citation"
require_text docs/release-checklist.md 'GitHub `nash87/parkhub-rust` remains the CI/review source of truth'
require_text docs/COMPLIANCE.md "Operator Legal Readiness Checklist"
require_text docs/COMPLIANCE.md "Module / Plugin Enablement Policy"

echo "ParkHub legal-readiness wording contract OK."
