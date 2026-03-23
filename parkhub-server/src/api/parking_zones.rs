//! Parking Zones with Pricing Tiers — zone-based pricing management.
//!
//! Extends the base zone system with pricing tiers (economy, standard, premium, vip),
//! multipliers, and max capacity per zone.
//!
//! Endpoints:
//! - `GET  /api/v1/lots/{id}/zones/pricing`       — list zones with pricing tiers
//! - `PUT  /api/v1/admin/zones/{id}/pricing`       — set zone pricing tier
//! - `GET  /api/v1/zones/{id}/price`               — calculate price for a zone

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, UserRole};

use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Available pricing tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PricingTier {
    Economy,
    Standard,
    Premium,
    Vip,
}

impl PricingTier {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Economy => "Economy",
            Self::Standard => "Standard",
            Self::Premium => "Premium",
            Self::Vip => "VIP",
        }
    }

    pub const fn default_multiplier(self) -> f64 {
        match self {
            Self::Economy => 0.8,
            Self::Standard => 1.0,
            Self::Premium => 1.5,
            Self::Vip => 2.5,
        }
    }

    pub const fn default_color(self) -> &'static str {
        match self {
            Self::Economy => "#22c55e",   // green
            Self::Standard => "#3b82f6",  // blue
            Self::Premium => "#eab308",   // gold
            Self::Vip => "#a855f7",       // purple
        }
    }
}

/// Zone pricing configuration stored alongside zone data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZonePricing {
    pub zone_id: Uuid,
    pub tier: PricingTier,
    pub pricing_multiplier: f64,
    pub max_capacity: Option<u32>,
    pub updated_at: DateTime<Utc>,
}

/// Combined zone info with pricing tier for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneWithPricing {
    pub id: Uuid,
    pub lot_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub tier: PricingTier,
    pub tier_display: String,
    pub tier_color: String,
    pub pricing_multiplier: f64,
    pub max_capacity: Option<u32>,
}

/// Calculated price for a zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZonePriceResponse {
    pub zone_id: Uuid,
    pub tier: PricingTier,
    pub base_price: f64,
    pub multiplier: f64,
    pub final_price: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetZonePricingRequest {
    pub tier: PricingTier,
    pub pricing_multiplier: Option<f64>,
    pub max_capacity: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings storage
// ─────────────────────────────────────────────────────────────────────────────

const ZONE_PRICING_KEY: &str = "zone_pricing";

async fn load_pricing(state: &crate::AppState) -> Vec<ZonePricing> {
    match state.db.get_setting(ZONE_PRICING_KEY).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => vec![],
    }
}

async fn save_pricing(state: &crate::AppState, pricing: &[ZonePricing]) -> Result<(), String> {
    let json = serde_json::to_string(pricing).map_err(|e| e.to_string())?;
    state.db.set_setting(ZONE_PRICING_KEY, &json).await.map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/{id}/zones/pricing` — list zones with pricing tiers.
#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}/zones/pricing",
    tag = "Parking Zones",
    summary = "List zones with pricing tiers",
    description = "Returns all zones for a lot including their pricing tier, multiplier, and capacity.",
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Zones with pricing"),
    )
)]
pub async fn list_zones_pricing(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
) -> Json<ApiResponse<Vec<ZoneWithPricing>>> {
    let state_guard = state.read().await;

    let zones = match state_guard.db.list_zones_by_lot(&lot_id).await {
        Ok(z) => z,
        Err(e) => {
            tracing::error!("Failed to list zones: {e}");
            return Json(ApiResponse::error("SERVER_ERROR", "Failed to list zones"));
        }
    };

    let pricing = load_pricing(&state_guard).await;

    let result: Vec<ZoneWithPricing> = zones
        .into_iter()
        .map(|z| {
            let zone_pricing = pricing.iter().find(|p| p.zone_id == z.id);
            let tier = zone_pricing.map_or(PricingTier::Standard, |p| p.tier);
            let multiplier = zone_pricing.map_or(tier.default_multiplier(), |p| p.pricing_multiplier);
            let capacity = zone_pricing.and_then(|p| p.max_capacity);

            ZoneWithPricing {
                id: z.id,
                lot_id: z.lot_id,
                name: z.name,
                description: z.description,
                color: z.color.or_else(|| Some(tier.default_color().to_string())),
                tier,
                tier_display: tier.display_name().to_string(),
                tier_color: tier.default_color().to_string(),
                pricing_multiplier: multiplier,
                max_capacity: capacity,
            }
        })
        .collect();

    Json(ApiResponse::success(result))
}

/// `PUT /api/v1/admin/zones/{id}/pricing` — set zone pricing tier.
#[utoipa::path(
    put,
    path = "/api/v1/admin/zones/{id}/pricing",
    tag = "Parking Zones",
    summary = "Set zone pricing tier",
    description = "Configure the pricing tier, multiplier, and capacity for a zone.",
    params(("id" = String, Path, description = "Zone ID")),
    request_body = SetZonePricingRequest,
    responses(
        (status = 200, description = "Pricing updated"),
        (status = 400, description = "Invalid zone ID"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn set_zone_pricing(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(zone_id): Path<String>,
    Json(req): Json<SetZonePricingRequest>,
) -> (StatusCode, Json<ApiResponse<ZonePricing>>) {
    let state_guard = state.read().await;

    // Admin check
    match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    let id = match Uuid::parse_str(&zone_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid zone ID")),
            );
        }
    };

    let multiplier = req.pricing_multiplier.unwrap_or_else(|| req.tier.default_multiplier());

    if multiplier <= 0.0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_MULTIPLIER", "Pricing multiplier must be positive")),
        );
    }

    let zone_pricing = ZonePricing {
        zone_id: id,
        tier: req.tier,
        pricing_multiplier: multiplier,
        max_capacity: req.max_capacity,
        updated_at: Utc::now(),
    };

    let mut all_pricing = load_pricing(&state_guard).await;
    all_pricing.retain(|p| p.zone_id != id);
    all_pricing.push(zone_pricing.clone());

    if let Err(e) = save_pricing(&state_guard, &all_pricing).await {
        tracing::error!("Failed to save pricing: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save pricing")),
        );
    }

    // Audit
    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("zone_pricing_set:{}:{:?}", id, req.tier))
        .log()
        .persist(&state_guard.db)
        .await;

    (StatusCode::OK, Json(ApiResponse::success(zone_pricing)))
}

/// `GET /api/v1/zones/{id}/price` — calculate price for a zone.
#[utoipa::path(
    get,
    path = "/api/v1/zones/{id}/price",
    tag = "Parking Zones",
    summary = "Calculate zone price",
    description = "Returns the calculated price for a zone based on its tier and pricing multiplier.",
    params(("id" = String, Path, description = "Zone ID")),
    responses(
        (status = 200, description = "Zone price"),
        (status = 404, description = "Zone not found"),
    )
)]
pub async fn get_zone_price(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(zone_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ZonePriceResponse>>) {
    let state_guard = state.read().await;

    let id = match Uuid::parse_str(&zone_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid zone ID")),
            );
        }
    };

    let pricing = load_pricing(&state_guard).await;
    let zone_pricing = pricing.iter().find(|p| p.zone_id == id);

    let tier = zone_pricing.map_or(PricingTier::Standard, |p| p.tier);
    let multiplier = zone_pricing.map_or(tier.default_multiplier(), |p| p.pricing_multiplier);

    // Default base price (lot-level pricing would override this in production)
    let base_price = 5.0;
    let final_price = base_price * multiplier;

    (
        StatusCode::OK,
        Json(ApiResponse::success(ZonePriceResponse {
            zone_id: id,
            tier,
            base_price,
            multiplier,
            final_price,
        })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_tier_display_names() {
        assert_eq!(PricingTier::Economy.display_name(), "Economy");
        assert_eq!(PricingTier::Standard.display_name(), "Standard");
        assert_eq!(PricingTier::Premium.display_name(), "Premium");
        assert_eq!(PricingTier::Vip.display_name(), "VIP");
    }

    #[test]
    fn test_pricing_tier_default_multipliers() {
        assert!((PricingTier::Economy.default_multiplier() - 0.8).abs() < f64::EPSILON);
        assert!((PricingTier::Standard.default_multiplier() - 1.0).abs() < f64::EPSILON);
        assert!((PricingTier::Premium.default_multiplier() - 1.5).abs() < f64::EPSILON);
        assert!((PricingTier::Vip.default_multiplier() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pricing_tier_colors() {
        assert_eq!(PricingTier::Economy.default_color(), "#22c55e");
        assert_eq!(PricingTier::Standard.default_color(), "#3b82f6");
        assert_eq!(PricingTier::Premium.default_color(), "#eab308");
        assert_eq!(PricingTier::Vip.default_color(), "#a855f7");
    }

    #[test]
    fn test_pricing_tier_serialization() {
        let json = serde_json::to_string(&PricingTier::Vip).unwrap();
        assert_eq!(json, "\"vip\"");
        let de: PricingTier = serde_json::from_str("\"premium\"").unwrap();
        assert_eq!(de, PricingTier::Premium);
    }

    #[test]
    fn test_zone_pricing_serialization() {
        let pricing = ZonePricing {
            zone_id: Uuid::new_v4(),
            tier: PricingTier::Premium,
            pricing_multiplier: 1.5,
            max_capacity: Some(50),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&pricing).unwrap();
        let de: ZonePricing = serde_json::from_str(&json).unwrap();
        assert_eq!(de.tier, PricingTier::Premium);
        assert!((de.pricing_multiplier - 1.5).abs() < f64::EPSILON);
        assert_eq!(de.max_capacity, Some(50));
    }

    #[test]
    fn test_zone_with_pricing_serialization() {
        let zone = ZoneWithPricing {
            id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            name: "VIP Area".to_string(),
            description: Some("Top floor".to_string()),
            color: Some("#a855f7".to_string()),
            tier: PricingTier::Vip,
            tier_display: "VIP".to_string(),
            tier_color: "#a855f7".to_string(),
            pricing_multiplier: 2.5,
            max_capacity: Some(20),
        };
        let json = serde_json::to_string(&zone).unwrap();
        assert!(json.contains("VIP Area"));
        assert!(json.contains("\"vip\""));
    }

    #[test]
    fn test_zone_price_response() {
        let resp = ZonePriceResponse {
            zone_id: Uuid::new_v4(),
            tier: PricingTier::Economy,
            base_price: 5.0,
            multiplier: 0.8,
            final_price: 4.0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let de: ZonePriceResponse = serde_json::from_str(&json).unwrap();
        assert!((de.final_price - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_zone_pricing_request() {
        let json = r#"{"tier":"premium","pricing_multiplier":1.75,"max_capacity":100}"#;
        let req: SetZonePricingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tier, PricingTier::Premium);
        assert!((req.pricing_multiplier.unwrap() - 1.75).abs() < f64::EPSILON);
        assert_eq!(req.max_capacity, Some(100));
    }

    #[test]
    fn test_set_zone_pricing_minimal() {
        let json = r#"{"tier":"economy"}"#;
        let req: SetZonePricingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tier, PricingTier::Economy);
        assert!(req.pricing_multiplier.is_none());
        assert!(req.max_capacity.is_none());
    }

    #[test]
    fn test_all_tiers_have_different_colors() {
        let colors = vec![
            PricingTier::Economy.default_color(),
            PricingTier::Standard.default_color(),
            PricingTier::Premium.default_color(),
            PricingTier::Vip.default_color(),
        ];
        for (i, c1) in colors.iter().enumerate() {
            for (j, c2) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(c1, c2);
                }
            }
        }
    }

    #[test]
    fn test_multiplier_ordering() {
        assert!(PricingTier::Economy.default_multiplier() < PricingTier::Standard.default_multiplier());
        assert!(PricingTier::Standard.default_multiplier() < PricingTier::Premium.default_multiplier());
        assert!(PricingTier::Premium.default_multiplier() < PricingTier::Vip.default_multiplier());
    }
}
