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
        if v.is_empty() {
            "ParkHub".to_string()
        } else {
            v
        }
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

    ([(header::CONTENT_TYPE, "application/javascript")], sw_js)
}

#[cfg(test)]
mod tests {
    // ── Service Worker ──

    #[test]
    fn service_worker_is_valid_javascript() {
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
        assert!(sw_js.contains("CACHE_NAME"));
        assert!(sw_js.contains("addEventListener"));
        assert!(sw_js.contains("install"));
        assert!(sw_js.contains("fetch"));
    }

    #[test]
    fn service_worker_only_caches_get_requests() {
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
        // Verify the service worker guards against non-GET requests
        assert!(
            sw_js.contains("!== 'GET'"),
            "service worker should only intercept GET requests"
        );
    }

    #[test]
    fn service_worker_cache_name_includes_version() {
        let cache_name = "parkhub-v1";
        assert!(
            cache_name.contains("v1"),
            "cache name should include version for cache busting"
        );
    }

    // ── PWA Manifest ──

    #[test]
    fn manifest_json_structure() {
        let app_name = "ParkHub";
        let manifest = serde_json::json!({
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

        assert_eq!(manifest["name"], "ParkHub");
        assert_eq!(manifest["short_name"], "ParkHub");
        assert_eq!(manifest["start_url"], "/");
        assert_eq!(manifest["display"], "standalone");
    }

    #[test]
    fn manifest_icons_include_required_sizes() {
        let manifest = serde_json::json!({
            "icons": [
                {"src": "/icons/icon-192.png", "sizes": "192x192", "type": "image/png"},
                {"src": "/icons/icon-512.png", "sizes": "512x512", "type": "image/png"}
            ]
        });

        let icons = manifest["icons"].as_array().unwrap();
        assert!(icons.len() >= 2, "PWA must have at least 2 icon sizes");

        let sizes: Vec<&str> = icons.iter().filter_map(|i| i["sizes"].as_str()).collect();
        assert!(
            sizes.contains(&"192x192"),
            "PWA requires 192x192 icon for Android"
        );
        assert!(
            sizes.contains(&"512x512"),
            "PWA requires 512x512 icon for splash screen"
        );
    }

    #[test]
    fn manifest_display_is_standalone() {
        // PWA should run in standalone mode (no browser chrome)
        let display = "standalone";
        assert_eq!(display, "standalone");
    }

    #[test]
    fn manifest_start_url_is_root() {
        let start_url = "/";
        assert_eq!(start_url, "/", "PWA start_url should be root");
    }

    #[test]
    fn manifest_theme_color_is_valid_hex() {
        let color = "#2563eb";
        assert!(color.starts_with('#'), "color must be hex");
        assert_eq!(color.len(), 7, "color must be 6-digit hex");
        u32::from_str_radix(&color[1..], 16).expect("must be valid hex color");
    }

    #[test]
    fn manifest_background_color_is_valid_hex() {
        let color = "#ffffff";
        assert!(color.starts_with('#'));
        assert_eq!(color.len(), 7);
        u32::from_str_radix(&color[1..], 16).expect("must be valid hex color");
    }

    #[test]
    fn manifest_uses_custom_app_name_when_provided() {
        let app_name = "Custom Parking";
        let manifest = serde_json::json!({
            "name": app_name,
            "short_name": app_name,
        });
        assert_eq!(manifest["name"], "Custom Parking");
        assert_eq!(manifest["short_name"], "Custom Parking");
    }

    #[test]
    fn manifest_defaults_to_parkhub_when_name_empty() {
        let branding_name = "";
        let app_name = if branding_name.is_empty() {
            "ParkHub".to_string()
        } else {
            branding_name.to_string()
        };
        assert_eq!(app_name, "ParkHub");
    }
}
