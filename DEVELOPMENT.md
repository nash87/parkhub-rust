# parkhub-rust — Developer Guide

This doc covers the local dev loop, the local CI mirror (Makefile + pre-commit + act),
and the GitHub Actions hardening we rely on. It is the companion to the
`.github/workflows/*.yml` files — those workflows remain the source of truth,
everything here exists to reproduce them locally before `git push`.

---

## 1. Quickstart

```bash
# Clone (Gitea = origin, GitHub = github)
git clone git@192.168.178.220:florian/parkhub-rust.git
cd parkhub-rust
git remote add github https://github.com/nash87/parkhub-rust.git

# Bootstrap (rust-toolchain.toml pins 1.94.1 — rustup installs it automatically)
cargo build --locked --package parkhub-common
cd parkhub-web && npm ci && npm run build && cd ..

# Run (headless dev server on :18181)
cargo run --package parkhub-server --no-default-features --features headless -- \
  --headless --port 18181 --data-dir /tmp/parkhub-dev-db
```

Requires: Rust toolchain as pinned in `rust-toolchain.toml` (currently 1.94.1),
Node 22, npm. `pre-commit` (Python) and `act` (Go / Docker) are optional but
recommended.

---

## 2. Pre-commit hooks

We use the [`pre-commit`](https://pre-commit.com) framework. All hook revs in
`.pre-commit-config.yaml` are SHA-pinned — same discipline as the Actions
workflows. Bump with `pre-commit autoupdate --freeze`.

```bash
pip install --user pre-commit
pre-commit install                        # runs on every `git commit`
pre-commit install --hook-type pre-push   # runs clippy on every `git push`
pre-commit run --all-files                # one-off, entire repo
```

Hooks (summary):

| Stage      | Hook                                    | Source                               |
|------------|-----------------------------------------|--------------------------------------|
| pre-commit | trailing-whitespace, end-of-file-fixer  | `pre-commit/pre-commit-hooks@v6.0.0` |
| pre-commit | check-yaml, check-json, check-toml, check-merge-conflict, check-added-large-files | same |
| pre-commit | `cargo fmt --all -- --check`            | local                                |
| pre-commit | `cargo check` (common + server headless) | local                               |
| pre-push   | `cargo clippy -D warnings`              | local                                |

Clippy is `pre-push` only — too slow for every commit.

---

## 3. `make ci` — the core local gate

The Makefile mirrors the **reproducible local subset** of
`.github/workflows/ci.yml`. Run **`make ci` before `git push`** for fast local
feedback, then use `make act` when you need to execute the actual workflow
YAML.

```bash
make ci            # fmt + clippy + check + test + frontend + drift
make lint          # fmt --check + clippy (mirrors fmt + clippy jobs)
make check         # cargo check headless (mirrors check job)
make test          # cargo test headless (mirrors test job)
make integration   # integration suite, -- --test-threads=1
make frontend      # parkhub-web vitest + build (mirrors frontend job)
make drift         # openapi snapshot diff (mirrors openapi-drift.yml)
make pre-push      # alias for make ci
```

`make ci` intentionally covers the fast local checks: formatting, clippy,
headless cargo check/test, frontend build/tests, and OpenAPI drift.
Workflow-only or advisory jobs such as `actionlint` and `integration` still
run in GitHub Actions / `act`.

Note: server targets build with `--no-default-features --features headless`
because `rust_embed` would otherwise fail when `parkhub-web/dist/` is absent.
`make embed` generates a placeholder `index.html` to unblock local builds;
release builds run the real `npm run build` first.

See the comment block at the top of `Makefile` — any target that claims to
mirror a workflow job **must not diverge** from that job. If a workflow job
changes, update the corresponding make target in the same commit.

Shared feature/API changes also need the cross-runtime docs kept in sync:
[docs/parity-governance.md](docs/parity-governance.md),
[docs/openapi-parity.md](docs/openapi-parity.md), and
[docs/release-checklist.md](docs/release-checklist.md).

---

## 4. `act` — run the actual workflows locally

[`nektos/act`](https://github.com/nektos/act) executes the YAML workflows
inside a container. This catches Actions-syntax bugs that `make ci` misses.

```bash
# Install
brew install act                                             # macOS / Linuxbrew
curl -fsSL https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

make act                     # runs .github/workflows/ci.yml
act -W .github/workflows/openapi-drift.yml
act -W .github/workflows/e2e.yml
act -l                       # list every job/workflow
```

`.actrc` (repo root) pins:

- `-P ubuntu-latest=catthehacker/ubuntu:act-latest` — smallest image that
  resolves ~95% of the actions we use (dtolnay/rust-toolchain,
  Swatinem/rust-cache, setup-node, cache, buildx). The full (~15 GB) image is
  overkill; `micro`/`medium` break too many actions.
- `--container-architecture linux/amd64` — QEMU emulation on arm is flaky for
  cargo builds, and our release targets are amd64 anyway.

---

## 5. Dual-remote push convention

Gitea is `origin` (private canonical). GitHub (`nash87/parkhub-rust`) is a
mirror for Actions + visibility.

```bash
git push origin main
git push github main
```

One-liner helper (add to `~/.gitconfig`):

```ini
[alias]
    pa = "!git push origin \"$(git rev-parse --abbrev-ref HEAD)\" && git push github \"$(git rev-parse --abbrev-ref HEAD)\""
```

Then `git pa` pushes both. **Always `git pull --rebase origin main` before
either push** — Flux-style automation may have rewritten tags.

---

## 6. GitHub Pro hardening we leverage

All workflows live in `.github/workflows/` and use these 2025-current
primitives ([docs.github.com/en/actions](https://docs.github.com/en/actions)):

- **SHA-pinned actions** — every `uses:` references a commit SHA with a
  `# v<tag>` comment. Dependabot (Actions ecosystem, weekly) keeps them fresh.
- **Concurrency groups** — every workflow sets
  `concurrency: { group: <workflow>-<ref>, cancel-in-progress: true }` so
  superseded PR pushes auto-cancel. See
  [docs.github.com/.../using-concurrency](https://docs.github.com/en/actions/using-jobs/using-concurrency).
- **Caching** — `Swatinem/rust-cache@v2` for the target dir + registry index
  (keyed on `Cargo.lock` + toolchain), npm via `setup-node@v6`, Playwright
  browsers (`~/.cache/ms-playwright`), and GHA-native BuildKit cache for
  Docker.
- **Artifact retention** — `actions/upload-artifact@v7` with
  `retention-days: 7` for Playwright reports + server logs.
- **CodeQL** — `codeql.yml` scans Rust + JS on every PR.
- **Dependency review** — PRs run `actions/dependency-review-action`, and the
  result is folded into the main `required` gate in `ci.yml`.
- **cargo-deny** — `cargo-deny check` (advisories, licenses, bans, sources)
  now runs directly in `ci.yml` as part of the main required gate. `deny.toml`
  is the source of truth.
- **Desktop client compile check** — `ci.yml` now runs a dedicated
  `parkhub-client` compile gate on Linux PRs so Slint/UI breakage is caught
  before release packaging.
- **Secret scan** — `gitleaks` (MIT) on every PR over the full git history. Replaced trufflehog (AGPL) on 2026-04-25 (#403).
- **Artifact attestations** — `docker/build-push-action@v7` chains
  `actions/attest-build-provenance@v4` to publish SLSA v1 provenance for every
  pushed image. See
  [docs.github.com/.../artifact-attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations).
- **SBOM** — generated per build (Syft via buildx), uploaded alongside the
  provenance attestation.
- **Branch protection** — `main` requires green `required` job and 1 review.
  `required` now aggregates workflow hygiene, Helm validation,
  dependency-review, cargo-deny, OpenAPI drift, and the core build/test jobs;
  integration remains advisory until its known flake is retired. Set in GitHub
  Settings → Branches.
- **Environments** — not wired yet (no external deploy targets on GitHub — we
  deploy from Gitea via Flux). When we do wire them, use GitHub
  [Environments](https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-deployments/managing-environments-for-deployment)
  with required reviewers + wait-timers.
- **Dependency graph** — native (Cargo ecosystem supported since 2024), used
  by Dependabot + dependency-review.

Periodic workflows: `nightly.yml` (extended tests + MSRV check),
`mutants.yml` (cargo-mutants, weekly), `e2e.yml` (Playwright),
`lighthouse.yml` (perf budget).

---

## 7. OpenAPI contract parity

`parkhub-php` and `parkhub-rust` both expose the same HTTP contract. Any
schema change must land in both repos in the same PR window.

- Snapshot: `docs/openapi/rust.json`
- Drift gate: `make drift` (boots headless server on :18181, dumps, diffs)
- Workflow: `.github/workflows/openapi-drift.yml` and the main `ci.yml`
  `openapi-drift` job
- Contract guide: [`docs/openapi-parity.md`](docs/openapi-parity.md)

If CI fails on `openapi-drift`, run `make drift` locally to regenerate and
commit the new `docs/openapi/rust.json`.

---

## 8. Troubleshooting

| Symptom                             | Fix                                                          |
|-------------------------------------|--------------------------------------------------------------|
| `cargo fmt --check` fails           | `cargo fmt --all`, then commit                               |
| `cargo clippy -D warnings` fails    | Fix warnings or use `#[allow(...)]` with a justification comment; globally-allowed lints are listed in the clippy Make target |
| `openapi-drift` fails               | `make drift` regenerates; commit `docs/openapi/rust.json`    |
| `rust_embed` build fails locally    | `make embed` (stubs `parkhub-web/dist/index.html`) or `cd parkhub-web && npm run build` |
| `act` fails but CI is green         | You probably need `--container-architecture linux/amd64` (already in `.actrc`) |
| Pre-commit wants to rewrite files   | It's auto-fixing whitespace/EOL — `git add -u` and commit again |

Always run `make pre-push` before pushing. CI on GitHub is slow; failing
locally is free.
