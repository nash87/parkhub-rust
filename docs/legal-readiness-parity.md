# Legal Readiness Parity

This report compares the Rust and PHP legal-readiness surfaces that operators
use before personal, business, German, EU, or international deployments. It is a
repo-local review aid, not legal advice and not a certification of any live
deployment.

## Parity Baseline

| Area | Rust surface | PHP surface | Required parity outcome |
| --- | --- | --- | --- |
| Operator hub | `docs/legal-readiness.md` | `docs/legal-readiness.md` | Both repos expose one audit entry point with German, EU, international, attorney-review, citation-verification, human-signoff, and deployment-specific configuration boundaries. |
| Deployment record | `docs/deployment-readiness-record.md` | `docs/deployment-readiness-record.md` | Both repos require one per-deployment record for jurisdiction, business context, modules/plugins, processors, CI/CD evidence, and final human go-live signoff. |
| Compliance matrix | `docs/COMPLIANCE.md` | `docs/COMPLIANCE.md` | Both repos keep operator obligations separate from shipped templates/features. |
| Data-protection guide | `docs/GDPR.md` | `docs/GDPR.md` | Both repos map data inventory, retention, data-subject rights, export, deletion, and processor evidence. |
| Legal templates | `legal/` and `docs/*-TEMPLATE.md` | `legal/` and `docs/*-TEMPLATE.md` | Templates remain operator-customizable starting points, not final legal texts. |
| Module/plugin review | `docs/COMPLIANCE.md`, `docs/legal-readiness.md`, deployment record | `docs/COMPLIANCE.md`, `docs/legal-readiness.md`, deployment record | Security-sensitive and legally sensitive modules/plugins require documented purpose, data categories, recipients, audit coverage, rollback path, review state, and launch decision. |
| AI/ML transparency | `legal/ai-act-transparency-template.md`, deployment record | `legal/ai-act-transparency-template.md`, deployment record | AI/ML, profiling, recommendation, and automated-support features require transparency and human-review evidence when enabled. |
| Release gate | `docs/release-checklist.md` | `docs/release-checklist.md` | Release checklists require legal wording and legal/module OpenAPI guards before tagging or deploying material legal/privacy changes. |
| Static wording guard | `scripts/tests/test-legal-readiness-wording.sh` | `scripts/tests/test-legal-readiness-wording.sh` | Absolute legal-status wording stays blocked; deployment-dependent review language remains required. |
| Legal/module OpenAPI guard | `scripts/tests/test-legal-openapi-contract.sh` | `scripts/tests/test-legal-openapi-contract.sh` | Legal, module, plugin, export, erasure, and privacy API surfaces remain explicitly reviewed. |
| Source of truth | GitHub `nash87/parkhub-rust` | GitHub `nash87/parkhub-php` | Review and release evidence comes from GitHub PRs/checks, not stale mirrors. |

## Cross-Repo Review Rules

- When one runtime changes legal, privacy, compliance, module, plugin, export,
  erasure, or deployment-readiness wording, review whether the sibling runtime
  needs the same operator-facing change.
- Keep runtime-specific implementation details separate, but keep operator
  obligations, human signoff boundaries, and release-review language equivalent.
- Use deployment-dependent language. Do not describe either runtime or a live
  deployment as having a final legal status.
- Treat the Nido/fop legal catalog service (current CLI entrypoint:
  `fop legal catalog --json`; `nido legal` is not exposed by the installed Nido
  CLI yet) as reference-only. Attorney review, citation verification,
  deployment-specific configuration review, human signoff, and final legal
  judgment remain required.
- Record accepted parity gaps in the release notes or a follow-up issue before
  tagging a release.

## Current T-6382 Status

- Rust and PHP both have a legal-readiness hub.
- Rust and PHP both have a deployment-readiness record.
- Rust and PHP both wire the legal-readiness wording guard into local PR CI.
- Rust and PHP both wire the legal/module OpenAPI guard into local PR CI.
- PHP PR evidence is published in GitHub PR #515.
- Rust branch evidence is local until the fop capacity guard allows the normal
  pre-push path to run.

## Operator Boundary

This parity report compares engineering controls and review artifacts. It does
not decide whether any operator is in scope for GDPR, DSGVO, TTDSG, DDG, BDSG,
GoBD, BFSG/EAA, NIS2, EU AI Act, UK GDPR, Swiss nDSG, CCPA/CPRA, LGPD, or any
sector-specific rule. The operator and qualified counsel must make that decision
for the actual deployment.
