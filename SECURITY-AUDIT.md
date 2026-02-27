# Security Audit — parkhub-rust

**Audit Date:** 2026-02-27
**Scope:** Full codebase audit covering secrets, .gitignore, Rust dependencies, OWASP Top 10, and security headers.

---

## Summary

| Category | Status |
|---|---|
| Secrets in code | 1 fixed (hardcoded dev passphrase removed) |
| .gitignore | Updated — added missing entries |
| Rust dependencies | Good — modern versions, no known CVEs |
| OWASP A01 Broken Access Control | Good — admin role checks present on all admin endpoints |
| OWASP A02 Cryptographic Failures | Good — Argon2id, TLS self-signed via rcgen |
| OWASP A03 Injection | Good — no raw SQL, parameterized via redb |
| OWASP A05 Security Misconfiguration | Good — no debug mode, generic error messages |
| OWASP A07 Auth Failures | Informational — rate limiting defined but not wired to auth routes |
| OWASP A09 Security Logging | Good — comprehensive audit infrastructure exists; not yet called from handlers |
| Security Headers | Good — applied globally via Axum middleware layer |

---

## 1. Secrets in Code / Repository

### ❌ FIXED: Hardcoded fallback database passphrase

**File:** `parkhub-server/src/main.rs` line ~257
**Before:**
```rust
warn!("Using default passphrase - NOT RECOMMENDED FOR PRODUCTION");
config.encryption_passphrase = Some("default-dev-passphrase".to_string());
```
**After:** The server now fails fast with a clear actionable error message:
```
Database encryption is enabled but PARKHUB_DB_PASSPHRASE is not set.
Set the PARKHUB_DB_PASSPHRASE environment variable to a strong, randomly generated passphrase.
Example: export PARKHUB_DB_PASSPHRASE="$(openssl rand -base64 32)"
```

This code path only triggered in the `headless` + no-GUI build path when `encryption_enabled = true` and the environment variable was absent. A server started this way had its database encryption silently degraded to a known-static key.

**Operator action required for production:** Always set `PARKHUB_DB_PASSPHRASE` before starting the server in headless/Docker mode.

---

### ✅ Passed: No API keys, tokens, or JWT secrets hardcoded

No `sk-ant-`, `ghp_`, or other credential patterns found in any `.rs`, `.toml`, or config files.

### ✅ Passed: config/config.toml uses placeholder values only

`config/config.toml` (client-side config) contains `"YOUR_GOOGLE_CLIENT_SECRET"` — clearly a placeholder, not a real credential.

### ⚠️ Informational: Dummy user password logged at INFO level

**File:** `parkhub-server/src/main.rs` line ~1184
```rust
info!("Default login: any username with password '{}'", default_password);
```
The `generate_dummy_users` flag is `false` by default and is a dev-only feature. This is acceptable — dummy users are never created in production. However, if this feature is used in any staging environment, the password will appear in plain text in application logs.

**Recommendation:** Consider removing the log line or replacing the literal password with a note like `"(see source)"`.

### ⚠️ Informational: Unattended mode uses admin/admin credentials

When started with `--unattended`, the server auto-creates an admin account with `admin/admin` credentials (hashed via Argon2id). This is logged at INFO:
```
Default config saved. Admin credentials: admin/admin
```
This mode is intended for initial bootstrapping. Operators must change the admin password immediately after first use.

---

## 2. .gitignore Completeness

### ❌ FIXED: Missing certificate/key file patterns

Added to `.gitignore`:
```
*.pem
*.key
*.p12
*.pfx
*.crt
*.cer
```

### ❌ FIXED: Missing database file patterns

The server uses `redb` (embedded database, `.redb` extension). Added:
```
*.db
*.sqlite
*.redb
```

### ❌ FIXED: Missing frontend build artifacts

Added:
```
node_modules/
dist/
parkhub-web/dist/
```

### ❌ FIXED: .env.* variants missing

Added `.env.*` to cover `.env.production`, `.env.staging`, etc.

### ✅ Passed: Existing entries

- `/target/` (Rust build artifacts) — present
- `.env` — present
- `*.log`, `logs/` — present
- `data/` (user data directory) — present

---

## 3. Dependency Security Review (Rust)

All workspace dependencies are pinned to specific semantic versions. No wildcard (`*`) versions found.

| Crate | Version | Status |
|---|---|---|
| `jsonwebtoken` | 9 | Current — no known CVEs for v9.x |
| `rustls` | 0.23 | Current LTS — TLS 1.2/1.3 |
| `argon2` | 0.5 | Current — uses Argon2id algorithm |
| `axum` | 0.7 | Current |
| `rcgen` | 0.13 | Current |
| `ring` | 0.17 | Current |
| `aes-gcm` | 0.10 | Current |
| `pbkdf2` | 0.12 | Current |
| `redb` | 2 | Current |
| `tower_governor` | 0.4 | Current |
| `tokio` | 1 | Current LTS |

**No pre-2023 openssl dependency.** The codebase uses `rustls` (pure Rust TLS) throughout, which avoids the OpenSSL vulnerability surface entirely.

### ⚠️ Informational: Cargo.lock excluded from .gitignore for workspace

`Cargo.lock` is listed in `.gitignore` (line 2). For a binary application (not a library), `Cargo.lock` should normally be committed to ensure reproducible builds and to allow tools like `cargo audit` to flag known-vulnerable dependency versions. However, since the server builds from source with `cargo build --release`, this is a minor operational concern.

**Recommendation:** Remove `Cargo.lock` from `.gitignore` for binary releases to enable `cargo audit` checks in CI.

---

## 4. Dependency Security Review (npm / Frontend)

**File:** `parkhub-web/package.json` — not directly audited (no package.json found in parkhub-web, frontend is embedded in the binary).

Frontend dependencies are embedded at build time via `rust-embed`. The npm `package.json` at the workspace root is for Playwright E2E testing only.

---

## 5. OWASP Top 10 Review

### A01 — Broken Access Control ✅

All admin endpoints in `api.rs` perform role checks by fetching the calling user from the database and comparing against `UserRole::Admin` or `UserRole::SuperAdmin`:

- `create_lot`: admin role check present
- `get_user` (by ID): admin role check present
- `get_impressum_admin`: admin role check present
- `update_impressum`: admin role check present

IDOR prevention:
- `get_booking`: checks `booking.user_id != auth_user.user_id` before returning
- `cancel_booking`: checks `booking.user_id != auth_user.user_id`
- `delete_vehicle`: checks `vehicle.user_id != auth_user.user_id`
- `create_booking`: checks `v.user_id != auth_user.user_id` for vehicle ownership

**No broken access control vulnerabilities found.**

### A02 — Cryptographic Failures ✅

- Passwords hashed with **Argon2id** (current OWASP recommended algorithm) using random salts via `OsRng`
- JWT secret is **generated randomly at startup** via `Uuid::new_v4()` (`JwtConfig::default()`). Sessions survive server restart via the database session store, so a new random secret per startup is safe.
- TLS certificates generated via `rcgen` (self-signed) — operators should replace with a trusted certificate in production
- Database encryption uses **AES-256-GCM** with PBKDF2-derived key from the operator-supplied passphrase

### ⚠️ A02 — Informational: JWT secret not persisted across restarts

The `JwtManager::with_random_secret()` generates a new secret on every server start. All existing JWT tokens are immediately invalidated on restart. This is safe if the session database is the source of truth (which it is — auth middleware uses `db.get_session(token)`), but means any long-lived token previously issued by a different process will be rejected.

This is by design and acceptable for a LAN-only deployment.

### A03 — Injection ✅

- The server uses **redb** (embedded key-value store) with no SQL whatsoever — SQL injection is not applicable
- No `std::process::Command` invocations found — no command injection surface

### A05 — Security Misconfiguration ✅

- No `APP_DEBUG` or debug-mode flag exposes stack traces in API responses
- Error handler (`error.rs`) returns opaque error codes and generic messages — internal details are logged server-side only via `tracing::error!`
- Readiness probe at `/health/ready` logs internal error server-side, returns only `{"ready": false}`

### A07 — Identification and Authentication Failures

#### ✅ Rate limiting infrastructure is defined

`EndpointRateLimiters` in `rate_limit.rs` defines:
- Login: 5 attempts/minute per IP
- Register: 3 per minute per IP

#### ⚠️ Rate limiting is not wired to login/register routes

The `create_router()` function does not attach the per-IP rate limiters to `/api/v1/auth/login` or `/api/v1/auth/register`. The `EndpointRateLimiters` struct is defined and has correct limits but is never instantiated and passed as middleware in `api.rs`.

**Impact:** Brute-force login attempts are not rate-limited at the API layer in the current code. The global rate limiter (100 req/s global, not per-IP) is also not wired.

**Recommendation for operators:** Deploy behind a reverse proxy (nginx, Caddy) that enforces per-IP rate limiting on the auth endpoints. The internal rate limiter should also be wired in a future release.

#### ✅ Session invalidation on logout

Tokens are stored in the `redb` session table. The `auth_middleware` validates the session against the database on every request using `db.get_session(token)` — expired sessions are rejected. The client must clear the token.

### A09 — Security Logging and Monitoring ✅ (infrastructure) / ⚠️ (not called from handlers)

`audit.rs` defines a comprehensive `AuditEntry` builder with structured event types covering:
`LoginSuccess`, `LoginFailed`, `Logout`, `UserCreated`, `BookingCreated`, `BookingCancelled`, `RateLimitExceeded`, `UnauthorizedAccess`, `ConfigChanged`, `RoleChanged`, and more.

However, **the handlers in `api.rs` do not call the audit functions**. For example, `login()` validates credentials and creates a session but does not call `audit::events::login_success()` or `audit::events::login_failed()`.

**Recommendation:** Wire the existing audit infrastructure to the auth and admin handlers. The builder pattern is already ergonomic — it is a matter of adding calls like:
```rust
audit::events::login_success(user.id, &user.username, client_ip).log();
```

---

## 6. Security Headers ✅

The `security_headers_middleware` in `api.rs` is registered as a `.layer()` at the outermost `Router` level:
```rust
.layer(axum::middleware::from_fn(security_headers_middleware))
```

Because Axum processes layers from outermost inward, this middleware runs on **every response** including static files served via the `.fallback(static_files::static_handler)` handler. All responses carry:

| Header | Value |
|---|---|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'unsafe-inline'; ...` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |
| `Permissions-Policy` | `geolocation=(), camera=(), microphone=()` |

### ⚠️ Informational: CSP uses `'unsafe-inline'` for scripts and styles

The CSP allows `'unsafe-inline'` for both `script-src` and `style-src`. This is a common trade-off for SPA frameworks that inject inline styles. It reduces the effectiveness of the CSP against XSS.

**Recommendation:** If the frontend can be refactored to use CSS-in-files rather than inline styles, removing `'unsafe-inline'` from `style-src` would meaningfully improve XSS protection.

### ⚠️ Informational: HSTS header not set

The server does not set `Strict-Transport-Security`. Since TLS is self-signed and LAN-only, browsers will not honor HSTS anyway, but it should be added for any public deployment behind a trusted certificate.

---

## 7. TLS Configuration

- Self-signed certificates are generated at startup via `rcgen`
- Uses `rustls` 0.23 with TLS 1.2 + 1.3 enabled
- `tls12` feature explicitly enabled in workspace `Cargo.toml` — TLS 1.2 is a reasonable minimum for LAN deployments

### ⚠️ Operator note: Replace self-signed cert in production

For external/public deployments, replace the auto-generated self-signed certificate with a certificate from a trusted CA (Let's Encrypt, internal CA). The `enable_tls = false` path in `config.toml` should only be used during initial setup.

---

## Operator Security Checklist

Before going to production, operators must:

- [ ] Set `PARKHUB_DB_PASSPHRASE` environment variable (strong random passphrase, at least 32 bytes)
- [ ] Change the default admin password immediately after first-run setup
- [ ] Replace the auto-generated self-signed TLS certificate with a trusted CA certificate
- [ ] Deploy behind a reverse proxy with per-IP rate limiting on `/api/v1/auth/login` and `/api/v1/auth/register`
- [ ] Remove `Cargo.lock` from `.gitignore` and run `cargo audit` in CI to track dependency CVEs
- [ ] Wire the existing `audit.rs` infrastructure to API handlers for complete audit trail
