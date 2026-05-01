---
title: "parkhub: local-first CI/CD attestation and GitHub CI thinning"
repo_path: "."
type: "implementation"
priority: "high"
status: active
task_id: "T-2133"
source_note: "Refactor ParkHub CI/CD so fop local verification is the primary PR gate, GitHub Actions are thin/attestation-aware, heavy checks move to local/nightly/main, and CD remains GitOps/signed."
verification:
  - "fop build --backend local . --preset custom -- bash -lc 'git diff --check && ./.github/scripts/fop-local-ci.sh --profile pr --dry-run'"
---

# parkhub: local-first CI/CD attestation and GitHub CI thinning

## Goal

Refactor ParkHub CI/CD so fop local verification is the primary PR gate, GitHub Actions are thin/attestation-aware, heavy checks move to local/nightly/main, and CD remains GitOps/signed.

## Constraints

- Keep `fop` as the only local build/test execution surface; no raw cargo/npm commands outside fop-managed steps.
- GitHub remains the PR UI and branch-protection surface, not the primary compute surface.
- Keep security-sensitive remote checks that need GitHub context: dependency review and secret scanning.
- Do not mix this with the final Rust PR merge train; land as a separate PR.

## Acceptance

- Local command `.github/scripts/fop-local-ci.sh --profile pr` runs the PR gate through fop.
- Local command can publish the commit status context `fop/local-ci/pr`.
- GitHub PR CI fails closed for same-repo human PRs until the explicit `fop/local-ci/pr` status is `success`; combined-status success is not accepted as a substitute.
- Heavy CI and CodeQL jobs still run on `main`, by manual dispatch/schedule, or on PRs labeled `github-ci-full`.
- CD stays release/GitOps-oriented and is not coupled to every PR push.

## Paths

- `.github/scripts/fop-local-ci.sh`
- `.github/workflows/ci.yml`
- `.github/workflows/e2e.yml`
- `.github/workflows/lighthouse.yml`
- `.github/workflows/openapi-drift.yml`
- `.github/workflows/docker-publish.yml`

## Platform Fit

ParkHub pilot for a platform-wide pattern: local-first fop attestation as the PR gate, with GitHub as verifier/audit surface. If the pilot works, promote the status context and profile model into fop core.

## Integration Boundary

The stable contract is the commit status context `fop/local-ci/pr`. GitHub branch protection should depend on `Required checks`; `Required checks` depends on the local attestation and cheap remote checks.

## Rollout

1. Pilot in `parkhub-rust`.
2. Require local PR attestation for normal PRs.
3. Move E2E/Lighthouse/OpenAPI remote workflows to main/manual.
4. Stop GitHub container publish/demo deploy from running on every `main` push; keep it for tags/manual fallback.
5. Keep `github-ci-full` label as the emergency remote-full override for full CI and CodeQL.
6. Mirror the pattern to `parkhub-php`, then extract into fop presets.

## Rollback / No-Adopt

Rollback is restoring PR triggers and removing `local-ci-attestation` from `ci.yml` `needs`. Keep the script even if branch protection is rolled back; it remains useful as local preflight.

## Verification

- `fop build --backend local . --preset custom -- bash -lc 'git diff --check && ./.github/scripts/fop-local-ci.sh --profile pr --dry-run'`

## Notes

- Based on current GitHub Actions primitives: workflow concurrency, reusable/lightweight workflows, and required checks remain the correct remote-side controls.
- 2026-05-01 hardening: the `local-ci-attestation` job no longer passes on timeout or combined commit-status success. It only passes after observing the explicit `fop/local-ci/pr` status.
