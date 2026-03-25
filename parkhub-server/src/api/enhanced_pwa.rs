//! Enhanced PWA — Dynamic manifest and offline data support.
//!
//! Extends the basic PWA module with branding-aware manifest generation
//! and minimal offline data for cached booking display.
//!
//! Endpoints:
//! - `GET /api/v1/pwa/manifest` — dynamic manifest based on branding settings
//! - `GET /api/v1/pwa/offline-data` — minimal data for offline mode

use axum::{
    extract::State,
    http::header,
    response::IntoResponse,
    Extension, Json,
};
use serde::Serialize;

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Dynamic PWA manifest response.
#[derive(Debug, Serialize)]
pub struct PwaManifest {
    pub name: String,
    pub short_name: String,
    pub start_url: String,
    pub display: String,
    pub background_color: String,
    pub theme_color: String,
    pub orientation: String,
    pub scope: String,
    pub icons: Vec<PwaIcon>,
    pub categories: Vec<String>,
    pub description: String,
    pub lang: String,
    pub dir: String,
}

/// PWA icon entry.
#[derive(Debug, Serialize)]
pub struct PwaIcon {
    pub src: String,
    pub sizes: String,
    #[serde(rename = "type")]
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

/// Minimal offline data for the mobile experience.
#[derive(Debug, Serialize)]
pub struct OfflineData {
    pub next_booking: Option<OfflineBooking>,
    pub lot_info: Vec<OfflineLot>,
    pub cached_at: String,
}

/// Minimal booking info for offline display.
#[derive(Debug, Serialize)]
pub struct OfflineBooking {
    pub id: String,
    pub lot_name: String,
    pub slot_label: String,
    pub date: String,
    pub start_time: String,
    pub end_time: String,
}

/// Minimal lot info for offline display.
#[derive(Debug, Serialize)]
pub struct OfflineLot {
    pub id: String,
    pub name: String,
    pub total_slots: u32,
    pub available_slots: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/pwa/manifest` — dynamic manifest based on branding settings.
pub async fn pwa_dynamic_manifest(State(state): State<SharedState>) -> impl IntoResponse {
    let state_guard = state.read().await;

    let app_name = state_guard
        .db
        .get_setting("branding_app_name")
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "ParkHub".to_string());

    let theme_color = state_guard
        .db
        .get_setting("branding_primary_color")
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "#2563eb".to_string());

    let bg_color = state_guard
        .db
        .get_setting("branding_bg_color")
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "#ffffff".to_string());

    let description = state_guard
        .db
        .get_setting("branding_description")
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Self-hosted parking management".to_string());

    let lang = state_guard
        .db
        .get_setting("default_language")
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "en".to_string());

    drop(state_guard);

    let manifest = PwaManifest {
        name: app_name.clone(),
        short_name: if app_name.len() > 12 {
            app_name[..12].to_string()
        } else {
            app_name
        },
        start_url: "/".to_string(),
        display: "standalone".to_string(),
        background_color: bg_color,
        theme_color,
        orientation: "any".to_string(),
        scope: "/".to_string(),
        icons: vec![
            PwaIcon {
                src: "/icons/icon-192.png".to_string(),
                sizes: "192x192".to_string(),
                mime_type: "image/png".to_string(),
                purpose: Some("any maskable".to_string()),
            },
            PwaIcon {
                src: "/icons/icon-512.png".to_string(),
                sizes: "512x512".to_string(),
                mime_type: "image/png".to_string(),
                purpose: Some("any maskable".to_string()),
            },
        ],
        categories: vec!["business".to_string(), "productivity".to_string()],
        description,
        lang,
        dir: "ltr".to_string(),
    };

    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        Json(manifest),
    )
}

/// `GET /api/v1/pwa/offline-data` — minimal data for offline mode.
///
/// Requires authentication. Returns the user's next upcoming booking
/// and basic lot availability info for cached display.
pub async fn pwa_offline_data(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<OfflineData>> {
    let state_guard = state.read().await;

    // Find user's next upcoming booking
    let next_booking = find_next_booking(&state_guard, &auth_user).await;

    // Get basic lot info
    let lot_info = get_lot_summaries(&state_guard).await;

    drop(state_guard);

    Json(ApiResponse::success(OfflineData {
        next_booking,
        lot_info,
        cached_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// Enhanced service worker with offline caching strategy.
pub async fn enhanced_service_worker() -> impl IntoResponse {
    let sw_js = r#"const CACHE_NAME = 'parkhub-v2';
const STATIC_ASSETS = ['/', '/manifest.json'];
const API_CACHE = 'parkhub-api-v1';

// Install: cache static assets
self.addEventListener('install', e => {
  e.waitUntil(
    caches.open(CACHE_NAME)
      .then(cache => cache.addAll(STATIC_ASSETS))
      .then(() => self.skipWaiting())
  );
});

// Activate: clean old caches
self.addEventListener('activate', e => {
  e.waitUntil(
    caches.keys().then(keys =>
      Promise.all(keys.filter(k => k !== CACHE_NAME && k !== API_CACHE).map(k => caches.delete(k)))
    ).then(() => self.clients.claim())
  );
});

// Fetch: network-first for API, cache-first for static
self.addEventListener('fetch', e => {
  if (e.request.method !== 'GET') return;
  const url = new URL(e.request.url);

  if (url.pathname.startsWith('/api/')) {
    // Network-first for API calls
    e.respondWith(
      fetch(e.request)
        .then(resp => {
          if (resp.ok && url.pathname.includes('/pwa/offline-data')) {
            const clone = resp.clone();
            caches.open(API_CACHE).then(cache => cache.put(e.request, clone));
          }
          return resp;
        })
        .catch(() => caches.match(e.request))
    );
  } else {
    // Cache-first for static assets
    e.respondWith(
      caches.match(e.request).then(cached => cached || fetch(e.request))
    );
  }
});

// Background sync for offline bookings (future)
self.addEventListener('message', e => {
  if (e.data && e.data.type === 'SKIP_WAITING') {
    self.skipWaiting();
  }
});
"#;

    ([(header::CONTENT_TYPE, "application/javascript")], sw_js)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn find_next_booking(
    state: &crate::AppState,
    auth_user: &AuthUser,
) -> Option<OfflineBooking> {
    let bookings = state.db.list_bookings().await.ok()?;
    let now = chrono::Utc::now();
    let user_id = auth_user.user_id.to_string();

    bookings
        .into_iter()
        .filter(|b| b.user_id.to_string() == user_id && b.end_time > now)
        .min_by_key(|b| b.start_time)
        .map(|b| {
            let lot_name = b.lot_id.to_string(); // Simplified — full lot name lookup would require lot table
            OfflineBooking {
                id: b.id.to_string(),
                lot_name,
                slot_label: b.slot_id.to_string(),
                date: b.start_time.format("%Y-%m-%d").to_string(),
                start_time: b.start_time.format("%H:%M").to_string(),
                end_time: b.end_time.format("%H:%M").to_string(),
            }
        })
}

async fn get_lot_summaries(state: &crate::AppState) -> Vec<OfflineLot> {
    let lots = match state.db.list_parking_lots().await {
        Ok(l) => l,
        Err(_) => return Vec::new(),
    };

    let mut summaries = Vec::new();
    for lot in &lots {
        let total = match state.db.list_slots_by_lot(&lot.id.to_string()).await {
            Ok(slots) => slots.len(),
            Err(_) => 0,
        };
        // Simplified available count — would need booking intersection check for accuracy
        summaries.push(OfflineLot {
            id: lot.id.to_string(),
            name: lot.name.clone(),
            total_slots: total as u32,
            available_slots: total as u32, // Simplified
        });
    }
    summaries
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pwa_manifest_short_name_truncation() {
        let long_name = "A Very Long Application Name That Exceeds Twelve Characters";
        let short = if long_name.len() > 12 {
            long_name[..12].to_string()
        } else {
            long_name.to_string()
        };
        assert_eq!(short.len(), 12);
    }

    #[test]
    fn test_pwa_manifest_serialization() {
        let manifest = PwaManifest {
            name: "ParkHub".to_string(),
            short_name: "ParkHub".to_string(),
            start_url: "/".to_string(),
            display: "standalone".to_string(),
            background_color: "#ffffff".to_string(),
            theme_color: "#2563eb".to_string(),
            orientation: "any".to_string(),
            scope: "/".to_string(),
            icons: vec![PwaIcon {
                src: "/icons/icon-192.png".to_string(),
                sizes: "192x192".to_string(),
                mime_type: "image/png".to_string(),
                purpose: Some("any maskable".to_string()),
            }],
            categories: vec!["business".to_string()],
            description: "Parking management".to_string(),
            lang: "en".to_string(),
            dir: "ltr".to_string(),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("ParkHub"));
        assert!(json.contains("standalone"));
        assert!(json.contains("icon-192.png"));
    }

    #[test]
    fn test_offline_data_serialization() {
        let data = OfflineData {
            next_booking: Some(OfflineBooking {
                id: "b-001".to_string(),
                lot_name: "Main Lot".to_string(),
                slot_label: "A-12".to_string(),
                date: "2026-03-24".to_string(),
                start_time: "08:00".to_string(),
                end_time: "17:00".to_string(),
            }),
            lot_info: vec![OfflineLot {
                id: "lot-1".to_string(),
                name: "Main Lot".to_string(),
                total_slots: 50,
                available_slots: 23,
            }],
            cached_at: "2026-03-23T14:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("Main Lot"));
        assert!(json.contains("A-12"));
        assert!(json.contains("2026-03-24"));
    }

    #[test]
    fn test_offline_data_no_booking() {
        let data = OfflineData {
            next_booking: None,
            lot_info: Vec::new(),
            cached_at: "2026-03-23T14:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("null"));
    }

    #[test]
    fn test_pwa_icon_purpose_skip() {
        let icon = PwaIcon {
            src: "/icon.png".to_string(),
            sizes: "512x512".to_string(),
            mime_type: "image/png".to_string(),
            purpose: None,
        };
        let json = serde_json::to_string(&icon).unwrap();
        assert!(!json.contains("purpose")); // skip_serializing_if
    }
}
