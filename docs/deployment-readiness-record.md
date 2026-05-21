# Deployment Readiness Record

Use this record before a ParkHub Rust deployment is opened for production,
business use, personal-data processing, or customer-facing evaluation. Keep one
completed copy per deployment or material configuration change. Do not store
secrets, access tokens, private keys, or raw personal data in this record.

This record is an engineering and operator evidence artifact. It is not legal
advice, does not verify citations, and does not replace attorney review,
citation verification, deployment-specific configuration review, human signoff,
or final legal judgment for a specific operator.

## Deployment Header

| Field | Operator value |
| --- | --- |
| Deployment name and environment | |
| Deployment purpose | |
| Personal, business, or mixed use | |
| Public, internal, or private exposure | |
| Controller / operator entity | |
| Deployment owner | |
| Launch approver | |
| Target launch date | |
| ParkHub Rust commit / tag | |
| CI run / local fop attestation | |
| Review record owner | |

## Nido/fop Legal Catalog Service Evidence

This section is the Nido/fop legal catalog service evidence for the release
candidate.

Capture the live Nido/fop legal catalog output for the exact release candidate:

```bash
NO_COLOR=true fop legal catalog --json > legal-catalog-<deployment-or-sha>.json
```

The current installed Nido CLI does not expose `nido legal`; use the
`fop legal catalog --json` entrypoint until Nido exposes an equivalent command.
Attach the JSON output or copy the fields below. Treat the catalog as
reference-only; attorney review, citation verification, deployment-specific
configuration review, human signoff, and final legal judgment remain required.

| Field | Captured value |
| --- | --- |
| Capture command | |
| Captured by / date | |
| Catalog id / source / version | |
| Catalog `source_revision` | |
| Catalog `generated_at` | |
| Catalog `requires_attorney_review` | |
| Catalog `requires_human_signoff` | |
| Catalog `execution_allowed` | |
| Catalog `safety_boundary` | |

## Jurisdiction And Business Context

- [ ] Countries, states, and regions where the service is offered are listed.
- [ ] Germany-specific obligations are reviewed when the operator, customers, or
      users are in Germany.
- [ ] EU/EEA GDPR obligations are reviewed when EU/EEA users or operators are in
      scope.
- [ ] International overlays such as UK GDPR, Swiss nDSG, CCPA/CPRA, and LGPD are
      reviewed where applicable.
- [ ] Consumer-facing, employee-facing, B2B, public-sector, and sector-specific
      obligations are explicitly marked in or out of scope.
- [ ] Accessibility scope is reviewed for BFSG / EU Accessibility Act relevance.
- [ ] NIS2-style cybersecurity scope is assessed for the operator category.

## Data And Processor Evidence

- [ ] Actual personal-data categories are listed.
- [ ] Purposes and legal bases are mapped to the privacy notice and VVT.
- [ ] Retention periods are set for bookings, payments, audit logs, sessions,
      uploads, backups, and exports.
- [ ] Export, deletion, anonymization, and data-subject request paths are tested
      or explicitly deferred with owner and date.
- [ ] Hosting regions, backup regions, email providers, payment providers,
      analytics providers, AI providers, monitoring providers, and support tools
      are listed.
- [ ] AVV/DPA/sub-processor evidence is attached for every external processor.
- [ ] Cross-border transfer basis and sub-processor evidence are recorded where
      data leaves the operator's primary jurisdiction.

## Module And Plugin Review

Security-sensitive or legally sensitive modules remain disabled until this table
has an owner, review state, rollback path, and launch decision.

| Module / plugin | Purpose | Data categories | External recipients | Audit coverage | Rollback path | Review state | Launch decision |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Auth / MFA / SSO | | | | | | | |
| RBAC / multi-tenant boundaries | | | | | | | |
| Payments / invoices / tax records | | | | | | | |
| Notifications / messaging | | | | | | | |
| Webhooks / third-party integrations | | | | | | | |
| Analytics / reporting | | | | | | | |
| AI/ML / recommendations | | | | | | | |
| Custom plugins | | | | | | | |

## Security And CI/CD Evidence

- [ ] Required GitHub checks are green for the release or PR.
- [ ] Local fop attestation is captured when required by the release process.
- [ ] `scripts/tests/test-legal-readiness-wording.sh` passes.
- [ ] `scripts/tests/test-legal-openapi-contract.sh` passes after legal, privacy,
      module, plugin, export, erasure, or OpenAPI changes.
- [ ] Vulnerability scan, dependency review, secret scan, and workflow/static
      analysis results are attached or linked.
- [ ] SBOM, provenance, image scan, and signature evidence are attached when a
      container or downloadable artifact is released.
- [ ] Backup, restore, incident response, vulnerability disclosure, and audit-log
      export paths are assigned to an operator owner.

## Required Signoff

| Review | Owner | Date | Decision | Notes |
| --- | --- | --- | --- | --- |
| Engineering readiness | | | | |
| Security review | | | | |
| Privacy / data-protection review | | | | |
| Attorney / qualified counsel review | | | | |
| Accessibility review | | | | |
| Business owner approval | | | | |
| Final human go-live signoff | | | | |

## Go / No-Go Decision

- [ ] All required review rows above are complete.
- [ ] No unresolved high-risk security, legal, privacy, accessibility, or data
      transfer issue remains without a named owner and accepted risk decision.
- [ ] Release notes describe legal-readiness changes as deployment-dependent
      support and operator obligations, not as final legal compliance.
- [ ] The launch owner has confirmed the exact configuration, modules, processors,
      regions, and legal texts deployed.
