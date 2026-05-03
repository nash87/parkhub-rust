//! `ParkHub` Desktop — Tauri 2 shell.
//!
//! Wraps the `parkhub-web` frontend (React 19 + Astro 6 + Tailwind 4) inside a
//! native window via Tauri's system-`WebView` backend (`WKWebView` on macOS,
//! `WebView2` on Windows, `WebKitGTK` on Linux). Backend-agnostic: the user
//! configures a `ParkHub` server URL (Rust or Laravel edition) on first run;
//! everything else is the same HTTP/WebSocket contract the web UI already
//! speaks.
//!
//! Why Tauri over Slint here:
//!  - Single UI codebase across web + PWA + desktop + mobile (iOS/Android
//!    arrive for free via Tauri 2's mobile targets).
//!  - MIT licensed — no commercial royalty question.
//!  - ~8 MB bundle per platform vs Electron's ~80 MB.
//!  - System `WebView` is security-patched by the OS, not by us.
//!
//! The Slint `parkhub-client` crate stays alongside this one permanently
//! — two distinct desktop experiences sharing the same backend contract.
//! `parkhub-client` is the pure-Rust GPU-native option; `parkhub-desktop` is
//! the web-UI-parity option. See `parkhub-desktop/README.md` for the
//! side-by-side comparison.

use tauri::Manager;

/// Greet command — placeholder used for the first-run handshake between
/// the web frontend and the native shell. Will be extended with:
///  - `save_server_url(url: String)` → persist via tauri-plugin-store
///  - `load_server_url() -> Option<String>` → restore on next launch
///  - `open_external(url: String)` → delegate to tauri-plugin-opener for
///    deep links and external docs
///
/// Kept minimal for the initial landing PR so reviewers can see the
/// shape without reading hundreds of lines of business logic.
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![ping])
        .setup(|_app| {
            // Tracing setup — mirrors parkhub-server so log events from
            // native commands surface through the same formatters when
            // the user runs with RUST_LOG set.
            #[cfg(debug_assertions)]
            {
                let window = _app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
