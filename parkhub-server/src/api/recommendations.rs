//! Smart parking slot recommendations based on user behavior and availability.

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use parkhub_common::models::{BookingStatus, SlotStatus};
use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

#[derive(Debug, Deserialize)]
pub struct RecommendationQuery {
    pub lot_id: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SlotRecommendation {
    pub slot_id: Uuid,
    pub slot_number: i32,
    pub lot_id: Uuid,
    pub lot_name: String,
    pub floor_name: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// `GET /api/v1/bookings/recommendations` — suggest optimal parking slots
#[utoipa::path(
    get,
    path = "/api/v1/bookings/recommendations",
    tag = "Bookings",
    summary = "Get smart parking recommendations",
    description = "Returns top slot recommendations based on user history, favorites, and availability.",
    params(("lot_id" = Option<String>, Query, description = "Filter by lot")),
    responses(
        (status = 200, description = "Slot recommendations"),
    )
)]
pub(crate) async fn get_recommendations(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<RecommendationQuery>,
) -> Json<ApiResponse<Vec<SlotRecommendation>>> {
    let state = state.read().await;

    // 1. Get user's booking history
    let bookings = match state.db.list_bookings_by_user(&auth_user.user_id.to_string()).await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to load bookings for recommendations: {}", e);
            return Json(ApiResponse::success(vec![]));
        }
    };

    // 2. Count slot usage frequency (completed/active bookings only)
    let mut slot_frequency: HashMap<Uuid, i32> = HashMap::new();
    let mut lot_frequency: HashMap<Uuid, i32> = HashMap::new();
    for b in &bookings {
        if matches!(b.status, BookingStatus::Active | BookingStatus::Completed | BookingStatus::Pending) {
            *slot_frequency.entry(b.slot_id).or_default() += 1;
            *lot_frequency.entry(b.lot_id).or_default() += 1;
        }
    }

    // 3. Get all lots and available slots
    let lots = match state.db.list_parking_lots().await {
        Ok(l) => l,
        Err(_) => return Json(ApiResponse::success(vec![])),
    };

    let mut candidates: Vec<SlotRecommendation> = Vec::new();

    for lot in &lots {
        // Filter by lot_id if specified
        if let Some(ref filter_lot) = query.lot_id {
            if lot.id.to_string() != *filter_lot {
                continue;
            }
        }

        let slots = match state.db.list_slots_by_lot(&lot.id.to_string()).await {
            Ok(s) => s,
            Err(_) => continue,
        };

        for slot in &slots {
            // Only recommend available slots
            if slot.status != SlotStatus::Available {
                continue;
            }
            if slot.current_booking.is_some() {
                continue;
            }

            let mut score = 0.0;
            let mut reasons = Vec::new();

            // Factor 1: User's favorite slot (past usage frequency)
            let freq = slot_frequency.get(&slot.id).copied().unwrap_or(0);
            if freq > 0 {
                score += (freq as f64).min(10.0) * 4.0; // max 40 points
                reasons.push(format!("Used {} times before", freq));
            }

            // Factor 2: User's preferred lot
            let lot_freq = lot_frequency.get(&lot.id).copied().unwrap_or(0);
            if lot_freq > 0 {
                score += (lot_freq as f64).min(10.0) * 2.0; // max 20 points
                if freq == 0 {
                    reasons.push(format!("In your preferred lot (used {} times)", lot_freq));
                }
            }

            // Factor 3: Slot features bonus
            if !slot.features.is_empty() {
                score += 5.0;
                let feature_names: Vec<String> = slot.features.iter()
                    .map(|f| format!("{:?}", f))
                    .collect();
                reasons.push(format!("Features: {}", feature_names.join(", ")));
            }

            // Factor 4: Low row number preference (closer to entrance typically)
            let row_bonus = 10.0 / (slot.row as f64 + 1.0).max(1.0);
            score += row_bonus;

            // Factor 5: Base availability score (available = good)
            score += 10.0;
            if reasons.is_empty() {
                reasons.push("Available now".to_string());
            }

            let floor_name = lot.floors.first()
                .map(|f| f.name.clone())
                .unwrap_or_else(|| "Ground".to_string());

            candidates.push(SlotRecommendation {
                slot_id: slot.id,
                slot_number: slot.slot_number,
                lot_id: lot.id,
                lot_name: lot.name.clone(),
                floor_name,
                score,
                reasons,
            });
        }
    }

    // Sort by score descending, take top 5
    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(5);

    Json(ApiResponse::success(candidates))
}
