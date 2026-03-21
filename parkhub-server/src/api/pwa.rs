//! PWA support — manifest.json and service worker.

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Json},
};
use serde_json::json;

use super::SharedState;

/// `GET /manifest.json` — Web App Manifest for PWA installation.
pub async fn pwa_manifest(State(state): State<SharedState>) -> impl IntoResponse {
    let state_guard = state.read().await;
    let app_name = if let Ok(Some(v)) = state_guard.db.get_setting("branding_app_name").await {
        if v.is_empty() { "ParkHub".to_string() } else { v }
    } else {
        "ParkHub".to_string()
    };
    drop(state_guard);

    let manifest = json!({
        "name": app_name,
        "short_name": app_name,
        "start_url": "/",
        "display": "standalone",
        "background_color": "#ffffff",
        "theme_color": "#2563eb",
        "icons": [
            {"src": "/icons/icon-192.png", "sizes": "192x192", "type": "image/png"},
            {"src": "/icons/icon-512.png", "sizes": "512x512", "type": "image/png"}
        ]
    });

    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        Json(manifest),
    )
}

/// `GET /sw.js` — Service Worker for offline caching.
pub async fn service_worker() -> impl IntoResponse {
    let sw_js = r#"const CACHE_NAME = 'parkhub-v1';
const STATIC_ASSETS = ['/'];
self.addEventListener('install', e => e.waitUntil(
    caches.open(CACHE_NAME).then(cache => cache.addAll(STATIC_ASSETS))
));
self.addEventListener('fetch', e => {
    if (e.request.method !== 'GET') return;
    e.respondWith(fetch(e.request).catch(() => caches.match(e.request)));
});
"#;

    (
        [(header::CONTENT_TYPE, "application/javascript")],
        sw_js,
    )
}
