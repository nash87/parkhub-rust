# Security Model — ParkHub Rust

Security architecture, controls, and responsible disclosure process for ParkHub Rust.

---

## Architecture Overview

ParkHub is a single-process server with an intentionally minimal attack surface:

- No external database daemon (redb is embedded in the process)
- No external cache service
- No background job runner process
- No third-party JavaScript loaded at runtime (all frontend assets are embedded in the binary)
- No CDN, analytics, or tracking integrations
- No cloud dependencies at runtime

---

## Authentication

### Session Tokens

| Property | Value |
|----------|-------|
| Token type | Opaque UUID Bearer token |
| Default expiry | 24 hours (configurable via `session_timeout_minutes`) |
| Storage | In the redb database, alongside expiry timestamp |
| Transport | `Authorization: Bearer <token>` header only — never in URLs or cookies |
| Refresh | Planned (v1.0.0 returns 501) |
| Revocation | On account deletion or password change |

### Password Security

| Control | Implementation |
|---------|---------------|
| Hashing algorithm | **Argon2id** (via the `argon2` crate) |
| Salt | Cryptographically random (OsRng) per password |
| Output | Never returned in API responses (explicitly excluded from all serialization) |
| Export | Excluded from GDPR data exports |

### Role-Based Access Control (RBAC)

Three roles are enforced at the HTTP handler level:

| Role | Capabilities |
|------|-------------|
| `user` | Own bookings (create, view, cancel), own vehicles (CRUD), own profile, GDPR export, GDPR deletion |
| `admin` | All user capabilities + create/manage lots, manage Impressum, view any user, list all bookings, admin user management |
| `superadmin` | Same as `admin` in v1.0.0 (reserved for future delegation) |

Ownership is verified on every request touching user-specific resources:
- Users can only cancel their own bookings
- Users can only delete their own vehicles
- The vehicle ownership check in `create_booking` prevents booking with another user's vehicle

---

## Transport Security

### TLS 1.3

When `enable_tls = true`, the server:

1. Looks for `data/cert.pem` and `data/key.pem`
2. If not found, auto-generates a self-signed certificate using the `rcgen` crate
3. Serves all traffic over TLS 1.3 via `axum-server` with `rustls` (no OpenSSL dependency)

For production deployments:
- Bring your own certificate (place cert and key in the data directory)
- Terminate TLS at a reverse proxy (nginx, Caddy, Traefik) and run ParkHub over plain HTTP internally

### Security Headers

Applied to every HTTP response by a global Axum middleware layer:

| Header | Value |
|--------|-------|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |
| `Permissions-Policy` | `geolocation=(), camera=(), microphone=()` |

For deployments behind a reverse proxy, add at the proxy level:

```
Strict-Transport-Security: max-age=31536000; includeSubDomains
```

### CORS

Same-origin requests only in production. Localhost origins (`http://localhost:*`,
`https://localhost:*`, `http://127.0.0.1:*`) are allowed for development.
No wildcard `*` origin is permitted.

---

## Database Security

### At-Rest Encryption (AES-256-GCM)

When `encryption_enabled = true`:

| Step | Detail |
|------|--------|
| Key derivation | PBKDF2-SHA256 derives a 256-bit key from the passphrase |
| Encryption | AES-256-GCM applied to the redb database file |
| Passphrase storage | Never written to disk (`#[serde(skip)]`). Supply via `PARKHUB_DB_PASSPHRASE` environment variable or GUI prompt |
| Memory safety | `zeroize` crate zeroes key material in memory on drop |

Supply the passphrase via environment variable:

```bash
PARKHUB_DB_PASSPHRASE="your-strong-passphrase" ./parkhub-server --headless
```

> Without the passphrase, the database cannot be read. Store the passphrase in a
> password manager or secret vault.

---

## Rate Limiting

Rate limiting is implemented using the `governor` crate (token bucket algorithm):

| Endpoint | Limit | Window |
|----------|-------|--------|
| `POST /api/v1/auth/login` | 5 requests | per minute per IP |
| `POST /api/v1/auth/register` | 3 requests | per minute per IP |
| All other routes | 100 requests/second global | burst: 200 |

Returns HTTP 429 (`AppError::RateLimited`) when exceeded.

---

## Input Validation

| Control | Implementation |
|---------|---------------|
| Body size limit | 1 MiB maximum via `RequestBodyLimitLayer`. Returns HTTP 413 |
| JSON deserialization | Rejects unknown fields via `serde` deny_unknown_fields |
| UUID validation | At the type level via the `uuid` crate |
| Booking duration | Must be positive (validated before arithmetic) |
| Licence plates | Auto-uppercased server-side before storage |

---

## Concurrency Safety

Booking creation and cancellation acquire an async `RwLock` write lock on the application
state. This ensures that the slot availability check and slot status update are atomic —
two concurrent requests cannot both read `SlotStatus::Available` and both succeed,
preventing double-bookings without requiring a separate database-level transaction.

---

## SQL Injection Prevention

ParkHub uses the `redb` embedded key-value database, not SQL. There is no SQL query
layer, no ORM, and therefore no SQL injection attack surface.

---

## XSS Prevention

- All user-supplied content is rendered through React's JSX, which escapes values by default
- The Content-Security-Policy header restricts script execution to `'self'`
- No user-supplied content is set as `innerHTML` or `dangerouslySetInnerHTML`

---

## CSRF

ParkHub is a stateless token-based API. All state-changing requests require a valid Bearer
token in the `Authorization` header. CSRF attacks via cookie-based session forgery are not
applicable. CORS prevents cross-origin JavaScript from reading the token.

---

## File Upload Security

In v1.0.0, ParkHub Rust does not accept file uploads. The 1 MiB request body limit
applies to all endpoints.

---

## Audit Log

When `audit_logging_enabled = true`, ParkHub writes entries for security-relevant events:

| Event | Logged |
|-------|--------|
| Login (success) | User ID, timestamp |
| Login (failure) | Attempted username, timestamp |
| Registration | User ID, timestamp |
| Booking created | User ID, booking ID, slot ID, timestamp |
| Booking cancelled | User ID, booking ID, timestamp |
| Account deleted (GDPR) | User ID, timestamp |
| Impressum updated | Admin user ID, timestamp |

Log format: structured JSON via `tracing` to stdout.

---

## Supply Chain

| Component | Security Properties |
|-----------|---------------------|
| Binary | Compiled as a static musl binary — no shared library vulnerabilities |
| Docker image | Non-root user inside minimal Alpine runtime image |
| TLS | `rustls` (pure Rust) — no OpenSSL dependency |
| Cryptography | Well-audited Rust crates: `argon2`, `aes-gcm`, `pbkdf2`, `rand`, `zeroize` |

Key security dependencies:

| Crate | Purpose |
|-------|---------|
| `argon2` | Argon2id password hashing |
| `aes-gcm` | AES-256-GCM database encryption |
| `pbkdf2` / `sha2` | Key derivation from encryption passphrase |
| `zeroize` | Zeroing sensitive key material in memory on drop |
| `rcgen` | Self-signed TLS certificate generation |
| `rustls` | TLS 1.3 implementation (no OpenSSL) |
| `rand` | Cryptographically secure random number generation (OsRng) |
| `governor` | Rate limiting (token bucket) |

---

## Known Limitations

| Limitation | Mitigation / Roadmap |
|-----------|---------------------|
| Token refresh not implemented | Clients must re-authenticate after 24 hours. Token refresh is planned |
| No account lockout beyond rate limiting | 5 login attempts/minute per IP. Manual deactivation via admin API |
| Self-signed certificates trigger browser warnings | Use a reverse proxy with Let's Encrypt, or add cert to trust store |
| Admin user management UI is a placeholder | Use the API (`GET/PATCH/DELETE /api/v1/admin/users/*`) |

---

## Responsible Disclosure

If you discover a security vulnerability in ParkHub:

1. **Do not open a public GitHub issue** for security vulnerabilities
2. For vulnerabilities in a deployed instance, email the instance operator
3. For vulnerabilities in the open-source code, open a GitHub Security Advisory:
   `https://github.com/nash87/parkhub/security/advisories/new`

Please include in your report:
- Description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Suggested fix (if you have one)

**Response times:**
- Acknowledgement: within 48 hours
- Fix timeline for critical issues: within 14 days
- Researchers credited in release notes (unless anonymity is requested)

**CVE history**: No CVEs at initial public release (v1.0.0).
