//! Dynamic pricing handlers: occupancy-based surge/discount pricing per lot.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use parkhub_common::{ApiResponse, DynamicPriceResult, DynamicPricingRules};

use super::SharedState;

/// Settings key prefix for storing dynamic pricing rules per lot.
const SETTINGS_KEY_PREFIX: &str = "dynamic_pricing:";

/// Build the settings key for a lot's dynamic pricing rules.
fn pricing_key(lot_id: &str) -> String {
    format!("{SETTINGS_KEY_PREFIX}{lot_id}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/lots/{id}/pricing/dynamic` — returns current dynamic price for a lot.
///
/// Calculates the effective price based on current occupancy and the lot's
/// dynamic pricing rules. If no rules are configured, returns a result with
/// `dynamic_pricing_active: false` and the lot's base hourly rate.
#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}/pricing/dynamic",
    tag = "Dynamic Pricing",
    summary = "Get current dynamic price for a lot",
    description = "Returns the current effective price based on lot occupancy and dynamic pricing rules.",
    params(("id" = String, Path, description = "Parking lot ID")),
    responses(
        (status = 200, description = "Current dynamic price"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn get_dynamic_pricing(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<DynamicPriceResult>>) {
    let state = state.read().await;

    // Fetch the lot
    let lot = match state.db.get_parking_lot(&id).await {
        Ok(Some(l)) => l,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get parking lot");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Load dynamic pricing rules from settings
    let rules = match state.db.get_setting(&pricing_key(&id)).await {
        Ok(Some(json)) => serde_json::from_str::<DynamicPricingRules>(&json).unwrap_or_default(),
        _ => DynamicPricingRules::default(),
    };

    // Calculate occupancy
    #[allow(clippy::cast_precision_loss)]
    let occupancy_percent = if lot.total_slots > 0 {
        let occupied = lot.total_slots - lot.available_slots;
        (f64::from(occupied) / f64::from(lot.total_slots)) * 100.0
    } else {
        0.0
    };

    let result = calculate_dynamic_price(&rules, occupancy_percent, &lot.pricing.currency);

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

/// `GET /api/v1/admin/lots/{id}/pricing/dynamic` — admin: get dynamic pricing rules for a lot.
#[utoipa::path(
    get,
    path = "/api/v1/admin/lots/{id}/pricing/dynamic",
    tag = "Dynamic Pricing",
    summary = "Get dynamic pricing rules (admin)",
    description = "Returns the configured dynamic pricing rules for a lot.",
    params(("id" = String, Path, description = "Parking lot ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Dynamic pricing rules"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn admin_get_dynamic_pricing_rules(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<DynamicPricingRules>>) {
    let state = state.read().await;

    // Verify lot exists
    match state.db.get_parking_lot(&id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get parking lot");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    }

    let rules = match state.db.get_setting(&pricing_key(&id)).await {
        Ok(Some(json)) => serde_json::from_str::<DynamicPricingRules>(&json).unwrap_or_default(),
        _ => DynamicPricingRules::default(),
    };

    (StatusCode::OK, Json(ApiResponse::success(rules)))
}

/// Request body for updating dynamic pricing rules.
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateDynamicPricingRequest {
    /// Enable or disable dynamic pricing
    pub enabled: Option<bool>,
    /// Base hourly rate before multipliers
    pub base_price: Option<f64>,
    /// Multiplier applied when occupancy exceeds `surge_threshold`
    pub surge_multiplier: Option<f64>,
    /// Multiplier applied when occupancy is below `discount_threshold`
    pub discount_multiplier: Option<f64>,
    /// Occupancy percentage (0-100) that triggers surge pricing
    pub surge_threshold: Option<f64>,
    /// Occupancy percentage (0-100) below which discount pricing activates
    pub discount_threshold: Option<f64>,
}

/// `PUT /api/v1/admin/lots/{id}/pricing/dynamic` — admin: set dynamic pricing rules.
#[utoipa::path(
    put,
    path = "/api/v1/admin/lots/{id}/pricing/dynamic",
    tag = "Dynamic Pricing",
    summary = "Update dynamic pricing rules (admin)",
    description = "Set the dynamic pricing rules for a lot. All fields are optional — only provided fields are updated.",
    params(("id" = String, Path, description = "Parking lot ID")),
    request_body = UpdateDynamicPricingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Updated dynamic pricing rules"),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Parking lot not found"),
    )
)]
pub async fn admin_update_dynamic_pricing_rules(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateDynamicPricingRequest>,
) -> (StatusCode, Json<ApiResponse<DynamicPricingRules>>) {
    let state = state.read().await;

    // Verify lot exists
    match state.db.get_parking_lot(&id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get parking lot");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    }

    // Load existing rules or start with defaults
    let mut rules = match state.db.get_setting(&pricing_key(&id)).await {
        Ok(Some(json)) => serde_json::from_str::<DynamicPricingRules>(&json).unwrap_or_default(),
        _ => DynamicPricingRules::default(),
    };

    // Apply partial updates
    if let Some(enabled) = req.enabled {
        rules.enabled = enabled;
    }
    if let Some(base_price) = req.base_price {
        if base_price < 0.0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "base_price must be >= 0",
                )),
            );
        }
        rules.base_price = base_price;
    }
    if let Some(surge_multiplier) = req.surge_multiplier {
        if surge_multiplier < 1.0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "surge_multiplier must be >= 1.0",
                )),
            );
        }
        rules.surge_multiplier = surge_multiplier;
    }
    if let Some(discount_multiplier) = req.discount_multiplier {
        if discount_multiplier <= 0.0 || discount_multiplier > 1.0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "discount_multiplier must be > 0 and <= 1.0",
                )),
            );
        }
        rules.discount_multiplier = discount_multiplier;
    }
    if let Some(surge_threshold) = req.surge_threshold {
        if !(0.0..=100.0).contains(&surge_threshold) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "surge_threshold must be 0-100",
                )),
            );
        }
        rules.surge_threshold = surge_threshold;
    }
    if let Some(discount_threshold) = req.discount_threshold {
        if !(0.0..=100.0).contains(&discount_threshold) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "discount_threshold must be 0-100",
                )),
            );
        }
        rules.discount_threshold = discount_threshold;
    }

    // Validate thresholds don't overlap
    if rules.discount_threshold >= rules.surge_threshold {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "discount_threshold must be less than surge_threshold",
            )),
        );
    }

    // Persist
    let json = serde_json::to_string(&rules).unwrap_or_default();
    if let Err(e) = state.db.set_setting(&pricing_key(&id), &json).await {
        tracing::error!(error = %e, "Failed to save dynamic pricing rules");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save dynamic pricing rules",
            )),
        );
    }

    tracing::info!(lot_id = %id, "Updated dynamic pricing rules");
    (StatusCode::OK, Json(ApiResponse::success(rules)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Price calculation logic
// ─────────────────────────────────────────────────────────────────────────────

/// Calculate the effective dynamic price for a lot given its rules and current occupancy.
pub fn calculate_dynamic_price(
    rules: &DynamicPricingRules,
    occupancy_percent: f64,
    currency: &str,
) -> DynamicPriceResult {
    if !rules.enabled {
        return DynamicPriceResult {
            current_price: rules.base_price,
            base_price: rules.base_price,
            applied_multiplier: 1.0,
            occupancy_percent,
            dynamic_pricing_active: false,
            tier: "normal".to_string(),
            currency: currency.to_string(),
        };
    }

    let (multiplier, tier) = if occupancy_percent >= rules.surge_threshold {
        (rules.surge_multiplier, "surge")
    } else if occupancy_percent <= rules.discount_threshold {
        (rules.discount_multiplier, "discount")
    } else {
        (1.0, "normal")
    };

    let current_price = (rules.base_price * multiplier * 100.0).round() / 100.0;

    DynamicPriceResult {
        current_price,
        base_price: rules.base_price,
        applied_multiplier: multiplier,
        occupancy_percent,
        dynamic_pricing_active: true,
        tier: tier.to_string(),
        currency: currency.to_string(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_rules() -> DynamicPricingRules {
        DynamicPricingRules {
            enabled: true,
            base_price: 2.50,
            surge_multiplier: 1.5,
            discount_multiplier: 0.8,
            surge_threshold: 80.0,
            discount_threshold: 20.0,
        }
    }

    #[test]
    fn test_normal_pricing_when_disabled() {
        let rules = DynamicPricingRules {
            enabled: false,
            ..default_rules()
        };
        let result = calculate_dynamic_price(&rules, 90.0, "EUR");
        assert!(!result.dynamic_pricing_active);
        assert!((result.current_price - 2.50).abs() < 1e-9);
        assert!((result.applied_multiplier - 1.0).abs() < 1e-9);
        assert_eq!(result.tier, "normal");
    }

    #[test]
    fn test_surge_pricing_above_threshold() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 85.0, "EUR");
        assert!(result.dynamic_pricing_active);
        assert!((result.current_price - 3.75).abs() < 1e-9); // 2.50 * 1.5
        assert!((result.applied_multiplier - 1.5).abs() < 1e-9);
        assert_eq!(result.tier, "surge");
        assert_eq!(result.currency, "EUR");
    }

    #[test]
    fn test_surge_pricing_at_exactly_threshold() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 80.0, "EUR");
        assert_eq!(result.tier, "surge");
        assert!((result.current_price - 3.75).abs() < 1e-9);
    }

    #[test]
    fn test_discount_pricing_below_threshold() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 15.0, "USD");
        assert!(result.dynamic_pricing_active);
        assert!((result.current_price - 2.0).abs() < 1e-9); // 2.50 * 0.8
        assert!((result.applied_multiplier - 0.8).abs() < 1e-9);
        assert_eq!(result.tier, "discount");
        assert_eq!(result.currency, "USD");
    }

    #[test]
    fn test_discount_pricing_at_exactly_threshold() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 20.0, "EUR");
        assert_eq!(result.tier, "discount");
        assert!((result.current_price - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_normal_pricing_between_thresholds() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 50.0, "EUR");
        assert!(result.dynamic_pricing_active);
        assert!((result.current_price - 2.50).abs() < 1e-9);
        assert!((result.applied_multiplier - 1.0).abs() < 1e-9);
        assert_eq!(result.tier, "normal");
    }

    #[test]
    fn test_occupancy_zero_triggers_discount() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 0.0, "EUR");
        assert_eq!(result.tier, "discount");
        assert!((result.current_price - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_occupancy_100_triggers_surge() {
        let rules = default_rules();
        let result = calculate_dynamic_price(&rules, 100.0, "EUR");
        assert_eq!(result.tier, "surge");
        assert!((result.current_price - 3.75).abs() < 1e-9);
    }

    #[test]
    fn test_high_surge_multiplier() {
        let rules = DynamicPricingRules {
            surge_multiplier: 3.0,
            ..default_rules()
        };
        let result = calculate_dynamic_price(&rules, 95.0, "EUR");
        assert!((result.current_price - 7.50).abs() < 1e-9); // 2.50 * 3.0
    }

    #[test]
    fn test_deep_discount() {
        let rules = DynamicPricingRules {
            discount_multiplier: 0.5,
            ..default_rules()
        };
        let result = calculate_dynamic_price(&rules, 10.0, "EUR");
        assert!((result.current_price - 1.25).abs() < 1e-9); // 2.50 * 0.5
    }

    #[test]
    fn test_base_price_zero() {
        let rules = DynamicPricingRules {
            base_price: 0.0,
            ..default_rules()
        };
        let result = calculate_dynamic_price(&rules, 90.0, "EUR");
        assert!((result.current_price).abs() < 1e-9); // 0 * 1.5 = 0
    }

    #[test]
    fn test_result_rounding() {
        let rules = DynamicPricingRules {
            base_price: 3.33,
            surge_multiplier: 1.7,
            ..default_rules()
        };
        let result = calculate_dynamic_price(&rules, 90.0, "EUR");
        // 3.33 * 1.7 = 5.661 → rounded to 5.66
        assert!((result.current_price - 5.66).abs() < 1e-9);
    }

    #[test]
    fn test_default_rules() {
        let rules = DynamicPricingRules::default();
        assert!(!rules.enabled);
        assert!((rules.base_price - 2.50).abs() < 1e-9);
        assert!((rules.surge_multiplier - 1.5).abs() < 1e-9);
        assert!((rules.discount_multiplier - 0.8).abs() < 1e-9);
        assert!((rules.surge_threshold - 80.0).abs() < 1e-9);
        assert!((rules.discount_threshold - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_serde_roundtrip() {
        let rules = default_rules();
        let json = serde_json::to_string(&rules).unwrap();
        let back: DynamicPricingRules = serde_json::from_str(&json).unwrap();
        assert!(back.enabled);
        assert!((back.base_price - 2.50).abs() < 1e-9);
        assert!((back.surge_multiplier - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_update_request_partial_serde() {
        let json = r#"{"base_price": 5.0}"#;
        let req: UpdateDynamicPricingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.base_price, Some(5.0));
        assert!(req.enabled.is_none());
        assert!(req.surge_multiplier.is_none());
        assert!(req.discount_multiplier.is_none());
        assert!(req.surge_threshold.is_none());
        assert!(req.discount_threshold.is_none());
    }

    #[test]
    fn test_update_request_full_serde() {
        let json = r#"{
            "enabled": true,
            "base_price": 3.0,
            "surge_multiplier": 2.0,
            "discount_multiplier": 0.7,
            "surge_threshold": 90.0,
            "discount_threshold": 10.0
        }"#;
        let req: UpdateDynamicPricingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled, Some(true));
        assert_eq!(req.base_price, Some(3.0));
        assert_eq!(req.surge_multiplier, Some(2.0));
        assert_eq!(req.discount_multiplier, Some(0.7));
        assert_eq!(req.surge_threshold, Some(90.0));
        assert_eq!(req.discount_threshold, Some(10.0));
    }

    #[test]
    fn test_update_request_empty_serde() {
        let json = r#"{}"#;
        let req: UpdateDynamicPricingRequest = serde_json::from_str(json).unwrap();
        assert!(req.enabled.is_none());
        assert!(req.base_price.is_none());
    }

    #[test]
    fn test_price_result_serde() {
        let result = DynamicPriceResult {
            current_price: 3.75,
            base_price: 2.50,
            applied_multiplier: 1.5,
            occupancy_percent: 85.0,
            dynamic_pricing_active: true,
            tier: "surge".to_string(),
            currency: "EUR".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: DynamicPriceResult = serde_json::from_str(&json).unwrap();
        assert!((back.current_price - 3.75).abs() < 1e-9);
        assert_eq!(back.tier, "surge");
        assert!(back.dynamic_pricing_active);
    }
}
