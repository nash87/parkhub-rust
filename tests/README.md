# ParkHub Test Suite

## Prerequisites

Build the server binary first:

```bash
cargo build -p parkhub-server --features "full,headless"
```

## Test Levels

### Unit Tests (inline)

```bash
cargo test -p parkhub-server --features full
cargo test -p parkhub-common
```

### Integration Tests

These start a real server process on a random port with a temporary database.
Each test gets a completely fresh environment.

```bash
# Run all integration tests
cargo test -p parkhub-server --test integration integration

# Run a specific suite
cargo test -p parkhub-server --test integration api_contract
cargo test -p parkhub-server --test integration auth_flow
cargo test -p parkhub-server --test integration booking_lifecycle
cargo test -p parkhub-server --test integration webhook_delivery
cargo test -p parkhub-server --test integration gdpr_compliance
cargo test -p parkhub-server --test integration multi_tenant
cargo test -p parkhub-server --test integration rate_limiting
cargo test -p parkhub-server --test integration email_templates
cargo test -p parkhub-server --test integration recurring_booking
cargo test -p parkhub-server --test integration db_migration
```

### Simulation Tests

1-month booking simulations with three profiles: small, campus, enterprise.
Campus and enterprise are `#[ignore]` by default (they take longer).

```bash
# Small profile only (fastest, default)
cargo test -p parkhub-server --test integration simulation_small

# All profiles including ignored
cargo test -p parkhub-server --test integration simulation -- --include-ignored

# Campus profile only
cargo test -p parkhub-server --test integration simulation_campus -- --ignored

# Enterprise profile only
cargo test -p parkhub-server --test integration simulation_enterprise -- --ignored
```

## Test Architecture

```
tests/
  main.rs                     # Test harness entry point
  common/
    mod.rs                    # Shared helpers: start_test_server(), create_test_*()
  integration/
    mod.rs                    # Module declarations
    api_contract.rs           # OpenAPI response shape validation
    auth_flow.rs              # Full auth lifecycle (register/login/refresh/logout)
    booking_lifecycle.rs      # Complete booking CRUD + conflict detection
    webhook_delivery.rs       # Webhook CRUD + HMAC secrets
    gdpr_compliance.rs        # Art. 15 export + Art. 17 erasure
    multi_tenant.rs           # Tenant isolation verification
    rate_limiting.rs          # 429 behavior and recovery
    email_templates.rs        # Email template data availability
    recurring_booking.rs      # Recurring booking CRUD + ownership
    db_migration.rs           # Fresh DB schema verification
  simulation/
    mod.rs                    # Simulation engine + test entry points
    profiles.rs               # Small / Campus / Enterprise configs
    generator.rs              # Realistic data generation
    injector.rs               # API-based data injection
    verifier.rs               # Post-simulation consistency checks
    reporter.rs               # JSON report output
```

## Environment Variables

- `DEMO_MODE=true` — starts with demo data seeded
- `PARKHUB_ADMIN_PASSWORD=Admin123!` — admin password for tests
- `RUST_LOG=warn` — suppress server output during tests

## Key Design Decisions

- Tests start a real server process (not in-process tower::oneshot) to validate
  the full HTTP stack including middleware, rate limiting, and TLS setup
- Each test gets a fresh `tempfile::TempDir` database
- The server runs with `--headless --unattended` and `DEMO_MODE=true`
- Simulation tests use `reqwest` HTTP client, identical to real API consumers
- Rate limit tests may be flaky if the system clock drifts; they include
  retry-aware assertions
