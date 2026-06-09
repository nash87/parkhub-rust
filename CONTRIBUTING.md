# Contributing to ParkHub Rust

Thank you for considering a contribution to ParkHub Rust! This guide covers everything
you need to get from zero to a merged pull request.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Good First Issues](#good-first-issues)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Module System (Feature Flags)](#module-system-feature-flags)
- [Running Tests](#running-tests)
- [Code Style](#code-style)
- [Local CI](#local-ci)
- [SOTA-2026 mirrors](#sota-2026-mirrors-prs-512543)
- [Pull Request Guidelines](#pull-request-guidelines)
- [Adding a New API Endpoint](#adding-a-new-api-endpoint)
- [Reporting Bugs](#reporting-bugs)
- [Security Vulnerabilities](#security-vulnerabilities)

---

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
By participating you agree to uphold a welcoming and respectful environment for everyone.
Unacceptable behaviour can be reported by opening a **private** GitHub Security Advisory or
by contacting the maintainers directly.

---

## Good First Issues

Looking for a place to start? Check the
[`good first issue`](https://github.com/nash87/parkhub-rust/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
label on GitHub.

Typical entry-point tasks:

| Area | Examples |
|------|---------|
| Documentation | Improve inline doc comments, fix typos, extend `docs/` |
| Frontend | Add missing `aria-label` attributes, improve keyboard navigation |
| Tests | Add edge-case unit tests for validation helpers or API handlers |
| Translations | Add or correct entries in `parkhub-web/src/i18n/` locales |
| Tooling | Improve Dockerfile, dev-compose, or CI configuration |

Feel free to **comment on an issue before opening a PR** so we can discuss scope and avoid
duplicate work.

---

## Spec-Driven Development

For non-trivial features and API changes, we follow a spec-first workflow before
writing implementation code. The goal is to align on requirements and design early —
before a large PR is opened — so review stays focused on correctness rather than scope.

**When a spec is required:**

- PRs labeled `feature` or `enhancement`
- Any new API endpoint or breaking change to an existing one
- Changes that affect the PHP parity contract (`docs/openapi-parity.md`)

**When a spec is not required:**

- Bug fixes
- Documentation updates
- Dependency bumps
- Tooling and CI changes

**The workflow:**

1. Create `specs/<feature-id>/spec.md` using the template in
   [`.specify/templates/spec.md`](.specify/templates/spec.md).
   Open a draft PR so maintainers can comment on requirements.
2. After the spec is agreed, add `specs/<feature-id>/plan.md` with the technical design.
3. Use `specs/<feature-id>/tasks.md` to track implementation slices.
4. Link the spec from the PR template's "Spec reference" field.

The [`specs/`](specs/) directory contains a full index and the workflow description.
The [`.specify/memory/constitution.md`](.specify/memory/constitution.md) captures the
binding engineering principles for this project.

For large features, the `/speckit.specify` slash command in Claude Code or Copilot
can scaffold the three spec files from the templates automatically.

---

## Development Setup

### Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust (stable) | 1.94+ | See `rust-toolchain.toml` |
| Node.js | 22 | [nodejs.org](https://nodejs.org) |
| npm | 10 | bundled with Node.js |
| Docker | any recent | optional, for integration testing |

### Clone and build

```bash
git clone https://github.com/nash87/parkhub-rust
cd parkhub-rust

# 1. Build the React frontend (must exist before the server compiles)
cd parkhub-web
npm ci
npm run build
cd ..

# 2. Build the headless server (recommended for development)
cargo build --package parkhub-server --no-default-features --features headless

# 3. Full workspace build (requires GUI deps — Linux needs glib-2.0)
cargo build
```

### Run in development mode

Start the backend in headless/unattended mode on a non-default port:

```bash
cargo run --package parkhub-server --no-default-features --features headless \
  -- --headless --unattended --debug --port 7878
```

Flag reference:

| Flag | Effect |
|------|--------|
| `--headless` | Console-only mode (no Slint GUI) |
| `--unattended` | Auto-configure defaults (admin/admin, no encryption, no TLS) |
| `--debug` | Verbose tracing output |
| `--port 7878` | Avoid conflicts with other local services |

Start the frontend with hot-reload (separate terminal):

```bash
cd parkhub-web
npm run dev          # http://localhost:5173, proxies /api/* → localhost:7878
```

### One-command hot-reload (with `cargo-watch`)

```bash
cargo install cargo-watch
cargo watch -x 'run --package parkhub-server --no-default-features --features headless \
  -- --headless --unattended --debug'
```

---

## Project Structure

```
parkhub-rust/
├── Cargo.toml                 # Workspace root (parkhub-common, parkhub-server, parkhub-client)
├── Dockerfile                 # Multi-stage: frontend build → cargo-chef → runtime (~40 MB)
├── docker-compose.yml         # Single-service compose with volume + health check
│
├── parkhub-common/            # Shared library — no I/O, no async
│   └── src/
│       ├── lib.rs             # Protocol version, default port, mDNS service type
│       ├── models.rs          # ~50 domain structs (User, Booking, ParkingLot, …)
│       ├── protocol.rs        # API request/response envelope types
│       └── error.rs           # Shared error types
│
├── parkhub-server/            # Axum HTTP server
│   └── src/
│       ├── main.rs            # CLI, config, DB init, startup, optional GUI
│       ├── api/               # HTTP handlers (auth, bookings, lots, admin, …)
│       ├── db.rs              # redb database layer (20+ tables, AES-256-GCM encryption)
│       ├── jwt.rs             # Access/refresh token generation & validation
│       ├── audit.rs           # Audit log recording
│       ├── error.rs           # AppError → HTTP status + JSON body
│       └── integration_tests.rs  # ~873-line integration test suite
│
├── parkhub-client/            # Desktop client crate (Slint UI)
│
├── parkhub-web/               # React 19 + Astro 6 frontend
│   ├── src/
│   │   ├── api/client.ts      # Typed fetch wrapper (all server calls go here)
│   │   ├── context/           # Auth, Theme, Features, UseCase providers
│   │   ├── components/        # Shared UI components
│   │   └── views/             # 20+ page views
│   ├── e2e/                   # Playwright E2E test specs
│   └── src/**/*.test.{ts,tsx} # Vitest unit/component tests
│
├── config/                    # config.toml template, dev-users.json fixtures
├── docs/                      # API.md, GDPR.md, SECURITY.md, INSTALLATION.md, …
├── legal/                     # German legal document templates
└── scripts/                   # seed_demo.py, smoke-test.sh, docker-entrypoint.sh
```

---

## Module System (Feature Flags)

ParkHub Rust uses Cargo feature flags to compile optional modules in or out of the binary.
This keeps the minimum headless deployment small while allowing the full edition to include
every feature.

### Key feature sets

| Feature | Description |
|---------|-------------|
| `default` | `gui` + `full` — everything enabled including the Slint desktop GUI |
| `headless` | Marker flag: disables GUI, suitable for Docker/server deployments |
| `full` | Enables all `mod-*` optional modules |

### Individual `mod-*` flags (selected examples)

| Feature flag | What it enables |
|-------------|-----------------|
| `mod-bookings` | Core booking CRUD |
| `mod-vehicles` | Vehicle registry |
| `mod-calendar` | Calendar view API |
| `mod-recurring` | Recurring bookings |
| `mod-guest` | Guest booking flow |
| `mod-waitlist` | Waitlist management |
| `mod-credits` | Credit/quota system |
| `mod-email` | SMTP email transport |
| `mod-webhooks` | Outgoing webhooks |
| `mod-push` | Web Push (VAPID) |
| `mod-export` | CSV/data export |
| `mod-analytics` | Usage analytics |
| `mod-oauth` | OAuth2 login |
| `mod-stripe` | Stripe payments |
| `mod-multi-tenant` | Multi-tenant support |

### Building with a custom set of features

```bash
# Headless server with only booking and vehicle support
cargo build --package parkhub-server \
  --no-default-features \
  --features "headless,mod-bookings,mod-vehicles"

# Run tests for a specific module (headless)
cargo test --package parkhub-server \
  --no-default-features --features headless

# Run calendar-specific tests
cargo test --package parkhub-server \
  --no-default-features --features "headless,mod-calendar" \
  -- calendar
```

> **Important**: `cargo test --workspace` requires `glib-2.0` (Slint GUI dep) on Linux.
> Use `--no-default-features --features headless` when running server tests in CI or
> without a GUI environment.

---

## Running Tests

### Rust unit and integration tests

```bash
# Recommended: server tests in headless mode (no GUI deps required)
cargo test --package parkhub-server --no-default-features --features headless

# Shared library tests
cargo test --package parkhub-common

# Run a specific test by name
cargo test --package parkhub-server \
  --no-default-features --features headless \
  -- test_booking_creation

# Show println! output
cargo test -- --nocapture
```

### Frontend unit tests (Vitest)

```bash
cd parkhub-web

# Run all unit/component tests once
npx vitest run

# Watch mode (re-runs on file save)
npx vitest

# Coverage report
npx vitest run --coverage
```

### Frontend E2E tests (Playwright)

```bash
cd parkhub-web

# Install browsers on first run
npx playwright install --with-deps

# Run all E2E specs
npx playwright test

# Run a single spec with browser visible
npx playwright test e2e/login.spec.ts --headed

# Show the HTML test report
npx playwright show-report
```

> E2E tests expect the server to be running on `http://localhost:7878`.
> Start it first with `cargo run --package parkhub-server --no-default-features --features headless -- --headless --unattended --port 7878`.

### Integration Tests

Integration tests exercise cross-module interactions and full API lifecycles:

```bash
# Run all integration tests (headless, no GUI deps)
cargo test --package parkhub-server --no-default-features --features headless \
  -- --test-threads=1 integration

# Run the 1-month booking simulation
cargo test --package parkhub-server --no-default-features --features headless \
  -- simulation
```

### Running the Simulation Engine

The booking simulation creates realistic 30-day booking patterns with configurable profiles:

```bash
# Default profile (small office, 10 users)
cargo test --package parkhub-server --no-default-features --features headless \
  -- simulation_small

# Campus profile (50 users, multi-lot)
cargo test --package parkhub-server --no-default-features --features headless \
  -- simulation_campus
```

### Running k6 Load Tests

Performance testing with [k6](https://grafana.com/docs/k6/):

```bash
# Install k6
brew install k6  # macOS
# or: https://grafana.com/docs/k6/latest/set-up/install-k6/

# Run smoke test (quick sanity check)
k6 run tests/load/smoke.js

# Run sustained load test (50 VUs, 5 minutes)
k6 run tests/load/load.js

# Run stress test (100 VUs, 10 minutes)
k6 run tests/load/stress.js

# Run spike test (1 → 200 → 1 VUs)
k6 run tests/load/spike.js

# Custom base URL
K6_BASE_URL=http://localhost:7878 k6 run tests/load/load.js
```

### Running with Docker Compose Test Profile

```bash
docker compose -f docker-compose.yml -f test.yml up -d
cargo test --package parkhub-server --no-default-features --features headless
docker compose -f docker-compose.yml -f test.yml down
```

### Full pre-PR check suite

```bash
# Backend
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --package parkhub-server --no-default-features --features headless
cargo test --package parkhub-common

# Frontend
cd parkhub-web
npm run type-check
npm run lint
npx vitest run
npm run build
```

---

## Code Style

### Rust

- **Format** with `rustfmt` before every commit: `cargo fmt --all`
- **Zero Clippy warnings**: `cargo clippy --workspace -- -D warnings`
- All public functions and modules must have doc comments (`///` / `//!`)
- Do **not** use `unwrap()` in production paths — use `?`, `anyhow`, or `match`
- Security-sensitive operations (password hashing, token generation) must use `OsRng`
- `unsafe` blocks are only permitted in proven FFI / performance-critical paths with
  an explanatory comment

```bash
cargo fmt --all           # fix formatting
cargo fmt --all -- --check  # check only (CI)
cargo clippy --workspace -- -D warnings
```

### TypeScript / React

- TypeScript **strict mode** is enabled — do not relax `tsconfig.json`
- Functional components only (no class components)
- Every interactive element must have an `aria-label` or be associated with a `<label>`
- No inline styles — use Tailwind CSS utility classes
- State via React hooks (`useState`, `useReducer`). No external state library.

### Commit messages

Use the imperative mood; reference issue numbers where applicable:

```
Add vehicle color validation
Fix slot double-booking race condition (#42)
Update GDPR export to include booking notes
docs: add CONTRIBUTING.md with development setup and PR guidelines
```

Prefix suggestions for clarity:

| Prefix | When to use |
|--------|------------|
| `feat:` | New user-facing feature |
| `fix:` | Bug fix |
| `docs:` | Documentation only |
| `refactor:` | No behaviour change |
| `test:` | Adding or updating tests |
| `chore:` | Tooling, deps, CI |
| `security:` | Security fix (coordinate via private advisory first) |

---

## Local CI

We run a local-first CI workflow on top of the GitHub Actions pipeline.
The goal: catch 90 %+ of CI failures before you push, replacing the
15-30 minute remote feedback loop with a near-instant local one.

### Cautionary tale

A single PR once landed with a 12-fail compile cascade because 15
`User { ..settings: None }` field initializers were missing. `cargo
check` would have caught it in five seconds. Local CI exists so that
class of failure is impossible to push.

### One-time setup

After cloning the repo and installing the Rust toolchain + Node deps,
install the git hooks:

```bash
npm install                   # parkhub-web deps (node_modules)
cargo build                   # warm cargo + Slint cache
npx lefthook install          # install pre-commit + pre-push hooks
```

The first `cargo build` after cloning surfaces a yellow `cargo:warning`
reminding you to install the hooks. Once installed, the warning
disappears.

### What runs and when

| Hook | Speed | Gates |
|------|-------|-------|
| `pre-commit` | < 5 s | `cargo fmt --check`, ESLint (if configured), `tsc --noEmit` |
| `pre-push` | 1-3 min cached | `cargo check`, `cargo clippy`, `cargo test --lib`, `vitest --changed`, OpenAPI drift, ts-rs types drift |

The pre-push gate set is the local mirror of the `Required checks`
job in `.github/workflows/ci.yml`. If pre-push is green, GitHub CI
should be green too.

### Replay locally without committing or pushing

```bash
lefthook run pre-commit       # fast, only on staged files
lefthook run pre-push         # full gates
```

You can also run individual scripts directly:

```bash
./scripts/check-openapi-drift.sh
./scripts/check-types-drift.sh
```

### Drift-gate quick reference

If `openapi-drift` or `types-drift` fails, regenerate locally and
commit the result:

```bash
# OpenAPI drift — start the server, then dump the spec:
cargo run --no-default-features --features 'full,headless' \
    -p parkhub-server -- --headless --port 18181
# (in another terminal)
./scripts/dump-openapi.sh 18181
git add docs/openapi/rust.json

# ts-rs TypeScript types drift:
cargo test --features gen-types -p parkhub-server --test ts_export -- --nocapture
git add parkhub-web/src/generated/
```

### SOTA-2026 mirrors (PRs #512–#543)

Beyond the lefthook gate, the repo ships **13 standalone `scripts/local-*.sh` mirrors** that cover every GHA gate with a workstation analog. Full index in [`docs/local-ci.md`](docs/local-ci.md). Quick orientation:

| Want to… | Run |
|----------|-----|
| Verify everything before push | `lefthook run pre-push` |
| Run the local-CI orchestrator manually | `.github/scripts/fop-local-ci.sh --profile full --post-status` |
| Same, in background (terminal returns instantly) | `… --background` (posts `fop/local-ci/full` status when complete) |
| First-time devcontainer | `devcontainer up` (pulls prebuilt `ghcr.io/nash87/parkhub-rust-devcontainer:latest`) |
| Audit gitea/github workflow drift | `./scripts/local-workflow-drift.sh` |
| Container image vuln scan | `./scripts/local-image-scan.sh` (stamp-cached) |
| Lighthouse perf + a11y | `./scripts/local-lighthouse.sh` |
| Coverage report (Rust + frontend) | `./scripts/local-coverage.sh [--html]` |
| SLSA-3 reproducibility check | `./scripts/local-reproducibility-check.sh` |
| Release-tag preflight | `./scripts/local-release-rehearsal.sh --tag v5.0.x` |
| Multi-browser E2E suite | `./scripts/local-e2e.sh [--project chromium]` |
| Build-time waterfall HTML | `./scripts/local-build-trace.sh` |
| SBOM (SPDX-JSON) generation | `./scripts/local-sbom.sh` |
| OSSF Scorecard | `GH_TOKEN=$(gh auth token) ./scripts/local-scorecard.sh` |
| Mutation testing | `./scripts/local-mutants.sh` |
| Fuzz smoke (jwt + webhook) | `./scripts/local-fuzz-smoke.sh` |
| Install smoke (docker-compose path) | `./scripts/local-install-smoke.sh` |

**Trust-inversion architecture**: `scripts/post-attestation-deferred.sh` (called from `lefthook.yml`) posts a `fop/local-ci/pr=success` status check via PAT after pre-push gates pass. GitHub's `local-ci-attestation` job (`ci.yml:90-207`) waits up to 36.5 min for it; if found, **all 7 Rust gates skip on the PR** ("LOCAL-FIRST: skipped on PR; covered by fop/local-ci/pr"). Bot/fork PRs (Dependabot, Copilot SWE Agent) skip the local-CI shortcut and run the full GHA suite in parallel.

**Dev container**: `.devcontainer/Containerfile` ships every binary at GHA-pinned versions (rust 1.94.1, node 22, helm v3.18.4, trivy 0.59.0, grype 0.91.0, syft, gitleaks 8.30.1, actionlint 1.7.12, cargo-audit/deny/geiger, typos, zizmor, lighthouse v0.13, lefthook, yamllint). After `devcontainer up`, `fop-local-ci.sh --profile full --post-status --background` runs the entire local CI ladder.

### Bypassing in emergencies

For a true emergency (e.g. broken upstream dep blocking a hotfix), you
can bypass the gates:

```bash
git push --no-verify
```

This is logged to your local `lefthook.log` and is frowned upon —
please open a follow-up issue describing why the gate was bypassed.
Per-gate bypasses also exist for the slow drift scripts:

```bash
SKIP_OPENAPI_DRIFT=1 git push
SKIP_TYPES_DRIFT=1 git push
```

### Merge Queue

We use **GitHub's native merge queue** (no third-party SaaS, no
push-access from external services). Activate it once via:

> Settings → Branches → Branch protection rules → `main` →
> "Require merge queue" → Enable.

While the queue is active, GitHub batches PRs into a serialised lane,
re-runs required checks against the merge candidate, and merges when
green. No `.mergify.yml` or other config in the repo is required.

### Auto-merge

Add the **`auto-merge`** label to a PR and the workflow at
`.github/workflows/auto-merge.yml` enables GitHub's native auto-merge
(squash strategy). The PR will then merge itself once:

- All required status checks pass (Required checks, CodeQL).
- All required reviews are approved.
- The PR is not in draft state.

Auto-merge respects branch protection — it does not bypass any gate.
To cancel, remove the label or click "Disable auto-merge" on the PR.

Repo prerequisites (one-time):

1. Settings → General → Pull Requests → "Allow auto-merge" enabled.
2. Branch protection on `main` lists `Required checks` (and CodeQL,
   if enabled) as required status checks.

### Operator action required (one-time, post-merge)

Three pieces of post-merge configuration cannot be expressed purely in
repo files:

1. **Lefthook hooks per contributor** — every contributor must run
   `npx lefthook install` on first clone. The cargo build script
   prints a reminder; documenting it here keeps it discoverable.

2. **Allow auto-merge in repo settings** — Settings → General → Pull
   Requests → "Allow auto-merge" must be enabled for the auto-merge
   workflow to take effect.

3. **Branch protection alignment** — branch protection on `main`
   should list `Required checks` (matching the aggregator job in
   `.github/workflows/ci.yml`) as a required status check, with
   "Require merge queue" enabled if you want serialised merges.

---

## Pull Request Guidelines

1. **Fork** the repository and create a feature branch from `main`:

   ```bash
   git checkout -b feature/my-feature-name
   ```

   Branch naming:
   - `feature/` — new functionality
   - `fix/` — bug fixes
   - `docs/` — documentation only
   - `refactor/` — code changes without behaviour change
   - `security/` — security fixes (coordinate via private advisory first)

2. **Write tests** for any new functionality. PRs without tests for new code will be
   asked to add coverage before merging.

3. **Run the full check suite** (see [Running Tests](#running-tests)) before pushing.

4. **Open a PR against `main`** and fill in the PR template:
   - What does this change do and why?
   - How was it tested?
   - Are there breaking changes (API, config schema, database format)?
   - Does it affect GDPR or legal compliance behaviour?

5. **Database schema changes**: Adding a new redb table is safe. Removing or renaming
   an existing table requires a migration helper in `db.rs` and documentation in the PR.

6. **API changes**: Must be backwards-compatible within a minor version. Breaking changes
   require bumping `PROTOCOL_VERSION` in `parkhub-common/src/lib.rs` and a `[Unreleased]`
   entry in `docs/CHANGELOG.md`.

7. **One approving review** is required before merging.

---

## Adding a New API Endpoint

1. Add the route in `api/mod.rs` (`create_router`) to `public_routes` or `protected_routes`
2. Implement the handler function in the appropriate `api/*.rs` file
3. Add request/response types to `parkhub-common/src/protocol.rs` or `models.rs`
4. Implement the database operation in `db.rs`
5. Annotate with `#[utoipa::path(…)]` and register in `openapi.rs`
6. Write a test (unit test in the module or integration test in `integration_tests.rs`)
7. Document the endpoint in `docs/API.md`

---

## Reporting Bugs

Open a [GitHub issue](https://github.com/nash87/parkhub-rust/issues/new) with:

- ParkHub version: `./parkhub-server --version`
- Operating system and deployment method (Docker, bare metal, etc.)
- Steps to reproduce
- Expected vs. actual behaviour
- Relevant log output: `RUST_LOG=debug ./parkhub-server --headless --debug`

---

## Security Vulnerabilities

See [SECURITY.md](SECURITY.md). Do **not** open a public GitHub issue for security
vulnerabilities. Use the
[GitHub Security Advisory](https://github.com/nash87/parkhub-rust/security/advisories/new)
process instead.

---

## License

By contributing you agree that your contributions will be licensed under the
[MIT License](LICENSE), the same license as the project.
