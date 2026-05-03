# Local CI/CD — script index

> **TL;DR**: `.github/scripts/fop-local-ci.sh --profile pr|full|cd [--background] [--post-status]` is the canonical orchestrator. It mirrors every GHA gate that has a workstation analog. Below is the menu of standalone scripts you can also invoke directly.

ParkHub's CI/CD is **local-first** — the lefthook pre-push hook posts a `fop/local-ci/pr=success` status check via PAT, and GitHub's `local-ci-attestation` job (`.github/workflows/ci.yml:90-207`) waits up to 36.5min for it. If found, all 7 Rust gates SKIP on the PR. Bot/fork PRs (`pull_request.user.type == 'Bot'`) skip the shortcut and run full GHA in parallel.

This doc indexes the 13 standalone `scripts/local-*.sh` mirrors so contributors know what's available.

---

## Quick start

| Want to... | Run |
|------------|-----|
| Verify everything before push | `lefthook run pre-push` (auto on `git push`) |
| Run the local-CI orchestrator manually | `.github/scripts/fop-local-ci.sh --profile full --post-status` |
| Same, in background | `… --background` (returns terminal instantly, posts `fop/local-ci/full` on completion) |
| First-time devcontainer | `devcontainer up` (pulls prebuilt `ghcr.io/nash87/parkhub-rust-devcontainer:latest`) |
| Audit gitea/github workflows | `fop ci-audit .` (auto-runs in fop-local-ci.sh Stage 2) |

---

## Standalone script index

### Container & supply chain

#### `scripts/local-image-scan.sh`
Build container image with podman + scan with trivy + grype. Stamp-cached on `Dockerfile + Cargo.lock + parkhub-web/package-lock.json` SHA so re-runs are fast when nothing changed. Auto-fires on pre-push when those files touch (PR #529).

#### `scripts/local-sbom.sh`
Generate SPDX-JSON SBOM via syft. Optional cosign sign-blob via `FOP_LOCAL_SBOM_SIGN_KEY` (developer key — keyless OIDC has no workstation analog). Wired as Stage 7c of `fop-local-ci.sh` cd profile.

#### `scripts/local-scorecard.sh`
Run OSSF Scorecard via `gcr.io/openssf/scorecard:stable` against the github remote. Auto-detects `nash87/parkhub-rust` from `git remote get-url github`, pulls `GH_TOKEN` from `gh auth token`. Manual invocation only (90+s, network-bound).

#### `scripts/local-reproducibility-check.sh`
SLSA-3 reproducibility: build twice from clean state with deterministic flags (`SOURCE_DATE_EPOCH`, `LC_ALL=C`, `TZ=UTC`), `sha256sum` both binaries. Match → ✓. Mismatch → diffoscope (if installed) for byte-level diff.

### Test & coverage

#### `scripts/local-e2e.sh`
Multi-browser Playwright E2E (chromium / firefox / webkit / mobile-chrome). Builds server with `full+headless+e2e-bypass` features, starts on :8081, runs the full suite. Supports `--project`, `--grep`, `--ui` flags.

#### `scripts/local-visual-regression.sh`
Playwright snapshot regression — local runs are the canonical reference (GHA is nightly + advisory because runner antialiasing differs). `--update-snapshots` rebases all snapshots after intentional UI changes.

#### `scripts/local-mutants.sh`
`cargo-mutants` against parkhub-common (workflow's v1 scope). Outputs `.fop/reports/mutants-<sha>/`. Surfaces escaped mutants (coverage gaps) but doesn't gate.

#### `scripts/local-fuzz-smoke.sh`
`cargo-fuzz` against the two parkhub fuzz targets (`jwt_parse`, `webhook_hmac`) for `FUZZ_SECONDS` each (default 60s). Skips silently if rustup nightly + cargo-fuzz not installed.

#### `scripts/local-coverage.sh`
Unified Rust + frontend coverage report. `cargo-llvm-cov` for the workspace + `vitest --coverage` for parkhub-web. JSON to `.fop/reports/coverage-<sha>/` + optional `--html`. Quick-look summary at end of run.

### Performance & UX

#### `scripts/local-lighthouse.sh`
Lighthouse CI runner — builds parkhub-web, starts astro preview on :18182, runs `npx lhci autorun` against `lighthouserc.json` (asserts INP threshold per #510). Memory-floor gated at 6 GiB (configurable via `FOP_LIGHTHOUSE_MIN_MEM_GIB`).

### Release path

#### `scripts/local-install-smoke.sh`
Cold-clone the local working tree to /tmp, run `podman compose up -d`, poll `/health/ready`, teardown. Verifies the docker-compose path documented in INSTALLATION.md still works.

#### `scripts/local-release-rehearsal.sh`
Mirrors release-rehearsal.yml. Validate-tag + headless+client builds + Syft SBOM + optional cosign sign-blob — WITHOUT pushing tags or creating a GitHub release. Use before `git push --tags v*`.

### Workflow audits

#### `scripts/local-workflow-drift.sh`
Compares `.gitea/workflows/*` to `.github/workflows/*`, flags missing files (gating) + trigger/job-key drift (advisory). Documented EXEMPT lists per side. Auto-fires on pre-push when workflow files touch (PR #529).

---

## Architecture invariants (for future tabs)

1. **`fop ci doctor [--fix]`** regenerates `.fop/local-ci.toml` (gitignored).
2. **`.github/scripts/fop-local-ci.sh`** is the canonical orchestrator. Profiles: `pr` (fast diff-aware), `full` (+ openapi drift, playwright chromium, helm, cargo-audit), `cd` (+ image scan, SBOM, install-smoke, release preflight). `--post-status` publishes `fop/local-ci/{profile}=success` via PAT; `--background` re-execs in detached subshell.
3. **GitHub `local-ci-attestation` job** waits for `fop/local-ci/pr=success` and skips Rust gates if found. Bot/fork PRs run full GHA.
4. **`.devcontainer/`** ships every binary at GHA-pinned versions. `devcontainer.json` defaults to the prebuilt image at `ghcr.io/nash87/parkhub-rust-devcontainer:latest` (published by `.github/workflows/devcontainer-publish.yml` weekly + on `.devcontainer/**` change).
5. **`fop ci-audit .`** flags workflow issues across both `.github` + `.gitea` sides. Auto-runs in Stage 2 of `fop-local-ci.sh`.

## Memory

Comprehensive context lives in `~/.claude/projects/-var-home-florian/memory/project_parkhub_cicd_sota_2026_audit_2026_05_03.md` — read first in any new tab working on parkhub-rust CI/CD.

The phoneos handoff equivalent lives in **fop task T-2523** ("Mirror parkhub-rust SOTA-2026 CI/CD + dev container setup to phoneos (securanido-mobile)") — fresh phoneos tab can claim + execute.
