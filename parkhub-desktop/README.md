# parkhub-desktop

Tauri 2 shell that ships the parkhub-web React/Astro UI as a native desktop (and mobile) application. Backend-agnostic: connects via HTTP/WebSocket to a ParkHub server — Rust (Axum) or Laravel edition — configured on first run.

## Why Tauri

See `src/lib.rs` header and [Plans/2026-04-18-parkhub-release-desktop-and-security.md](../../Obsidian/Forge/Plans/2026-04-18-parkhub-release-desktop-and-security.md) for the decision record. Short version: one UI codebase across web + PWA + desktop + mobile, MIT licensed, ~8 MB bundle, system WebView is OS-security-patched.

## Dev workflow

**Prerequisites**: Rust toolchain, Node 22+, the Tauri 2 system deps for your platform (see <https://v2.tauri.app/start/prerequisites/>).

```bash
# From parkhub-rust/ root. The Astro dev server + Tauri window start together.
cargo tauri dev -p parkhub-desktop
```

## Production build (per platform)

```bash
cargo tauri build -p parkhub-desktop
```

Outputs land in `target/release/bundle/` — `.dmg` / `.app` on macOS, `.msi` / `.exe` installer on Windows, `.deb` / `.AppImage` / `.rpm` on Linux.

## Status

Initial Tauri 2.1 scaffold. Not yet feature-complete; tracking to Slint-client parity in **T-1844**:

- [x] Tauri shell wraps parkhub-web/dist
- [x] Capability allowlist (minimum surface: http + fs + store + notification + dialog + deep-link)
- [x] Ping command (handshake placeholder)
- [ ] First-run server-URL picker (store-backed, persists across launches)
- [ ] Deep-link handler: `parkhub://book?slot=L2-14` → navigate in-app
- [ ] System-tray integration (optional; disabled by default)
- [ ] macOS universal binary + Linux ARM64 + Windows installer in release.yml

## Coexistence with parkhub-client (Slint)

The Slint-based `parkhub-client` crate stays in the workspace through this migration. Once parkhub-desktop reaches parity (T-1844) and the v4.15 release ships both without regressions, parkhub-client is deprecated and removed in v4.16 — tracked as **T-1848**.

## Icons

This scaffold assumes you drop generated icons under `icons/` (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`). Use `cargo tauri icon <source.png>` to generate the full set from a single source.
