# Changelog

All notable changes to ParkHub Rust are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [3.7.0] - 2026-03-22

### Added
- **Enhanced Waitlist with Notifications**: Priority-based waitlist with auto-notification when slots become available. Accept/decline offers with 15-minute expiry. `POST /api/v1/lots/:id/waitlist/subscribe`, `GET /api/v1/lots/:id/waitlist`, `DELETE /api/v1/lots/:id/waitlist`, `POST .../accept`, `POST .../decline`. Frontend: WaitlistPage with join button, position indicator, accept/decline UI. Feature flag: `mod-waitlist-ext`. 9 backend + 7 frontend tests. (#241)
- **Digital Parking Pass / QR Badge**: Generate digital passes with QR codes from active bookings. Public verification endpoint for QR scanning. `GET /api/v1/bookings/:id/pass`, `GET /api/v1/pass/verify/:code` (public), `GET /api/v1/me/passes`. Mobile-optimized full-screen pass display. Feature flag: `mod-parking-pass`. 10 backend + 7 frontend tests. (#242)
- **Interactive API Documentation**: Embedded Swagger UI at `/api/v1/docs` for exploring and testing the REST API. Raw OpenAPI 3.0 JSON spec at `/api/v1/docs/openapi.json`. Admin sidebar link. Feature flag: `mod-api-docs`. 5 backend + 3 frontend tests. (#243)
- **i18n**: waitlistExt, parkingPass, apiDocs keys added to all 10 locales
- **48 feature flags**: Added `mod-waitlist-ext`, `mod-parking-pass`, `mod-api-docs` (was 45)

---

## [3.6.0] - 2026-03-22

### Added
- **Personal Parking History**: Paginated booking history with lot/date filters. Personal stats dashboard: total bookings, favorite lot, avg duration, busiest day, monthly trend chart, credits spent. `GET /api/v1/bookings/history`, `GET /api/v1/bookings/stats`. Timeline view with status badges. Feature flag: `mod-history`. 8 backend + 6 frontend tests. (#238)
- **Geofencing & Auto Check-in**: Auto check-in when user enters lot geofence area using GPS proximity (haversine distance). `POST /api/v1/geofence/check-in`, `GET /api/v1/lots/:id/geofence`, `PUT /api/v1/admin/lots/:id/geofence`. Auto check-in toggle in Profile settings. Feature flag: `mod-geofence`. 8 backend + 4 frontend tests. (#239)
- **i18n**: History and geofence keys added to all 10 locales
- **43 feature flags**: Added `mod-history`, `mod-geofence` (was 41)

### Fixed
- **Icon Audit**: Synced test mocks with component icon imports across AdminLots, AdminUsers, and Book tests (#237)

---

## [3.5.0] - 2026-03-22

### Added
- **Visitor Pre-Registration**: Pre-register visitors with name, email, vehicle plate, and visit date. Auto-generated QR code passes with visitor pass URL. `POST /api/v1/visitors/register`, `GET /api/v1/visitors`, `GET /api/v1/admin/visitors`, `PUT /api/v1/visitors/:id/check-in`, `DELETE /api/v1/visitors/:id`. Admin view with search/filter and stats. Feature flag: `mod-visitors`. 8 backend + 6 frontend tests. (#230)
- **EV Charging Station Management**: Manage EV chargers per lot with Type2, CCS, CHAdeMO, Tesla connector types. Start/stop charging sessions with kWh tracking. `GET /api/v1/lots/:id/chargers`, `POST /api/v1/chargers/:id/start`, `POST /api/v1/chargers/:id/stop`, `GET /api/v1/chargers/sessions`, `GET /api/v1/admin/chargers`, `POST /api/v1/admin/chargers`. Admin utilization stats. Feature flag: `mod-ev-charging`. 10 backend + 5 frontend tests. (#231)
- **Smart Slot Recommendations**: Enhanced recommendation engine with weighted scoring algorithm: frequency (40%), availability (30%), price (20%), distance (10%). Recommendation badges (Your usual spot, Best price, Closest, Available now, Preferred lot, Accessible). Star rating visualization. "Recommended for you" section in booking flow. `GET /api/v1/recommendations/stats` for admin analytics. 8 backend + 4 frontend tests. (#232)
- **i18n**: Visitors, EV charging, recommendations keys added to all 10 locales
- **41 feature flags**: Added `mod-visitors`, `mod-ev-charging` (was 39)

---

## [3.4.0] - 2026-03-22

### Added
- **Accessible Parking System**: `is_accessible` field on ParkingSlot, `accessibility_needs` on User (wheelchair, reduced_mobility, visual, hearing, none). `GET /api/v1/lots/:id/slots/accessible`, `PUT /api/v1/admin/lots/:id/slots/:slot_id/accessible`, `GET /api/v1/bookings/accessible-stats`, `PUT /api/v1/users/me/accessibility-needs`. 30-min priority booking for accessible users. Admin page with stats and slot toggles. Wheelchair icon in booking flow. Feature flag: `mod-accessible`. 9 backend + 8 frontend tests. (#226)
- **Maintenance Scheduling**: Full CRUD for maintenance windows (`POST/GET/PUT/DELETE /api/v1/admin/maintenance`), `GET /api/v1/maintenance/active` (public). Auto-block affected slots (all or specific). Booking overlap validation. Admin page with calendar list, create/edit form, active banner. Feature flag: `mod-maintenance`. 9 backend + 6 frontend tests. (#227)
- **Cost Center Billing**: `cost_center` and `department` fields on User. `GET /api/v1/admin/billing/by-cost-center`, `GET /api/v1/admin/billing/by-department`, `GET /api/v1/admin/billing/export` (CSV), `POST /api/v1/admin/billing/allocate`. Admin page with summary cards, tab switcher, data table, CSV export. Feature flag: `mod-cost-center`. 6 backend + 6 frontend tests. (#228)
- **i18n**: Accessible, maintenance, billing keys in all 10 locales
- **39 feature flags**: Added `mod-accessible`, `mod-maintenance`, `mod-cost-center` (was 36)

---

## [3.3.0] - 2026-03-22

### Added
- **Audit Log UI + Export**: Paginated admin audit log at `/admin/audit-log` with action/user/date filters, color-coded badges, and CSV export. Extended `AuditLogEntry` with `target_type`, `target_id`, `ip_address`. New event types: `PaymentCompleted`, `TwoFactorEnabled/Disabled`, `ApiKeyCreated/Revoked`. 7 backend + 6 frontend tests. (#217)
- **Data Import/Export Suite**: `POST /api/v1/admin/import/{users,lots}` for CSV/JSON bulk import with validation and error reporting. `GET /api/v1/admin/data/export/{users,lots,bookings}` for enhanced CSV exports with booking stats. Drag-and-drop upload UI with preview and import results. Feature flag: `mod-data-import`. 8 backend + 6 frontend tests. (#218)
- **Fleet / Vehicle Management**: `GET /api/v1/admin/fleet` (all vehicles with stats), `GET /api/v1/admin/fleet/stats` (types distribution, electric ratio), `PUT /api/v1/admin/fleet/:id/flag` (flag/unflag vehicles). Added `Bicycle` to `VehicleType` enum. Feature flag: `mod-fleet`. 6 backend + 6 frontend tests. (#222)
- **i18n**: Audit log, data management, fleet keys added to all 10 locales
- **36 feature flags**: Added `mod-data-import`, `mod-fleet` (was 34)

---

## [3.2.0] - 2026-03-22

### Added
- **iCal Calendar Sync**: `GET /api/v1/calendar/ical` (authenticated feed), `GET /api/v1/calendar/ical/:token` (public subscription via personal token), `POST /api/v1/calendar/token` (generate/rotate subscription token). VEVENTs with DTSTART, DTEND, SUMMARY, LOCATION, DESCRIPTION, DTSTAMP. Subscribe button in Calendar view with copy-link modal and instructions for Google Calendar, Outlook, Apple Calendar. Feature flag: `mod-ical`. 8 backend + 3 frontend tests. (#214)
- **API Rate Limiting Dashboard**: `GET /api/v1/admin/rate-limits` (stats per endpoint group: auth 5/min, api 100/min, public 30/min, webhook 50/min), `GET /api/v1/admin/rate-limits/history` (blocked requests over last 24h in hourly bins). Admin Rate Limits page at `/admin/rate-limits` with progress bars and 24h blocked-request bar chart. 4 backend + 5 frontend tests. (#215)
- **Multi-Tenant Isolation**: `tenant_id: Option<String>` added to User, ParkingLot, Booking models. `GET /POST /api/v1/admin/tenants` (list/create), `PUT /api/v1/admin/tenants/:id` (update). Super-admin sees all tenants; regular admins scoped to their own. AdminTenants page at `/admin/tenants` with create/edit modal and branding support. Feature flag: `mod-multi-tenant`. 10 backend + 5 frontend tests. (#216)
- **i18n**: Calendar subscribe, rate limits, tenants keys added to all 10 locales
- **34 feature flags**: Added `mod-ical`, `mod-multi-tenant` (was 31)

---

## [3.1.0] - 2026-03-22

### Added
- **Interactive Map View**: `GET /api/v1/lots/map` returns lots with coordinates, live availability, and color-coded markers (green/yellow/red/gray). `PUT /api/v1/admin/lots/{id}/location` for setting lot coordinates. Leaflet.js + OpenStreetMap frontend at `/map` with click-to-book popups. Feature flag: `mod-map`. 12 backend + 6 frontend tests. (#211)
- **Web Push Notifications**: Structured `PushPayload` with event types (booking confirmed/reminder/cancelled, new announcement). Service worker push handler with action buttons and notification click routing. `useNotifications` hook for subscribe/unsubscribe flow. 7 new backend + 4 frontend tests. (#212)
- **Stripe Payment Integration**: `POST /api/v1/payments/create-checkout` for credit purchase, `POST /api/v1/payments/webhook` for Stripe webhook events, `GET /api/v1/payments/history` for payment history, `GET /api/v1/payments/config` for Stripe status. Feature flag: `mod-stripe`. 14 backend tests. (#213)
- **i18n**: Map, payments keys added to all 10 locales
- **31 feature flags**: Added `mod-map`, `mod-stripe` (was 29)

---

## [3.0.0] - 2026-03-22

### Added
- **10-Language Support**: Complete translations for FR, ES, IT, PT, TR, PL, JA, ZH — all 904 keys matching EN. Language selector dropdown in sidebar with flag + native name. 29 new i18n tests. (#207)
- **Admin Analytics Dashboard**: `GET /api/v1/admin/analytics/overview` — daily bookings, revenue, peak hours histogram (24 bins), top 10 lots by utilization, user growth (12 months), avg booking duration. Frontend with stat cards, SVG charts, heatmap, date range picker, CSV export. Feature flag: `mod-analytics`. 6 backend + 7 frontend tests. (#208)
- **Email Notification Templates**: 6 professional HTML email templates with inline CSS — booking confirmation, reminder, cancellation, password reset, welcome, weekly admin summary. Template engine with `{{key}}` variable substitution. Feature flag: `mod-email-templates`. 9 unit tests. (#209)

---

## [2.9.0] - 2026-03-22

### Added
- **Lobby Display / Kiosk Mode**: Public `GET /api/v1/lots/:id/display` endpoint for digital signage monitors — no auth required, rate-limited 10 req/min per IP. Returns lot name, available/total slots, occupancy percentage, color status (green/yellow/red), and per-floor breakdown. Feature flag: `mod-lobby-display`. (#198)
- **LobbyDisplay frontend**: Full-screen view at `/lobby/:lotId` with auto-refresh every 10 seconds, 8rem+ numbers, color-coded occupancy bar, floor breakdown cards, dark background for screen burn-in prevention. i18n for en/de.
- **Interactive Onboarding Wizard**: 4-step setup wizard at `/setup` — company info (name/logo/timezone), create lot (floors/slots), user invites, theme picker (all 12 themes). Feature flag: `mod-setup-wizard`. (#200)
- **Wizard API**: `GET /api/v1/setup/wizard/status` + `POST /api/v1/setup/wizard` with per-step persistence and validation
- **12 backend tests**: 6 lobby display (color boundaries, serialization) + 8 wizard (DTO serialization, theme list, step validation)
- **12 frontend tests**: 6 lobby display (loading, display, floors, error, occupancy bar) + 6 wizard (render, validation, navigation, themes, redirect)

### Closed
- **#199 Digital Parking Pass**: Deferred — requires Apple Developer and Google Pay API accounts

---

## [2.8.0] - 2026-03-22

### Added
- **WebSocket real-time updates**: Token-based auth via `?token=` query param, heartbeat with missed-pong tracking, initial occupancy snapshot on connect (`mod-websocket`)
- **WsEvent factory methods**: `BookingCreated`, `BookingCancelled`, `OccupancyChanged`, `AnnouncementPublished`, `SlotStatusChange`
- **Live booking broadcasts**: Booking create/cancel handlers broadcast WebSocket events to all connected clients
- **Frontend useWebSocket hook**: Returns `{ connected, lastMessage, occupancy }` with token auth and exponential backoff reconnect
- **Dashboard live indicator**: Green dot shows active WebSocket connection status
- **Bookings real-time toasts**: Toast notifications on WebSocket booking events in Bookings page

### Changed
- **API module extraction (Phase 3)**: `mod.rs` reduced from 4517 to 1503 lines
  - `system.rs`: health, version, maintenance, handshake, middleware (345 lines)
  - `users.rs`: profile CRUD, GDPR, password, preferences, stats (757 lines)
  - `admin_handlers.rs`: user/booking mgmt, stats, reports, audit, settings (1412 lines)
  - `lots_ext.rs`: lot QR codes, admin dashboard charts (267 lines)
  - `misc.rs`: legal/Impressum, public occupancy/display (384 lines)

---

## [2.7.0] - 2026-03-22

### Added
- **Dynamic pricing**: Occupancy-based surge/discount with admin-configurable multipliers and thresholds (`mod-dynamic-pricing`)
- **Operating hours**: Per-lot 7-day schedule with open/close times, booking validation, "Open Now" badges (`mod-operating-hours`)
- **SMS/WhatsApp stubs**: Notification channel expansion with phone number input and per-event toggles
- **PDF invoices**: Professional booking invoices with VAT breakdown via `printpdf` (`mod-invoices`)
- **OAuth/Social login**: Self-service Google + GitHub OAuth configuration (`mod-oauth`)
- **12 design themes**: Added Wabi-Sabi, Scandinavian, Cyberpunk, Terracotta, Oceanic, Art Deco (was 6)
- **Playwright E2E**: 65 tests covering API, pages, devtools, parking flow, GDPR, PWA
- **Lighthouse CI**: Automated quality gates (a11y >= 95, perf >= 90, SEO >= 95)
- **httpOnly cookie auth**: XSS-proof authentication with CSRF protection and Bearer fallback

### Fixed
- Workspace lint override for Slint FFI on Windows builds
- ThemeSwitcher test updated for 12 themes
- Frontend test mocks for all new API endpoints

---

## [2.2.0] - 2026-03-22

### Added
- **Glass morphism UI**: Bento grid dashboard with frosted-glass cards, animated counters, and modern gradients
- **2FA/TOTP authentication**: QR code enrollment via `totp-rs`, backup codes, per-account enable/disable
- **Accessibility score 100**: Full ARIA compliance, contrast fixes, confirm dialogs replacing `window.confirm`
- **CI badges and GitOps polish**: README overhaul, SECURITY.md, issue/PR templates, CHANGELOG in Keep a Changelog format

### Changed
- Bumped version to 2.2.0
- README badges switched from for-the-badge to flat-square style with CI status badge
- Added Security link to README navigation

---

## [2.1.0] - 2026-03-22

### Added
- **28 Cargo feature flags**: Full modularity system — build only the modules you need (`mod-bookings`, `mod-vehicles`, `mod-absences`, etc.)
- **Headless mode**: `--no-default-features --features headless` for pure MIT server builds without GUI dependencies
- **Module documentation**: Feature flag table in README with build examples

### Changed
- Workspace Rust version updated to 1.85
- Axum upgraded from 0.7 to 0.8
- `rand` upgraded from 0.8 to 0.9

---

## [2.0.0] - 2026-03-22

### Added
- **Full modularity system**: 28 feature-gated modules for compile-time customization
- **Smart slot recommendations**: Heuristic scoring engine (slot frequency, lot frequency, features, proximity) — top 5 returned
- **Community translation management**: Proposal submission, up/down voting, admin review with comments
- **Runtime translation overrides**: Approved translations hot-loaded into i18n at app startup
- **Favorites UI**: Full view for managing pinned parking slots with live availability status
- **Dashboard analytics**: 7-day booking activity bar chart with real booking data
- **DataTable CSV export**: Download any data table as CSV with proper cell escaping
- **Demo reset tracking**: `last_reset_at`, `next_scheduled_reset`, `reset_in_progress` in status API

### Changed
- Major version bump to reflect the modularity system and feature flag architecture
- Clippy pedantic and nursery lints enforced with zero warnings

### Tests
- **505 Rust + 401 Frontend + 484 PHP** = 1,390 total tests

---

## [1.9.0] - 2026-03-21

### Added
- **Community translation management**: Proposal submission, up/down voting, admin review (approve/reject with comments)
- **Runtime translation overrides**: Approved translations hot-loaded into i18n at app startup
- **Smart slot recommendations**: Heuristic scoring engine (slot frequency, lot frequency, features, proximity, base) — top 5 returned
- **Favorites UI**: Full view for managing pinned parking slots with live availability status
- **OpenAPI docs**: 30+ annotated endpoints — translations and recommendations schemas registered
- **Dashboard analytics**: 7-day booking activity bar chart with real booking data
- **DataTable CSV export**: Download any data table as CSV with proper cell escaping
- **A11y audit fixes**: ARIA labels on icon buttons, contrast fixes, confirm dialogs replacing window.confirm
- **Demo reset tracking**: `last_reset_at`, `next_scheduled_reset`, `reset_in_progress` in status API + overlay
- **PUSH_SUBSCRIPTIONS drain**: Demo reset now properly clears push subscription table

### Changed
- Clippy pedantic: `map_or`, `let...else`, format string inlining across translation + recommendation handlers
- API client: 4 `any` types replaced with proper TypeScript interfaces
- Version bumped to 1.9.0

### Tests
- **505 Rust + 484 PHP + 401 Frontend** = 1,390 total tests

---

## [1.6.0] - 2026-03-20

### Added
- **Typed AppError handling**: Structured error responses with consistent error codes across all endpoints
- **Demo reset with DB wipe**: Full database clear and re-seed on demo reset (not just soft reset)
- **Auto-reset scheduler**: Demo mode auto-resets every 6 hours with countdown in DemoOverlay
- **React 19 useActionState**: Form handling migrated to React 19 `useActionState` pattern
- **Tailwind CSS 4 @utility**: Custom utilities via Tailwind CSS 4 `@utility` directives
- **Admin user search**: Search/filter users by name, email, or role in admin panel
- **Rate-limited demo endpoints**: Demo reset and status endpoints are rate-limited to prevent abuse

### Tests
- **965 tests total**: 426 Rust + 213 Vitest + 326 PHP (up from 727 in v1.5.4)

---

## [1.5.4] - 2026-03-20

### Added
- **Book a Spot page**: 3-step guided booking flow — lot → slot → confirm (fixes #20)
- **Command Palette** (Ctrl+K): quick navigation and actions from anywhere
- **Admin bar chart**: visual booking statistics on admin dashboard
- **Forgot Password page**: self-service password reset flow with email link
- **404 page**: custom not-found page with navigation back to dashboard
- **Playwright E2E tests**: browser-based end-to-end test suite
- **Lighthouse CI**: automated performance, accessibility, and best practices auditing

### Fixed
- **Dark mode (Tailwind CSS 4)**: resolved compatibility issues with Tailwind CSS v4 dark mode
- **Shared constants**: extracted magic numbers and strings into shared constants (code review)
- **N+1 query elimination**: optimized database queries to batch-load related records (code review)

### Tests
- **727 tests total**: 327 Rust + 197 Vitest + 203 PHP (up from 434 in v1.4.8)

---

## [1.4.8] - 2026-03-19

### Design
- **Full UI overhaul**: Eliminated AI slop patterns across all 12+ views
- Welcome: left-aligned layout, inline features, no floating shapes or 3-column grid
- Login: dark panel with specific copy, clean form, no decorative elements
- Dashboard: clean stat cards, tabular-nums, real action buttons
- Bookings: 2px left-border status accents, text badges
- Profile: neutral avatar, clean stats, GDPR section
- Layout: flat sidebar, left-border active indicator, no glass/blur
- Admin: plain text headers, clean data tables
- CSS: 12px card radius, 8px button radius, solid backgrounds, system font
- Specific copy replacing generic AI marketing language

### Added
- **434 tests**: 147 Rust + 150 PHP (376 assertions) + 137 frontend vitest
- **Maestro E2E**: 5 browser flows (welcome, login, dashboard, admin, login failure)
- **1-month simulation**: 294 bookings, EUR 5,007 revenue simulated successfully
- **Prometheus metrics middleware**: HTTP request duration/count, auth/booking events
- **Global rate limiting**: 100 req/s burst 200 on all routes
- **OpenAPI annotations**: 18 handler endpoints in Swagger UI
- **Skeleton loading**: contextual skeleton screens for Dashboard, Bookings, Vehicles
- **i18n**: 50+ translation keys for notifications, calendar, team, profile (EN + DE)
- **Dynamic version**: reads from package.json at build time
- **Render env var automation**: deploy workflow sets env vars via API

### Fixed
- Demo login credentials (admin@parkhub.test / demo) — seeder, entrypoint, env vars
- DemoOverlay [object Object] / NaN — normalize nested API response
- FeaturesContext crash (api.getFeatures not a function)
- Welcome screen not showing for first-time visitors
- PHP DemoController wrong config key (test_mode → demo_mode)
- PHP User $fillable missing 'role' — setup wizard admin got role=user
- PHP audit_log table name typo in GDPR anonymize
- Rate limiter panic on zero config values
- Admin password exposed via CLI arg (now env var)

### Security
- Rate limiter: clamp config values to >=1 (prevents panic)
- Admin password: passed via env var, not CLI arg
- cargo audit: 1 known advisory (RSA timing in jsonwebtoken, no fix available)

---

## [1.3.7] - 2026-03-19

### Added
- **Prometheus metrics middleware**: HTTP request duration/count, auth events (login success/fail), booking events (created/cancelled) recorded for every request
- **Global rate limiting**: 100 req/s with burst 200 on all routes (in addition to per-IP auth rate limits)
- **Periodic gauge updates**: Lot occupancy and active booking counts updated every 5 minutes via cron
- **OpenAPI annotations**: 18 handler endpoints annotated with `#[utoipa::path]` — Swagger UI now fully populated for auth, lots, and credits APIs
- **Frontend Vitest tests**: 33 tests across 3 files (API client, DemoOverlay, Login) — vitest + @testing-library/react
- **Use-case context providers**: `UseCaseProvider` and `FeaturesProvider` wired into App.tsx provider tree
- **i18n keys**: Added `useCase.*` and `features.*` translation keys in English and German for UseCaseSelector page
- **PWA support**: manifest.json, service worker registration, apple-mobile-web-app meta tags

### Fixed
- **AdminSettings use-case dropdown**: Options now match backend presets (company, residential, shared, rental, personal) instead of stale corporate/university/other
- **Metric path normalization**: UUIDs and numeric IDs collapsed to `:id` to prevent Prometheus label cardinality explosion
- **Clippy clean**: Resolved `if_same_then_else` in metric path normalization

### Improved
- **Test coverage**: 77 Rust tests (60 server + 17 common), 33 frontend vitest tests, all passing
- **OpenAPI schemas**: Request/response types registered in ApiDoc for complete Swagger documentation

---

## [1.3.0] - 2026-03-18

### Added
- **Demo auto-reset**: Scheduled auto-reset every 6 hours when `DEMO_MODE=true` — clears all data and re-seeds
- **Demo reset button**: Manual reset via `POST /api/v1/demo/reset` with actual database wipe + re-seed
- **Demo status tracking**: `GET /api/v1/demo/status` now returns `last_reset_at`, `next_scheduled_reset`, `reset_in_progress`
- **DemoOverlay countdown**: Frontend shows time since last reset, countdown to next auto-reset, and reset-in-progress indicator
- **Database clear method**: `Database::clear_all_data()` for full table drain while preserving settings

### Fixed
- **Silent error ignores**: Replaced all `let _ =` patterns with `tracing::warn` logging for credit transactions, GDPR operations, and settings saves
- **Absence date parsing**: Replaced `unwrap()` with safe `Option` chaining in absence date filtering (prevented potential panics)
- **CI pipeline**: Removed `|| true` from clippy and test steps in Gitea CI (errors were silently ignored)
- **Duplicate scheduling**: Removed duplicate auto-release job in PHP scheduler (ran twice every 5 min)
- **GDPR export route**: Fixed broken `/users/me/export` route pointing to wrong method name (PHP)
- **Swap race condition**: Wrapped slot swap in `DB::transaction` with `lockForUpdate` (PHP)
- **Admin pagination**: Added pagination to admin bookings endpoint to prevent memory exhaustion (PHP)

### Improved
- **Dead code warnings**: Reduced from 46 to 0 by adding `#[allow(dead_code)]` on scaffolding modules
- **Auth response**: Removed unnecessary `User::clone()` in login/register responses
- **iCal import**: Added date validation and title truncation to prevent crashes on malformed input (PHP)
- **Demo reset error handling**: Returns HTTP 500 on failure instead of silently swallowing exceptions (PHP)

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
