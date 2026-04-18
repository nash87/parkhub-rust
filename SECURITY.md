# Security Policy

## Supported Versions

| Version | Supported              |
|---------|------------------------|
| 4.9.x   | Yes (current)          |
| 4.8.x   | Security fixes only    |
| < 4.8   | No                     |

## Known Accepted Advisories

The following advisories are known and accepted with documented mitigations. They are
silenced in `deny.toml` with rationale comments and mirrored in the nightly
`cargo audit --ignore …` list in `.github/workflows/nightly.yml`. See `deny.toml`
for the canonical list. Every entry below is re-evaluated when touched or
whenever the upstream crate ships a fix.

| Advisory ID        | Crate              | Severity        | Rationale |
|--------------------|--------------------|-----------------|-----------|
| RUSTSEC-2023-0019  | kuchiki            | Unmaintained    | Pulled transitively via `printpdf` HTML rendering. No current maintained fork with a compatible API; invoice renderer is server-side only and never sees attacker-controlled HTML. |
| RUSTSEC-2023-0071  | rsa                | Medium (Marvin) | Pulled transitively via `web-push` (VAPID JWT signing). ParkHub uses HS256 HMAC JWTs only — the timing side-channel is not reachable from our usage. Tracked: upgrade as soon as `web-push` ships an RSA-free path. |
| RUSTSEC-2024-0370  | proc-macro-error   | Unmaintained    | Build-time-only crate pulled by `zeroize_derive`. No runtime exposure; replace on next `zeroize` minor that drops the dep. |
| RUSTSEC-2024-0384  | instant            | Unmaintained    | Transitive via `web-push → isahc`. No maintained upgrade path; `instant` has no known vulnerabilities, just a maintenance flag. |
| RUSTSEC-2024-0412  | atk (gtk-rs)       | Unmaintained    | Only pulled under the optional `gui` feature via `tray-icon`. The default `headless` build excludes all gtk-rs code entirely. |
| RUSTSEC-2024-0413  | gdk (gtk-rs)       | Unmaintained    | Same as RUSTSEC-2024-0412 — `gui` feature only. |
| RUSTSEC-2024-0415  | gdk-pixbuf (gtk-rs)| Unmaintained    | Same as RUSTSEC-2024-0412 — `gui` feature only. |
| RUSTSEC-2024-0416  | gdk-sys (gtk-rs)   | Unmaintained    | Same as RUSTSEC-2024-0412 — `gui` feature only. |
| RUSTSEC-2024-0418  | gtk (gtk-rs)       | Unmaintained    | Same as RUSTSEC-2024-0412 — `gui` feature only. |
| RUSTSEC-2024-0419  | gtk-sys (gtk-rs)   | Unmaintained    | Same as RUSTSEC-2024-0412 — `gui` feature only. |
| RUSTSEC-2024-0420  | gtk3-macros        | Unmaintained    | Same as RUSTSEC-2024-0412 — `gui` feature only. |
| RUSTSEC-2024-0436  | paste              | Unmaintained    | Build-time macro crate used by `wayland-sys`; itself has no runtime footprint. |
| RUSTSEC-2025-0057  | fxhash             | Unmaintained    | Transitive via `selectors` (HTML parser). Replaced by `rustc-hash` upstream; we pick it up automatically when the dependency tree moves. |
| RUSTSEC-2026-0097  | rand 0.9.2         | Unsoundness     | Only triggers with a custom `log` logger that reads `thread_rng` during its own logging; we use `tracing`, not `log`, so the faulty code path is never executed. |

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

## Kubernetes Hardening

The bundled Helm chart (`helm/parkhub/`) ships with the full [Pod Security
Standards **restricted**](https://kubernetes.io/docs/concepts/security/pod-security-standards/#restricted)
profile, default on:

- `runAsNonRoot: true`, `runAsUser: 1000`
- `allowPrivilegeEscalation: false`
- `capabilities: drop: [ALL]`
- `readOnlyRootFilesystem: true`
- `seccompProfile: { type: RuntimeDefault }`

`parkhub-server` is a static Rust binary backed by `redb`; the only
writable path is the PVC mount at `/data`. No `emptyDir` shims are
needed — unlike the Laravel/Apache stack in `parkhub-php`.

## Full Security Documentation

See [docs/SECURITY.md](docs/SECURITY.md) for the complete security model, architecture details, and audit log reference.

## CVE History

No CVEs have been reported against ParkHub.
