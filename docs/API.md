# API Reference — ParkHub Rust

Full REST API reference for ParkHub Rust (Axum 0.8).

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
- [Setup Wizard](#setup-wizard)
- [Auth](#auth)
- [Users & GDPR](#users--gdpr)
- [User Stats & Preferences](#user-stats--preferences)
- [Parking Lots](#parking-lots)
- [Slots](#slots)
- [Zones](#zones)
- [Bookings](#bookings)
- [Booking Check-in](#booking-check-in)
- [Calendar & iCal](#calendar--ical)
- [Vehicles](#vehicles)
- [Vehicle Photos](#vehicle-photos)
- [Credits](#credits)
- [Favorites](#favorites)
- [Notifications](#notifications)
- [Absences](#absences)
- [Team](#team)
- [Waitlist](#waitlist)
- [Swap Requests](#swap-requests)
- [Recurring Bookings](#recurring-bookings)
- [Guest Bookings](#guest-bookings)
- [Announcements](#announcements)
- [Webhooks](#webhooks)
- [Web Push](#web-push)
- [Public Display](#public-display)
- [QR Codes](#qr-codes)
- [Legal (DDG §5)](#legal-ddg-5)
- [Admin — User Management](#admin--user-management)
- [Admin — Bookings & Export](#admin--bookings--export)
- [Admin — Settings](#admin--settings)
- [Admin — Reports & Dashboard](#admin--reports--dashboard)
- [Admin — Database Reset](#admin--database-reset)
- [Demo Mode](#demo-mode)
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
| `INVALID_STATUS` | 409 | Booking is not in the right status for this action |
| `INVALID_CREDENTIALS` | 401 | Wrong username or password |
| `ACCOUNT_DISABLED` | 403 | Account deactivated by an admin |
| `SETUP_COMPLETED` | 400 | Initial setup has already been completed |
| `CONFIRMATION_REQUIRED` | 400 | Destructive action needs explicit confirmation |
| `PROTOCOL_MISMATCH` | 400 | Client and server protocol versions incompatible |
| `VALIDATION_ERROR` | 400 | Invalid request body or parameters |
| `RATE_LIMITED` | 429 | Too many requests |
| `SERVER_ERROR` | 500 | Internal server error |

---

## Rate Limits

| Endpoint | Limit | Window |
|----------|-------|--------|
| `POST /api/v1/auth/login` | 5 requests | per minute per IP |
| `POST /api/v1/auth/register` | 3 requests | per minute per IP |
| `POST /api/v1/auth/forgot-password` | 3 requests | per 15 minutes per IP |
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

### GET /api/v1/features

Return enabled feature flags. **No authentication required.** Used by the frontend to
conditionally enable UI elements.

---

## Setup Wizard

*Added in v1.3.0.* These endpoints drive the initial deployment wizard. Once setup
is completed they return HTTP 400 `SETUP_COMPLETED`.

### GET /api/v1/setup/status

Check if initial setup has been completed. **No authentication required.**

```bash
curl -s http://localhost:8080/api/v1/setup/status
```

Response:

```json
{
  "success": true,
  "data": {
    "setup_completed": false,
    "has_admin": false,
    "has_parking_lots": false,
    "has_users": false
  }
}
```

### POST /api/v1/setup

Complete initial setup: create admin user, configure company name, optionally seed
sample data. **No authentication required** (only works once).

```bash
curl -s -X POST http://localhost:8080/api/v1/setup \
  -H "Content-Type: application/json" \
  -d '{
    "company_name": "Muster GmbH",
    "admin_username": "admin",
    "admin_password": "Secure2026!",
    "admin_email": "admin@example.com",
    "admin_name": "Admin User",
    "use_case": "corporate",
    "create_sample_data": true
  }'
```

Response includes an access token for the newly created admin:

```json
{
  "success": true,
  "data": {
    "message": "Setup completed successfully",
    "tokens": {
      "access_token": "...",
      "token_type": "Bearer"
    }
  }
}
```

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
the email exists (prevents user enumeration). Rate limited: 3/15min per IP.

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

### PUT /api/v1/users/me

Update the authenticated user's profile (name, phone, avatar URL).

```bash
curl -s -X PUT http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "New Name", "phone": "+49 170 123456"}'
```

### PATCH /api/v1/users/me/password

*Added in v1.3.0.* Change the authenticated user's password.

```bash
curl -s -X PATCH http://localhost:8080/api/v1/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"current_password": "old-pass", "new_password": "new-secure-pass"}'
```

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

1. Anonymizes the user record: `name`, `email`, `username`, `phone`, `picture` -> `[DELETED]`
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

## User Stats & Preferences

*Added in v1.3.0.*

### GET /api/v1/user/stats

Return personal statistics for the authenticated user.

```bash
curl -s http://localhost:8080/api/v1/user/stats \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "total_bookings": 42,
    "active_bookings": 1,
    "cancelled_bookings": 3,
    "total_credits_spent": 38,
    "favorite_lot": "Parkplatz A",
    "member_since": "2026-01-15T10:00:00Z"
  }
}
```

### GET /api/v1/user/preferences

Return the authenticated user's preferences (theme, language, notification settings).

```bash
curl -s http://localhost:8080/api/v1/user/preferences \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "language": "en",
    "theme": "system",
    "notifications_enabled": true,
    "email_reminders": false,
    "default_duration_minutes": null
  }
}
```

### PUT /api/v1/user/preferences

Update the authenticated user's preferences.

```bash
curl -s -X PUT http://localhost:8080/api/v1/user/preferences \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"theme": "dark", "email_reminders": true}'
```

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
      "address": "Musterstrasse 1, 80331 Munchen",
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
    "name": "Parkplatz B",
    "address": "Hauptstrasse 5, 80333 Munchen",
    "total_slots": 30,
    "status": "active"
  }'
```

### PUT /api/v1/lots/:id

Update a parking lot. **Requires admin or superadmin role.**

```bash
curl -s -X PUT "http://localhost:8080/api/v1/lots/LOT_UUID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Renamed Lot", "status": "maintenance"}'
```

### DELETE /api/v1/lots/:id

Delete a parking lot. **Requires admin or superadmin role.**

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/lots/LOT_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Slots

*Slot CRUD added in v1.3.0.*

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

### POST /api/v1/lots/:id/slots

Create a new slot in a lot. **Requires admin or superadmin role.**

```bash
curl -s -X POST "http://localhost:8080/api/v1/lots/LOT_UUID/slots" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "slot_number": 51,
    "floor_id": "FLOOR_UUID",
    "slot_type": "standard",
    "status": "available"
  }'
```

### PUT /api/v1/lots/:lot_id/slots/:slot_id

Update a slot (status, type, position). **Requires admin or superadmin role.**

```bash
curl -s -X PUT "http://localhost:8080/api/v1/lots/LOT_UUID/slots/SLOT_UUID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status": "maintenance"}'
```

### DELETE /api/v1/lots/:lot_id/slots/:slot_id

Delete a slot from a lot. **Requires admin or superadmin role.**

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/lots/LOT_UUID/slots/SLOT_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Zones

*Added in v1.3.0.* Zones group slots within a lot (e.g. "Visitor", "Reserved", "EV Charging").

### GET /api/v1/lots/:lot_id/zones

List zones for a parking lot. Auth required.

```bash
curl -s "http://localhost:8080/api/v1/lots/LOT_UUID/zones" \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/lots/:lot_id/zones

Create a zone. **Requires admin or superadmin role.**

```bash
curl -s -X POST "http://localhost:8080/api/v1/lots/LOT_UUID/zones" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "EV Charging", "description": "Electric vehicle spots", "color": "#22c55e"}'
```

### DELETE /api/v1/lots/:lot_id/zones/:zone_id

Delete a zone. **Requires admin or superadmin role.**

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/lots/LOT_UUID/zones/ZONE_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

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
| `duration_minutes` | Yes | Positive integer. Pricing: 2 EUR/hour + 19% VAT |
| `notes` | No | Optional free-text notes |

Response: created `Booking` object (HTTP 201). Includes a QR code ID.

Returns HTTP 409 `SLOT_UNAVAILABLE` if the slot is already booked.

### POST /api/v1/bookings/quick

*Added in v1.3.0.* Quick-book: automatically selects the first available slot in the
given lot.

```bash
curl -s -X POST http://localhost:8080/api/v1/bookings/quick \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"lot_id": "LOT_UUID", "duration_minutes": 60}'
```

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

Get an HTML invoice for a booking (printer-friendly, use browser Print -> Save as PDF).

```bash
curl -s "http://localhost:8080/api/v1/bookings/BOOKING_UUID/invoice" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Booking Check-in

*Added in v1.3.0.*

### POST /api/v1/bookings/:id/checkin

Mark a booking as checked in. Transitions the booking from `confirmed`/`pending` to `active`.
The booking owner or an admin can perform this action.

```bash
curl -s -X POST "http://localhost:8080/api/v1/bookings/BOOKING_UUID/checkin" \
  -H "Authorization: Bearer $TOKEN"
```

Returns HTTP 409 `INVALID_STATUS` if the booking is not in a checkable state.

---

## Calendar & iCal

*Added in v1.3.0.*

### GET /api/v1/calendar/events

Return the authenticated user's bookings as calendar events (JSON).

```bash
curl -s http://localhost:8080/api/v1/calendar/events \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/user/calendar.ics

Export the user's bookings as an iCal (`.ics`) file for import into calendar apps.

```bash
curl -s http://localhost:8080/api/v1/user/calendar.ics \
  -H "Authorization: Bearer $TOKEN" \
  -o bookings.ics
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

### PUT /api/v1/vehicles/:id

*Added in v1.3.0.* Update a registered vehicle's details.

```bash
curl -s -X PUT "http://localhost:8080/api/v1/vehicles/VEHICLE_UUID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"make": "Audi", "model": "A4", "color": "Silber"}'
```

### DELETE /api/v1/vehicles/:id

Delete a registered vehicle. Only the vehicle owner can delete it.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/vehicles/VEHICLE_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/vehicles/city-codes

*Added in v1.3.0.* Return the list of German city codes for licence plate validation/autocomplete.

```bash
curl -s http://localhost:8080/api/v1/vehicles/city-codes \
  -H "Authorization: Bearer $TOKEN"
```

---

## Vehicle Photos

*Added in v1.3.0.* Upload and retrieve vehicle photos (max 2 MB, base64-encoded).

### POST /api/v1/vehicles/:id/photo

Upload a photo for a vehicle.

```bash
curl -s -X POST "http://localhost:8080/api/v1/vehicles/VEHICLE_UUID/photo" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"photo": "<base64-encoded-image>"}'
```

### GET /api/v1/vehicles/:id/photo

Retrieve the photo for a vehicle.

```bash
curl -s "http://localhost:8080/api/v1/vehicles/VEHICLE_UUID/photo" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Credits

*Added in v1.3.0.* Credit-based booking system. Users spend credits to book slots;
admins manage quotas and grants.

### GET /api/v1/user/credits

Return the authenticated user's credit balance, monthly quota, and recent transactions.

```bash
curl -s http://localhost:8080/api/v1/user/credits \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "credits_balance": 15,
    "credits_monthly_quota": 20,
    "credits_last_refilled": "2026-03-01T00:00:00Z",
    "recent_transactions": [
      {
        "id": "uuid",
        "amount": -1,
        "transaction_type": "deduction",
        "description": "Booking #123"
      }
    ]
  }
}
```

### POST /api/v1/admin/users/:id/credits

Grant credits to a user. **Admin only.** Amount must be 1-10000.

```bash
curl -s -X POST "http://localhost:8080/api/v1/admin/users/USER_UUID/credits" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"amount": 10, "description": "Bonus credits"}'
```

### PUT /api/v1/admin/users/:id/quota

Update a user's monthly credit quota. **Admin only.**

```bash
curl -s -X PUT "http://localhost:8080/api/v1/admin/users/USER_UUID/quota" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"monthly_quota": 30}'
```

### POST /api/v1/admin/credits/refill-all

Refill all active non-admin users' credits to their monthly quota. **Admin only.**

```bash
curl -s -X POST http://localhost:8080/api/v1/admin/credits/refill-all \
  -H "Authorization: Bearer $TOKEN"
```

---

## Favorites

*Added in v1.3.0.* Users can pin preferred parking slots for quick access.

### GET /api/v1/user/favorites

List the authenticated user's favorite slots.

```bash
curl -s http://localhost:8080/api/v1/user/favorites \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/user/favorites

Add a slot to favorites.

```bash
curl -s -X POST http://localhost:8080/api/v1/user/favorites \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"slot_id": "SLOT_UUID", "lot_id": "LOT_UUID"}'
```

### DELETE /api/v1/user/favorites/:slot_id

Remove a slot from favorites.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/user/favorites/SLOT_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Notifications

### GET /api/v1/notifications

List notifications for the authenticated user.

```bash
curl -s http://localhost:8080/api/v1/notifications \
  -H "Authorization: Bearer $TOKEN"
```

### PUT /api/v1/notifications/:id/read

Mark a single notification as read.

```bash
curl -s -X PUT "http://localhost:8080/api/v1/notifications/NOTIF_UUID/read" \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/notifications/read-all

Mark all notifications as read.

```bash
curl -s -X POST http://localhost:8080/api/v1/notifications/read-all \
  -H "Authorization: Bearer $TOKEN"
```

---

## Absences

### GET /api/v1/absences

List the authenticated user's absences (vacation, sick days, etc.).

```bash
curl -s http://localhost:8080/api/v1/absences \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/absences

Create an absence entry.

```bash
curl -s -X POST http://localhost:8080/api/v1/absences \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"type": "vacation", "start_date": "2026-04-01", "end_date": "2026-04-05"}'
```

### DELETE /api/v1/absences/:id

Delete an absence entry.

### GET /api/v1/absences/team

List team absences (all users the caller can see).

### GET /api/v1/absences/pattern

Get the caller's recurring absence pattern (e.g. "home office every Friday").

### POST /api/v1/absences/pattern

Save/update the caller's recurring absence pattern.

---

## Team

*Added in v1.3.0.*

### GET /api/v1/team

List all team members with basic profile info.

```bash
curl -s http://localhost:8080/api/v1/team \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/team/today

Show who is in/out today (combines bookings + absences).

```bash
curl -s http://localhost:8080/api/v1/team/today \
  -H "Authorization: Bearer $TOKEN"
```

---

## Waitlist

### GET /api/v1/waitlist

List the authenticated user's waitlist entries.

### POST /api/v1/waitlist

Join the waitlist for a fully booked lot/slot.

### DELETE /api/v1/waitlist/:id

Leave the waitlist.

---

## Swap Requests

### GET /api/v1/swap-requests

List swap requests involving the authenticated user.

### POST /api/v1/bookings/:id/swap-request

Request to swap this booking's slot with another user.

### PUT /api/v1/swap-requests/:id

Accept or decline a swap request.

---

## Recurring Bookings

### GET /api/v1/recurring-bookings

List the authenticated user's recurring booking rules.

### POST /api/v1/recurring-bookings

Create a recurring booking (e.g. "every Monday 08:00-17:00").

### DELETE /api/v1/recurring-bookings/:id

Delete a recurring booking rule.

---

## Guest Bookings

### POST /api/v1/bookings/guest

Create a booking for a guest (visitor). Auth required.

### GET /api/v1/admin/guest-bookings

List all guest bookings. **Admin only.**

### PATCH /api/v1/admin/guest-bookings/:id/cancel

Cancel a guest booking. **Admin only.**

---

## Announcements

### GET /api/v1/announcements/active

Return currently active announcements. **No authentication required.**

```bash
curl -s http://localhost:8080/api/v1/announcements/active
```

### GET /api/v1/admin/announcements

List all announcements (including expired). **Admin only.**

### POST /api/v1/admin/announcements

Create a new announcement. **Admin only.**

```bash
curl -s -X POST http://localhost:8080/api/v1/admin/announcements \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Maintenance Notice",
    "message": "Lot B closed Saturday 10:00-14:00",
    "severity": "warning",
    "starts_at": "2026-03-22T00:00:00Z",
    "ends_at": "2026-03-22T14:00:00Z"
  }'
```

### PUT /api/v1/admin/announcements/:id

Update an announcement. **Admin only.**

### DELETE /api/v1/admin/announcements/:id

Delete an announcement. **Admin only.**

---

## Webhooks

*Added in v1.3.0.* All webhook endpoints require admin role.
Webhook URLs must be HTTPS (HTTP allowed only for localhost in debug builds).
SSRF protection: private IPs and localhost are blocked in release builds.

Valid event types: `booking.created`, `booking.cancelled`, `booking.updated`,
`user.created`, `user.deleted`, `lot.created`, `lot.updated`, `lot.deleted`, `test`.

Deliveries include an `X-Webhook-Signature` header (HMAC-SHA256).

### GET /api/v1/webhooks

List all configured webhooks.

```bash
curl -s http://localhost:8080/api/v1/webhooks \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": [
    {
      "id": "uuid",
      "url": "https://example.com/hook",
      "secret": "whsec_...",
      "events": ["booking.created", "booking.cancelled"],
      "active": true,
      "created_at": "...",
      "updated_at": "..."
    }
  ]
}
```

### POST /api/v1/webhooks

Create a new webhook.

```bash
curl -s -X POST http://localhost:8080/api/v1/webhooks \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com/hook",
    "events": ["booking.created"],
    "active": true
  }'
```

Response includes the generated HMAC secret (HTTP 201).

### PUT /api/v1/webhooks/:id

Update a webhook (URL, events, active flag). Set `"regenerate_secret": true` to rotate the HMAC secret.

```bash
curl -s -X PUT "http://localhost:8080/api/v1/webhooks/WEBHOOK_UUID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"events": ["booking.created", "user.created"], "regenerate_secret": true}'
```

### DELETE /api/v1/webhooks/:id

Delete a webhook.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/webhooks/WEBHOOK_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/webhooks/:id/test

Send a test event to the webhook URL. Returns delivery status and HTTP response code.

```bash
curl -s -X POST "http://localhost:8080/api/v1/webhooks/WEBHOOK_UUID/test" \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "delivered": true,
    "status_code": 200
  }
}
```

---

## Web Push

*Added in v1.3.0.* Web Push notifications via the Push API (RFC 8030).

### GET /api/v1/push/vapid-key

Return the server's public VAPID key for push subscription. **No authentication required.**
Returns HTTP 404 if `VAPID_PUBLIC_KEY` is not configured.

```bash
curl -s http://localhost:8080/api/v1/push/vapid-key
```

Response:

```json
{
  "success": true,
  "data": {
    "public_key": "BN..."
  }
}
```

### POST /api/v1/push/subscribe

Register a push subscription for the authenticated user.

```bash
curl -s -X POST http://localhost:8080/api/v1/push/subscribe \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "endpoint": "https://fcm.googleapis.com/fcm/send/...",
    "keys": {
      "p256dh": "...",
      "auth": "..."
    }
  }'
```

### DELETE /api/v1/push/unsubscribe

Remove all push subscriptions for the authenticated user.

```bash
curl -s -X DELETE http://localhost:8080/api/v1/push/unsubscribe \
  -H "Authorization: Bearer $TOKEN"
```

---

## Public Display

*Added in v1.3.0.* Unauthenticated endpoints for lobby screens and signage.

### GET /api/v1/public/occupancy

Return live occupancy data for all lots as JSON. **No authentication required.**

```bash
curl -s http://localhost:8080/api/v1/public/occupancy
```

Response:

```json
{
  "success": true,
  "data": [
    {
      "lot_id": "uuid",
      "lot_name": "Parkplatz A",
      "total_slots": 50,
      "occupied_slots": 27,
      "available_slots": 23
    }
  ]
}
```

### GET /api/v1/public/display

Return a self-refreshing HTML page showing lot availability with color-coded counts
(green/yellow/red). Auto-refreshes every 30 seconds. Designed for embedding on
large displays. **No authentication required.**

```bash
curl -s http://localhost:8080/api/v1/public/display
# Returns text/html
```

---

## QR Codes

*Added in v1.3.0.*

### GET /api/v1/lots/:id/qr

Generate a QR code image for a parking lot (links to the lot booking page).

```bash
curl -s "http://localhost:8080/api/v1/lots/LOT_UUID/qr" \
  -H "Authorization: Bearer $TOKEN" \
  -o lot-qr.png
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
    "street": "Musterstrasse 1",
    "zip_city": "80331 Munchen",
    "country": "Deutschland",
    "email": "info@muster.de",
    "phone": "+49 89 123456",
    "register_court": "Amtsgericht Munchen",
    "register_number": "HRB 123456",
    "vat_id": "DE123456789",
    "responsible_person": "Max Mustermann",
    "custom_text": ""
  }
}
```

---

## Admin -- User Management

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
    "street": "Musterstrasse 1",
    "zip_city": "80331 Munchen",
    "email": "info@muster.de"
  }'
```

### GET /api/v1/admin/users

List all users.

```bash
curl -s http://localhost:8080/api/v1/admin/users \
  -H "Authorization: Bearer $TOKEN"
```

### PUT /api/v1/admin/users/:id/update

*Added in v1.3.0.* Update a user's profile fields (name, email, etc.).

```bash
curl -s -X PUT "http://localhost:8080/api/v1/admin/users/USER_UUID/update" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Updated Name", "email": "new@example.com"}'
```

### PATCH /api/v1/admin/users/:id/role

Update a user's role.

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

Delete a user account.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/admin/users/USER_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Admin -- Bookings & Export

### GET /api/v1/admin/bookings

List all bookings across all users.

```bash
curl -s http://localhost:8080/api/v1/admin/bookings \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/admin/users/export-csv

*Added in v1.3.0.* Export all users as a CSV file. Includes CSV injection protection.

```bash
curl -s http://localhost:8080/api/v1/admin/users/export-csv \
  -H "Authorization: Bearer $TOKEN" \
  -o users.csv
```

Response: `text/csv` with columns `id,username,email,name,role,is_active,created_at`.

### GET /api/v1/admin/bookings/export-csv

*Added in v1.3.0.* Export all bookings as a CSV file.

```bash
curl -s http://localhost:8080/api/v1/admin/bookings/export-csv \
  -H "Authorization: Bearer $TOKEN" \
  -o bookings.csv
```

Response: `text/csv` with columns `id,user_id,lot_name,slot_number,start_time,end_time,status,vehicle_plate`.

---

## Admin -- Settings

### GET /api/v1/admin/settings

Return system settings (company name, credits config, etc.).

```bash
curl -s http://localhost:8080/api/v1/admin/settings \
  -H "Authorization: Bearer $TOKEN"
```

### PUT /api/v1/admin/settings

Update system settings.

### GET /api/v1/admin/settings/auto-release

*Added in v1.3.0.* Return auto-release configuration (unclaimed booking timeout).

```bash
curl -s http://localhost:8080/api/v1/admin/settings/auto-release \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "auto_release_enabled": true,
    "auto_release_minutes": 30
  }
}
```

### PUT /api/v1/admin/settings/auto-release

Update auto-release settings.

```bash
curl -s -X PUT http://localhost:8080/api/v1/admin/settings/auto-release \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"auto_release_enabled": true, "auto_release_minutes": 15}'
```

### GET /api/v1/admin/settings/email

*Added in v1.3.0.* Return SMTP email configuration (password is masked).

```bash
curl -s http://localhost:8080/api/v1/admin/settings/email \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "smtp_host": "smtp.example.com",
    "smtp_port": 587,
    "smtp_username": "noreply@example.com",
    "smtp_password": "********",
    "smtp_from": "noreply@example.com",
    "smtp_enabled": true
  }
}
```

### PUT /api/v1/admin/settings/email

Update SMTP settings. Send `"smtp_password": "********"` to keep the existing password unchanged.

```bash
curl -s -X PUT http://localhost:8080/api/v1/admin/settings/email \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"smtp_host": "smtp.gmail.com", "smtp_port": 587, "smtp_enabled": true}'
```

### GET /api/v1/admin/privacy

*Added in v1.3.0.* Return GDPR/privacy settings.

```bash
curl -s http://localhost:8080/api/v1/admin/privacy \
  -H "Authorization: Bearer $TOKEN"
```

Response:

```json
{
  "success": true,
  "data": {
    "privacy_policy_url": "https://example.com/privacy",
    "data_retention_days": 365,
    "require_consent": true,
    "anonymize_on_delete": true
  }
}
```

### PUT /api/v1/admin/privacy

Update privacy settings.

### GET /api/v1/admin/features

Return feature flags configuration.

### PUT /api/v1/admin/features

Update feature flags.

---

## Admin -- Reports & Dashboard

*Added in v1.3.0.*

### GET /api/v1/admin/stats

Return aggregate statistics (total users, bookings, lots, etc.).

```bash
curl -s http://localhost:8080/api/v1/admin/stats \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/admin/reports

Return reporting data (occupancy trends, usage patterns).

```bash
curl -s http://localhost:8080/api/v1/admin/reports \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/admin/heatmap

Return booking heatmap data (hour-of-day x day-of-week matrix).

```bash
curl -s http://localhost:8080/api/v1/admin/heatmap \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/admin/dashboard/charts

*Added in v1.3.0.* Return time-series chart data for the admin dashboard (bookings per day, revenue, etc.).

```bash
curl -s http://localhost:8080/api/v1/admin/dashboard/charts \
  -H "Authorization: Bearer $TOKEN"
```

### GET /api/v1/admin/audit-log

*Added in v1.3.0.* Return the audit log (login, register, booking, role change, config change events).

```bash
curl -s http://localhost:8080/api/v1/admin/audit-log \
  -H "Authorization: Bearer $TOKEN"
```

---

## Admin -- Database Reset

*Added in v1.3.0.*

### POST /api/v1/admin/reset

Wipe all data and re-create the calling admin user. Requires explicit confirmation.

```bash
curl -s -X POST http://localhost:8080/api/v1/admin/reset \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"confirm": "RESET"}'
```

Returns HTTP 400 if `confirm` is not exactly `"RESET"`.

---

## Demo Mode

These endpoints are always available (no auth). They use in-memory state separate from
the production database.

### GET /api/v1/demo/status

Return demo status including reset schedule.

### POST /api/v1/demo/vote

Submit a demo feedback vote.

### POST /api/v1/demo/reset

Manually trigger a demo data reset (clears all data, re-seeds).

### GET /api/v1/demo/config

Return the demo configuration (feature flags, sample data info).

```bash
curl -s http://localhost:8080/api/v1/demo/config
```

---

## Metrics

### GET /metrics

Prometheus-format metrics. Optionally protected by `METRICS_TOKEN` env var (Bearer auth).

```bash
curl http://localhost:8080/metrics
# or with auth:
curl -H "Authorization: Bearer $METRICS_TOKEN" http://localhost:8080/metrics
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
| Maximum request body | 4 MiB (HTTP 413 if exceeded) |
| Maximum photo upload | 2 MB raw |
| Login rate limit | 5 requests/minute per IP |
| Register rate limit | 3 requests/minute per IP |
| Forgot-password rate limit | 3 requests/15 minutes per IP |
| Global rate limit | 100 req/s (burst: 200) |
| Token expiry | 24 hours |
