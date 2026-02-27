# Third-Party Licenses — ParkHub Rust

ParkHub Rust is MIT licensed. This file documents all third-party dependencies
and their licenses to ensure compatibility with open-source distribution.

## License Compatibility Summary

All production dependencies use MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause,
ISC, or dual MIT/Apache-2.0 licenses. These are fully compatible with this
project's MIT license. The `slint` GUI framework uses the GNU GPL-3.0 for its
community edition — this is addressed in the note at the bottom of this file.

---

## Backend Dependencies (Rust/Cargo)

### parkhub-common

| Crate | Version | License | Notes |
|-------|---------|---------|-------|
| serde | 1.x | MIT OR Apache-2.0 | Serialization framework |
| serde_json | 1.x | MIT OR Apache-2.0 | JSON support |
| chrono | 0.4.x | MIT OR Apache-2.0 | Date/time library |
| uuid | 1.x | MIT OR Apache-2.0 | UUID generation |
| thiserror | 2.x | MIT OR Apache-2.0 | Error derive macros |

### parkhub-server

| Crate | Version | License | Notes |
|-------|---------|---------|-------|
| axum | 0.7.x | MIT | Web framework |
| axum-extra | 0.9.x | MIT | Axum typed headers |
| axum-server | 0.7.x | MIT | TLS server support |
| tower | 0.5.x | MIT | Service abstraction |
| tower-http | 0.6.x | MIT | HTTP middleware |
| tower_governor | 0.4.x | MIT | Rate limiting middleware |
| governor | 0.7.x | MIT OR Apache-2.0 | Rate limiting core |
| tokio | 1.x | MIT | Async runtime |
| tokio-cron-scheduler | 0.13.x | MIT | Cron job scheduling |
| redb | 2.x | MIT OR Apache-2.0 | Embedded database |
| serde | 1.x | MIT OR Apache-2.0 | Serialization framework |
| serde_json | 1.x | MIT OR Apache-2.0 | JSON support |
| chrono | 0.4.x | MIT OR Apache-2.0 | Date/time library |
| uuid | 1.x | MIT OR Apache-2.0 | UUID generation |
| thiserror | 2.x | MIT OR Apache-2.0 | Error derive macros |
| anyhow | 1.x | MIT OR Apache-2.0 | Error handling |
| tracing | 0.1.x | MIT | Structured logging |
| tracing-subscriber | 0.3.x | MIT | Log subscriber |
| aes-gcm | 0.10.x | MIT OR Apache-2.0 | AES-GCM encryption |
| pbkdf2 | 0.12.x | MIT OR Apache-2.0 | Password hashing |
| sha2 | 0.10.x | MIT OR Apache-2.0 | SHA-2 hash functions |
| rand | 0.8.x | MIT OR Apache-2.0 | Random number generation |
| rand_core | 0.6.x | MIT OR Apache-2.0 | RNG core traits |
| zeroize | 1.x | MIT OR Apache-2.0 | Secure memory zeroing |
| hex | 0.4.x | MIT OR Apache-2.0 | Hex encoding |
| rcgen | 0.13.x | MIT OR Apache-2.0 | TLS certificate generation |
| rustls | 0.23.x | MIT OR Apache-2.0 OR ISC | TLS implementation |
| rustls-pemfile | 2.x | MIT OR Apache-2.0 OR ISC | PEM file parsing |
| ring | 0.17.x | MIT AND ISC AND OpenSSL | Cryptographic primitives |
| argon2 | 0.5.x | MIT OR Apache-2.0 | Argon2 password hashing |
| jsonwebtoken | 9.x | MIT | JWT encode/decode |
| mdns-sd | 0.11.x | MIT OR Apache-2.0 | mDNS/DNS-SD discovery |
| validator | 0.18.x | MIT OR Apache-2.0 | Input validation |
| utoipa | 5.x | MIT OR Apache-2.0 | OpenAPI documentation |
| utoipa-swagger-ui | 8.x | MIT OR Apache-2.0 | Swagger UI |
| metrics | 0.24.x | MIT | Metrics facade |
| metrics-exporter-prometheus | 0.16.x | MIT | Prometheus metrics exporter |
| toml | 0.8.x | MIT OR Apache-2.0 | TOML parsing |
| directories | 5.x | MIT OR Apache-2.0 | Platform directories |
| hostname | 0.4.x | MIT OR Apache-2.0 | System hostname |
| once_cell | 1.21.x | MIT OR Apache-2.0 | Lazy/once initialization |
| regex | 1.12.x | MIT OR Apache-2.0 | Regular expressions |
| rust-embed | 8.11.x | MIT | Embed files into binary |
| mime_guess | 2.0.x | MIT | MIME type detection |
| slint | 1.14.x | **GPL-3.0** (community) | GUI framework — see note below |
| tray-icon | 0.18.x | MIT OR Apache-2.0 | System tray icon (optional) |
| windows-sys | 0.59.x | MIT OR Apache-2.0 | Windows API bindings (Windows only) |
| winres | 0.1.x | MIT OR Apache-2.0 | Windows resource compiler (build) |
| tempfile | 3.x | MIT OR Apache-2.0 | Temp files (dev only) |

### parkhub-client

| Crate | Version | License | Notes |
|-------|---------|---------|-------|
| slint | 1.14.x | **GPL-3.0** (community) | GUI framework — see note below |
| slint-build | 1.14.x | **GPL-3.0** (community) | Slint build support — see note below |
| raw-window-handle | 0.6.x | MIT OR Apache-2.0 OR Zlib | Window handle abstraction |
| tokio | 1.x | MIT | Async runtime |
| reqwest | 0.12.x | MIT OR Apache-2.0 | HTTP client |
| serde | 1.x | MIT OR Apache-2.0 | Serialization |
| serde_json | 1.x | MIT OR Apache-2.0 | JSON support |
| toml | 0.8.x | MIT OR Apache-2.0 | TOML parsing |
| chrono | 0.4.x | MIT OR Apache-2.0 | Date/time |
| uuid | 1.x | MIT OR Apache-2.0 | UUID generation |
| directories | 5.x | MIT OR Apache-2.0 | Platform directories |
| tracing | 0.1.x | MIT | Structured logging |
| tracing-subscriber | 0.3.x | MIT | Log subscriber |
| thiserror | 2.x | MIT OR Apache-2.0 | Error macros |
| anyhow | 1.x | MIT OR Apache-2.0 | Error handling |
| mdns-sd | 0.11.x | MIT OR Apache-2.0 | mDNS/DNS-SD discovery |
| rustls | 0.23.x | MIT OR Apache-2.0 OR ISC | TLS |
| rustls-pemfile | 2.x | MIT OR Apache-2.0 OR ISC | PEM parsing |
| rand | 0.8.x | MIT OR Apache-2.0 | RNG |
| windows-sys | 0.59.x | MIT OR Apache-2.0 | Windows API (Windows only) |
| winres | 0.1.x | MIT OR Apache-2.0 | Windows resources (build) |

---

## Frontend Dependencies (npm — parkhub-web)

### Runtime Dependencies

| Package | Version | License | Notes |
|---------|---------|---------|-------|
| react | ^19.2.0 | MIT | UI library |
| react-dom | ^19.2.0 | MIT | React DOM renderer |
| react-router-dom | ^7.13.0 | MIT | Client-side routing |
| @phosphor-icons/react | ^2.1.10 | MIT | Icon library |
| @tanstack/react-query | ^5.90.20 | MIT | Server state management |
| framer-motion | ^12.33.0 | MIT | Animation library |
| date-fns | ^4.1.0 | MIT | Date utility library |
| react-hot-toast | ^2.6.0 | MIT | Toast notifications |
| zustand | ^5.0.11 | MIT | State management |

### Development Dependencies

| Package | Version | License | Notes |
|---------|---------|---------|-------|
| vite | ^7.2.4 | MIT | Build tool |
| typescript | ~5.9.3 | Apache-2.0 | TypeScript compiler |
| @vitejs/plugin-react | ^5.1.1 | MIT | Vite React plugin |
| tailwindcss | ^3.4.0 | MIT | CSS framework |
| postcss | ^8.5.6 | MIT | CSS processing |
| autoprefixer | ^10.4.24 | MIT | CSS vendor prefixes |
| @tailwindcss/forms | ^0.5.0 | MIT | Tailwind forms plugin |
| eslint | ^9.39.1 | MIT | JavaScript linter |
| @eslint/js | ^9.39.1 | MIT | ESLint JS config |
| eslint-plugin-react-hooks | ^7.0.1 | MIT | React hooks lint rules |
| eslint-plugin-react-refresh | ^0.4.24 | MIT | React Refresh lint rules |
| typescript-eslint | ^8.46.4 | MIT | TypeScript ESLint |
| globals | ^16.5.0 | MIT | Global variable definitions |
| @types/react | ^19.2.5 | MIT | React TypeScript types |
| @types/react-dom | ^19.2.3 | MIT | React DOM TypeScript types |
| @types/node | ^24.10.1 | MIT | Node.js TypeScript types |

---

## E2E Test Dependencies (root package.json)

| Package | Version | License | Notes |
|---------|---------|---------|-------|
| playwright | ^1.58.1 | Apache-2.0 | End-to-end browser testing |

---

## Notes on Specific Dependencies

### ring (0.17.x) — MIT AND ISC AND OpenSSL

The `ring` crate uses a custom combination of MIT, ISC, and a permissive
OpenSSL-derived license. All three are permissive and compatible with MIT
distribution. No action required for open-source release.

### rustls (0.23.x) — MIT OR Apache-2.0 OR ISC

Triple-licensed under three permissive licenses. Fully compatible with MIT.

### slint (1.14.x) — GPL-3.0 Community Edition

**This is the most important licensing consideration in this project.**

Slint is used in `parkhub-server` (optional `gui` feature) and `parkhub-client`
(the desktop application). Slint's community edition is licensed under
**GNU General Public License v3.0 (GPL-3.0)**.

**Impact analysis:**

- The GPL-3.0 is a strong copyleft license. If you distribute a binary that
  links against GPL-3.0 code, the entire combined work must be distributed
  under GPL-3.0 (or a compatible copyleft license), not MIT.

- **For purely open-source distribution** (source available on GitHub, users
  build from source): GPL-3.0 is compatible with this use case. You can publish
  your source under MIT and users who build with `slint` will receive a
  GPL-3.0-governed binary. This is a valid model used by many open-source
  projects that depend on GPL libraries.

- **If you distribute pre-built binaries**: The binaries that include `slint`
  must be offered under GPL-3.0 terms, including providing the source.

- **The `headless` feature flag** (`cargo build --features headless` or
  `--no-default-features`) builds `parkhub-server` without Slint, removing
  the GPL dependency entirely for the server binary.

- **Commercial option**: Slint offers a commercial license
  (https://slint.dev/pricing) that removes the GPL restriction if needed.

**Recommended action**: Document in your README that the `gui` feature (default)
produces a GPL-3.0 binary, and the `headless` feature produces a pure MIT
binary. The `parkhub-client` binary is always GPL-3.0 unless a commercial
Slint license is obtained.

### typescript (Apache-2.0)

TypeScript is Apache-2.0 licensed. Apache-2.0 is compatible with MIT for
open-source distribution (users can use the combined work under either license).

---

## License Compatibility Conclusion

| Category | Status |
|----------|--------|
| Rust backend (headless) | All MIT/Apache-2.0/BSD/ISC — fully compatible |
| Rust backend (gui feature) | Contains GPL-3.0 (slint) — binary is GPL-3.0 |
| Rust client | Contains GPL-3.0 (slint) — binary is GPL-3.0 |
| npm runtime | All MIT — fully compatible |
| npm devDependencies | All MIT/Apache-2.0 — fully compatible |
| E2E tests | Apache-2.0 (playwright) — compatible |

For open-source publication on GitHub, the recommended approach is:

1. Publish source under MIT as declared.
2. Note in README that binaries built with the default `gui` feature are
   governed by GPL-3.0 due to the Slint dependency.
3. Offer a `headless` server build path that is purely MIT-licensed.
