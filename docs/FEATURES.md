# ParkHub Feature Showcase

> **Self-hosted parking management for enterprises, universities, and residential complexes.**
> Single Rust binary · Zero cloud · Zero tracking · 100% GDPR compliant.

[Live Demo](https://parkhub-rust-demo.onrender.com) · [API Docs](API.md) · [Installation](INSTALLATION.md) · [GDPR Guide](GDPR.md)

---

## Table of Contents

1. [For Building Owners](#1-for-building-owners)
2. [For Developers](#2-for-developers)
3. [For Operations](#3-for-operations)
4. [API Examples](#4-api-examples)
5. [Integration Guide](#5-integration-guide)
6. [Deployment Options](#6-deployment-options)
7. [Pricing Model](#7-pricing-model)

---

## 1. For Building Owners

ParkHub gives property managers full visibility and control over every parking space — from a single lot to a multi-building campus — without handing data to a third-party cloud.

### Multi-Tenant Isolation

Manage multiple buildings or customer organisations from a single ParkHub instance. Each tenant gets isolated data, custom branding (logo, colours, company name), and its own admin team — with a super-admin account that spans all tenants.

- Tenant CRUD via admin UI and REST API
- `tenant_id` scoping on every user, lot, and booking record
- Per-tenant branding: logo upload, primary colour, company name
- Super-admin cross-tenant reporting and user management

### Analytics & Revenue Dashboard

The built-in analytics dashboard (`/admin/analytics`) gives building owners actionable insight without connecting external BI tools.

| Metric | Details |
|--------|---------|
| Daily bookings & revenue | Line/bar chart with date-range selector |
| Peak-hour heatmap | 7-day × 24-hour occupancy matrix |
| Top lots by utilisation | Ranked table with occupancy % |
| User growth | New registrations over time |
| Cost-centre billing | Revenue attribution per department/team |

All charts export to CSV. Scheduled email digests (daily/weekly/monthly) deliver the same data straight to inboxes — no login required to stay informed.

### Revenue Tracking

- **Credits system** — Users draw down monthly quotas; admins top up or adjust per-user
- **Dynamic pricing** — Occupancy-based surge and discount thresholds (configurable multipliers)
- **Parking zones with pricing tiers** — Economy / Standard / Premium / VIP tiers per zone
- **PDF invoices** — Auto-generated per booking with VAT breakdown, downloadable by users and admins
- **Stripe integration** — Credit purchases via hosted Stripe Checkout; webhook handler for payment events
- **Cost-centre billing** — Allocate credits and export billing data by department code, CSV-ready

### Occupancy & Space Management

- Visual per-floor grid editor with drag-and-drop slot placement
- Real-time occupancy counters and colour-coded availability
- Accessible parking slots with 30-minute priority booking for users with disabilities
- EV charging stations: Type 2 / CCS / CHAdeMO / Tesla, live kWh tracking, utilisation reports
- Maintenance windows that automatically block affected slots and notify users
- Lobby / kiosk display mode (`/lobby/:lotId`) for on-site monitors — no authentication required

### Communication & Announcements

- Admin-authored announcements with configurable expiry, shown to all users on login
- In-app notification feed with read/unread tracking
- Web Push notifications (VAPID) with action buttons, handled by service worker
- Email notification templates: booking confirmation, reminder, cancellation, password reset, welcome, weekly summary

---

## 2. For Developers

ParkHub is designed to be embedded, extended, and automated. Every feature exposed in the UI has a corresponding REST endpoint, most operations are also available via GraphQL, and the modular build system means you ship only the code you need.

### REST API — 125+ Endpoints

All endpoints are documented in [API.md](API.md) and exposed interactively at `/swagger-ui` (OpenAPI 3.0). The API follows a standard JSON envelope:

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "meta": null
}
```

Key resource groups:

| Group | Endpoints |
|-------|-----------|
| Auth (login, register, refresh, 2FA, OAuth, SAML) | `/api/v1/auth/*` |
| Bookings (CRUD, quick-book, checkin, invoice) | `/api/v1/bookings/*` |
| Parking lots, slots, zones | `/api/v1/lots/*` |
| Vehicles & photos | `/api/v1/vehicles/*` |
| Recurring bookings | `/api/v1/recurring-bookings/*` |
| Guest bookings | `/api/v1/bookings/guest` |
| Waitlist | `/api/v1/waitlist/*` |
| Swap requests | `/api/v1/swap-requests/*` |
| Webhooks | `/api/v1/webhooks/*` |
| Admin — users, bookings, analytics, settings | `/api/v1/admin/*` |
| Metrics (Prometheus) | `/metrics` |
| Public display / occupancy | `/api/v1/public/*` |

Full OpenAPI JSON spec: `GET /api-docs/openapi.json`

### GraphQL API

A full GraphQL schema runs alongside the REST API, with an interactive GraphiQL playground at `/graphql`. Use it to fetch exactly the fields you need in a single round-trip — no over-fetching.

### 64 Cargo Feature Flags

Every major feature is an independent Cargo feature flag. Enable only what you need to minimise binary size, attack surface, and compile time.

```toml
# Minimal booking-only build (~8 MB)
--no-default-features --features "headless,mod-bookings,mod-email"

# Full build with all modules
--features full
```

Selected flags:

| Flag | Module |
|------|--------|
| `mod-bookings` | Core booking lifecycle |
| `mod-recurring` | Recurring reservation patterns |
| `mod-guest` | Guest bookings without accounts |
| `mod-waitlist` / `mod-waitlist-ext` | Basic and enhanced waitlist |
| `mod-webhooks` | Outbound HMAC-signed webhooks |
| `mod-graphql` | GraphQL schema and playground |
| `mod-plugins` | Trait-based plugin architecture |
| `mod-analytics` | Admin analytics dashboard |
| `mod-multi-tenant` | Multi-tenant isolation |
| `mod-saml` / `mod-oauth` | Enterprise SSO and social login |
| `mod-compliance` | GDPR compliance reports and audit trail |
| `mod-ev-charging` | EV charging station management |
| `mod-geofence` | GPS-based auto check-in |
| `mod-stripe` | Stripe payment integration |
| `mod-map` | Interactive Leaflet map view |
| `mod-scheduled-reports` | Email digest delivery |
| `mod-api-versioning` | `X-API-Version` header and sunset notices |
| `mod-cost-center` | Cost-centre billing analytics |
| `mod-maintenance` | Maintenance scheduling |
| `mod-fleet` | Fleet / vehicle management overview |

See the [Module System section in README.md](../README.md#module-system) for the full list.

### Plugin / Extension System

ParkHub exposes a trait-based plugin API with lifecycle hooks. Plugins can react to booking events, inject custom logic, and surface admin UI panels.

Two built-in plugins ship out of the box:

- **Slack Notifier** — Posts booking confirmations and cancellations to a Slack channel
- **Auto-Assign Preferred Spot** — Automatically books a user's favourite slot when available

To write a plugin, implement the `ParkHubPlugin` trait and register it at startup. See the plugin module in `parkhub-server/src/plugins/` for examples.

### API Versioning & Deprecation

- `X-API-Version` request header selects the target API version
- Deprecated endpoints return a `Deprecation` response header with a sunset date
- `Sunset` header signals the removal date per RFC 8594
- Version changelog available at `GET /api/v1/version`

### Authentication Options

| Method | Details |
|--------|---------|
| Email / password | Argon2id hashing, httpOnly cookie + Bearer token |
| 2FA / TOTP | QR code enrollment, 8 single-use backup codes |
| OAuth (Google, GitHub) | Self-service; operators configure their own app credentials |
| SAML 2.0 / SSO | Full IdP integration with redirect and response parsing |
| API keys | Long-lived keys for service-to-service calls |
| Session management | List and revoke active tokens from account settings |

### Developer Tooling

- **Postman collection** — 100+ pre-configured requests in 17 folders (`docs/postman/`)
- **k6 load test suite** — Smoke, load, stress, and spike scripts in `tests/load/`
- **OpenAPI spec** — Auto-generated from code via `utoipa`, always in sync
- **WebSocket endpoint** — Real-time booking and occupancy events with token auth and heartbeat

---

## 3. For Operations

### Fleet Management

The fleet overview (`/admin/fleet`) gives operations teams a live snapshot of every registered vehicle on the platform:

- Type distribution chart (car, motorcycle, van, truck)
- Electric vehicle ratio and EV-charger utilisation
- Per-vehicle flagging for compliance checks
- Bulk CSV/JSON import for initial fleet onboarding
- CSV export for audits and insurance reporting

### Maintenance Scheduling

Schedule maintenance windows per lot or per slot without touching code:

- CRUD for maintenance windows with start/end time and description
- Affected slots are automatically blocked — no double-bookings possible
- Active maintenance banners appear in the booking UI
- Booking overlap validation rejects requests that conflict with a window
- Audit log entry created for every window change

### EV Charging Stations

Manage EV infrastructure alongside parking:

- Register chargers with connector type (Type 2, CCS, CHAdeMO, Tesla)
- Start and stop charging sessions via API or admin UI
- kWh tracking per session for accurate cost allocation
- Admin utilisation statistics and session history

### Compliance & Audit

ParkHub is audited for GDPR/DSGVO, UK GDPR, CCPA, nDSG, and TTDSG.

| Compliance feature | Details |
|--------------------|---------|
| Art. 15 — Data access | `GET /api/v1/users/me/export` returns a full JSON data package |
| Art. 17 — Right to erasure | `DELETE /api/v1/users/me/delete` anonymises PII; booking records retained per AO §147 (10-year tax retention) |
| Art. 30 — Processing record | Auto-generated data map in compliance dashboard |
| Audit log | Every write operation logged with actor, action, timestamp, and affected resource |
| Audit export | Filter by action type, user, or date range; export as CSV or PDF |
| Compliance dashboard | 10 automated GDPR checks with pass/fail status |
| TOM summary | Technical and organisational measures documented and exportable |
| DDG §5 Impressum | Operator-customisable legal notice served at `/api/v1/legal/impressum` |

### Observability

- **Prometheus metrics** at `/metrics` — booking counts, latency histograms, active sessions, error rates
- **Structured logging** — JSON-formatted log lines with request IDs, compatible with Loki / ELK
- **K8s health probes** — `/health/live` (liveness) and `/health/ready` (readiness)
- **Lighthouse CI** — Automated accessibility (≥ 95), performance (≥ 90), SEO (≥ 95) scores on every commit
- **Distributed tracing** — OpenTelemetry-compatible spans

### Security Operations

| Control | Implementation |
|---------|---------------|
| Authentication | httpOnly SameSite=Lax cookies + Bearer fallback |
| Password hashing | Argon2id, always in `spawn_blocking` |
| Database at rest | Optional AES-256-GCM (`PARKHUB_DB_PASSPHRASE`) |
| TLS | rustls 1.3, auto-generated cert, no OpenSSL |
| Token comparison | constant-time via `subtle` crate |
| Rate limiting | 5 login/min per IP, 100 req/s global (burst 200) |
| Security headers | Nonce-based CSP, HSTS, `X-Frame-Options`, `X-Content-Type-Options` |
| Input validation | Length-bounded on every endpoint, 4 MiB body limit |
| Session hygiene | Password change invalidates all other sessions |
| Login history | IP address and user-agent stored per session |

---

## 4. API Examples

Set up a shell variable with your access token before running the examples:

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin@example.com","password":"yourpassword"}' \
  | jq -r '.data.tokens.access_token')
```

### Create a Booking

```bash
curl -X POST http://localhost:8080/api/v1/bookings \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "lot_id": "lot_abc123",
    "slot_id": "slot_def456",
    "start_time": "2025-06-01T08:00:00Z",
    "end_time":   "2025-06-01T18:00:00Z"
  }'
```

### Quick-Book (auto-assign best available slot)

```bash
curl -X POST http://localhost:8080/api/v1/bookings/quick \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "lot_id": "lot_abc123",
    "start_time": "2025-06-01T08:00:00Z",
    "end_time":   "2025-06-01T18:00:00Z"
  }'
```

### Check In to a Booking

```bash
curl -X POST http://localhost:8080/api/v1/bookings/booking_xyz789/checkin \
  -H "Authorization: Bearer $TOKEN"
```

### List Admin Analytics Stats

```bash
curl -X GET "http://localhost:8080/api/v1/admin/stats" \
  -H "Authorization: Bearer $TOKEN"
```

Sample response:

```json
{
  "success": true,
  "data": {
    "total_bookings": 4823,
    "active_bookings": 47,
    "total_revenue_cents": 982400,
    "occupancy_rate": 0.73,
    "top_lots": [
      { "id": "lot_abc123", "name": "North Garage", "occupancy": 0.91 }
    ]
  }
}
```

### Retrieve Revenue Chart Data

```bash
curl -X GET "http://localhost:8080/api/v1/admin/dashboard/charts?range=30d" \
  -H "Authorization: Bearer $TOKEN"
```

### Register a Webhook

```bash
curl -X POST http://localhost:8080/api/v1/webhooks \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-system.example.com/parkhub-events",
    "events": ["booking.created", "booking.cancelled"],
    "secret": "your_signing_secret"
  }'
```

### Test a Webhook (trigger a sample delivery)

```bash
curl -X POST http://localhost:8080/api/v1/webhooks/wh_id123/test \
  -H "Authorization: Bearer $TOKEN"
```

### Export User Data (GDPR Art. 15)

```bash
curl -X GET http://localhost:8080/api/v1/users/me/export \
  -H "Authorization: Bearer $TOKEN" \
  -o my-data.json
```

### Request Account Erasure (GDPR Art. 17)

```bash
curl -X DELETE http://localhost:8080/api/v1/users/me/delete \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"confirm": true}'
```

### Subscribe to iCal Feed

```bash
# Obtain the personal iCal URL
curl -X GET http://localhost:8080/api/v1/user/calendar.ics \
  -H "Authorization: Bearer $TOKEN"
# Add the returned URL to Google Calendar, Outlook, or Apple Calendar
```

### Prometheus Metrics Scrape

```bash
curl http://localhost:8080/metrics
# Returns Prometheus text format — no authentication required
```

---

## 5. Integration Guide

### Webhooks v2

ParkHub delivers outbound events to any HTTPS endpoint. Every delivery is signed with HMAC-SHA256 using a per-webhook secret, so your receiver can verify authenticity.

**Supported event types:**

| Event | Triggered when |
|-------|---------------|
| `booking.created` | A booking is confirmed |
| `booking.cancelled` | A booking is cancelled by user or admin |
| `booking.checked_in` | A user checks in to a booking |
| `user.registered` | A new user account is created |
| `waitlist.offer` | A waitlist slot becomes available |

**Payload structure:**

```json
{
  "event": "booking.created",
  "timestamp": "2025-06-01T08:01:22Z",
  "data": {
    "booking_id": "booking_xyz789",
    "user_id": "user_abc",
    "lot_id": "lot_abc123",
    "slot_id": "slot_def456",
    "start_time": "2025-06-01T08:00:00Z",
    "end_time": "2025-06-01T18:00:00Z"
  }
}
```

**Signature verification (Node.js example):**

```js
const crypto = require('crypto');

function verifySignature(payload, signature, secret) {
  const expected = crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(`sha256=${expected}`),
    Buffer.from(signature)
  );
}
```

Failed deliveries are retried with exponential back-off. Delivery logs (status code, response body, latency) are available at `GET /api/v1/webhooks/:id/deliveries`.

### SSO / SAML 2.0

Enterprise customers can replace email/password login with their corporate Identity Provider (IdP). ParkHub acts as the Service Provider (SP).

**Setup steps:**

1. Go to **Admin → Settings → Authentication → SAML**
2. Copy the SP metadata URL (`/api/v1/auth/saml/metadata`) and register it in your IdP (Okta, Azure AD, Keycloak, etc.)
3. Paste the IdP metadata XML or URL into the ParkHub SAML configuration form
4. Save — users will now see a **"Log in with SSO"** button on the login page

**Attribute mapping:** ParkHub expects `email`, `displayName` (or `givenName` + `sn`), and optionally `role` in the SAML assertion. Custom attribute mapping is configurable in settings.

**OAuth (Google / GitHub):** Operators configure their own OAuth app credentials in **Admin → Settings → OAuth**. No credentials are hard-coded in the binary.

### Calendar Sync (iCal)

Each user has a personal iCal feed URL authenticated by a token embedded in the URL. Subscribe from any standards-compliant calendar client:

- **Google Calendar** — "Other calendars → From URL"
- **Microsoft Outlook** — "Add calendar → Subscribe from web"
- **Apple Calendar** — "File → New Calendar Subscription"

The feed contains all confirmed bookings for the authenticated user and auto-updates as bookings change.

Admins can also export lot-wide booking calendars for operational visibility.

### GraphQL

The GraphQL endpoint is available at `POST /api/v1/graphql`. An interactive GraphiQL playground is at `/graphql` when `mod-graphql` is enabled.

Example query:

```graphql
query MyBookings {
  bookings(status: CONFIRMED) {
    id
    lot { name }
    slot { label floor }
    startTime
    endTime
    checkedIn
  }
}
```

### Mobile / PWA

ParkHub ships a full Progressive Web App (PWA) experience without requiring an app store listing:

- **Offline support** — Service worker caches critical data for offline viewing
- **Install prompt** — Users can add ParkHub to their home screen on iOS and Android
- **Web Push** — VAPID-based push notifications work on Android Chrome and supported iOS versions
- **Bottom navigation** — Mobile-optimised navigation bar
- **Pull-to-refresh** — Native-feel refresh gesture

For native mobile clients, use the REST API directly. Authentication via Bearer token in the `Authorization` header is fully supported alongside the cookie-based web flow. The Postman collection (`docs/postman/`) provides a ready-made starting point.

### Email (SMTP)

Configure SMTP in **Admin → Settings → Email** or via `config.toml`. Supports TLS/STARTTLS. Six HTML email templates are included and can be customised per tenant:

1. Booking confirmation
2. Booking reminder (configurable advance notice)
3. Booking cancellation
4. Password reset
5. Welcome / account activation
6. Weekly summary digest

### Stripe Payments

Enable Stripe to let users purchase credit packs:

1. Add your Stripe secret key and webhook secret in **Admin → Settings → Payments**
2. ParkHub creates a hosted Checkout Session — no card data touches your server
3. Stripe sends a `checkout.session.completed` event to `POST /api/v1/payments/webhook`
4. Credits are automatically credited to the user's account

---

## 6. Deployment Options

ParkHub is a single ~15 MB Rust binary. It embeds the React frontend, the database engine, and the TLS stack — no separate processes required.

### Docker Compose (recommended)

```bash
git clone https://github.com/nash87/parkhub-rust.git
cd parkhub-rust
docker compose up -d
```

The first build compiles Rust + React from source (5–10 minutes). Subsequent starts are instant. The admin password is printed to the container logs on first run.

```bash
docker compose logs parkhub | grep "Admin password"
```

For production, copy and customise the override example:

```bash
cp docker-compose.override.yml.example docker-compose.override.yml
# Set PARKHUB_DB_PASSPHRASE, port, and any feature flags
docker compose up -d
```

### Kubernetes / Helm

A production-ready Helm chart lives in `helm/parkhub/`. It includes:

- Horizontal Pod Autoscaler (HPA) based on CPU/memory
- Persistent Volume Claim (PVC) for the redb database file
- All 57 module feature flags as configurable values
- Ingress resource with optional TLS via cert-manager
- ConfigMap for `config.toml`

```bash
helm install parkhub ./helm/parkhub \
  --set image.tag=latest \
  --set ingress.host=parking.example.com \
  --set ingress.tls.enabled=true \
  --set features.modAnalytics=true \
  --set features.modMultiTenant=true
```

See [helm/README.md](../helm/README.md) for the full values reference.

### Bare Metal / VPS

Download a pre-built binary from [GitHub Releases](https://github.com/nash87/parkhub-rust/releases) or build from source:

```bash
cargo build --release --package parkhub-server \
  --no-default-features --features headless

./target/release/parkhub-server \
  --headless \
  --unattended \
  --port 8080
```

Multi-arch container images are published to `ghcr.io/nash87/parkhub-rust:latest` (x86_64 + ARM64).

### Raspberry Pi

The ARM64 binary runs on a Raspberry Pi 4 or 5 without modification. Recommended setup:

```bash
# On the Pi (ARM64 Raspberry Pi OS)
docker compose up -d          # uses the pre-built multi-arch image
# Or build natively (~20 min on Pi 4)
cargo build --release -p parkhub-server --no-default-features --features headless
```

mDNS auto-discovery (`mod-mdns`) lets LAN clients find the Pi at `parkhub.local` without DNS configuration.

### Windows Desktop

The optional Slint GUI client provides a system-tray icon, integrated setup wizard, and a native window. Build with the `gui` feature:

```bash
cargo build --release --features gui
```

Note: The `gui` feature includes Slint (GPL-3.0 community edition). The headless server binary remains MIT-licensed.

### PaaS (Render, Fly.io, Koyeb)

One-click deploy configurations are included:

| Platform | Config file |
|----------|-------------|
| Render | `render.yaml` |
| Koyeb | `koyeb.yaml` |

The [live demo](https://parkhub-rust-demo.onrender.com) runs on Render and resets every 6 hours.

---

## 7. Pricing Model

### Open Source (MIT)

The entire headless server codebase is published under the **MIT license**. You can:

- Use it commercially without paying any licence fee
- Self-host on your own infrastructure
- Fork and modify without restrictions
- Redistribute under the MIT terms

There is no open-core split, no feature paywalling in the community edition, and no telemetry. Every feature documented here ships in the MIT-licensed binary.

### GUI Client (GPL-3.0)

The optional Windows/macOS desktop client uses the Slint UI framework (GPL-3.0 community edition). Building with `--features gui` therefore produces a GPL-3.0 binary. The headless server (`--no-default-features --features headless`) remains MIT.

### Enterprise Add-Ons (Concept)

The following capabilities are natural candidates for a commercial support or add-ons tier, if the project transitions to a dual-licence model in the future. **They are not currently pay-gated** — this section is informational for investors and evaluators.

| Add-on concept | Description |
|----------------|-------------|
| **Professional support SLA** | Guaranteed response times, escalation path, private Slack channel |
| **Managed cloud hosting** | Hosted ParkHub instance with SLA, backups, and zero-ops onboarding |
| **Advanced SAML / IdP provisioning** | SCIM user sync, Just-In-Time provisioning, directory group → role mapping |
| **Extended audit retention** | Audit log retention beyond the default rolling window |
| **White-label licensing** | Rebrand the binary for resale under your own product name |
| **Custom plugin development** | Bespoke integrations built and maintained by the core team |

### No Lock-In Guarantee

Because ParkHub uses an embedded file-based database (redb), your data is always portable. Export the entire database as a single file, or use the GDPR export endpoints to retrieve structured JSON. Migrating away requires no ETL pipeline.

---

*For installation instructions, see [INSTALLATION.md](INSTALLATION.md).*
*For the full API reference, see [API.md](API.md).*
*For GDPR compliance details, see [GDPR.md](GDPR.md).*
