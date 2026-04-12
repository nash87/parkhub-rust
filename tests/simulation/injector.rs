//! API-based data injection for simulations.
//!
//! Uses real HTTP requests via reqwest to create all entities.
//! Respects rate limits and retries on 429.

use crate::common::{admin_login, auth_delete, auth_get, auth_post, TestServer};
use crate::simulation::generator;
use crate::simulation::profiles::SimProfile;
use chrono::{Duration, Utc};
use serde_json::Value;
use std::time::{Duration as StdDuration, Instant};

/// Context holding IDs and tokens for all created entities.
pub struct SimContext {
    pub admin_token: String,
    /// (token, user_id, username) for each simulated user
    pub users: Vec<(String, String, String)>,
    /// (lot_id, Vec<slot_id>) for each lot
    pub lots: Vec<(String, Vec<String>)>,
}

/// Results of the booking injection phase.
#[derive(Default)]
pub struct InjectionResults {
    pub total_booking_attempts: usize,
    pub successful_bookings: usize,
    pub rejected_conflicts: usize,
    pub cancellations: usize,
    pub recurring_created: usize,
    pub waitlist_entries: usize,
    pub errors: usize,
    /// Booking IDs that were successfully created
    pub booking_ids: Vec<String>,
    /// Booking IDs that were cancelled
    pub cancelled_ids: Vec<String>,
    /// Per-request latency samples (milliseconds)
    pub latencies_ms: Vec<u64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Retry helper
// ─────────────────────────────────────────────────────────────────────────────

/// Retry a request if we get 429 (rate limited).
async fn retry_on_429<F, Fut>(max_retries: u32, mut f: F) -> (u16, Value)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = (u16, Value)>,
{
    for attempt in 0..max_retries {
        let (status, body) = f().await;
        if status != 429 {
            return (status, body);
        }
        // Exponential backoff: 200ms, 400ms, 800ms, ...
        let delay = StdDuration::from_millis(200 * 2u64.pow(attempt));
        tokio::time::sleep(delay).await;
    }
    // Last attempt
    f().await
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1: Infrastructure setup
// ─────────────────────────────────────────────────────────────────────────────

pub async fn setup_infrastructure(srv: &TestServer, profile: &SimProfile) -> SimContext {
    let (admin_token, _) = admin_login(srv).await;

    // Create lots and slots
    let mut lots = Vec::with_capacity(profile.lots);
    for lot_idx in 0..profile.lots {
        let lot_name = format!("Sim Lot {} ({})", lot_idx + 1, profile.name);
        let lot_id = crate::common::create_test_lot(srv, &admin_token, &lot_name).await;

        let mut slot_ids = Vec::with_capacity(profile.slots_per_lot);
        for slot_num in 1..=profile.slots_per_lot {
            let slot_id =
                crate::common::create_test_slot(srv, &admin_token, &lot_id, slot_num as i32).await;
            slot_ids.push(slot_id);
        }
        lots.push((lot_id, slot_ids));
    }

    // Create users (limit to a reasonable number for the test server)
    let user_count = profile.users.min(100); // Cap for speed in tests
    let mut users = Vec::with_capacity(user_count);
    for i in 0..user_count {
        let suffix = format!("sim_{}_{}", profile.name, i);
        let result = crate::common::create_test_user(srv, &suffix).await;
        users.push(result);
    }

    SimContext {
        admin_token,
        users,
        lots,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 2: Booking injection
// ─────────────────────────────────────────────────────────────────────────────

pub async fn inject_bookings(
    srv: &TestServer,
    ctx: &SimContext,
    profile: &SimProfile,
) -> InjectionResults {
    let mut results = InjectionResults::default();
    let base_date = Utc::now() + Duration::hours(24); // Start tomorrow

    for day in 0..profile.days {
        let traffic = generator::day_traffic_multiplier(day);
        let day_bookings = (profile.bookings_per_day as f64 * traffic) as usize;
        // Cap daily bookings to actual user count to avoid token reuse issues
        let day_bookings = day_bookings.min(ctx.users.len());

        let day_offset = Duration::days(day as i64);

        for _booking_idx in 0..day_bookings {
            let user_idx = generator::random_user_index(ctx.users.len());
            let lot_idx = generator::random_lot_index(ctx.lots.len());
            let (ref lot_id, ref slot_ids) = ctx.lots[lot_idx];
            let slot_idx = generator::random_slot_index(slot_ids.len());
            let slot_id = &slot_ids[slot_idx];
            let (ref user_token, _, _) = ctx.users[user_idx];

            let is_weekday = generator::is_weekday(day);
            let hour = generator::booking_start_hour(is_weekday, profile.enable_peak_hours);
            let duration = generator::booking_duration_minutes();
            let plate = generator::random_license_plate();

            let start_time = base_date + day_offset + Duration::hours(hour as i64);

            results.total_booking_attempts += 1;

            // Intentional conflict attempt
            if profile.enable_conflicts && generator::is_conflict_attempt() {
                // Try to double-book a recently-booked slot
                if let Some(_last_booking_id) = results.booking_ids.last() {
                    let start = Instant::now();
                    let (status, _) = retry_on_429(3, || {
                        auth_post(
                            srv,
                            user_token,
                            "/api/v1/bookings",
                            &serde_json::json!({
                                "lot_id": lot_id,
                                "slot_id": slot_id,
                                "start_time": start_time.to_rfc3339(),
                                "duration_minutes": duration,
                                "vehicle_id": "00000000-0000-0000-0000-000000000000",
                                "license_plate": plate,
                            }),
                        )
                    })
                    .await;
                    results.latencies_ms.push(start.elapsed().as_millis() as u64);

                    if status == 409 {
                        results.rejected_conflicts += 1;
                    } else if status == 200 || status == 201 {
                        results.successful_bookings += 1;
                    } else {
                        results.errors += 1;
                    }
                    continue;
                }
            }

            // Normal booking
            let start = Instant::now();
            let (status, body) = retry_on_429(3, || {
                auth_post(
                    srv,
                    user_token,
                    "/api/v1/bookings",
                    &serde_json::json!({
                        "lot_id": lot_id,
                        "slot_id": slot_id,
                        "start_time": start_time.to_rfc3339(),
                        "duration_minutes": duration,
                        "vehicle_id": "00000000-0000-0000-0000-000000000000",
                        "license_plate": plate,
                    }),
                )
            })
            .await;
            results.latencies_ms.push(start.elapsed().as_millis() as u64);

            match status {
                200 | 201 => {
                    results.successful_bookings += 1;
                    if let Some(id) = body["data"]["id"].as_str() {
                        results.booking_ids.push(id.to_string());

                        // Maybe cancel this booking
                        if profile.enable_cancellations && generator::should_cancel() {
                            let cancel_start = Instant::now();
                            let (cs, _) = retry_on_429(3, || {
                                auth_delete(
                                    srv,
                                    user_token,
                                    &format!("/api/v1/bookings/{id}"),
                                )
                            })
                            .await;
                            results.latencies_ms.push(cancel_start.elapsed().as_millis() as u64);

                            if cs == 200 || cs == 204 {
                                results.cancellations += 1;
                                results.cancelled_ids.push(id.to_string());
                            }
                        }
                    }
                }
                409 => {
                    results.rejected_conflicts += 1;

                    // If conflict and waitlist is enabled, try to join waitlist
                    if profile.enable_waitlist && generator::should_waitlist() {
                        let (ws, _) = retry_on_429(3, || {
                            auth_post(
                                srv,
                                user_token,
                                "/api/v1/waitlist",
                                &serde_json::json!({ "lot_id": lot_id }),
                            )
                        })
                        .await;
                        if ws == 200 || ws == 201 {
                            results.waitlist_entries += 1;
                        }
                    }
                }
                _ => {
                    results.errors += 1;
                }
            }
        }

        // Recurring bookings: create once on day 0 for qualifying users
        if day == 0 && profile.enable_recurring {
            for (ref user_token, _, _) in &ctx.users {
                if !generator::is_recurring_user() {
                    continue;
                }

                let lot_idx = generator::random_lot_index(ctx.lots.len());
                let (ref lot_id, ref slot_ids) = ctx.lots[lot_idx];
                let slot_idx = generator::random_slot_index(slot_ids.len());
                let slot_id = &slot_ids[slot_idx];

                let today = Utc::now().format("%Y-%m-%d").to_string();
                let end = (Utc::now() + Duration::days(30))
                    .format("%Y-%m-%d")
                    .to_string();

                let (status, _) = retry_on_429(3, || {
                    auth_post(
                        srv,
                        user_token,
                        "/api/v1/recurring-bookings",
                        &serde_json::json!({
                            "lot_id": lot_id,
                            "slot_id": slot_id,
                            "days_of_week": [1, 2, 3, 4, 5],
                            "start_date": today,
                            "end_date": end,
                            "start_time": "08:00",
                            "end_time": "17:00",
                            "vehicle_plate": generator::random_license_plate(),
                        }),
                    )
                })
                .await;

                if status == 200 || status == 201 {
                    results.recurring_created += 1;
                }
            }
        }
    }

    results
}
