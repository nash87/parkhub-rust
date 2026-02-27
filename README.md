# ParkHub Rust — Self-Hosted Parking Management

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.83+](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org/)
[![React 19](https://img.shields.io/badge/react-19-61dafb.svg)](https://react.dev/)
[![Docker](https://img.shields.io/badge/docker-ready-2496ed.svg)](docker-compose.yml)
[![GDPR Compliant](https://img.shields.io/badge/DSGVO-konform-green.svg)](docs/GDPR.md)

> **ParkHub — Ihre Parkplatzverwaltung. Ihre Daten. Ihr Server.**
> Keine Cloud. Keine Drittanbieter. DSGVO-konform durch Design.
> ParkHub ist eine selbst gehostete Open-Source-Lösung für die Verwaltung von Parkplätzen,
> Buchungen und Fahrzeugen — vollständig unter Ihrer Kontrolle, auf Ihrer eigenen Infrastruktur.

ParkHub is a self-hosted, on-premise parking management platform built with Rust (Axum 0.7) on
the backend and React 19 + TypeScript on the frontend. All data is stored in an embedded
[redb](https://github.com/cberner/redb) database with optional AES-256-GCM encryption at rest.
No external database server required, no cloud dependencies, no third-party data processors.

## Features

| Feature | Status |
|---|---|
| Parking lot management with visual slot layout editor | Done |
| Interactive slot grid (available / occupied / reserved / favorites) | Done |
| Booking flow: lot -> slot -> duration -> vehicle -> confirm | Done |
| Booking cancellation and booking history | Done |
| Vehicle registry (license plate, make, model, color, default flag) | Done |
| JWT-based session authentication (24 h expiry) | Done |
| User registration and login (username or email) | Done |
| Role-based access control (user / admin / superadmin) | Done |
| Admin dashboard: stats, lot management, inline layout editor | Done |
| GDPR Art. 15 — full data export as JSON | Done |
| GDPR Art. 17 — account erasure (anonymizes PII, retains booking records per §147 AO) | Done |
| Impressum editor (DDG §5) — configurable via admin panel | Done |
| German legal templates: Impressum, Datenschutz, AGB, AVV | Done |
| Prometheus metrics endpoint (`/metrics`) | Done |
| Swagger UI at `/swagger-ui` | Done |
| Kubernetes health probes (`/health`, `/health/live`, `/health/ready`) | Done |
| mDNS LAN autodiscovery | Done |
| TLS 1.3 (auto-generated self-signed cert or bring-your-own) | Done |
| Argon2id password hashing | Done |
| AES-256-GCM database encryption at rest (optional) | Done |
| Security headers (CSP, X-Frame-Options, Referrer-Policy, Permissions-Policy) | Done |
| Rate limiting: 5 login attempts/min per IP, 100 req/s global | Done |
| Dark mode and light mode | Done |
| Mobile-responsive UI | Done |
| Docker Compose deployment | Done |
| Kubernetes deployment | Done |
| Windows GUI (system tray, setup wizard) | Done |
| Headless / unattended server mode | Done |
| Automatic daily backups with configurable retention | Done |
| Audit logging | Done |
| Accessibility (ARIA labels, keyboard navigation, screen reader support) | Done |
| Token refresh | Planned |
| Full admin user management UI | Planned |
| Admin booking overview UI | Planned |
| Email notifications (SMTP) | Planned |

## Quick Start (3 commands)

```bash
git clone https://github.com/nash87/parkhub
cd parkhub
docker compose up -d
```

Open `http://localhost:8080` in your browser.
Default credentials: username `admin`, password `admin`.

> Change the admin password immediately after first login.

To run without Docker:

```bash
cargo build --release --package parkhub-server
./target/release/parkhub-server --headless --unattended
```

## Screenshots

Screenshots are in the `screenshots/` directory.

| File | Description |
|---|---|
| `screenshots/01-login.png` | Login page |
| `screenshots/02-register.png` | Registration |
| `screenshots/05-dashboard.png` | Dashboard with occupancy stats and lot overview |
| `screenshots/06-book.png` | Booking flow with slot selection grid |
| `screenshots/07-bookings.png` | Active and past bookings |
| `screenshots/08-vehicles.png` | Vehicle management |
| `screenshots/09-admin.png` | Admin panel |
| `screenshots/10-dark-mode.png` | Dark mode |

## Architecture

```
Browser
  |
  +-- React 19 SPA (TypeScript + Tailwind CSS)
  |     Served as static files embedded in the Rust binary
  |
  +-- HTTP/HTTPS --> Axum 0.7 (Rust)
                      |
                      +-- /api/v1/*    REST API (auth required)
                      +-- /metrics     Prometheus (no auth)
                      +-- /swagger-ui  OpenAPI docs
                      +-- /health*     Health probes
                      |
                      +-- redb (embedded single-file database)
                            Optional: AES-256-GCM encryption at rest
                            Data dir: ./data/ (portable) or /data (Docker)
```

The server binary embeds the compiled React frontend — no separate web server or
reverse proxy is required for single-host deployments.

## Installation

See [docs/INSTALLATION.md](docs/INSTALLATION.md) for full instructions:
- Docker Compose (recommended)
- Kubernetes with Flux GitOps
- Bare metal build from source
- TLS setup and reverse proxy configuration (nginx, Caddy, Traefik)

## Configuration

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for all options.

Key environment variables for Docker:

| Variable | Default | Description |
|---|---|---|
| `PARKHUB_HOST` | `0.0.0.0` | Bind address |
| `PARKHUB_PORT` | `8080` | Listen port |
| `PARKHUB_DB_PASSPHRASE` | — | AES-256-GCM passphrase (enables encryption) |
| `RUST_LOG` | `info` | Log level (`debug`, `info`, `warn`, `error`) |

## Security

- **Passwords**: Argon2id hashing (never stored in plaintext)
- **Database**: optional AES-256-GCM encryption at rest via PBKDF2-derived key
- **Transport**: TLS 1.3 auto-generated or custom certificate
- **Rate limiting**: 5 login/register attempts per minute per IP, 100 req/s global burst 200
- **Security headers**: Content-Security-Policy, X-Frame-Options: DENY, Referrer-Policy, Permissions-Policy
- **Request size**: 1 MiB limit on all request bodies
- **CORS**: same-origin only (localhost allowed in development)
- **RBAC**: user / admin / superadmin roles enforced on each endpoint

See [docs/SECURITY.md](docs/SECURITY.md) for the full security model.

## GDPR and German Legal Compliance

ParkHub is designed for on-premise deployment in German-regulated environments:

- **Data sovereignty**: all data stays on your server — no cloud, no third-party processors
- **Art. 15 (Auskunft)**: users export their full data as a JSON file from the UI
- **Art. 17 (Loschung)**: account deletion anonymizes all PII; booking records are retained
  per German tax law (§147 AO, 10-year accounting record retention)
- **DDG §5 (Impressum)**: configurable from the admin panel, always publicly accessible
- **Legal templates** in `legal/`: Impressum, Datenschutz, AGB, AVV — ready to customize

See [docs/GDPR.md](docs/GDPR.md) for the operator compliance checklist.

## API Reference

Swagger UI is available at `/swagger-ui` when the server is running.

See [docs/API.md](docs/API.md) for the complete REST API reference with curl examples.

## License

MIT — see [LICENSE](LICENSE).

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).
