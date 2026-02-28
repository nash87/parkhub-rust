# Changelog — ParkHub Rust

All notable changes to ParkHub Rust are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

---

## [1.2.0] - 2026-02-28

### Added
- **Audit logging wired**: All sensitive operations (login, register, booking create/cancel, vehicle add/remove, user delete, role change, password reset, GDPR deletion) now emit structured audit log entries via the existing `audit.rs` infrastructure
- **Booking confirmation email**: `POST /api/v1/bookings` now sends an HTML booking confirmation email (non-fatal if SMTP not configured)
- **Profile editing**: New `PUT /api/v1/users/me` endpoint allows users to update their name, phone, and avatar URL; frontend Profile page now has an edit form
- **Admin UI**: User management page now fully implemented — list users, change role, toggle active/inactive, delete user; Bookings overview tab added
- **Booking filter**: Bookings page now has status/date/search filter bar (client-side filtering)
- **Koyeb deployment**: Added `koyeb.yaml` for one-command Koyeb deployment

### Fixed
- Email verification config flag `require_email_verification` is now documented as unimplemented (not silently ignored)
- parkhub-client: `on_admin_search_users` now implements real client-side user search filtering
- parkhub-client: `ServerConnection::connect_with_cert()` added for proper TLS cert pinning; `connect()` documents the self-signed cert limitation

---

## [1.1.1] — 2026-02-28

### Fixed

- **Self-registration enforcement**: `POST /api/v1/auth/register` now returns HTTP 403 `REGISTRATION_DISABLED`
  when `allow_self_registration = false` in config. Previously the flag had no effect.
- **Floor name UUID**: Booking confirmation response showed the internal UUID of the floor (e.g.
  `"Floor 82936167-..."`) instead of the human-readable name. Now resolved from the lot's floors array.
- **CI Kaniko build**: `Cargo.lock` was gitignored, causing all CI builds to fail with
  `lstat /workspace/src/Cargo.lock: no such file or directory`. Binary crates must commit
  their lockfile for reproducible Docker builds.

---

## [1.1.0] — 2026-02-28

### Added
- Per-endpoint rate limiting middleware (login: 5/min, register: 3/min, forgot-password: 3/15min — all per-IP)
- SMTP email notifications: welcome email on registration, booking confirmation
- Password reset flow via email (`POST /api/v1/auth/forgot-password`, `POST /api/v1/auth/reset-password`)
- Token refresh endpoint (`POST /api/v1/auth/refresh`)
- Booking invoice endpoint (`GET /api/v1/bookings/:id/invoice`)
- Cookie consent UI (TTDSG §25 compliant — localStorage only, no HTTP cookies)
- GDPR transparency page (`/transparency`)
- Legal templates: Widerrufsbelehrung (§356 BGB) and updated cookie policy
- Admin user management UI with role management
- Admin booking overview UI

### Security
- JWT secret now uses 256-bit cryptographically random bytes (CSPRNG) instead of UUID
- HSTS header added (`max-age=31536000; includeSubDomains; preload`)
- CSP hardened: removed `script-src 'unsafe-inline'`
- X-Forwarded-For only trusted from private/loopback IP ranges (proxy trust validation)
- Past booking creation rejected (start_time must be future)
- Slot status update failure no longer silently ignored — returns HTTP 500

### Fixed
- Docker: Dockerfile now uses `rust:alpine` (latest) for edition2024 + MSRV compatibility
- Docker: `parkhub-client` (GUI workspace member) excluded from server build
- Docker: `curl` added to Alpine deps for utoipa-swagger-ui asset download
- Docker: server compiled with `--no-default-features --features headless` (no GTK/systray)
- Docker: health checks, named volumes, restart policy
- UX: empty states, loading states, error handling, mobile layout, accessibility polish
- Password reset page and admin endpoint authorization checks

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
| 1.1.0 | 2026-02-28 | Security hardening, Docker fixes, legal documentation |
| 1.0.0 | 2026-02-27 | Initial public release |

[Unreleased]: https://github.com/nash87/parkhub-rust/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/nash87/parkhub-rust/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/nash87/parkhub-rust/releases/tag/v1.0.0
