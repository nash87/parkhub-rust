<p align="center">
  <img src="assets/app.png" alt="ParkHub" width="96">
</p>

<h1 align="center">ParkHub — Self-Hosted Parking Management</h1>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.84%2B-orange.svg?style=for-the-badge&logo=rust&logoColor=white" alt="Rust 1.84+"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge" alt="MIT License"></a>
  <a href="CHANGELOG.md"><img src="https://img.shields.io/badge/Release-v1.9.0-brightgreen.svg?style=for-the-badge" alt="v1.9.0"></a>
  <a href="https://react.dev/"><img src="https://img.shields.io/badge/React-19-61DAFB.svg?style=for-the-badge&logo=react&logoColor=black" alt="React 19"></a>
  <a href="docs/GDPR.md"><img src="https://img.shields.io/badge/DSGVO-konform-green.svg?style=for-the-badge" alt="GDPR Compliant"></a>
  <a href="COMPLIANCE-REPORT.md"><img src="https://img.shields.io/badge/Compliance-Audited-brightgreen.svg?style=for-the-badge" alt="Compliance Audited"></a>
  <a href="docs/SECURITY.md"><img src="https://img.shields.io/badge/OWASP-Audited-green.svg?style=for-the-badge" alt="OWASP Audited"></a>
  <a href="docker-compose.yml"><img src="https://img.shields.io/badge/Docker-ready-2496ED.svg?style=for-the-badge&logo=docker&logoColor=white" alt="Docker Ready"></a>
</p>

<p align="center">
  <strong>Ihre Daten. Ihr Server. Ihre Kontrolle.</strong><br>
  The on-premise parking management platform for enterprises, universities, and residential complexes.<br>
  Ships as a <strong>single binary</strong> with zero external dependencies. Zero cloud. Zero tracking.<br>
  100% GDPR compliant by design.
</p>

<p align="center">
  <a href="https://parkhub-rust-demo.onrender.com"><strong>Try the Live Demo</strong></a> &nbsp;·&nbsp;
  <a href="docs/INSTALLATION.md">Installation</a> &nbsp;·&nbsp;
  <a href="docs/API.md">API Docs</a> &nbsp;·&nbsp;
  <a href="docs/GDPR.md">GDPR Guide</a> &nbsp;·&nbsp;
  <a href="CHANGELOG.md">Changelog</a>
</p>

---

## Why Self-Hosted?

Most parking management SaaS costs €200–2,000/month, stores your data on US cloud infrastructure, and requires a data processing agreement just to get started.

ParkHub is different. It runs on your server — a Raspberry Pi, a VPS, or your company network. Your data never leaves your premises, which means **no GDPR processor agreement needed**, no CLOUD Act exposure, and no monthly fees. The entire source code is MIT-licensed and auditable.

---

## Quick Start

```bash
git clone https://github.com/nash87/parkhub-rust.git && cd parkhub-rust
docker compose up -d
# Open http://localhost:8080 — admin password is in the logs
```

The first build takes 5–10 minutes (compiles Rust + React from source). After that, starts are instant.

For a **single binary** without Docker:

```bash
cargo build --release --package parkhub-server --no-default-features --features headless
./target/release/parkhub-server --headless --unattended --port 8080
```

**[Live Demo →](https://parkhub-rust-demo.onrender.com)** &nbsp; Login: `admin@parkhub.test` / `demo` &nbsp; (auto-resets every 6 hours)

---

## What You Get

**Booking System** — Full booking lifecycle from lot selection to QR code parking pass. One-tap quick booking, recurring reservations, guest bookings without accounts, swap requests between users, waitlists for full lots, and automatic no-show release. Every booking sends email confirmations and cancellation notices via SMTP.

**Smart Recommendations** — A heuristic scoring engine learns from usage patterns (slot frequency, lot proximity, feature preferences) and suggests the best available slots. Users can pin favorites for one-tap access.

**Parking Lot Management** — Create multiple lots with per-floor visual slot layouts. The interactive grid editor lets admins design layouts with drag-and-drop, while users see real-time occupancy with color-coded availability.

**User & Access Control** — Four-tier RBAC (user, premium, admin, superadmin) with JWT session auth, token refresh, and self-service password reset. A credits system supports monthly quotas with per-booking deduction. Absence tracking covers homeoffice, vacation, sick leave, and team overview.

**Admin Dashboard** — Occupancy statistics, 7-day booking activity charts, heatmaps by weekday and hour, CSV export for bookings/users/revenue, announcements, and a full settings panel for branding, booking rules, and use-case theming (company, residential, shared parking, rental).

**Internationalization** — Ships with 10 languages (EN, DE, FR, ES, IT, PT, TR, PL, JA, ZH). Community translation proposals with up/down voting and admin review. Approved translations are hot-loaded at runtime without restarts.

**GDPR & German Legal Compliance** — Art. 15 data export, Art. 17 erasure (PII anonymized, bookings retained per §147 AO), DDG §5 Impressum, and 7 ready-to-use legal templates (Impressum, Datenschutzerklärung, AGB, AVV, Widerrufsbelehrung, Cookie-Policy, VVt). Audited for DSGVO, TTDSG, UK GDPR, CCPA, and nDSG. [Full Compliance Report →](COMPLIANCE-REPORT.md)

**Security** — Argon2id password hashing, optional AES-256-GCM database encryption at rest, auto-generated TLS 1.3 certificates, IP-based rate limiting (5 login/min, 100 req/s global), CSP + security headers, 4 MiB request size limit, and a complete audit log. [Security Audit →](SECURITY-AUDIT.md)

**Observability** — Prometheus metrics at `/metrics`, OpenAPI 3.0 docs at `/swagger-ui` with 125+ annotated endpoints, Kubernetes health probes (`/health/live`, `/health/ready`), and structured logging via `tracing`.

---

## Screenshots

| | |
|---|---|
| ![Dashboard](screenshots/05-dashboard.png) | ![Booking](screenshots/06-book.png) |
| Dashboard with occupancy stats | Interactive booking flow |
| ![Admin Panel](screenshots/09-admin.png) | ![Dark Mode](screenshots/10-dark-mode.png) |
| Admin panel with layout editor | Full dark mode support |

---

## Architecture

```
                    ┌─────────────────────────────────┐
                    │     React 19 + Astro 6 SPA      │
                    │   TypeScript · Tailwind CSS 4    │
                    └───────────────┬─────────────────┘
                                    │ Bearer Token (JWT-style)
                    ┌───────────────▼─────────────────┐
                    │       Axum 0.8 HTTP Server       │
                    │   /api/v1/*  · /swagger-ui       │
                    │   /metrics   · /health           │
                    ├─────────────────────────────────┤
                    │  redb (embedded key-value DB)    │
                    │  Optional AES-256-GCM at rest    │
                    └─────────────────────────────────┘
                          Single Rust binary (~15 MB)
```

The entire stack — API server, database, and frontend — compiles into a **single binary**. No PostgreSQL, no Redis, no nginx. Just download and run. The React frontend is embedded via `rust-embed` and served as static files.

For LAN deployments, mDNS autodiscovery lets clients find the server without any DNS configuration. A desktop client (Slint UI) with system tray integration is available for Windows and macOS.

---

## Deployment

ParkHub runs anywhere — from a Raspberry Pi to Kubernetes.

**Docker Compose** (recommended) — `docker compose up -d` and you're done. Works on any Linux, macOS, or Windows machine with Docker installed.

**Kubernetes** — Health probes, Prometheus metrics, and a Helm-ready manifest. Designed for GitOps with Flux CD.

**Bare Metal** — Download the single binary, run it. No runtime dependencies. Works on x86_64 and ARM64.

**Windows** — GUI installer with system tray icon and setup wizard.

See [docs/INSTALLATION.md](docs/INSTALLATION.md) for detailed guides.

---

## Testing

**1,390 tests** across Rust backend (505), React frontend (401), and PHP sibling edition (484), plus Playwright E2E and Maestro mobile flows. Clippy runs in pedantic + nursery mode with zero warnings.

```bash
cargo test --workspace           # Rust backend
cd parkhub-web && npx vitest run # Frontend
npx playwright test              # E2E
```

---

## Configuration

All configuration via environment variables or `config.toml`. Key settings:

- `PARKHUB_DB_PASSPHRASE` — Enable AES-256-GCM database encryption
- `SMTP_HOST` / `SMTP_USER` / `SMTP_PASS` — Email notifications
- `PARKHUB_ADMIN_PASSWORD` — Set admin password (auto-generated if omitted)
- `DEMO_MODE=true` — Enable demo overlay with 6-hour auto-reset
- `RUST_LOG=info` — Log level

Full reference: [docs/CONFIGURATION.md](docs/CONFIGURATION.md)

---

## PHP Edition

A feature-equivalent **PHP edition** (Laravel 12 + MySQL/SQLite/PostgreSQL) exists for environments where shared hosting compatibility matters. Both editions share the same React frontend and REST API surface, so they're fully interchangeable.

**[nash87/parkhub-php →](https://github.com/nash87/parkhub-php)**

---

## License

MIT — see [LICENSE](LICENSE).

The default build includes [Slint](https://slint.dev/) for the desktop GUI (GPL-3.0 community edition). Server/Docker builds use `--features headless` and are purely MIT. See [LICENSES.md](LICENSES.md) for the full dependency license inventory.

---

## Contributing

Contributions welcome — see [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for setup and PR process.

Bug reports and feature requests: [GitHub Issues](https://github.com/nash87/parkhub-rust/issues)
