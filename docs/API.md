# API Reference

Complete REST API reference for ParkHub Rust.

All API endpoints are available at `http(s)://your-server:port/api/v1/`.

A machine-readable OpenAPI spec and interactive Swagger UI are available at `/swagger-ui`
when the server is running.

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

### Common error codes

| Code | HTTP | Meaning |
|---|---|---|
| `UNAUTHORIZED` | 401 | Missing or invalid Bearer token |
| `FORBIDDEN` | 403 | Authenticated but insufficient role |
| `NOT_FOUND` | 404 | Resource does not exist |
| `CONFLICT` | 409 | Duplicate resource or state conflict |
| `SLOT_UNAVAILABLE` | 409 | Slot is already booked |
| `ALREADY_CANCELLED` | 409 | Booking is already in cancelled state |
| `INVALID_CREDENTIALS` | 401 | Wrong username or password |
| `ACCOUNT_DISABLED` | 403 | Account has been deactivated |
| `SERVER_ERROR` | 500 | Internal error |

---

## Authentication

All protected endpoints require a `Bearer` token in the `Authorization` header:

```
Authorization: Bearer <access_token>
```

Tokens are obtained from the login or register endpoints. They expire after 24 hours.

---

## Health and Discovery

### GET /health

Simple health check. Returns `OK` (plain text).

```bash
curl http://localhost:8080/health
# OK
```

### GET /health/live

Kubernetes liveness probe. Returns HTTP 200 if the process is alive.

### GET /health/ready

Kubernetes readiness probe. Checks if the database is accessible.

```bash
curl http://localhost:8080/health/ready
# {"ready": true}
```

Returns HTTP 503 with `{"ready": false}` if the database is not operational.

### GET /status

Server statistics (no auth required).

```bash
curl http://localhost:8080/status
```

Response:

```json
{
  "success": true,
  "data": {
    "uptime_seconds": 0,
    "connected_clients": 0,
    "total_users": 5,
    "total_bookings": 42
  }
}
```

### POST /handshake

Protocol version negotiation. Used by native clients to verify compatibility.

Request body:

```json
{
  "client_version": "0.1.0",
  "protocol_version": "1"
}
```

Response:

```json
{
  "success": true,
  "data": {
    "server_name": "ParkHub Server",
    "server_version": "0.1.0",
    "protocol_version": "1",
    "requires_auth": true,
    "certificate_fingerprint": ""
  }
}
```

Returns error code `PROTOCOL_MISMATCH` if versions do not match.

---

## Auth

### POST /api/v1/auth/login

Authenticate with username (or email) and password. Returns a Bearer token.

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin"}'
```

Request body:

```json
{
  "username": "admin",
  "password": "admin"
}
```

Response:

```json
{
  "success": true,
  "data": {
    "user": {
      "id": "uuid",
      "username": "admin",
      "email": "admin@example.com",
      "name": "Admin",
      "role": "admin",
      "is_active": true
    },
    "tokens": {
      "access_token": "uuid-token",
      "refresh_token": "uuid-refresh",
      "expires_at": "2026-01-01T12:00:00Z",
      "token_type": "Bearer"
    }
  }
}
```

The `password_hash` field is never included in responses.

### POST /api/v1/auth/register

Register a new user account. Only available when `allow_self_registration = true` in config.

```bash
curl -s -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "secure123", "name": "Max Mustermann"}'
```

Request body:

```json
{
  "email": "user@example.com",
  "password": "secure123",
  "name": "Max Mustermann"
}
```

Response: same as login (201 Created). Username is auto-generated from the email prefix.

### POST /api/v1/auth/refresh

Refresh an expired access token using the refresh token. **Not yet implemented** — returns 501.

---

## Users

### GET /api/v1/users/me

Get the currently authenticated user's profile.

```bash
curl -s http://localhost:8080/api/v1/users/me \
  -H "Authorization: Bearer $TOKEN"
```

Response: the `User` object (password hash excluded).

### GET /api/v1/users/:id

Get any user by ID. **Requires admin or superadmin role.**

```bash
curl -s "http://localhost:8080/api/v1/users/550e8400-e29b-41d4-a716-446655440000" \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

---

## GDPR

### GET /api/v1/users/me/export

Export all personal data for the authenticated user (GDPR Art. 15 — Right of Access).

Returns a JSON file containing the user's profile, all bookings, and all vehicles.
The `password_hash` is intentionally excluded from exports.

```bash
curl -s http://localhost:8080/api/v1/users/me/export \
  -H "Authorization: Bearer $TOKEN" \
  -o my-data-export.json
```

Response is a JSON object:

```json
{
  "exported_at": "2026-01-01T12:00:00Z",
  "gdpr_basis": "GDPR Art. 15 — Right of Access",
  "profile": { ... },
  "bookings": [ ... ],
  "vehicles": [ ... ]
}
```

### DELETE /api/v1/users/me/delete

Delete the authenticated user's account (GDPR Art. 17 — Right to Erasure).

**What this does:**
- Anonymizes the user record: name, email, username, phone, picture are replaced with `[DELETED]`
- Deletes all vehicles registered by the user
- Booking records are **retained but anonymized** (license plate replaced with `[DELETED]`) as
  required by German tax law (§147 AO — 10-year accounting record retention)
- Invalidates all active sessions

```bash
curl -s -X DELETE http://localhost:8080/api/v1/users/me/delete \
  -H "Authorization: Bearer $TOKEN"
```

Response: `{"success": true}` on success.

---

## Parking Lots

### GET /api/v1/lots

List all parking lots with availability counts.

```bash
curl -s http://localhost:8080/api/v1/lots \
  -H "Authorization: Bearer $TOKEN"
```

Response: array of `ParkingLot` objects, each containing:

```json
{
  "id": "uuid",
  "name": "Parkplatz A",
  "address": "Musterstraße 1, 80331 München",
  "total_slots": 50,
  "available_slots": 23,
  "status": "active"
}
```

### GET /api/v1/lots/:id

Get a single parking lot by ID.

```bash
curl -s "http://localhost:8080/api/v1/lots/LOT_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

### POST /api/v1/lots

Create a new parking lot. **Requires admin or superadmin role.**

```bash
curl -s -X POST http://localhost:8080/api/v1/lots \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "id": "new-uuid",
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

List all parking slots in a lot with their current availability status.

```bash
curl -s "http://localhost:8080/api/v1/lots/LOT_UUID/slots" \
  -H "Authorization: Bearer $TOKEN"
```

Response: array of `ParkingSlot` objects:

```json
[
  {
    "id": "uuid",
    "lot_id": "lot-uuid",
    "floor_id": "floor-uuid",
    "slot_number": 1,
    "status": "available",
    "slot_type": "standard"
  }
]
```

Slot statuses: `available`, `occupied`, `reserved`, `maintenance`, `disabled`.

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

Create a new booking. The slot must be in `available` status. A write lock is held during
the check-and-insert to prevent double-booking race conditions.

```bash
curl -s -X POST http://localhost:8080/api/v1/bookings \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "slot_id": "SLOT_UUID",
    "lot_id": "LOT_UUID",
    "vehicle_id": "VEHICLE_UUID",
    "start_time": "2026-01-15T09:00:00Z",
    "duration_minutes": 60,
    "license_plate": "M-AB 1234"
  }'
```

Request fields:

| Field | Required | Description |
|---|---|---|
| `slot_id` | Yes | UUID of the slot to book |
| `lot_id` | Yes | UUID of the parking lot |
| `vehicle_id` | No | UUID of a registered vehicle. If omitted, `license_plate` is used |
| `license_plate` | No | License plate for ad-hoc bookings (used when `vehicle_id` is absent) |
| `start_time` | Yes | ISO 8601 UTC datetime |
| `duration_minutes` | Yes | Positive integer. Pricing: 2 EUR/hour + 10% tax |
| `notes` | No | Optional free-text notes |

Response: the created `Booking` object (HTTP 201). Includes a QR code ID.

### GET /api/v1/bookings/:id

Get a specific booking. Users can only access their own bookings.

### DELETE /api/v1/bookings/:id

Cancel a booking. Only the booking owner can cancel. Only `confirmed` and `pending`
bookings can be cancelled. Cancelling frees the slot back to `available`.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/bookings/BOOKING_UUID" \
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
|---|---|---|
| `license_plate` | Yes | License plate (auto-uppercased) |
| `make` | No | Vehicle manufacturer |
| `model` | No | Vehicle model |
| `color` | No | Vehicle color |

Response: the created `Vehicle` object (HTTP 201).

### DELETE /api/v1/vehicles/:id

Delete a vehicle. Only the vehicle owner can delete it.

```bash
curl -s -X DELETE "http://localhost:8080/api/v1/vehicles/VEHICLE_UUID" \
  -H "Authorization: Bearer $TOKEN"
```

---

## Legal (DDG §5)

### GET /api/v1/legal/impressum

Retrieve the Impressum data. **No authentication required** (DDG §5 requires public access).

```bash
curl -s http://localhost:8080/api/v1/legal/impressum
```

Response:

```json
{
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
```

---

## Admin

### GET /api/v1/admin/impressum

Retrieve the Impressum configuration for editing. **Requires admin or superadmin role.**

```bash
curl -s http://localhost:8080/api/v1/admin/impressum \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### PUT /api/v1/admin/impressum

Update the Impressum fields. **Requires admin or superadmin role.**

```bash
curl -s -X PUT http://localhost:8080/api/v1/admin/impressum \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
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

---

## Metrics

### GET /metrics

Prometheus-format metrics. No authentication required.

```bash
curl http://localhost:8080/metrics
```

Returns metrics in the `text/plain; charset=utf-8` Prometheus exposition format.

---

## Swagger UI

The interactive API documentation is available at `/swagger-ui` when the server is running.
The OpenAPI JSON spec is at `/api-docs/openapi.json`.

---

## Request Limits

- Maximum request body size: **1 MiB** (returns HTTP 413 if exceeded)
- Login rate limit: **5 requests per minute per IP**
- Registration rate limit: **3 requests per minute per IP**
- Global rate limit: **100 requests/second** (burst: 200)
