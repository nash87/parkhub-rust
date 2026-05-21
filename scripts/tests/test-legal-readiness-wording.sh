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

    if [[ ! -r "$file" ]]; then
        echo "ERROR: required legal-readiness file is missing or unreadable: $file" >&2
        exit 1
    fi

    if ! grep -Fq "$text" "$file"; then
        echo "ERROR: $file is missing required legal-readiness text: $text" >&2
        exit 1
    fi
}

require_text docs/release-checklist.md "scripts/tests/test-legal-readiness-wording.sh"
require_text docs/release-checklist.md "scripts/tests/test-legal-openapi-contract.sh"
require_text docs/release-checklist.md "docs/legal-readiness.md"
require_text docs/release-checklist.md "docs/deployment-readiness-record.md"
require_text docs/release-checklist.md "docs/legal-readiness-parity.md"
require_text docs/release-checklist.md "fop legal catalog"
require_text docs/release-checklist.md "reference-only, not legal advice"
require_text docs/release-checklist.md "attorney review"
require_text docs/release-checklist.md "citation"
require_text docs/release-checklist.md "human signoff"
require_text docs/release-checklist.md "deployment-specific configuration"
require_text docs/release-checklist.md 'GitHub `nash87/parkhub-rust` remains the CI/review source of truth'
require_text README.md "docs/deployment-readiness-record.md"
require_text README.md "docs/legal-readiness-parity.md"
require_text docs/legal-readiness.md "# ParkHub Legal Readiness Hub"
require_text docs/legal-readiness.md "operator-facing audit hub"
require_text docs/legal-readiness.md "German, EU, and international"
require_text docs/legal-readiness.md "deployment-dependent"
require_text docs/legal-readiness.md "not legal advice"
require_text docs/legal-readiness.md "reference-only catalog"
require_text docs/legal-readiness.md "attorney review"
require_text docs/legal-readiness.md "citation verification"
require_text docs/legal-readiness.md "human signoff"
require_text docs/legal-readiness.md "deployment-specific configuration"
require_text docs/legal-readiness.md "docs/deployment-readiness-record.md"
require_text docs/legal-readiness.md "docs/legal-readiness-parity.md"
require_text docs/deployment-readiness-record.md "# Deployment Readiness Record"
require_text docs/deployment-readiness-record.md "Personal, business, or mixed use"
require_text docs/deployment-readiness-record.md "Germany-specific obligations"
require_text docs/deployment-readiness-record.md "Module And Plugin Review"
require_text docs/deployment-readiness-record.md "AI/ML / recommendations"
require_text docs/deployment-readiness-record.md "Required Signoff"
require_text docs/deployment-readiness-record.md "Final human go-live signoff"
require_text docs/legal-readiness-parity.md "# Legal Readiness Parity"
require_text docs/legal-readiness-parity.md "Rust and PHP"
require_text docs/legal-readiness-parity.md "Module/plugin review"
require_text docs/legal-readiness-parity.md "fop legal catalog"
require_text docs/legal-readiness-parity.md "qualified counsel"
require_text docs/COMPLIANCE.md "Operator Legal Readiness Checklist"
require_text docs/COMPLIANCE.md "Module / Plugin Enablement Policy"

if rg --pcre2 -n "GDPR compliant|DSGVO compliant|legally compliant|certified|guaranteed" docs/legal-readiness.md docs/release-checklist.md; then
    echo "ERROR: legal-readiness hub/checklist contain absolute legal-status wording." >&2
    exit 1
fi

echo "ParkHub legal-readiness wording contract OK."
