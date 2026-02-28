# Changelog — ParkHub Rust

All notable changes to ParkHub Rust are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Planned

- Token refresh (`POST /api/v1/auth/refresh` — currently returns 501)
- Full admin user management UI
- Admin booking overview UI
- Email notifications (SMTP)

---

## [1.0.0] — 2026-02-27

Initial public release.

### Added

#### Backend (parkhub-server)

**Core server**
- Axum 0.7 HTTP server with async Tokio runtime
- Single-binary deployment — server binary embeds the compiled React frontend
- Headless and `--unattended` modes for Docker and server deployments
- Windows GUI mode: Slint-based setup wizard and system tray icon via `tray-icon`
- CLI flags: `--headless`, `--unattended`, `--debug`, `--port`, `--data-dir`, `--version`
- Portable mode: all data stored next to the binary (no system directory installation required)
- mDNS LAN autodiscovery via `mdns-sd`

**Database**
- Embedded `redb` key-value database — no external database server required
- Optional AES-256-GCM at-rest encryption with PBKDF2-SHA256 key derivation
- `zeroize` crate for zeroing key material in memory on drop

**Authentication**
- UUID-based session tokens with 24-hour expiry
- Argon2id password hashing with OsRng cryptographic salts
- RBAC: three roles (`user`, `admin`, `superadmin`) enforced at the handler level
- Rate limiting: 5 login/3 register attempts per minute per IP
- Global rate limit: 100 req/s (burst 200) via `governor` crate

**Parking management**
- Parking lot management: create lots, define floors and slots
- Booking creation with write-lock race condition protection (no double-bookings)
- Booking cancellation with automatic slot status restoration
- Vehicle registry with ownership enforcement

**GDPR & legal**
- Art. 15/20 — full data export as JSON (profile, bookings, vehicles)
- Art. 17 — account erasure (PII anonymization, §147 AO compliant booking retention)
- DDG §5 Impressum — configurable via admin API, public endpoint at `/api/v1/legal/impressum`

**Security**
- TLS 1.3 with auto-generated self-signed certificate via `rcgen` + `rustls` (no OpenSSL)
- Security headers middleware (CSP, X-Frame-Options, Referrer-Policy, Permissions-Policy)
- CORS: same-origin only; localhost allowed in development
- Request body size limit: 1 MiB (`RequestBodyLimitLayer`)

**Observability**
- Prometheus metrics endpoint (`/metrics`)
- OpenAPI specification with Swagger UI (`/swagger-ui`)
- Kubernetes health probes: `/health`, `/health/live`, `/health/ready`
- Structured audit logging via `tracing`

**Operations**
- Automatic daily database backups with configurable retention
- Multi-stage Dockerfile (Node 22 frontend build, Rust 1.83 + musl backend, Alpine runtime)
- Docker Compose with named volume, health check, and Traefik labels
- Kubernetes deployment manifests (see `docs/INSTALLATION.md`)
- German legal templates: `impressum-template.md`, `datenschutz-template.md`, `agb-template.md`, `avv-template.md`

#### Frontend (parkhub-web)

- React 19 + TypeScript + Tailwind CSS SPA
- Login page (username or email)
- Registration page
- Dashboard: occupancy stats, active bookings, parking lot grid, quick-action buttons
- Book: 3-step booking flow (lot → slot grid → duration + vehicle)
  - Slot favourites (persisted in localStorage)
  - Duration options: 30 min, 1h, 2h, 4h, 8h, 12h
  - Booking summary with confirmation
- My Bookings: active bookings with expiry countdown and cancel button; booking history
- Vehicles: register vehicle (plate, make, model, color), delete with confirmation
- Admin panel: occupancy stats, lot management with inline layout editor
- Impressum page: renders DDG §5 data from server
- Dark mode and light mode
- Mobile-responsive layout
- Accessibility: ARIA labels, roles, live regions, keyboard navigation
- Animated UI with Framer Motion; toast notifications via `react-hot-toast`

#### Common (parkhub-common)

- Shared data models: `User`, `ParkingLot`, `ParkingFloor`, `ParkingSlot`, `Booking`, `Vehicle`
- Protocol types: `ApiResponse`, `HandshakeRequest/Response`, `LoginRequest/Response`
- Enums: `UserRole`, `SlotStatus`, `BookingStatus`, `VehicleType`, `LotStatus`
- `PROTOCOL_VERSION` constant for client-server compatibility negotiation

### Known Limitations in 1.0.0

- `POST /api/v1/auth/refresh` returns HTTP 501 — token refresh not yet implemented
- Admin user management UI is a placeholder (full functionality via API)
- Admin booking overview UI is a placeholder (full functionality via API)
- No email/SMTP notification support

---

## Version History

| Version | Date | Notes |
|---------|------|-------|
| 1.0.0 | 2026-02-27 | Initial public release |

[Unreleased]: https://github.com/nash87/parkhub/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/nash87/parkhub/releases/tag/v1.0.0
