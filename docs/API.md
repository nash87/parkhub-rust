# API Reference — ParkHub Rust

Full REST API reference for ParkHub Rust (Axum 0.7).

All API endpoints are prefixed with `/api/v1/`.
An interactive Swagger UI is available at `/swagger-ui` when the server is running.
The OpenAPI JSON spec is at `/api-docs/openapi.json`.

---

## Table of Contents

- [Authentication](#authentication)
- [Response Envelope](#response-envelope)
- [Error Codes](#error-codes)
- [Rate Limits](#rate-limits)
- [Health & Discovery](#health--discovery)
- [Auth](#auth)
- [Users & GDPR](#users--gdpr)
- [Parking Lots](#parking-lots)
- [Bookings](#bookings)
- [Vehicles](#vehicles)
- [Legal (DDG §5)](#legal-ddg-5)
- [Admin](#admin)
- [Metrics](#metrics)

---

## Authentication

All protected endpoints require a Bearer token in the `Authorization` header:

```
Authorization: Bearer <access_token>
```

Tokens are obtained from `POST /api/v1/auth/login` or `POST /api/v1/auth/register`.
They expire after **24 hours** (configurable via `session_timeout_minutes`).

Set the token as a shell variable for the curl examples below:

```bash
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}' \
  | jq -r '.data.tokens.access_token')
```

---

## Response Envelope

All API responses use a standard envelope:

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "meta": null
}
```

Error response:

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "INVALID_CREDENTIALS",
    "message": "Invalid username or password"
  }
}
```

---

## Error Codes

| Code | HTTP | Meaning |
|------|------|---------|
| `UNAUTHORIZED` | 401 | Missing or expired Bearer token |
| `FORBIDDEN` | 403 | Authenticated but insufficient role |
| `NOT_FOUND` | 404 | Resource does not exist |
| `CONFLICT` | 409 | Duplicate resource or state conflict |
| `SLOT_UNAVAILABLE` | 409 | Slot is already booked for the requested time |
| `ALREADY_CANCELLED` | 409 | Booking is already cancelled |
| `INVALID_CREDENTIALS` | 401 | Wrong username or password |
| `ACCOUNT_DISABLED` | 403 | Account deactivated by an admin |
| `PROTOCOL_MISMATCH` | 400 | Client and server protocol versions incompatible |
| `RATE_LIMITED` | 429 | Too many requests |
| `SERVER_ERROR` | 500 | Internal server error |

---

## Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| `POST /api/v1/auth/login` | 5 requests | per minute per IP |
| `POST /api/v1/auth/register` | 3 requests | per minute per IP |
| All other routes | 100 req/s global | burst: 200 |

Returns HTTP 429 when exceeded.

---

## Health & Discovery

### GET /health

Simple liveness. Returns `OK` (plain text, HTTP 200).

```bash
curl http://localhost:8080/health
# OK
```

### GET /health/live

Kubernetes liveness probe. HTTP 200 if the process is alive.

```bash
curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health/live
# 200
```

### GET /health/ready

Kubernetes readiness probe. Checks that the database is accessible.

```bash
curl http://localhost:8080/health/ready
# {"ready":true}
```

Returns HTTP 503 with `{"ready":false}` if the database is not available.

### GET /status

Server statistics. No authentication required.

```bash
curl http://localhost:8080/status
```

Response:

```json
{
  "success": true,
  "data": {
    "uptime_seconds": 3600,
    "connected_clients": 0,
    "total_users": 5,
    "total_bookings": 42
  }
}
```

### POST /handshake

Protocol version negotiation for native clients.

```bash
curl -s -X POST http://localhost:8080/handshake \
  -H "Content-Type: application/json" \
  -d '{"client_version":"1.0.0","protocol_version":"1"}'
```

Response:

```json
{
  "success": true,
  "data": {
    "server_name": "ParkHub Server",
    "server_version": "1.0.0",
    "protocol_version": "1",
    "requires_auth": true,
    "certificate_fingerprint": ""
  }
}
```

Returns `PROTOCOL_MISMATCH` if versions are incompatible.

---

## Auth

### POST /api/v1/auth/login

Authenticate with username (or email) and password. Rate limited: 5/min per IP.

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}'
```

Request body:

```json
{
  "username": "admin",
  "password": "admin"
}
```

The `username` field accepts either a username or an email address.

Response (HTTP 200):

```json
{
  "success": true,
  "data": {
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "admin",
      "email": "admin@example.com",
      "name": "Admin",
      "role": "admin",
      "is_active": true
    },
    "tokens": {
      "access_token": "550e8400-e29b-41d4-a716-446655440001",
      "refresh_token": "550e8400-e29b-41d4-a716-446655440002",
      "expires_at": "2026-03-01T12:00:00Z",
      "token_type": "Bearer"
    }
  }
}
```

The `password_hash` field is never included in responses.

### POST /api/v1/auth/register

Register a new user. Only available when `allow_self_registration = true` in config.
Rate limited: 3/min per IP.

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure123",
    "name": "Max Mustermann"
  }'
```

Response: same as login (HTTP 201 Created).

### POST /api/v1/auth/refresh

Refresh an access token using the refresh token.

> Note: This endpoint returns HTTP 501 in v1.0.0 — token refresh is planned for a future release.
> Clients must re-authenticate after the 24-hour session expires.

### POST /api/v1/auth/forgot-password

Request a password reset link. Returns a generic success message regardless of whether
the email exists (prevents user enumeration).

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"user@example.com"}'
```

### POST /api/v1/auth/reset-password

Complete a password reset using the token received via email.

---

## Users & GDPR

### GET /api/v1/users/me

Return the authenticated user's profile.

```bash
curl -s http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $TOKEN"
```

Response: `User` object (password hash excluded).

### GET /api/v1/users/:id

Get a user by ID. **Requires admin or superadmin role.**

```bash
curl -s "http://localhost:8080/api/v1/users/550e8400-e29b-41d4-a716-446655440000" \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/users/me/export

Export all personal data for the authenticated user.
Implements **GDPR Art. 15 (Right of Access)** and **Art. 20 (Data Portability)**.

```bash
curl -s http://localhost:8080/api/v1/users/me/export \
  -H "Authorization: Bearer $TOKEN" \
  -o my-data-export.json
```

Response (JSON file):

```json
{
  "exported_at": "2026-02-27T12:00:00Z",
  "gdpr_basis": "GDPR Art. 15 — Right of Access",
  "profile": {
    "id": "...",
    "username": "user",
    "email": "user@example.com",
    "name": "Max Mustermann",
    "role": "user"
  },
  "bookings": [ ... ],
  "vehicles": [ ... ]
}
```

The `password_hash` is intentionally excluded from exports.

### DELETE /api/v1/users/me/delete

Delete the authenticated user's account.
Implements **GDPR Art. 17 (Right to Erasure)** while complying with **§147 AO**.

**What this endpoint does:**

1. Anonymizes the user record: `name`, `email`, `username`, `phone`, `picture` → `[DELETED]`
2. Deletes all registered vehicles
3. Retains booking records but replaces the licence plate with `[DELETED]`
   (§147 AO requires 10-year retention of accounting records)
4. Invalidates all active sessions

```bash
curl -s -X DELETE http://localhost:8080/api/v1/users/me/delete \
  -H "Authorization: Bearer $TOKEN"
```

Response: `{"success":true}` on success (HTTP 200).

---

## Parking Lots

### GET /api/v1/lots

List all parking lots with live availability counts. Auth required.

```bash
curl -s http://localhost:8080/api/v1/lots \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "name": "Parkplatz A",
      "address": "Musterstraße 1, 80331 München",
      "total_slots": 50,
      "available_slots": 23,
      "status": "active"
    }
  ]
}
```

### GET /api/v1/lots/:id

Get a single parking lot by UUID. Auth required.

```bash
curl -s "http://localhost:8080/api/v1/lots/LOT_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/lots

Create a new parking lot. **Requires admin or superadmin role.**

```bash
curl -s -X POST http://localhost:8080/api/v1/lots \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "550e8400-0000-0000-0000-000000000001",
    "name": "Parkplatz B",
    "address": "Hauptstraße 5, 80333 München",
    "total_slots": 30,
    "available_slots": 30,
    "floors": [],
    "amenities": [],
    "pricing": {"hourly_rate": 2.0, "daily_rate": 0, "currency": "EUR"},
    "operating_hours": {"open": "06:00", "close": "22:00", "is_24h": false},
    "images": [],
    "status": "active"
  }'
```

### GET /api/v1/lots/:id/slots

List all parking slots in a lot with their current status. Auth required.

```bash
curl -s "http://localhost:8080/api/v1/lots/LOT_UUID/slots" \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "lot_id": "lot-uuid",
      "floor_id": "floor-uuid",
      "slot_number": 1,
      "status": "available",
      "slot_type": "standard"
    }
  ]
}
```

Slot statuses: `available`, `occupied`, `reserved`, `maintenance`, `disabled`

---

## Bookings

### GET /api/v1/bookings

List all bookings for the authenticated user.

```bash
curl -s http://localhost:8080/api/v1/bookings \
  -H "Authorization: Bearer $TOKEN"
```

Response: array of `Booking` objects including lot name, slot number, vehicle plate,
start/end times, status, and pricing.

### POST /api/v1/bookings

Create a new booking. The slot must be in `available` status.
A write lock is held during the availability check and insert, preventing double-bookings.

```bash
curl -s -X POST http://localhost:8080/api/v1/bookings \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "slot_id": "SLOT_UUID",
    "lot_id": "LOT_UUID",
    "vehicle_id": "VEHICLE_UUID",
    "start_time": "2026-03-01T09:00:00Z",
    "duration_minutes": 480,
    "license_plate": "M-AB 1234"
  }'
```

Request fields:

| Field | Required | Description |
|-------|----------|-------------|
| `slot_id` | Yes | UUID of the slot to book |
| `lot_id` | Yes | UUID of the parking lot |
| `vehicle_id` | No | UUID of a registered vehicle. If omitted, `license_plate` is used |
| `license_plate` | No | Licence plate for ad-hoc bookings (used when `vehicle_id` is absent) |
| `start_time` | Yes | ISO 8601 UTC datetime |
| `duration_minutes` | Yes | Positive integer. Pricing: 2 EUR/hour + 10% tax |
| `notes` | No | Optional free-text notes |

Response: created `Booking` object (HTTP 201). Includes a QR code ID.

Returns HTTP 409 `SLOT_UNAVAILABLE` if the slot is already booked.

### GET /api/v1/bookings/:id

Get a specific booking. Users can only access their own bookings.

```bash
curl -s "http://localhost:8080/api/v1/bookings/BOOKING_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### DELETE /api/v1/bookings/:id

Cancel a booking. Only `confirmed` and `pending` bookings can be cancelled.
Cancelling automatically restores the slot to `available` status.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/bookings/BOOKING_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/bookings/:id/invoice

Get an HTML invoice for a booking (printer-friendly, use browser Print → Save as PDF).

```bash
curl -s "http://localhost:8080/api/v1/bookings/BOOKING_UUID/invoice" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Vehicles

### GET /api/v1/vehicles

List all vehicles registered by the authenticated user.

```bash
curl -s http://localhost:8080/api/v1/vehicles \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/vehicles

Register a new vehicle.

```bash
curl -s -X POST http://localhost:8080/api/v1/vehicles \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "license_plate": "M-AB 1234",
    "make": "BMW",
    "model": "3er",
    "color": "Schwarz"
  }'
```

Request fields:

| Field | Required | Description |
|-------|----------|-------------|
| `license_plate` | Yes | Licence plate (auto-uppercased server-side) |
| `make` | No | Vehicle manufacturer (e.g. `BMW`) |
| `model` | No | Vehicle model (e.g. `3er`) |
| `color` | No | Vehicle color |

Response: created `Vehicle` object (HTTP 201).

### DELETE /api/v1/vehicles/:id

Delete a registered vehicle. Only the vehicle owner can delete it.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/vehicles/VEHICLE_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Legal (DDG §5)

### GET /api/v1/legal/impressum

Retrieve the Impressum data. **No authentication required.**
DDG §5 requires this endpoint to be publicly accessible.

```bash
curl -s http://localhost:8080/api/v1/legal/impressum
```

Response:

```json
{
  "success": true,
  "data": {
    "provider_name": "Muster GmbH",
    "provider_legal_form": "GmbH",
    "street": "Musterstraße 1",
    "zip_city": "80331 München",
    "country": "Deutschland",
    "email": "info@muster.de",
    "phone": "+49 89 123456",
    "register_court": "Amtsgericht München",
    "register_number": "HRB 123456",
    "vat_id": "DE123456789",
    "responsible_person": "Max Mustermann",
    "custom_text": ""
  }
}
```

---

## Admin

All admin endpoints require `role=admin` or `role=superadmin`.

### GET /api/v1/admin/impressum

Retrieve the Impressum configuration for the admin editor.

```bash
curl -s http://localhost:8080/api/v1/admin/impressum \
  -H "Authorization: Bearer $TOKEN"
```

### PUT /api/v1/admin/impressum

Update the Impressum fields.

```bash
curl -s -X PUT http://localhost:8080/api/v1/admin/impressum \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "provider_name": "Muster GmbH",
    "provider_legal_form": "GmbH",
    "street": "Musterstraße 1",
    "zip_city": "80331 München",
    "country": "Deutschland",
    "email": "info@muster.de",
    "phone": "+49 89 123456",
    "register_court": "Amtsgericht München",
    "register_number": "HRB 123456",
    "vat_id": "DE123456789",
    "responsible_person": "Max Mustermann",
    "custom_text": ""
  }'
```

### GET /api/v1/admin/users

List all users. **Requires admin or superadmin role.**

```bash
curl -s http://localhost:8080/api/v1/admin/users \
  -H "Authorization: Bearer $TOKEN"
```

### PATCH /api/v1/admin/users/:id/role

Update a user's role. **Requires admin or superadmin role.**

```bash
curl -s -X PATCH "http://localhost:8080/api/v1/admin/users/USER_UUID/role" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"role": "admin"}'
```

Valid roles: `user`, `admin`, `superadmin`

### PATCH /api/v1/admin/users/:id/status

Activate or deactivate a user account.

```bash
curl -s -X PATCH "http://localhost:8080/api/v1/admin/users/USER_UUID/status" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"is_active": false}'
```

### DELETE /api/v1/admin/users/:id

Delete a user account. **Requires admin or superadmin role.**

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/admin/users/USER_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/admin/bookings

List all bookings across all users. **Requires admin or superadmin role.**

```bash
curl -s http://localhost:8080/api/v1/admin/bookings \
  -H "Authorization: Bearer $TOKEN"
```

---

## Metrics

### GET /metrics

Prometheus-format metrics. No authentication required.
Use with a Prometheus scrape job to monitor the server.

```bash
curl http://localhost:8080/metrics
```

Returns metrics in Prometheus exposition format (`text/plain; charset=utf-8`).

Example scrape config:

```yaml
scrape_configs:
  - job_name: parkhub
    static_configs:
      - targets: ['parkhub:8080']
    metrics_path: /metrics
```

---

## Swagger UI

The interactive API documentation is available at `/swagger-ui` when the server is running.
The OpenAPI JSON spec is at `/api-docs/openapi.json`.

```bash
# Download OpenAPI spec
curl http://localhost:8080/api-docs/openapi.json -o parkhub-openapi.json
```

---

## Request Limits Summary

| Constraint | Value |
|-----------|-------|
| Maximum request body | 1 MiB (HTTP 413 if exceeded) |
| Login rate limit | 5 requests/minute per IP |
| Register rate limit | 3 requests/minute per IP |
| Global rate limit | 100 req/s (burst: 200) |
| Token expiry | 24 hours |
