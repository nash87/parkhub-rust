# ParkHub Legal Readiness Hub

This operator-facing audit hub gives operators one audit entry point for
ParkHub's German, EU, and international legal-readiness posture. It maps the
existing product controls, templates, and release checks to deployment-dependent
operator obligations. It is informational only and is not legal advice.

ParkHub ships readiness-supporting controls and templates. Final production use
depends on the operator's jurisdiction, deployment model, enabled modules,
processor contracts, retention choices, privacy notices, accessibility posture,
AI/ML feature choices, and organization-specific risk review. Required reviews
include attorney review, citation verification, human signoff, and
deployment-specific configuration review before go-live.

## Audit Boundary

- Use this document to find the readiness materials that need operator review.
- Use `docs/COMPLIANCE.md` for the detailed German, EU, and international
  obligation matrix.
- Use `docs/GDPR.md` for the data-protection feature map, retention notes,
  data subject rights, and German DSGVO-oriented implementation notes.
- Use `legal/` and the `docs/*-TEMPLATE.md` files as editable starting points
  for operator-specific legal texts.
- Use `docs/release-checklist.md` before release to confirm the legal-readiness
  wording and OpenAPI static guards still pass.
- Use `docs/deployment-readiness-record.md` as the per-deployment evidence
  record for jurisdiction, modules, processors, CI/CD evidence, and signoff.
- Use `docs/legal-readiness-parity.md` when comparing Rust and PHP
  legal-readiness obligations, release gates, and module/plugin review policy.
- Treat the Nido/fop legal catalog service (current CLI entrypoint:
  `fop legal catalog --json`; `nido legal` is not exposed by the installed Nido
  CLI yet) as a reference-only catalog. It helps locate legal topics and
  citations, but it is not legal advice and does not replace attorney review,
  citation verification, human signoff, or deployment-specific configuration
  review.

## Source Map

| Area | Primary sources | Operator review focus |
|------|-----------------|-----------------------|
| German provider identification | `docs/IMPRESSUM-TEMPLATE.md`, `legal/impressum-template.md`, `/api/v1/legal/impressum` | Complete DDG Section 5 provider fields, publication path, and organization-specific details. |
| German/EU privacy notice | `docs/PRIVACY-TEMPLATE.md`, `legal/datenschutz-template.md`, `docs/GDPR.md` | Match actual processing activities, legal bases, recipients, retention, rights handling, and contact details. |
| Processing records and processors | `legal/vvt-template.md`, `legal/avv-template.md`, `docs/COMPLIANCE.md` | Keep the VVT, AVV/DPA coverage, subprocessors, hosting model, and transfer basis current. |
| Retention and erasure | `docs/GDPR.md`, `docs/COMPLIANCE.md`, API export/erasure endpoints | Align retention settings with accounting, tenancy, contract, and local policy obligations. |
| Accessibility | `legal/bfsg-barrierefreiheit-template.md`, frontend accessibility tests, release checklist | Verify BFSG/EAA applicability, statement accuracy, issue reporting channel, and deployment-specific accessibility evidence. |
| AI/ML transparency | `legal/ai-act-transparency-template.md`, module enablement policy | Use only when AI/ML features are enabled; document transparency notices, human review boundaries, and feature-specific risk review. |
| Security and audit trail | `docs/SECURITY.md`, `docs/FEATURES.md`, audit log behavior | Confirm authentication, encryption, audit logging, backup, incident, and vulnerability-response settings for the target deployment. |
| International privacy posture | `docs/COMPLIANCE.md` | Review UK GDPR, Swiss nDSG, CCPA, LGPD, and any sector or local rules that apply to the operator. |
| Deployment signoff record | `docs/deployment-readiness-record.md` | Capture jurisdiction, business context, enabled modules, processors, CI/CD evidence, legal review, and final human go-live decision for each deployment. |
| Rust/PHP legal parity | `docs/legal-readiness-parity.md` | Compare the two runtimes' legal-readiness hubs, release gates, module/plugin review policy, and remaining operator boundaries. |

## Release Audit Steps

1. Run `scripts/tests/test-legal-readiness-wording.sh` after changes to public
   legal, privacy, module, release, or README wording.
2. Run `scripts/tests/test-legal-openapi-contract.sh` after changes to legal,
   privacy, module, export, erasure, plugin, or OpenAPI surfaces.
3. Review `docs/COMPLIANCE.md` and `docs/GDPR.md` for drift from the actual
   enabled modules, integrations, processors, jurisdictions, and retention
   settings.
4. Complete or update `docs/deployment-readiness-record.md` for the target
   deployment before production use, business use, or customer-facing evaluation.
5. Review `docs/legal-readiness-parity.md` when a change should stay aligned
   across Rust and PHP.
6. Confirm legal templates are marked as operator-customizable starting points,
   not final legal texts.
7. Confirm the release notes describe material legal-readiness changes as
   readiness support or operator obligations, not as absolute legal status.

## Module Enablement Rules

Low-risk presentation and convenience modules can usually be reviewed through
normal release checks. Security-sensitive or legally sensitive modules need
explicit operator review before enablement, especially when they affect:

- authentication, authorization, or tenant boundaries;
- payments, invoices, tax/accounting retention, or financial records;
- outbound messaging, analytics, webhooks, third-party integrations, or custom
  plugins;
- AI/ML features, profiling, automated recommendations, or transparency notices;
- new personal-data categories, new recipients, or changed retention behavior.

Before enabling those modules, operators should update the privacy notice, VVT,
processor list, retention schedule, audit-log expectations, rollback plan, and
deployment configuration. Legal counsel should verify jurisdiction-specific
citations and obligations.

## Non-Advice Notice

This hub, the `legal/` templates, `docs/COMPLIANCE.md`, `docs/GDPR.md`, and
`fop legal catalog` are reference materials for operator review. They do not
create legal advice, attorney-client relationship, or a final legal decision for
any specific deployment, and they do not replace required human signoff.
