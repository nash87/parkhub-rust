# Contributing to ParkHub Rust

Thank you for your interest in contributing to ParkHub Rust.

---

## Table of Contents

- [Development Setup](#development-setup)
- [Running Tests](#running-tests)
- [Code Style](#code-style)
- [Project Structure](#project-structure)
- [Pull Request Process](#pull-request-process)
- [Reporting Bugs](#reporting-bugs)
- [Security Vulnerabilities](#security-vulnerabilities)

---

## Development Setup

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.83+ | `rustup update stable` |
| Node.js | 22+ | [nodejs.org](https://nodejs.org) |
| npm | 10+ | bundled with Node.js |
| Docker | optional | for integration testing |

### Clone and build

```bash
git clone https://github.com/nash87/parkhub
cd parkhub

# Build the React frontend
cd parkhub-web
npm ci
npm run build
cd ..

# Build the full Rust workspace
cargo build
```

### Run in development mode

Start the backend:

```bash
cargo run --package parkhub-server -- --headless --unattended --debug --port 7878
```

Flags explained:
- `--headless` — no GUI, console mode
- `--unattended` — auto-configure with defaults (admin/admin, no encryption, no TLS)
- `--debug` — verbose logging
- `--port 7878` — avoid conflict with other local services

Start the frontend with hot reload (in a separate terminal):

```bash
cd parkhub-web
npm run dev
```

The Vite dev server proxies `/api/*` requests to `http://localhost:7878`.
Open `http://localhost:5173` in your browser.

### One-command dev start (with `cargo-watch`)

```bash
cargo install cargo-watch
cargo watch -x 'run --package parkhub-server -- --headless --unattended --debug'
```

---

## Running Tests

### Rust unit and integration tests

```bash
# Run all tests in the workspace
cargo test

# Run tests for a specific package
cargo test --package parkhub-server
cargo test --package parkhub-common

# Run a specific test by name
cargo test test_booking_creation

# Show println! output (for debugging test failures)
cargo test -- --nocapture

# Run with verbose output
cargo test -- --test-output immediate
```

### Frontend type checking and linting

```bash
cd parkhub-web

# TypeScript type checking (no emit)
npm run type-check

# ESLint
npm run lint

# Production build (catches type errors and bundler warnings)
npm run build
```

### Run all checks before submitting a PR

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
cd parkhub-web && npm run build
```

---

## Code Style

### Rust

- Format with `rustfmt` before every commit: `cargo fmt --all`
- Pass `clippy` with zero warnings: `cargo clippy --workspace -- -D warnings`
- All public functions and modules must have doc comments (`///` or `//!`)
- Do not use `unwrap()` in production code — use `?`, `anyhow`, or explicit `match`
- Security-sensitive operations (password hashing, token generation) must use `OsRng`
- No `unsafe` blocks except in proven FFI/performance-critical paths with a comment explaining why

```bash
# Check formatting
cargo fmt --all -- --check

# Fix formatting
cargo fmt --all

# Run linter
cargo clippy --workspace -- -D warnings
```

### TypeScript / React

- TypeScript strict mode is already configured in `tsconfig.json` — do not relax it
- Functional components only (no class components)
- Every interactive element must have an `aria-label` or be associated with a `<label>`
- No inline styles — use Tailwind CSS utility classes
- State: use React hooks (`useState`, `useReducer`). No external state management library is needed.

---

## Project Structure

```
parkhub/
  parkhub-common/           Shared types and protocol definitions (no I/O, no async)
    src/
      lib.rs                Re-exports
      models.rs             All domain models: User, ParkingLot, ParkingSlot, Booking, Vehicle
      protocol.rs           API request/response types
      error.rs              Shared error types

  parkhub-server/           Axum HTTP server
    src/
      main.rs               Startup, CLI parsing, TLS setup, mDNS, GUI integration
      api.rs                All HTTP routes (public + protected + admin)
      config.rs             ServerConfig struct and TOML serialization
      db.rs                 redb database layer (all CRUD operations)
      auth.rs               Session management helpers
      tls.rs                TLS certificate generation and loading
      rate_limit.rs         Governor-based rate limiting
      metrics.rs            Prometheus metrics initialization
      discovery.rs          mDNS service registration
      audit.rs              Audit log write helpers
      health.rs             Health check handlers
      static_files.rs       Embedded React SPA serving (fallback handler)
      validation.rs         Input validation helpers
      openapi.rs            utoipa OpenAPI spec definition
      error.rs              AppError type and IntoResponse implementation

  parkhub-web/              React 19 frontend
    src/
      api/client.ts         Typed API client (all fetch calls go through here)
      context/              React contexts (AuthContext)
      components/           Reusable components (ParkingLotGrid, LotLayoutEditor, ...)
      pages/                Page components (Dashboard, Book, Bookings, Vehicles, Admin, ...)

  legal/                    German legal document templates (Markdown)
  screenshots/              Application screenshots
  docs/                     Documentation (this directory)
```

---

## Pull Request Process

1. **Fork** the repository and create a feature branch from `main`:

   ```bash
   git checkout -b feature/my-feature-name
   ```

   Branch naming conventions:
   - `feature/` — new functionality
   - `fix/` — bug fixes
   - `docs/` — documentation only
   - `refactor/` — code changes without behavior change
   - `security/` — security fixes (coordinate via responsible disclosure first)

2. **Write tests** for any new functionality. PRs without tests for new code will be asked
   to add coverage before merging.

3. **Run the full check suite**:

   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   cd parkhub-web && npm run build
   ```

4. **Commit messages** — use the imperative mood, reference issues:

   ```
   Add vehicle color validation
   Fix slot double-booking race condition (#42)
   Update GDPR export to include booking notes
   ```

5. **Open a PR** against `main`. Fill in the template:
   - What does this change do and why?
   - How was it tested?
   - Are there breaking changes (API, config schema, database format)?
   - Does it affect GDPR or legal compliance behaviour?

6. **Database schema changes**: Adding a new redb table is safe. Removing or renaming an
   existing table requires a migration helper in `db.rs` and documentation in the PR.

7. **API changes**: Must be backwards-compatible within a minor version. Breaking changes
   require bumping `PROTOCOL_VERSION` in `parkhub-common/src/lib.rs` and documentation in
   `docs/CHANGELOG.md` under a new `[Unreleased]` section.

8. **One approving review** is required before merging.

---

## Adding a New API Endpoint

1. Add the route in `api.rs` (`create_router` function) to either `public_routes` or `protected_routes`
2. Implement the handler function in `api.rs`
3. Add any new request/response types to `parkhub-common/src/protocol.rs` or `models.rs`
4. Implement the database operation in `db.rs`
5. Add the endpoint to the OpenAPI spec in `openapi.rs`
6. Write a test in `tests/` or as a unit test in the module
7. Document the endpoint in `docs/API.md`

---

## Reporting Bugs

Open a GitHub issue with:

- ParkHub version: `./parkhub-server --version`
- Operating system and deployment method (Docker, bare metal, etc.)
- Steps to reproduce
- Expected behaviour vs actual behaviour
- Relevant log output: `RUST_LOG=debug ./parkhub-server --headless --debug`

---

## Security Vulnerabilities

See [SECURITY.md](SECURITY.md). Do **not** open a public GitHub issue for security vulnerabilities.
Use the GitHub Security Advisory process instead.

---

## License

By contributing, you agree that your contributions will be licensed under the MIT License,
the same license as the project.

See [LICENSE](../LICENSE).
