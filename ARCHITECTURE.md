# ParkHub Rust -- Architecture

ParkHub is a self-hosted parking management system built as a Rust workspace with
three crates: a server (Axum + redb), a desktop client (Slint), and a shared
library. A React 19 / Astro 6 frontend is served as static assets from the
server binary.

This is the **Rust edition** of ParkHub. A feature-equivalent **PHP edition**
(Laravel 12) exists in a sibling repository. Both backends expose the same
`/api/v1/*` REST surface and share the identical `parkhub-web` frontend, so
they are fully interchangeable.

## Directory Structure

```
parkhub-rust/
├── Cargo.toml                 # Workspace root (3 members)
├── Dockerfile                 # Multi-stage: frontend build -> cargo-chef -> runtime
├── docker-compose.yml         # Single-service compose with volume + health check
├── koyeb.yaml                 # Koyeb free-tier deployment manifest
├── package.json               # Root: Playwright for Maestro E2E
│
├── parkhub-common/            # Shared library crate
│   └── src/
│       ├── lib.rs             # Protocol version, default port, mDNS service type
│       ├── models.rs          # ~50 domain structs (User, Booking, ParkingLot, etc.)
│       ├── protocol.rs        # Client-server handshake, API request/response envelopes
│       └── error.rs           # Common error types
│
├── parkhub-server/            # HTTP API server crate
│   └── src/
│       ├── main.rs            # CLI parsing, config, DB init, server startup, optional GUI
│       ├── api/
│       │   ├── mod.rs         # Router definition (8,100 LoC), middleware stack, handlers
│       │   ├── auth.rs        # Login, register, forgot/reset password, token refresh
│       │   ├── bookings.rs    # CRUD + quick-book, guest booking, swap
│       │   ├── lots.rs        # Parking lot & slot management
│       │   ├── credits.rs     # Credit system (monthly quota, grants, refills)
│       │   ├── admin.rs       # Admin user/booking management
│       │   ├── export.rs      # CSV export (bookings, users)
│       │   ├── favorites.rs   # Favorite slot bookmarking
│       │   ├── push.rs        # Web Push (VAPID) subscriptions + dispatch
│       │   ├── setup.rs       # First-run setup wizard endpoints
│       │   ├── webhooks.rs    # Webhook CRUD + test delivery
│       │   └── zones.rs       # Parking zone management
│       ├── db.rs              # redb database layer (147K, 20+ tables, AES-256-GCM encryption)
│       ├── jwt.rs             # JWT access/refresh token generation & validation
│       ├── audit.rs           # Audit log recording
│       ├── config.rs          # TOML-based server configuration
│       ├── demo.rs            # Demo mode: collaborative reset voting, auto-reset timer
│       ├── discovery.rs       # mDNS/DNS-SD LAN autodiscovery
│       ├── email.rs           # SMTP email transport (lettre)
│       ├── error.rs           # AppError enum -> HTTP status + JSON error body
│       ├── health.rs          # /health + /ready endpoints
│       ├── metrics.rs         # Prometheus metrics exporter
│       ├── openapi.rs         # utoipa OpenAPI spec generation
│       ├── rate_limit.rs      # Per-endpoint IP-based rate limiting (Governor)
│       ├── requests.rs        # Request validation structs (Validator)
│       ├── static_files.rs    # rust-embed static file serving
│       ├── tls.rs             # Self-signed TLS certificate generation (rcgen)
│       ├── validation.rs      # Input validation rules
│       └── integration_tests.rs  # 873-line integration test suite
│
├── parkhub-client/            # Desktop client crate (Slint UI)
│   ├── src/
│   │   ├── main.rs            # Slint app with mock API, layout editor, bookings
│   │   ├── discovery.rs       # mDNS client for server autodiscovery
│   │   └── server_connection.rs  # HTTP client for server communication
│   └── ui/                    # Slint UI definitions (.slint files)
│       ├── main.slint         # Root window
│       ├── parking.slint      # Parking view with slot grid
│       ├── calendar.slint     # Calendar view
│       └── ...                # 15+ .slint component files
│
├── parkhub-web/               # Shared React frontend (Astro 6 + React 19)
│   ├── astro.config.mjs       # Static output, React compiler, Tailwind, chunk splitting
│   ├── package.json           # v1.4.6
│   ├── vitest.config.ts       # Unit test config
│   ├── playwright.config.ts   # E2E test config
│   ├── public/
│   │   ├── manifest.json      # PWA manifest
│   │   ├── sw.js              # Service worker (offline, background sync)
│   │   └── icons/             # PWA icons (192, 512, maskable)
│   ├── e2e/                   # 14 Playwright E2E test specs
│   └── src/
│       ├── App.tsx            # Router, providers, lazy-loaded routes
│       ├── api/client.ts      # Typed fetch wrapper with auto-401 redirect
│       ├── components/        # Shared UI: Layout, ErrorBoundary, CommandPalette, etc.
│       ├── views/             # 20+ page views (Dashboard, Book, Admin, Calendar, etc.)
│       ├── context/           # Auth, Theme, Features, UseCase providers
│       ├── hooks/             # useKeyboardShortcuts
│       ├── i18n/              # 10 locales (de, en, es, fr, it, ja, pl, pt, tr, zh)
│       ├── constants/         # Animation config, absence types
│       └── styles/            # Tailwind v4 global styles
│
├── e2e/                       # Maestro E2E test flows (YAML)
│   ├── 01-welcome.yaml
│   ├── 02-login.yaml
│   ├── 03-dashboard.yaml
│   ├── 04-admin.yaml
│   └── 05-login-failure.yaml
│
├── config/                    # Default config files
│   ├── config.toml            # Server configuration template
│   └── dev-users.json         # Development user fixtures
│
├── legal/                     # German legal document templates (GDPR)
├── docs/                      # API.md, SECURITY.md, GDPR.md, etc.
├── scripts/                   # seed_demo.py, docker-entrypoint.sh, smoke-test.sh
└── .github/workflows/         # CI, Docker publish, release
```

## Backend Architecture

### Request Flow

```
Client Request
  │
  ▼
┌─────────────────────────────────────────────────┐
│  Tower Middleware Stack                          │
│  ┌────────────────────────────────────────────┐ │
│  │ SetRequestId (X-Request-Id: UUIDv4)        │ │
│  │ PropagateRequestId                         │ │
│  │ TraceLayer (request/response logging)      │ │
│  │ CompressionLayer (gzip + brotli)           │ │
│  │ RequestBodyLimit (4 MiB)                   │ │
│  │ CorsLayer                                  │ │
│  │ SecurityHeaders (CSP, HSTS, X-Frame, etc.) │ │
│  └────────────────────────────────────────────┘ │
│                                                  │
│  Per-endpoint rate limiters (Governor)           │
│  ├── login:           5 req/min/IP              │
│  ├── register:        3 req/min/IP              │
│  ├── forgot-password: 3 req/15min/IP            │
│  └── general:         60 req/min/IP             │
│                                                  │
│  JWT Auth Middleware (Bearer token extraction)    │
│  │                                              │
│  ▼                                              │
│  Axum Router → Handler Function                 │
│  │                                              │
│  ▼                                              │
│  Input Validation (validator crate)              │
│  │                                              │
│  ▼                                              │
│  Business Logic                                  │
│  │                                              │
│  ▼                                              │
│  Database Layer (redb + optional AES-256-GCM)    │
│  │                                              │
│  ▼                                              │
│  JSON Response (ApiResponse<T> envelope)         │
└─────────────────────────────────────────────────┘
```

### Authentication

- **JWT tokens** with access (24h) + refresh (30d) pair
- Access tokens signed with HMAC-SHA256 (256-bit random secret)
- Password hashing via Argon2id
- Claims include: sub (user ID), username, role, token_type
- `AuthUser` extracted from request parts via `FromRequestParts`

### Error Handling

All errors flow through `AppError`, a typed enum that maps to HTTP status codes
and structured JSON responses:

```
AppError::NotFound("user")
  → 404 { "code": "NOT_FOUND", "message": "Resource not found: user" }

AppError::ValidationFailed(errors)
  → 400 { "code": "VALIDATION_FAILED", "details": [{ "field": "email", "message": "..." }] }
```

Error variants: `InvalidCredentials`, `TokenExpired`, `Unauthorized`, `Forbidden`,
`ValidationFailed`, `NotFound`, `AlreadyExists`, `Conflict`, `SlotNotAvailable`,
`BookingNotModifiable`, `InvalidBookingTime`, `RateLimited`, `Database`, `Internal`.

## Frontend Architecture

### Stack

| Layer          | Technology                                           |
|---------------|------------------------------------------------------|
| Meta-framework | Astro 6 (static output mode)                        |
| UI framework   | React 19 with React Compiler                        |
| Styling        | Tailwind CSS v4                                     |
| Animations     | Framer Motion                                       |
| State          | Zustand (stores) + React Context (auth/theme/features) |
| Routing        | React Router v7 (lazy-loaded routes)                |
| i18n           | i18next + browser language detection (10 locales)   |
| Icons          | Phosphor Icons                                      |
| HTTP client    | Typed `fetch` wrapper (`api/client.ts`)             |
| Testing        | Vitest (32 unit test files) + Playwright (14 E2E specs) |

### Routing

All pages are lazy-loaded via `React.lazy()`. Auth-required pages are wrapped in
`ProtectedRoute`; admin pages additionally wrapped in `AdminRoute`:

- `/welcome` -> Welcome/language selector
- `/login`, `/register`, `/forgot-password` -> Auth pages
- `/` -> Dashboard (redirects to `/welcome` or `/login` if unauthenticated)
- `/book`, `/bookings`, `/calendar` -> Booking management
- `/vehicles`, `/absences`, `/credits` -> User resources
- `/profile`, `/team`, `/notifications` -> User pages
- `/admin`, `/admin/settings`, `/admin/users`, `/admin/lots`, `/admin/reports`,
  `/admin/announcements` -> Admin panel

### Theme System

- Three modes: `light`, `dark`, `system`
- Persisted in `localStorage` as `parkhub_theme`
- OS preference tracked via `useSyncExternalStore` + `matchMedia`
- Use-case theming via `data-usecase` attribute on `<html>`
- Tailwind dark mode via `.dark` class toggle

### Code Splitting

Manual chunks configured in Astro/Vite:
- `vendor-react` (react, react-dom, react-router)
- `vendor-motion` (framer-motion)
- `vendor-i18n` (i18next stack)

## Database

### Engine

**redb** -- a pure Rust, single-file, ACID-compliant embedded database. No
external processes, no network dependencies.

### Encryption

Optional AES-256-GCM encryption at rest:
- Key derived via PBKDF2-HMAC-SHA256 from passphrase + random salt
- Passphrase provided via `PARKHUB_DB_PASSPHRASE` env var or setup wizard
- Each value encrypted individually before storage

### Tables (20+ redb tables)

```
users               → User { id, username, email, password_hash, role, credits, ... }
users_by_username   → username → user_id (index)
users_by_email      → email → user_id (index)
sessions            → Session { user_id, refresh_token, expires_at }
bookings            → Booking { id, user_id, lot_id, slot_id, start/end, status, pricing }
parking_lots        → ParkingLot { id, name, address, slots, floors, pricing, hours }
parking_slots       → ParkingSlot { id, lot_id, number, status, features }
slots_by_lot        → lot_id → [slot_ids] (index)
vehicles            → Vehicle { id, user_id, plate, make, model, type, photo }
absences            → Absence { id, user_id, type, start/end, pattern }
waitlist            → WaitlistEntry { id, user_id, lot_id, date }
guest_bookings      → GuestBooking { guest_name, guest_code, ... }
swap_requests       → SwapRequest { requester, target, status }
recurring_bookings  → RecurringBooking { days_of_week, start/end_time }
credit_transactions → CreditTransaction { user_id, amount, type, reason }
announcements       → Announcement { title, message, severity, active }
notifications       → Notification { user_id, type, title, read }
favorites           → Favorite { user_id, slot_id }
webhooks            → Webhook { url, events, secret, active }
push_subscriptions  → PushSubscription { user_id, endpoint, p256dh, auth }
zones               → Zone { id, lot_id, name, color }
settings            → key-value settings store
audit_log           → AuditEntry { user_id, action, details, ip }
```

## API Design

### Versioned REST

All API endpoints are prefixed with `/api/v1/`. The API follows RESTful
conventions with consistent JSON envelope responses:

```json
{
  "success": true,
  "data": { ... }
}
```

Error responses:
```json
{
  "success": false,
  "error": { "code": "NOT_FOUND", "message": "Resource not found: lot" }
}
```

### Key Endpoint Groups

| Group          | Prefix                  | Auth     | Methods                              |
|---------------|-------------------------|----------|--------------------------------------|
| Health         | `/health`, `/ready`     | None     | GET                                  |
| Auth           | `/api/v1/auth/*`        | None*    | POST login, register, forgot, reset  |
| Setup          | `/api/v1/setup/*`       | None     | GET status, POST complete            |
| Lots           | `/api/v1/lots/*`        | Bearer   | CRUD + slots, occupancy              |
| Bookings       | `/api/v1/bookings/*`    | Bearer   | CRUD + quick-book, guest, swap       |
| Vehicles       | `/api/v1/vehicles/*`    | Bearer   | CRUD + photo upload                  |
| Users          | `/api/v1/users/me`      | Bearer   | GET/PUT profile, password, prefs     |
| Credits        | `/api/v1/credits/*`     | Bearer   | Balance, history                     |
| Admin          | `/api/v1/admin/*`       | Admin    | Users, bookings, reports, settings   |
| Webhooks       | `/api/v1/webhooks/*`    | Bearer   | CRUD + test                          |
| Push           | `/api/v1/push/*`        | Bearer   | Subscribe/unsubscribe                |
| Demo           | `/api/v1/demo/*`        | None     | Status, vote-reset                   |
| Metrics        | `/metrics`              | None     | Prometheus scrape                    |
| OpenAPI        | `/swagger-ui/*`         | None     | Swagger UI                           |

### OpenAPI

Full OpenAPI 3.0 spec generated via `utoipa` crate, served at `/swagger-ui/`.

## Testing Strategy

### Backend (Rust)

| Type          | Count | Framework       | Location                      |
|--------------|-------|-----------------|-------------------------------|
| Unit tests    | ~493  | `#[test]`       | Inline `mod tests` blocks     |
| Integration   | ~30   | `#[tokio::test]`| `integration_tests.rs`        |

Tests run with `cargo test` in the workspace root. Integration tests create
temporary redb databases and exercise the full handler → DB round-trip.

### Frontend (React)

| Type          | Count | Framework        | Location                     |
|--------------|-------|------------------|------------------------------|
| Unit/component| 32    | Vitest + Testing Library | `src/**/*.test.{ts,tsx}` |
| E2E           | 14    | Playwright       | `e2e/*.spec.ts`              |

### System E2E (Maestro)

5 YAML-based E2E flows in `/e2e/` testing the full stack via Maestro.

## Deployment

### Docker (self-hosted)

Multi-stage Dockerfile:
1. **Frontend build** -- Node 22 Alpine, `npm ci && npm run build`
2. **Cargo chef** -- Dependency recipe caching
3. **Rust build** -- Headless feature, fat LTO, stripped binary
4. **Runtime** -- Debian slim, non-root user, ~40 MB final image

`docker-compose.yml` provides single-service deployment with persistent volume
and Traefik labels.

### Cloud

- **Render** -- Pre-built GHCR image, free tier
- **Koyeb** -- GHCR image, nano instance, Frankfurt region

Both use `--headless --unattended` flags with auto-generated admin password.

### Demo Mode

Activated via `DEMO_MODE=true`:
- Collaborative vote-to-reset (3 votes triggers data reset)
- Auto-reset every 6 hours
- Demo overlay in frontend showing countdown + viewer count
- Seeded with realistic demo data via `scripts/seed_demo.py`

## PWA

### Manifest

Full PWA manifest with standalone display, maskable icons, app shortcuts
(Quick Book, My Bookings, Calendar), and screenshot metadata.

### Service Worker

Custom service worker (`sw.js`) with three strategies:

1. **Static assets** -- Cache-first with version-keyed cache name (purged on update)
2. **API responses** -- Stale-while-revalidate for read endpoints (24h max age)
3. **Offline mutations** -- Queued in IndexedDB, replayed via Background Sync API

Precached: `/`, `/favicon.svg`, `/offline.html`, icon assets.

### Offline

- Offline fallback page (`/offline.html`)
- Cached API data for read-only browsing
- Background sync for mutations made while offline

## Key Design Decisions

### Why Dual-Stack (Rust + PHP)?

Rust provides maximum performance (single binary, sub-millisecond responses, 256
MB memory limit) and encryption-at-rest via redb. PHP (Laravel) provides rapid
prototyping, a massive ecosystem, and is the lingua franca of web hosting. Both
expose the same API surface, so the React frontend works with either backend.

### Why redb?

Zero-dependency embedded database: no SQLite C bindings, no external processes.
Pure Rust means cross-compilation works out of the box, the database is a single
file, and encryption can be layered at the application level without needing
SQLCipher or similar.

### Why Astro?

Astro's static output mode produces pre-rendered HTML with React islands. The
built assets are embedded into the Rust binary via `rust-embed`, so the server
is a single file that serves both API and frontend. Build scripts
(`build:rust`, `build:php`) copy the same `dist/` output into either backend.

### Why SQLite (in the PHP edition)?

SQLite is the default for development and Render free-tier (ephemeral storage).
The PHP edition also supports MySQL 8 for production via `docker-compose.yml`.
This mirrors the Rust edition's single-file redb approach.
