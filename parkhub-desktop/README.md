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

## Coexistence with parkhub-client (Slint) — both kept, permanently

Two distinct desktop clients ship side-by-side, both fully supported:

| | `parkhub-client` (Slint) | `parkhub-desktop` (Tauri) |
|---|---|---|
| **UI** | Native Rust widgets, GPU-direct rendering via skia/femtovg | System WebView wrapping the parkhub-web React app |
| **Best for** | Embedded / kiosk deployments, low-resource devices, "prefers pure-Rust" users | Standard desktop users who want UX parity with the web / PWA / mobile |
| **Bundle size** | ~15 MB | ~8 MB (shared OS WebView) |
| **Feature cadence** | Per-surface hand-port in Slint DSL | Automatic — every web feature ships in desktop on rebuild |
| **Mobile (iOS/Android)** | No | Yes (Tauri 2 mobile targets) |
| **License surface** | Slint 1.15 dual-GPL/commercial | Tauri 2 MIT |

Both build from the same workspace; both connect to the same ParkHub server contract. Users pick whichever fits.

Future releases (T-1843) add the same per-platform runners for parkhub-desktop that parkhub-client already has: Linux x64 + Linux ARM64 + macOS universal + Windows x64.

## Icons

This scaffold assumes you drop generated icons under `icons/` (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`). Use `cargo tauri icon <source.png>` to generate the full set from a single source.
