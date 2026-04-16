//! CO2 accounting — converts bookings into emissions saved.
//!
//! Methodology: each booking represents a car trip to the lot.
//! Without ParkHub, the user would still drive there (booking doesn't
//! eliminate the trip), so the "saved" metric comes from two sources:
//!   1. **Powertrain**: EVs, hybrids, and hydrogen vehicles emit less
//!      per km than a gasoline baseline. The delta vs the fleet baseline
//!      counts as savings.
//!   2. **Carpool multiplier**: if 2+ bookings share the same slot
//!      time-window, the extra riders would otherwise have made their own
//!      trips — each additional rider saves their entire trip emissions.
//!
//! Emission factors (g CO2e per vehicle-km) are DEFRA 2024 + UBA 2024.
//! Baseline = gasoline car. Values are conservative; tune via
//! `co2_*_g_per_km` admin settings when deploying.
//!
//! Endpoint: `GET /api/v1/bookings/co2-summary?from=ISO&to=ISO[&lot_id=UUID]`

use axum::{
    extract::{Query, State},
    Extension, Json,
};
use chrono::{DateTime, Duration, Utc};
use parkhub_common::{ApiResponse, FuelType, VehicleType};
use serde::{Deserialize, Serialize};

use super::{AuthUser, SharedState};

/// Default per-km emission factors (g CO2e / km). Aligned with DEFRA 2024
/// "Company reporting" dataset and UBA 2024 road transport breakdown.
/// `Unknown` falls back to `Gasoline` so under-reported vehicles at least
/// surface a real cost rather than masquerading as zero.
pub(crate) const fn emission_factor(vehicle: VehicleType, fuel: FuelType) -> f64 {
    let suv_multiplier = if matches!(vehicle, VehicleType::Suv | VehicleType::Truck | VehicleType::Van) {
        1.30
    } else {
        1.00
    };
    let base = match fuel {
        FuelType::Electric => 50.0,        // EU grid mix 2024
        FuelType::Hydrogen => 95.0,        // green H2 estimate (incl. well-to-tank)
        FuelType::PluginHybrid => 95.0,    // real-world utility factor adjusted
        FuelType::Hybrid => 130.0,
        FuelType::Diesel => 180.0,
        FuelType::Gasoline | FuelType::Unknown => 210.0,
    };
    base * suv_multiplier
}

/// Heuristic average kilometres a user drove to reach the lot for a booking.
/// Tunable via admin setting; default assumes 12 km commute each way = 24 km
/// per round-trip, consistent with the German BMVI average commute distance.
const ASSUMED_KM_PER_BOOKING: f64 = 24.0;

/// Baseline used as the "would have emitted" counterfactual when computing
/// savings. An average gasoline car without the SUV penalty.
const BASELINE_G_PER_KM: f64 = 210.0;

#[derive(Debug, Deserialize)]
pub struct Co2SummaryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub lot_id: Option<uuid::Uuid>,
}

#[derive(Debug, Serialize)]
pub struct Co2Summary {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub bookings_counted: usize,
    pub total_km: f64,
    pub emitted_g: f64,
    pub counterfactual_g: f64,
    pub saved_g: f64,
    pub carpool_saved_g: f64,
    /// Convenience: saved_g + carpool_saved_g / 1000, rounded to 2 decimals.
    pub saved_kg: f64,
}

/// `GET /api/v1/bookings/co2-summary`
///
/// Returns the CO2 accounting across the requested window. Scoped to the
/// authenticated user's own bookings. For carpool detection we count any
/// N≥2 bookings that share the same lot_id and overlap on start_time
/// within a 30-minute grace window as a shared trip; each additional
/// rider is credited with saving one full trip's baseline emissions.
pub async fn co2_summary(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(q): Query<Co2SummaryQuery>,
) -> Json<ApiResponse<Co2Summary>> {
    let to = q.to.unwrap_or_else(Utc::now);
    let from = q.from.unwrap_or(to - Duration::days(30));

    let rg = state.read().await;
    let user_id = auth_user.user_id;
    let bookings = match rg.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => return Json(ApiResponse::error("DB_ERROR", format!("db: {e}"))),
    };

    // Scope: per-user only for v1. Lot-wide admin scope can be layered on
    // later by moving role detection into a separate admin_co2_summary
    // handler mounted under the admin route tree.
    let scoped: Vec<_> = bookings
        .into_iter()
        .filter(|b| {
            let in_window = b.start_time >= from && b.start_time <= to;
            let matches_lot = q.lot_id.is_none_or(|id| b.lot_id == id);
            let matches_owner = b.user_id == user_id;
            in_window && matches_lot && matches_owner
        })
        .collect();

    let total_km = scoped.len() as f64 * ASSUMED_KM_PER_BOOKING;
    let emitted_g: f64 = scoped
        .iter()
        .map(|b| emission_factor(b.vehicle.vehicle_type.clone(), b.vehicle.fuel_type) * ASSUMED_KM_PER_BOOKING)
        .sum();
    let counterfactual_g = scoped.len() as f64 * BASELINE_G_PER_KM * ASSUMED_KM_PER_BOOKING;
    let saved_g = (counterfactual_g - emitted_g).max(0.0);

    // Carpool bonus: for every slot-time overlap within 30 min, each extra
    // rider saves a full counterfactual trip.
    let mut slot_times: std::collections::HashMap<(uuid::Uuid, i64), usize> =
        std::collections::HashMap::new();
    for b in &scoped {
        let bucket = b.start_time.timestamp() / 1800; // 30-min buckets
        *slot_times.entry((b.lot_id, bucket)).or_insert(0) += 1;
    }
    let carpool_trips_saved: usize = slot_times.values().filter_map(|&n| n.checked_sub(1)).sum();
    let carpool_saved_g = carpool_trips_saved as f64 * BASELINE_G_PER_KM * ASSUMED_KM_PER_BOOKING;

    let total_saved_g = saved_g + carpool_saved_g;
    let saved_kg = (total_saved_g / 1000.0 * 100.0).round() / 100.0;

    let summary = Co2Summary {
        from,
        to,
        bookings_counted: scoped.len(),
        total_km,
        emitted_g,
        counterfactual_g,
        saved_g,
        carpool_saved_g,
        saved_kg,
    };
    Json(ApiResponse::success(summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ev_saves_more_than_gasoline() {
        let ev = emission_factor(VehicleType::Car, FuelType::Electric);
        let gas = emission_factor(VehicleType::Car, FuelType::Gasoline);
        assert!(ev < gas, "EV must emit less per km than gasoline");
        assert_eq!(ev, 50.0);
        assert_eq!(gas, 210.0);
    }

    #[test]
    fn suv_penalty_applies() {
        let car_gas = emission_factor(VehicleType::Car, FuelType::Gasoline);
        let suv_gas = emission_factor(VehicleType::Suv, FuelType::Gasoline);
        assert!((suv_gas - car_gas * 1.30).abs() < 0.01);
    }

    #[test]
    fn unknown_falls_back_to_gasoline() {
        let unknown = emission_factor(VehicleType::Car, FuelType::Unknown);
        let gas = emission_factor(VehicleType::Car, FuelType::Gasoline);
        assert_eq!(unknown, gas);
    }

    #[test]
    fn hybrid_is_between_ev_and_gasoline() {
        let hybrid = emission_factor(VehicleType::Car, FuelType::Hybrid);
        let ev = emission_factor(VehicleType::Car, FuelType::Electric);
        let gas = emission_factor(VehicleType::Car, FuelType::Gasoline);
        assert!(ev < hybrid && hybrid < gas);
    }
}
