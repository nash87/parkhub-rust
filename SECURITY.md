# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 2.2.x   | Yes                |
| 2.1.x   | Yes                |
| 2.0.x   | Security fixes only |
| < 2.0   | No                 |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, use one of these channels:

1. **GitHub Security Advisory** (preferred):
   [Create a private advisory](https://github.com/nash87/parkhub-rust/security/advisories/new)

2. **Email**: Open a private security advisory on GitHub (see above)

Please include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact assessment
- Suggested fix (if available)

### Response Times

| Severity | Acknowledgement | Fix Timeline |
|----------|----------------|--------------|
| Critical | Within 24 hours | Within 7 days |
| High     | Within 48 hours | Within 14 days |
| Medium   | Within 72 hours | Within 30 days |
| Low      | Within 1 week   | Next release |

Researchers are credited in release notes unless anonymity is requested.

## Security Features

### Authentication
- **2FA/TOTP** -- QR code enrollment, backup codes, per-account toggle
- **Argon2id** password hashing with cryptographically random salts (OsRng)
- **Constant-time token comparison** via opaque UUID Bearer tokens
- **4-tier RBAC** (user, premium, admin, superadmin) enforced at handler level

### Encryption
- **AES-256-GCM** database encryption at rest (optional, via `PARKHUB_DB_PASSPHRASE`)
- **PBKDF2-SHA256** key derivation from passphrase
- **zeroize** crate zeroes key material in memory on drop
- **TLS 1.3** via rustls (pure Rust, no OpenSSL dependency)

### Transport Security
- **CSP headers** -- `default-src 'self'`, frame-ancestors none
- **HSTS** -- `max-age=31536000; includeSubDomains`
- **X-Frame-Options** DENY, X-Content-Type-Options nosniff
- **Referrer-Policy** strict-origin-when-cross-origin
- **Permissions-Policy** disables geolocation, camera, microphone

### Rate Limiting
- Login: 5 requests/minute per IP
- Registration: 3 requests/minute per IP
- Global: 100 req/s with burst 200

### Input Validation
- 4 MiB request body size limit
- Strict JSON deserialization (unknown fields rejected)
- UUID validation at the type level
- Positive duration validation before arithmetic

### Concurrency
- Async RwLock on booking creation prevents double-booking race conditions
- No SQL injection surface (redb is a key-value store, not SQL)

### Supply Chain
- Static musl binary -- no shared library vulnerabilities
- All cryptography via audited Rust crates (argon2, aes-gcm, rustls, rand)
- No third-party JavaScript loaded at runtime

## Full Security Documentation

See [docs/SECURITY.md](docs/SECURITY.md) for the complete security model, architecture details, and audit log reference.

## CVE History

No CVEs have been reported against ParkHub.
