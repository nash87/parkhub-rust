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

## Development Setup

### Prerequisites

| Tool | Minimum version | Install |
|------|----------------|---------|
| Rust (stable) | 1.94 | See `rust-toolchain.toml` |
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
