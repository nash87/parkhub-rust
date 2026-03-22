# Security Model — ParkHub Rust

> **Version:** 3.3.0 | **Last updated:** 2026-03-22

Security architecture, controls, OWASP compliance, and responsible disclosure for
ParkHub Rust (Axum + React 19).

---

## Table of Contents

1. [Security Architecture Overview](#security-architecture-overview)
2. [Authentication](#authentication)
3. [Password Security](#password-security)
4. [Two-Factor Authentication (2FA/TOTP)](#two-factor-authentication-2fatotp)
5. [API Key Authentication](#api-key-authentication)
6. [Authorization](#authorization)
7. [Encryption](#encryption)
8. [Rate Limiting](#rate-limiting)
9. [Input Validation](#input-validation)
10. [OWASP Top 10 Compliance](#owasp-top-10-compliance-matrix)
11. [Security Headers](#security-headers)
12. [File Upload Security](#file-upload-security)
13. [CSRF / XSS Prevention](#csrf--xss-prevention)
14. [Audit Log](#audit-log)
15. [Known Limitations](#known-limitations)
16. [Vulnerability Disclosure Process](#vulnerability-disclosure-process)
17. [Security Contact](#security-contact)

---

## Security Architecture Overview

```
              Optional Reverse Proxy (Nginx/Caddy)
                              |
              Axum Application Layer (single binary)
  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐
  │ Rate Limiter  │  │ SecurityHdrs │  │ RequireAdmin MW  │
  │ (Tower layer) │  │ (Tower)      │  │ (role gate)      │
  └──────┬───────┘  └──────┬───────┘  └──────┬───────────┘
         │                 │                  │
  ┌──────▼─────────────────▼──────────────────▼───────────┐
  │              Session Auth Layer                        │
  │   Bearer Token (SPA) + 2FA/TOTP + Session Expiry      │
  └──────┬────────────────────────────────────────────────┘
         │
  ┌──────▼────────────────────────────────────────────────┐
  │              Typed Rust Input Validation               │
  │   serde deserialization · type-safe extractors         │
  └──────┬────────────────────────────────────────────────┘
         │
  ┌──────▼────────────────────────────────────────────────┐
  │              Audit Log (all write operations)         │
  │   User · Action · Details · IP · Timestamp            │
  └───────────────────────────────────────────────────────┘
                              |
    Database: redb (embedded, no SQL)
    Argon2id passwords · AES-256-GCM encryption at rest
    Memory-safe Rust · no buffer overflows
```

**Key principles:**
- Defense in depth — multiple layers of security controls
- Principle of least privilege — users access only their own data
- Secure by default — security features enabled without configuration
- Self-hosted — no third-party data exposure by default
- Memory safety — Rust eliminates entire classes of vulnerabilities

---

## Authentication

ParkHub Rust uses session-based authentication:

### Bearer Token Authentication

Used by the React frontend and external API consumers.

| Property | Value |
|----------|-------|
| Token type | Opaque Bearer token (UUID-based) |
| Token expiry | Configurable via `session_timeout_minutes` (default: 60) |
| Storage | Token stored in redb; plaintext shown only once on login |
| Revocation on password change | Yes — all sessions invalidated |
| Revocation on deletion | Yes — all sessions invalidated |

---

## Password Security

| Control | Value |
|---------|-------|
| Hashing algorithm | Argon2id |
| Parameters | Memory: 19456 KiB, iterations: 2, parallelism: 1 (OWASP recommended) |
| Minimum length | 8 characters (enforced in registration and change endpoints) |
| Maximum length | 128 characters (prevents DoS on very long inputs) |
| Configurable policies | Minimum length, require uppercase, require numbers, require special characters |
| Password change | Requires current password |
| Account deletion | Requires current password |
| GDPR anonymization | Requires current password |

---

## Two-Factor Authentication (2FA/TOTP)

| Feature | Details |
|---------|---------|
| Standard | TOTP (RFC 6238) — compatible with Google Authenticator, Authy, 1Password, etc. |
| Enrollment | `POST /api/v1/2fa/enable` — returns QR code and secret |
| Verification | `POST /api/v1/2fa/verify` — validates 6-digit TOTP code |
| Backup codes | 8 single-use recovery codes generated on enrollment |
| Disable | `POST /api/v1/2fa/disable` — requires current password |
| Login flow | After password verification, 2FA code is required as a second step |
| Per-account | Each user independently enables/disables 2FA |

---

## API Key Authentication

See [Authentication](#3-api-key-authentication) above.

---

## Authorization

### Role Hierarchy

Three roles with ascending privilege levels:

| Role | Level | Capabilities |
|------|-------|-------------|
| `user` | 1 | Own bookings, vehicles, absences, preferences |
| `admin` | 2 | All user data, reports, settings, user management |
| `superadmin` | 3 | Admin + system configuration, database operations |

### Role Checks

Admin-only endpoints use a `require_admin` extractor that validates the user's role.
This is an application-level check applied per handler — it prevents privilege escalation
if a route is accidentally exposed.

### Resource Ownership

All user resources (vehicles, bookings, absences, favourites, notifications) are scoped
to `WHERE user_id = $request->user()->id`. A user cannot access another user's data
even by guessing a UUID.

---

## Encryption

| Layer | Method | Details |
|-------|--------|---------|
| **Passwords** | Argon2id | Memory-hard, OWASP recommended parameters |
| **In transit** | TLS 1.3 | Built-in TLS or reverse proxy |
| **At rest** | AES-256-GCM | Optional redb encryption with PBKDF2-SHA256 key derivation |
| **API tokens** | UUID-based | Stored in encrypted redb |
| **2FA secrets** | Database encrypted | Stored in redb with AES-256-GCM |

---

## Rate Limiting

| Endpoint | Limit | Window | Key |
|----------|-------|--------|-----|
| `POST /api/v1/auth/login` | 10 requests | 1 minute | Per IP |
| `POST /api/v1/auth/register` | 10 requests | 1 minute | Per IP |
| `POST /api/v1/auth/forgot-password` | 5 requests | 15 minutes | Per IP |
| `POST /api/v1/payments/*` | 10 requests | 1 minute | Per user |
| `POST /api/v1/webhooks/*` | 60 requests | 1 minute | Per IP |
| General API | 60 requests | 1 minute | Per user |

Failed login attempts are recorded in the audit log with action `login_failed`,
including the attempted username and the IP address.

The Rate Limit Dashboard (`GET /api/v1/admin/rate-limits`) provides real-time monitoring
of rate limit groups and a 24-hour history of blocked requests.

---

## Input Validation

Every API endpoint uses Axum extractors with serde deserialization and explicit type
validation. There is no SQL — redb is an embedded key-value store with typed Rust APIs.

Key validation patterns:
- **Email**: validated via regex at deserialization
- **Password**: minimum 8, maximum 128 characters
- **Plate numbers**: string with length validation
- **Dates**: chrono types with automatic parsing
- **IDs**: UUID with type-safe extraction
- **JSON payloads**: serde struct deserialization — only declared fields accepted

---

## OWASP Top 10 Compliance Matrix

| OWASP Category | Status | ParkHub Implementation |
|----------------|--------|----------------------|
| **A01: Broken Access Control** | Mitigated | RBAC (user/admin/superadmin), resource ownership scoping, admin middleware |
| **A02: Cryptographic Failures** | Mitigated | Argon2id passwords, AES-256-GCM at rest, TLS 1.3 in transit |
| **A03: Injection** | Mitigated | No SQL (embedded redb); typed Rust API prevents injection by design |
| **A04: Insecure Design** | Mitigated | Privacy by design (self-hosted), defense in depth, memory-safe Rust |
| **A05: Security Misconfiguration** | Mitigated | Secure defaults; single binary with no separate config files to misconfigure |
| **A06: Vulnerable Components** | Monitored | All dependencies MIT/Apache-2.0; Cargo.lock pinned, `cargo audit` recommended |
| **A07: Authentication Failures** | Mitigated | Rate limiting, 2FA/TOTP, session expiry, Argon2id, configurable password policies |
| **A08: Software and Data Integrity** | Mitigated | Cargo.lock + npm lock files, static binary compilation |
| **A09: Security Logging** | Implemented | Full audit log — login, registration, deletion, password changes, admin actions |
| **A10: Server-Side Request Forgery** | Not applicable | No server-side URL fetching from user input |

---

## Security Headers

Security headers are applied via Tower middleware layers:

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Content-Type-Options` | `nosniff` | Prevent MIME type sniffing |
| `X-Frame-Options` | `SAMEORIGIN` | Prevent clickjacking |
| `X-XSS-Protection` | `0` | Disabled (CSP is preferred) |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Limit referrer information |
| `Permissions-Policy` | `camera=(), microphone=(), geolocation=()` | Restrict browser APIs |

Additional headers recommended at the reverse proxy level:

```nginx
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self'" always;
```

Set `APP_URL=https://...` in `.env` to ensure all generated URLs use HTTPS.

---

## File Upload Security

File uploads are processed through security controls:

1. **MIME type validation**: Content-Type header checked against allowed types
2. **File size limit**: Configurable per endpoint via Tower body limit layer
3. **No file system access**: ParkHub Rust stores all data in redb — no file system upload directory

---

## CSRF / XSS Prevention

### CSRF

ParkHub Rust is a pure SPA communicating via JSON API with Bearer token authentication.
CSRF protection via cookies is not applicable. All state-changing requests require a
valid Bearer token in the `Authorization` header.

### XSS

- All user-supplied content is rendered through React's JSX, which escapes values by default
- No user-supplied content is rendered as raw HTML without sanitization
- All frontend assets are embedded in the binary and served with correct Content-Type headers
- Content Security Policy headers prevent inline script injection

---

## Audit Log

All write operations create an entry in the `audit_log` table.
The table has no delete endpoint — deletion requires direct database access.

| Action | Triggered by |
|--------|-------------|
| `login` | Successful login |
| `login_failed` | Failed login attempt |
| `register` | New user registration |
| `account_deleted` | User deletes own account |
| `gdpr_erasure` | GDPR Art. 17 anonymization |
| `forgot_password` | Password reset request |
| `password_changed` | Password change |
| `2fa_enabled` | User enables two-factor authentication |
| `2fa_disabled` | User disables two-factor authentication |
| `impressum_updated` | Admin edits Impressum |
| `settings_updated` | Admin changes system settings |
| `database_reset` | Admin resets the database |
| `user_role_changed` | Admin changes a user's role |

Each entry stores: `user_id` (nullable), `username`, `action`, `details` (JSON),
`ip_address`, `created_at`.

---

## Known Limitations

| Limitation | Mitigation |
|-----------|-----------|
| redb is single-writer | Horizontal scaling requires a reverse proxy with sticky sessions |
| No built-in WAF | Deploy behind Cloudflare, AWS WAF, or ModSecurity |
| No automatic dependency vulnerability scanning | Configure Dependabot or `cargo audit` in CI |

---

## Vulnerability Disclosure Process

ParkHub follows a responsible disclosure process:

### Reporting

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. **Preferred**: Create a [GitHub Security Advisory](https://github.com/nash87/parkhub-rust/security/advisories/new) (private)
3. **Alternative**: Email the security contact below

### What to Include

- Description of the vulnerability
- Steps to reproduce (proof of concept if possible)
- Potential impact assessment
- Affected versions
- Suggested fix (if available)

### Response Timeline

| Severity | Acknowledgement | Fix Timeline |
|----------|----------------|--------------|
| Critical (RCE, auth bypass, data leak) | Within 24 hours | Within 7 days |
| High (privilege escalation, XSS) | Within 48 hours | Within 14 days |
| Medium (information disclosure) | Within 72 hours | Within 30 days |
| Low (best practice) | Within 1 week | Next release |

### Recognition

- Security researchers are credited in release notes (unless anonymity is requested)
- Significant findings may be assigned a CVE

### CVE History

No CVEs have been reported against ParkHub Rust.

---

## Security Contact

- **GitHub Security Advisory**: [Create advisory](https://github.com/nash87/parkhub-rust/security/advisories/new)
- **Repository**: [github.com/nash87/parkhub-rust](https://github.com/nash87/parkhub-rust)
- **Supported versions**: See [SECURITY.md](/SECURITY.md) (root) for version support matrix

---

*This security documentation covers ParkHub Rust v3.3.0. For GDPR compliance, see
[GDPR.md](GDPR.md). For the full compliance matrix, see [COMPLIANCE.md](COMPLIANCE.md).*
