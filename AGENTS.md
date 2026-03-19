# AGENTS.md — ParkHub Rust

## Overview
Self-hosted parking management platform for enterprises, universities, and residential complexes. Ships as a single Rust binary with an embedded React 19 frontend. Features booking management, GDPR/DSGVO compliance (Art. 15/17), Bitwarden-level encryption at rest, mDNS LAN discovery, and a comprehensive admin dashboard. Open source (MIT), live demo on Render.

## Tech Stack
- Rust (edition 2021), Axum 0.8, Tokio
- Workspace: parkhub-common, parkhub-server, parkhub-client
- redb 2 (embedded single-file database, optional AES-256-GCM encryption)
- React 19 + TypeScript + Tailwind CSS (embedded SPA, built via parkhub-web)
- Argon2id password hashing, rustls TLS 1.3, JWT-style sessions
- mdns-sd for LAN auto-discovery
- utoipa + Swagger UI for OpenAPI docs

## Build
```sh
# Server (headless, no GUI — for Docker/server deployments)
RUSTC_WRAPPER="" RUSTC=/home/florian/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc cargo build --release --package parkhub-server --no-default-features --features headless

# Full build (includes Slint GUI for Windows/desktop)
RUSTC_WRAPPER="" RUSTC=/home/florian/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc cargo build --release

# React frontend (must be built before server if modifying frontend)
cd parkhub-web && npm install && npm run build
```

## Test
```sh
cargo test --workspace          # All workspace tests
cargo test -p parkhub-server    # Server tests only
cargo test -p parkhub-common    # Common crate tests
```

## Lint
```sh
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Architecture
```
parkhub-common/     # Shared types, models, encryption, validation
parkhub-server/     # Axum HTTP server, API routes, database, auth
├── /api/v1/*       # REST API (auth required, RBAC enforced)
├── /metrics        # Prometheus metrics (unauthenticated)
├── /swagger-ui     # OpenAPI 3.0 docs
├── /health*        # K8s liveness + readiness probes
└── /api/v1/legal   # Public legal pages (DDG §5 Impressum)
parkhub-client/     # CLI client / SDK
parkhub-web/        # React 19 SPA (embedded in binary at build time)
```

**Key features**: Booking flow, recurring bookings, guest bookings, waitlist, vehicle registry, RBAC (user/premium/admin/superadmin), GDPR export/erasure, audit logging, announcements, use-case theming.

**Port**: 8080 (default, configurable via `--port` flag or `config.toml`)

## Conventions
- Workspace structure: shared types in parkhub-common, never duplicate
- RBAC enforced on every endpoint — check role in handler
- All write operations must create audit log entries
- GDPR: Art. 17 erasure anonymizes PII but retains booking records per AO §147 (10-year tax retention)
- Single binary deployment — React frontend embedded via `include_dir!` or similar

## Guardrails
- Never commit secrets or .env files
- Never force-push to main
- Never store passwords in plaintext — Argon2id only
- DB passphrase (PARKHUB_DB_PASSPHRASE) enables at-rest encryption — strongly recommended for production
- No external database server dependency — redb is embedded
- Legal templates in `legal/` must be customized by the operator before production use
- MIT license for headless builds; GUI builds include Slint (GPL-3.0 community edition)

## Deploy
```sh
# Docker Compose (recommended for users)
docker compose up -d

# Direct binary
./target/release/parkhub-server --headless --unattended --port 8080

# Container build for registry
podman build --network=host -t parkhub-rust -f Dockerfile .
podman tag parkhub-rust 192.168.178.250:5000/parkhub-rust:latest
podman push 192.168.178.250:5000/parkhub-rust:latest

# Also deployed on Render (demo): https://parkhub-rust-demo.onrender.com
# GitHub repo: https://github.com/nash87/parkhub-rust
```
