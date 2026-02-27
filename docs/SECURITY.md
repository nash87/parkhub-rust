# Security Model

Security architecture and disclosure policy for ParkHub Rust.

---

## Architecture Overview

ParkHub is a single-process server that embeds both the API and the React frontend.
The attack surface is intentionally minimal:

- No external database daemon (redb is embedded)
- No external cache service
- No background job runner process
- No third-party JavaScript loaded at runtime (all assets are embedded)
- No CDN, analytics, or tracking integrations

---

## Authentication

### Session Tokens

- Sessions are created on login and registration
- Each session has a UUID access token and a UUID refresh token
- Sessions expire after 24 hours (configurable via `session_timeout_minutes`)
- Tokens are stored in the redb database alongside their expiry timestamp
- The `Authorization: Bearer <token>` header is required for all protected routes
- Tokens are never sent in URLs or cookies

### Password Security

- All passwords are hashed with **Argon2id** using a random salt (via `argon2` crate, OsRng)
- Password hashes are never returned in API responses (explicitly zeroed before serialization)
- Password hashes are excluded from GDPR data exports

### Role-Based Access Control

Three roles are enforced at the handler level:

| Role | Capabilities |
|---|---|
| `user` | Own bookings (create, view, cancel), own vehicles (create, delete), own profile, GDPR export/delete |
| `admin` | All user capabilities + create parking lots, manage Impressum, view any user by ID |
| `superadmin` | All admin capabilities (same enforcement as admin in current implementation) |

Ownership is verified on every request that touches user-specific resources:
- A user can only cancel their own bookings
- A user can only delete their own vehicles
- The vehicle ownership check in `create_booking` prevents booking with another user's vehicle

---

## Transport Security

### TLS 1.3

When `enable_tls = true`, the server:
1. Looks for `data/cert.pem` and `data/key.pem`
2. If not found, auto-generates a self-signed certificate using the `rcgen` crate
3. Serves all traffic over TLS 1.3 via `axum-server` with `rustls`

For production use, either:
- Bring your own certificate (place cert and key in the data directory)
- Terminate TLS at a reverse proxy (nginx, Caddy, Traefik) and run ParkHub over plain HTTP internally

### Security Headers

Applied to every response by a global Axum middleware:

| Header | Value | Purpose |
|---|---|---|
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'` | Restrict resource origins |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Limit referrer leakage |
| `Permissions-Policy` | `geolocation=(), camera=(), microphone=()` | Disable unneeded browser APIs |

### CORS

CORS is configured to allow only same-origin requests in production. Localhost origins
(`http://localhost:*`, `https://localhost:*`, `http://127.0.0.1:*`) are allowed for
development. No wildcard `*` origin is ever permitted.

---

## Database Security

### At-Rest Encryption (AES-256-GCM)

When `encryption_enabled = true`:
- The passphrase is derived into a 256-bit key using PBKDF2-SHA256
- The redb database is encrypted with AES-256-GCM
- The passphrase is held only in memory (`#[serde(skip)]` — never written to config file)
- Supply the passphrase via the `PARKHUB_DB_PASSPHRASE` environment variable or via GUI prompt

The `zeroize` crate is used for sensitive key material to overwrite memory on drop.

---

## Rate Limiting

Rate limiting is implemented using the `governor` crate.

| Endpoint | Limit |
|---|---|
| `POST /api/v1/auth/login` | 5 requests per minute per IP |
| `POST /api/v1/auth/register` | 3 requests per minute per IP |
| All other routes | 100 requests per second global (burst: 200) |

Returns HTTP 429 with an `AppError::RateLimited` response when exceeded.

---

## Input Validation

- All request bodies are limited to **1 MiB** by a `RequestBodyLimitLayer`
- JSON deserialization rejects unknown fields via `serde`
- UUIDs are validated at the type level (Rust's `uuid` crate)
- Booking duration must be positive (validated before arithmetic)
- License plates are uppercased server-side before storage

---

## Concurrency Safety

Booking creation and cancellation use an async `RwLock` write lock on the application state.
This ensures that slot availability checks and slot status updates are atomic — two concurrent
requests cannot both read `SlotStatus::Available` and both succeed, which would create a
double-booking.

---

## Supply Chain

All dependencies are pure Rust or well-audited C libraries compiled into a static musl binary
in Docker. The Docker image runs as a non-root user inside a minimal Alpine runtime image.

Key security-relevant dependencies and their purposes:

| Crate | Purpose |
|---|---|
| `argon2` | Password hashing (Argon2id) |
| `aes-gcm` | Database at-rest encryption |
| `pbkdf2` / `sha2` | Key derivation for encryption passphrase |
| `zeroize` | Zeroing sensitive key material in memory |
| `rcgen` | Self-signed TLS certificate generation |
| `rustls` | TLS implementation (no OpenSSL) |
| `rand` | Cryptographically secure random number generation (OsRng) |

---

## Known Limitations

- Token refresh (`POST /api/v1/auth/refresh`) is not yet implemented — clients must re-login
  after the 24-hour session expires
- There is no built-in brute-force account lockout beyond rate limiting
- Self-signed certificates require manual trust store additions or browser warnings
- The admin user management UI is a placeholder — user administration must be done via the API

---

## Vulnerability Disclosure

If you discover a security vulnerability in ParkHub, please report it responsibly:

1. **Do not open a public GitHub issue** for security vulnerabilities
2. Email the operator of the ParkHub instance you are using
3. For vulnerabilities in the open-source code itself, email the maintainer or
   open a GitHub Security Advisory on the repository

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)

We aim to acknowledge reports within 48 hours and provide a fix within 14 days for
critical issues.
