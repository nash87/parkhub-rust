# 2026-05-21 Exact-Cover Nido/fop Legal Catalog Evidence

This release-candidate evidence was captured for the ParkHub Rust
`exact_cover_v1` allocation work before release or customer-facing claims.

## Candidate

| Field | Value |
| --- | --- |
| Repository | `nash87/parkhub-rust` |
| Pull request | `#663` |
| Head SHA | `e8bbb475d638469d4d9a0421ef3d7a7c4d61d3fa` |
| Local Nido PR gate report | `.fop/reports/local-ci-pr-e8bbb475d638469d4d9a0421ef3d7a7c4d61d3fa.json` |

## Capture

| Field | Value |
| --- | --- |
| Capture command | `NO_COLOR=true fop legal catalog --json` |
| Captured by / date | Codex on 2026-05-21 |
| Catalog id / source / version | `anthropic-claude-for-legal` / `claude-for-legal` / `2026-05-15-review` |
| Catalog `source_revision` | `9cecd91` |
| Catalog `generated_at` | `2026-05-21T21:32:11.995244005Z` |
| Catalog `requires_attorney_review` | `true` |
| Catalog `requires_human_signoff` | `true` |
| Catalog `execution_allowed` | `false` |
| Installed Nido legal entrypoint | Not exposed by the installed Nido CLI; use `fop legal catalog --json` until `nido legal` exists. |
| Catalog `safety_boundary` | Reference catalog only. Claude for Legal plugins can help draft, triage, and monitor legal work, but attorney review, citation verification, client authorization, and final legal judgment remain required. fop exposes install and deploy commands as text only. |

## Release Interpretation

- `exact_cover_v1` remains operational scheduling support only.
- This catalog output is reference-only evidence, not legal advice.
- Public ToS, privacy, profiling, accessibility, or compliance wording still
  requires attorney review, citation verification, deployment-specific configuration review,
  and final human signoff.
- The release remains blocked for production/customer-facing legal claims until
  the deployment readiness record is completed for the actual operator,
  processors, jurisdictions, enabled modules, and hosting model.
