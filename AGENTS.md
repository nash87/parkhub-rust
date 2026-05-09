# AGENTS.md — ParkHub Rust

## Overview
Self-hosted parking management platform for enterprises, universities, and residential complexes. Ships as a single Rust binary with an embedded React 19 frontend. Features booking management, GDPR/DSGVO compliance (Art. 15/17), Bitwarden-level encryption at rest, mDNS LAN discovery, and a comprehensive admin dashboard. Open source (MIT), live demo on Render.

## Tech Stack
- Rust (edition 2024), Axum 0.8, Tokio
- Workspace: parkhub-common, parkhub-server, parkhub-client
- redb 2 (embedded single-file database, optional AES-256-GCM encryption)
- React 19 + TypeScript + Tailwind CSS (embedded SPA, built via parkhub-web)
- Argon2id password hashing, rustls TLS 1.3, JWT-style sessions
- mdns-sd for LAN auto-discovery
- utoipa + Swagger UI for OpenAPI docs

## Build
```sh
# Default build (pure MIT, headless, no GUI — for Docker/server deployments)
# `./scripts/fop-wrap.sh` runs through fop's queue with the
# `interactive-small` resource profile when fop is on PATH; otherwise it
# runs the command directly so contributors without fop installed can
# still build.
./scripts/fop-wrap.sh cargo build --release --package parkhub-server

# Optional desktop GUI build (pulls Slint, GPL-3.0)
./scripts/fop-wrap.sh cargo build --release --package parkhub-server --features gui

# React frontend (must be built before server if modifying frontend)
cd parkhub-web && npm install && npm run build
```

## Test
```sh
fop test . -- --workspace          # All workspace tests
fop test . -- -p parkhub-server    # Server tests only
fop test . -- -p parkhub-common    # Common crate tests
```

## Lint
```sh
fop clippy .
./scripts/fop-wrap.sh cargo fmt --all -- --check
```

## Pre-Push Gate (mandatory)
Every push must go through the local CI mirror first — it runs the same jobs as `.github/workflows/*.yml`:
```sh
make ci         # fmt + clippy + check + test + frontend + openapi drift
make act        # optional: run the actual workflow files locally via nektos/act (.actrc preconfigured)
```
Install pre-commit hooks once per clone: `pre-commit install` (config in `.pre-commit-config.yaml`). See [DEVELOPMENT.md](DEVELOPMENT.md) for the full loop. Mutation testing runs weekly via `.github/workflows/mutants.yml` (`.cargo/mutants.toml` gates survivors). OpenAPI parity with the PHP edition is enforced via [docs/openapi-parity.md](docs/openapi-parity.md) + `scripts/dump-openapi.sh` / `scripts/diff-openapi.sh`.

## Remote Convention
GitHub `nash87/parkhub-rust` is the source of truth for this repo. Fresh clones
should use GitHub as `origin`.

Some workstation clones still have stale Gitea as `origin` and GitHub as
`github`. In those clones, fetch/rebase/push via `github`; do not base ParkHub
work on `origin/main`. Keep any Gitea remote as `gitea-restore` or similar
unless an operator explicitly asks to restore mirroring. CI and PR review run
from GitHub.

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
- Legal templates in `legal/` must be customized by the operator before production use (includes BFSG accessibility statement + EU AI Act Art. 50 transparency notice)
- MIT license for headless builds; GUI builds include Slint (GPL-3.0 community edition)
- **SHA-pinned GitHub Actions** — every `uses:` in `.github/workflows/*.yml` must reference a full commit SHA with the v-tag as a trailing comment (e.g. `uses: actions/checkout@1d96c772d19495a3b5c517cd2bc0cb401ea0529f # v5.0.0`). 23+ actions are pinned this way today; never introduce a bare `@v5` / `@main` reference.

## Deploy
```sh
# Docker Compose (recommended for users)
docker compose up -d

# Direct binary
./target/release/parkhub-server --headless --unattended --port 8080

# Container build
docker build -t parkhub-rust -f Dockerfile .
docker push ghcr.io/nash87/parkhub-rust:latest

# Live demo: https://parkhub-rust-demo.onrender.com
# GitHub: https://github.com/nash87/parkhub-rust
```
