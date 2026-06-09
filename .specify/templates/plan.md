---
id: "{{feature-id}}"
title: "{{Feature Title}}"
type: plan
status: draft
spec: "specs/{{feature-id}}/spec.md"
created: {{YYYY-MM-DD}}
author: "@{{github-handle}}"
nido_task: "T-{{task-id}}"
---

# Plan: {{Feature Title}}

> **What this is:** Technical design and architecture decisions.
> The spec (what and why) lives in `spec.md`. This doc covers how.

---

## Summary

<!-- Two to four sentences: what we are building, the key trade-off chosen, and why. -->

---

## Architecture

### Affected crates

<!-- Which crates change? What new crates or dependencies are required? -->

| Crate | Change type | Notes |
|-------|-------------|-------|
| `parkhub-common` | | |
| `parkhub-server` | | |
| `parkhub-web` | | |

### New dependencies

<!-- List any new Cargo or npm dependencies. For each: why it was chosen over alternatives,
     license (must be MIT/Apache-2.0 compatible — see deny.toml), and audit status.
     Zero new dependencies is always preferable. -->

| Dep | Version | License | Rationale |
|-----|---------|---------|-----------|

### Data model

<!-- New redb tables, changed key or value schemas, migration path if any.
     Remember: adding a table is safe; removing or renaming requires a migration helper in db.rs. -->

### API surface

<!-- New or changed endpoints. Follow the utoipa annotation pattern.
     For each: method, path, auth tier required, request/response types. -->

| Method | Path | Auth | Request | Response |
|--------|------|------|---------|----------|

OpenAPI parity review (per `docs/openapi-parity.md`):
- [ ] Endpoints are matched in parkhub-php OR documented as Rust-only with a parity ticket

### Feature flags

<!-- Is this behind a `mod-*` Cargo feature? Which one?
     Default-on or opt-in? -->

### Frontend

<!-- React components added or changed.
     ts-rs types impacted?
     Accessibility checklist: aria-label, keyboard nav, contrast. -->

---

## Implementation sequence

<!-- Order matters. List the logical implementation phases.
     Each phase should be independently testable. -->

1.
2.
3.

---

## Testing strategy

<!-- Unit tests: what modules/functions.
     Integration tests: which scenarios in integration_tests.rs.
     Vitest: which components/views.
     E2E: new Playwright specs if applicable. -->

---

## Rollback plan

<!-- How do we revert if this ships with a critical bug?
     Rust: feature flag toggle? config env var guard?
     DB schema: forward-compatible? migration reversible? -->

---

## Open questions

<!-- Outstanding design decisions that still need resolution. -->

| # | Question | Decision | Date |
|---|----------|----------|------|

---

## References

- Spec: [spec.md](spec.md)
- OpenAPI parity: [docs/openapi-parity.md](../../docs/openapi-parity.md)
- Release checklist: [docs/release-checklist.md](../../docs/release-checklist.md)
