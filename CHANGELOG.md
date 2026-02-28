# Changelog

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

## [1.0.0] — 2026-02-27 — Initial Public Release

### Backend (parkhub-server)

- Axum 0.7 HTTP server with async Tokio runtime
- Embedded redb database — no external database server required
- Optional AES-256-GCM at-rest encryption (PBKDF2-SHA256 key derivation)
- JWT-style session authentication (UUID tokens, 24-hour expiry)
- Argon2id password hashing with OsRng salts
- RBAC with three roles: user, admin, superadmin
- Parking lot management: create lots, define floors and slots
- Booking creation with write-lock race condition protection
- Booking cancellation with automatic slot status restoration
- Vehicle registry: create and delete vehicles, ownership enforcement
- GDPR Art. 15 — full data export as JSON (profile, bookings, vehicles)
- GDPR Art. 17 — account erasure (PII anonymization, §147 AO compliant booking retention)
- DDG §5 Impressum — configurable via admin API, public endpoint
- Prometheus metrics endpoint (`/metrics`)
- OpenAPI specification with Swagger UI (`/swagger-ui`)
- Kubernetes health probes (`/health`, `/health/live`, `/health/ready`)
- mDNS LAN autodiscovery via `mdns-sd`
- TLS 1.3 with auto-generated self-signed certificate via `rcgen` + `rustls`
- Security headers middleware (CSP, X-Frame-Options, Referrer-Policy, Permissions-Policy)
- CORS: same-origin only, localhost allowed in development
- Rate limiting: per-IP for auth endpoints (5 login/3 register per minute), global 100 req/s
- Request body size limit: 1 MiB
- Automatic daily backups with configurable retention
- Audit logging
- Windows GUI mode: Slint setup wizard, system tray via `tray-icon`
- Headless and unattended modes for servers and Docker
- CLI flags: `--headless`, `--unattended`, `--debug`, `--port`, `--data-dir`, `--version`
- Portable mode: data stored next to binary (no system directory installation required)

### Frontend (parkhub-web)

- React 19 + TypeScript + Tailwind CSS
- Login page (username or email)
- Registration page
- Dashboard: occupancy stats, active bookings list, parking lot grid overview, quick action
- Book page: 3-step flow (lot selection → slot grid → duration + vehicle)
  - Slot favorites (persisted in localStorage)
  - Duration options: 30 min, 1h, 2h, 4h, 8h, 12h
  - Booking summary card with confirmation
- My Bookings: active bookings with expiry countdown and cancel button; booking history
- Vehicles: add vehicle (plate, make, model, color), delete with confirmation dialog
- Admin panel: overview stats, lot management with inline layout editor, user management placeholder, bookings placeholder
- Impressum page: renders DDG §5 data from server or shows setup notice
- Dark mode and light mode
- Mobile-responsive layout
- Accessibility: ARIA labels, roles, live regions, keyboard navigation
- Animated UI with Framer Motion
- Toast notifications via react-hot-toast

### Common (parkhub-common)

- Shared data models: User, ParkingLot, ParkingFloor, ParkingSlot, Booking, Vehicle
- Protocol types: ApiResponse, HandshakeRequest/Response, LoginRequest/Response
- UserRole, SlotStatus, BookingStatus, VehicleType, LotStatus enums
- PROTOCOL_VERSION constant for client-server compatibility negotiation

### Deployment

- Multi-stage Dockerfile (Node 22 for frontend, Rust 1.83 + musl-dev for backend, Alpine runtime)
- Docker Compose with named volume, health check, and Traefik labels
- German legal templates: impressum-template.md, datenschutz-template.md, agb-template.md, avv-template.md

### Known Limitations in 1.0.0

- Token refresh endpoint returns 501 Not Implemented
- Admin user management UI is a placeholder (use API)
- Admin booking overview UI is a placeholder (use API)
- No email/SMTP notification support
