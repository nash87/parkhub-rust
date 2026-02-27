# Contributing to ParkHub

Thank you for your interest in contributing to ParkHub Rust.

---

## Development Setup

### Prerequisites

- **Rust** 1.83 or later: `rustup update stable`
- **Node.js** 22+ and **npm** (for the frontend)
- **Docker** (optional, for integration testing)

### Clone and build

```bash
git clone https://github.com/nash87/parkhub
cd parkhub

# Build the web frontend
cd parkhub-web
npm ci
npm run build
cd ..

# Build the Rust workspace
cargo build
```

### Run in development

Start the backend (headless, auto-configure, no encryption for speed):

```bash
cargo run --package parkhub-server -- --headless --unattended --debug --port 7878
```

Start the frontend in hot-reload mode:

```bash
cd parkhub-web
npm run dev
```

The frontend dev server proxies `/api/*` requests to `http://localhost:7878`.

---

## Running Tests

### Rust unit and integration tests

```bash
# Run all tests in the workspace
cargo test

# Run tests for a specific package
cargo test --package parkhub-server
cargo test --package parkhub-common

# Run a specific test
cargo test test_config_save_load

# Run with output shown (for debugging)
cargo test -- --nocapture
```

### Frontend type checking and linting

```bash
cd parkhub-web
npm run type-check   # TypeScript type checking
npm run lint         # ESLint
npm run build        # Full production build (catches type errors)
```

---

## Code Style

### Rust

- Format with `rustfmt` before every commit: `cargo fmt --all`
- Pass `clippy` with no warnings: `cargo clippy --workspace -- -D warnings`
- Every public function and module must have a doc comment (`///` or `//!`)
- Avoid `unwrap()` in production code — use `?`, `anyhow`, or explicit error handling
- Security-sensitive operations (password hashing, token generation) must use `OsRng`

```bash
# Check and fix formatting
cargo fmt --all

# Run linter
cargo clippy --workspace -- -D warnings
```

### TypeScript / React

- Use TypeScript strict mode (already configured in `tsconfig.json`)
- Functional components only (no class components)
- Accessibility: every interactive element must have an `aria-label` or be associated with a
  visible `<label>`. Follow the existing ARIA patterns in the pages
- No inline styles — use Tailwind CSS utility classes

---

## Project Structure

```
parkhub/
  parkhub-common/        Shared types and protocol definitions (no I/O)
    src/
      lib.rs             Re-exports
      models.rs          All data models (User, Booking, Vehicle, ...)
      protocol.rs        API request/response types, WebSocket messages
      error.rs           Shared error types

  parkhub-server/        Axum HTTP server
    src/
      main.rs            Startup, CLI parsing, TLS, mDNS, GUI integration
      api.rs             All HTTP routes (public + protected)
      config.rs          ServerConfig struct and TOML serialization
      db.rs              redb database layer (CRUD operations)
      auth.rs            Session management helpers
      jwt.rs             Token utilities
      tls.rs             Certificate generation and loading
      rate_limit.rs      Governor-based rate limiting
      metrics.rs         Prometheus metrics initialization
      discovery.rs       mDNS service registration
      audit.rs           Audit log writes
      health.rs          Health check helpers
      static_files.rs    Embedded React SPA serving
      validation.rs      Input validation helpers
      openapi.rs         utoipa OpenAPI spec definition
      error.rs           AppError type and IntoResponse impl

  parkhub-web/           React 19 frontend
    src/
      api/client.ts      Typed API client (all fetch calls)
      context/           React contexts (AuthContext)
      components/        Reusable components (ParkingLotGrid, LotLayoutEditor, ...)
      pages/             Page components (Dashboard, Book, Bookings, Vehicles, Admin, ...)

  legal/                 German legal document templates (Markdown)
  screenshots/           Application screenshots
  docs/                  This documentation
```

---

## Pull Request Process

1. **Fork** the repository and create a feature branch from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Write tests** for new functionality. PRs without tests for new code will be asked
   to add coverage before merging.

3. **Run the full check suite** before opening a PR:
   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   cd parkhub-web && npm run build
   ```

4. **Commit messages** should be clear and concise. Use the imperative mood:
   - "Add vehicle color validation" not "Added vehicle color validation"
   - Reference issues where relevant: "Fix slot double-booking race condition (#42)"

5. **Open a PR** against `main`. Fill in the PR template:
   - What does this change do?
   - How was it tested?
   - Are there any breaking changes?
   - Does it affect the API, config schema, or database format?

6. **Database schema changes** must be backwards-compatible or include a migration path.
   The redb schema uses named tables — adding a new table is safe; removing or renaming
   an existing table requires a migration helper.

7. **API changes** must be backwards-compatible within a minor version. If you need to
   break the API, update `PROTOCOL_VERSION` in `parkhub-common/src/lib.rs` and document
   the change in `docs/CHANGELOG.md`.

---

## Reporting Bugs

Open a GitHub issue with:
- ParkHub version (`parkhub-server --version`)
- Operating system and deployment method (Docker, bare metal, etc.)
- Steps to reproduce
- Expected behavior vs. actual behavior
- Relevant log output (`RUST_LOG=debug`)

For security vulnerabilities, see [SECURITY.md](SECURITY.md) — do not open public issues.

---

## License

By contributing, you agree that your contributions will be licensed under the MIT License,
the same license as the project.
