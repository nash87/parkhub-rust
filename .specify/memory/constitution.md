---
version: "1.0"
project: parkhub-rust
last_reviewed: 2026-06-09
status: active
---

# ParkHub Rust — Engineering Constitution

This file captures the binding engineering principles for the ParkHub Rust project.
It is the authoritative reference for spec authoring, PR review, and agent guidance.
Update this file when a principle changes; treat its contents as law until changed.

---

## What this project is

ParkHub Rust is a self-hosted, single-binary parking management server written in
Rust (Axum + redb). It ships as one executable with embedded React/Astro frontend,
zero external runtime dependencies, optional AES-256-GCM database encryption, and
full GDPR compliance by design.

---

## Non-negotiable boundaries

These are the "we never do" rules. A spec or PR that crosses any of these lines
must be rejected or escalated, regardless of other merit.

### Security

- **No `unsafe` without documented justification.** Every `unsafe` block must carry
  an inline comment explaining why it is correct and why it is the only option.
- **All passwords hashed with Argon2id + OsRng.** Never `bcrypt`, `scrypt`, or a
  fixed-cost scheme. No MD5/SHA1 for passwords.
- **Secrets via environment variables or Vault.** No secrets in source, config files,
  or test fixtures committed to the repo.
- **Constant-time comparisons for tokens.** Use the `subtle` crate. No `==` on bearer
  tokens or HMAC digests.
- **No new `openssl` dependencies.** All TLS via `rustls`; all crypto via audited
  pure-Rust crates.
- **Security issues go through the private advisory process** (SECURITY.md), never
  through public GitHub issues.

### Data and compliance

- **No third-party data processors by default.** The default build sends no data
  outside the operator's server. Features that contact external services (OAuth, Stripe,
  SMTP, webhooks) are gated behind explicit configuration and documented as processors.
- **GDPR user rights must be preserved.** Every data model change must be reviewed
  against Article 15–22 (access, rectification, erasure, portability). See docs/GDPR.md.
- **Audit log is append-only.** No handler may delete or redact audit log entries.

### Architecture

- **Single-binary contract.** The default headless server (`--no-default-features
  --features headless`) must build with no external services, no migrations, and no
  network egress. A bare `./parkhub-server --headless --unattended` must reach a
  healthy state with zero external calls.
- **OpenAPI spec is source of truth.** `docs/openapi/rust.json` must stay in sync with
  the running server. The `make drift` gate enforces this; never suppress it.
- **No SQL injection surface.** The database layer is redb (key-value). Any future SQL
  integration must use parameterized queries exclusively.
- **Feature flags over conditional compilation.** New optional functionality belongs
  behind a `mod-*` Cargo feature, not a runtime switch that always compiles in.

### API and parity

- **Backwards-compatible minor versions.** Breaking API changes require a
  `PROTOCOL_VERSION` bump in `parkhub-common/src/lib.rs` and a clear migration note.
- **PHP parity gate.** Any endpoint added or changed in parkhub-rust must be reviewed
  against `docs/openapi-parity.md` and either replicated in parkhub-php or explicitly
  documented as Rust-only with a parity ticket.

### Code quality

- **Zero Clippy warnings in CI.** The pedantic + nursery profile. No `#[allow]`
  without an explanatory comment.
- **Every public API function has a doc comment.** Undocumented public functions
  block the `cargo doc` step.
- **Tests for every new code path.** PRs adding functionality without tests will be
  asked to add coverage before merge. Unit tests in the module, integration tests in
  `integration_tests.rs`.

### Contributor experience

- **External contributors can build without the homelab toolchain.** `fop` is an
  optional build accelerator, not a requirement. `./scripts/fop-wrap.sh` provides the
  safe public surface; bare `cargo` and `npm` must always work.
- **No internal infrastructure details in public docs.** No private IPs, no operator
  usernames, no internal hostnames in any committed file.

---

## How to update this constitution

Open a PR with `docs(constitution):` prefix. The PR must include:
1. The changed principle and a rationale
2. Any spec or code that needs to be updated as a consequence
3. A review by at least one maintainer

---

## Relationship to other governance files

| File | Role |
|------|------|
| `CONTRIBUTING.md` | Contributor workflow (how to contribute, not what to build) |
| `AGENTS.md` | Agent-facing build commands and CI gates |
| `docs/parity-governance.md` | Cross-runtime ownership and parity rules |
| `docs/openapi-parity.md` | Endpoint-level parity tracking |
| `docs/release-checklist.md` | Release gate discipline |
| `SECURITY.md` | Disclosure policy and supported versions |
| `specs/<feature-id>/` | Per-feature requirements, design, and tasks |
