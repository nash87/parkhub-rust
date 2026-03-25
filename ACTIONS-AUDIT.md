# GitHub Actions audit for `parkhub-rust`

## Existing state before this change

### Inventory
- **GitHub Actions** already had separate workflows for CI, CodeQL, security scanning, dependency review, releases, Docker publishing, E2E, Lighthouse, Dependabot auto-merge, and Copilot setup.
- **Gitea CI** already existed at `.gitea/workflows/ci.yml` as a legacy mirror pipeline.
- The Rust workspace is `parkhub-common`, `parkhub-server`, and `parkhub-client`, with `rust-version = "1.85"` in `/Cargo.toml`.
- The shipped product includes an embedded frontend from `/parkhub-web`, so frontend build health is part of release confidence.
- `deny.toml` already defines advisory and license policy, including documented exceptions for optional Slint/GTK transitive issues.
- The Docker build is multi-stage and release-oriented, with GHCR publishing already present.

### Local baseline verified before editing
- `cargo fmt --all -- --check` currently fails on pre-existing formatting drift in Rust source files.
- `cargo clippy ... -D warnings` currently fails on pre-existing warnings in `parkhub-common/src/lib.rs`.
- Frontend tests and `parkhub-web` production build pass locally, albeit with existing React test warnings.
- Headless Rust `cargo check` succeeds locally for `parkhub-common` and `parkhub-server`.

## Risks and gaps found
- **GitHub vs. Gitea ownership was ambiguous.** The Gitea workflow had drifted from GitHub CI and still used `rust:latest` plus broad workspace commands.
- **CodeQL coverage was incomplete.** GitHub CodeQL only analyzed JavaScript/TypeScript and missed the Rust server/common code.
- **Workflow linting was missing.** There was no `actionlint` validation for workflow changes.
- **Security checks were not aligned to review timing.** `cargo-audit` only ran weekly/manual, not as part of normal PR-era policy. `cargo-deny` existed but lived separately from the overall CI design.
- **Release and container workflows mixed concerns.** The container workflow included dead Render deployment logic even though the workflow only triggered on tags/manual runs.
- **Least-privilege and governance posture was inconsistent.** Several workflows lacked explicit concurrency, uniform timeouts, or intentionally minimized permissions.
- **E2E validation was low-signal.** The E2E workflow embedded a placeholder frontend rather than the real shipped frontend build.
- **Toolchain policy was inconsistent.** The repo declares Rust 1.85, but GitHub and Gitea CI used floating toolchains in key places.

## What changed

### GitHub Actions
- **CI (`.github/workflows/ci.yml`)**
  - Added `actionlint`.
  - Pinned Rust validation to the declared toolchain baseline (`1.85`).
  - Kept PR CI focused on the repository's realistic headless Rust path plus the real frontend build/test path.
  - Intentionally did **not** force `--workspace --all-features` into PR CI because that would drag the optional Slint GUI/client path into every Linux PR run even though releases already validate desktop artifacts separately.
  - Kept `fmt` and `clippy -D warnings` visible in PR CI, but left them outside the required aggregate gate until the repo's pre-existing failures are cleaned up.
  - Added workflow-level concurrency cancellation and explicit job timeouts.
  - Kept a single gate job (`CI`) for branch protection.

- **Security (`.github/workflows/security.yml`)**
  - Kept `cargo-deny` as the PR/push policy check for advisories, bans, licenses, and sources.
  - Kept `cargo-audit` as a scheduled/manual backstop instead of duplicating advisory noise on every PR.
  - Hardened the workflow with concurrency and timeouts.
  - Limited Trivy filesystem SARIF upload to trusted non-PR runs.

- **CodeQL (`.github/workflows/codeql.yml`)**
  - Added **Rust** analysis in addition to JavaScript/TypeScript.
  - Switched to repository-aware manual build steps so CodeQL sees the real headless Rust build and the actual frontend build.
  - Added concurrency and explicit runtime limits.

- **Release (`.github/workflows/release.yml`)**
  - Pinned Rust/Node versions.
  - Added `--locked` release builds.
  - Added artifact retention, checksums, provenance attestation, and explicit job timeouts.
  - Kept Linux + Windows release packaging because that matches the current shipped binaries.
  - Added an explicit manual-dispatch tag validation step so maintainers get a clear failure instead of a silent no-op.

- **Container publish (`.github/workflows/docker-publish.yml`)**
  - Split the workflow back down to one responsibility: build/publish the GHCR image for release tags.
  - Removed the dead inline Render deployment logic from this workflow until deployment protections and ownership are explicitly modeled.
  - Added `linux/arm64` alongside `linux/amd64` for release images.
  - Added image provenance attestation and a CycloneDX SBOM artifact.
  - Kept Trivy image scanning and SARIF upload.
  - Added an explicit manual-dispatch tag validation step so maintainers get a clear failure instead of a silent no-op.

- **Dependency review, E2E, and Lighthouse**
  - Added concurrency and timeouts where missing.
  - Kept E2E informational, but changed it to build the **real frontend** before exercising the embedded server.
  - Kept Lighthouse as a non-core quality signal rather than a required protection rule.

### Gitea CI
- **Decision:** keep Gitea CI, but narrow its role.
- `.gitea/workflows/ci.yml` is now aligned to a **mirror/headless Rust validation subset** instead of trying to duplicate GitHub release logic.
- It now uses Rust `1.85` instead of `latest` and matches the same headless/common Rust validation strategy used in GitHub CI.

## Required vs. optional checks

### Required for branch protection
Use these as the protected checks on GitHub:
- `CI`
- `Dependency Review`
- `Analyze (rust)`
- `Analyze (javascript-typescript)`
- `Cargo Deny`

### Recommended but optional
Keep these visible but not required unless the maintainer wants slower PR gating:
- `Playwright E2E`
- `Lighthouse Audit`
- scheduled/manual `Cargo Audit`
- scheduled/main-branch Trivy filesystem SARIF uploads

## Toolchain and platform strategy
- **Rust CI baseline:** pinned to the declared repository MSRV/toolchain floor (`1.85`) for deterministic behavior.
- **PR validation platform:** Linux-only for speed and because the server/headless path is the main review surface.
- **Release validation platform:** Linux + Windows binaries remain justified because the repository ships both server and desktop client artifacts.
- **Container targets:** publish `linux/amd64` and `linux/arm64` because the headless server is a strong candidate for ARM self-hosted deployments.
- **macOS:** not added to default CI because there is no repository evidence that the cost is justified today.

## GitHub vs. Gitea ownership decision
**GitHub Actions is now the source of truth for public CI, security scanning, releases, and container publishing.**

**Gitea CI remains as a compatibility/mirror layer for lightweight headless Rust validation only.** This keeps mirror users functional without duplicating GitHub-only security and release features.

## Branch protection recommendations
- Protect `main`.
- Require the checks listed under **Required for branch protection**.
- Require at least one human review.
- Dismiss stale approvals on new commits.
- Restrict release tags (`v*`) to maintainers.
- If GitHub Environments are later introduced for deployments, require reviewer approval before any production/demo deployment job can read secrets.

## Notes for maintainers
- This workflow set intentionally does **not** pretend the repo is already clean on pre-existing `fmt` and `clippy -D warnings` failures; it surfaces them.
- If/when desktop GUI validation becomes a routine maintenance goal, add a dedicated scheduled Windows/Linux desktop build smoke workflow rather than bloating PR CI.
