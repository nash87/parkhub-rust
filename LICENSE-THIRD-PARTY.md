# Third-Party Licenses -- ParkHub Rust

ParkHub Rust is MIT licensed. This file documents all third-party dependencies and their
licenses to ensure compatibility with open-source distribution.

> **Last updated:** 2026-03-22 (v3.3.0)

---

## License Compatibility Summary

All production dependencies use permissive licenses (MIT, Apache-2.0, BSD-3-Clause).
No GPL, LGPL, or other copyleft licenses are present in the dependency tree.
This project is fully compatible with MIT distribution of both source and binaries.

---

## Rust / Cargo Dependencies

### Core Runtime Dependencies

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| axum | 0.8 | MIT | HTTP framework |
| tokio | 1 | MIT | Async runtime |
| tower | 0.5 | MIT | Service abstraction layer |
| tower-http | 0.6 | MIT | HTTP middleware (CORS, compression, tracing) |
| serde | 1 | MIT OR Apache-2.0 | Serialization/deserialization |
| serde_json | 1 | MIT OR Apache-2.0 | JSON processing |
| chrono | 0.4 | MIT OR Apache-2.0 | Date/time handling |
| uuid | 1 | MIT OR Apache-2.0 | UUID generation |
| thiserror | 2 | MIT OR Apache-2.0 | Error derive macros |
| anyhow | 1 | MIT OR Apache-2.0 | Error handling |
| tracing | 0.1 | MIT | Structured logging |
| tracing-subscriber | 0.3 | MIT | Log subscriber |
| reqwest | 0.12 | MIT OR Apache-2.0 | HTTP client |

### Database

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| redb | 2 | MIT OR Apache-2.0 | Embedded key-value database |

### Cryptography

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| argon2 | 0.5 | MIT OR Apache-2.0 | Password hashing (Argon2id) |
| aes-gcm | 0.10 | MIT OR Apache-2.0 | AES-256-GCM encryption at rest |
| pbkdf2 | 0.12 | MIT OR Apache-2.0 | Key derivation |
| sha2 | 0.10 | MIT OR Apache-2.0 | SHA-256 hashing |
| rand | 0.9 | MIT OR Apache-2.0 | Cryptographic random number generation |
| zeroize | 1 | MIT OR Apache-2.0 | Secure memory zeroing |

### TLS

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| rustls | 0.23 | MIT OR Apache-2.0 | TLS 1.3 implementation |
| rcgen | 0.14 | MIT OR Apache-2.0 | Self-signed certificate generation |

### 2FA

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| totp-rs | 5 | MIT | TOTP two-factor authentication |

### Network

| Crate | Version | License | Purpose |
|-------|---------|---------|---------|
| mdns-sd | 0.18 | MIT OR Apache-2.0 | mDNS service discovery |

---

## Frontend Dependencies (npm)

### Runtime Dependencies

| Package | Version | License | Purpose |
|---------|---------|---------|---------|
| react | ^19 | MIT | UI library |
| react-dom | ^19 | MIT | React DOM renderer |
| react-router-dom | ^7 | MIT | Client-side routing |
| @phosphor-icons/react | ^2 | MIT | Icon library |
| framer-motion | ^12 | MIT | Animation library |
| react-hot-toast | ^2 | MIT | Toast notifications |
| i18next | ^25 | MIT | Internationalization framework |
| react-i18next | ^16 | MIT | React bindings for i18next |

### Development Dependencies

| Package | Version | License | Purpose |
|---------|---------|---------|---------|
| vite | ^7 | MIT | Build tool and dev server |
| typescript | ~5.9 | Apache-2.0 | TypeScript compiler |
| tailwindcss | ^3 | MIT | CSS framework |
| vitest | latest | MIT | Unit testing framework |
| @playwright/test | ^1 | Apache-2.0 | End-to-end testing |

---

## License Details

### MIT License

The majority of dependencies use the MIT License, which permits:
- Commercial use
- Modification
- Distribution
- Private use

With the condition that the license and copyright notice are included.

### Apache-2.0

Many Rust crates are dual-licensed under MIT OR Apache-2.0. Apache-2.0 is compatible
with MIT. Key additions over MIT: explicit patent grant and contribution terms.

---

## Verification

To verify current dependency licenses:

```bash
# Rust dependencies
cargo deny check licenses

# npm dependencies
npx license-checker --summary
```

---

## License Compatibility Conclusion

| Category | Status |
|----------|--------|
| Rust runtime dependencies | All MIT or MIT/Apache-2.0 dual -- fully compatible |
| npm runtime dependencies | All MIT -- fully compatible |
| npm dev dependencies | MIT + Apache-2.0 -- fully compatible |

**This project is fully cleared for open-source MIT release.** No copyleft dependencies
are present in the production build.
