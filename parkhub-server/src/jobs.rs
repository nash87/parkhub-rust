//! Background job scheduler.
//!
//! Uses tokio interval tasks (no external cron dependency beyond what's already in the tree):
//! - **AutoRelease** (every 5 min): cancel no-show bookings after the configured threshold
//! - **ExpandRecurring** (every 1 h): create future booking instances for recurring series
//! - **PurgeExpired** (every 24 h): remove old cancelled/expired bookings beyond retention period
//! - **AggregateOccupancy** (every 15 min): persist aggregated occupancy stats to settings

use std::sync::Arc;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Utc};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::AppState;

pub type SharedState = Arc<RwLock<AppState>>;

/// Start all background jobs.  Call once after `AppState` is initialised.
#[allow(clippy::needless_pass_by_value)] // state is cloned into multiple spawned tasks
pub fn start_background_jobs(state: SharedState) {
    // ── AutoRelease: every 5 minutes ────────────────────────────────────────
    {
        let s = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                if let Err(e) = auto_release_no_shows(&s).await {
                    error!("AutoRelease job error: {e}");
                }
            }
        });
    }

    // ── ExpandRecurring: every hour ──────────────────────────────────────────
    {
        let s = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if let Err(e) = expand_recurring_bookings(&s).await {
                    error!("ExpandRecurring job error: {e}");
                }
            }
        });
    }

    // ── PurgeExpired: every 24 hours (first run after 60 s) ─────────────────
    {
        let s = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));
            loop {
                interval.tick().await;
                if let Err(e) = purge_expired_bookings(&s).await {
                    error!("PurgeExpired job error: {e}");
                }
            }
        });
    }

    // ── AggregateOccupancy: every 15 minutes ────────────────────────────────
    {
        let s = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(900));
            loop {
                interval.tick().await;
                if let Err(e) = aggregate_occupancy_stats(&s).await {
                    error!("AggregateOccupancy job error: {e}");
                }
            }
        });
    }

    info!(
        "Background jobs started: AutoRelease (5m), ExpandRecurring (1h), \
         PurgeExpired (24h), AggregateOccupancy (15m)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Job implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Cancel bookings that are Active/Confirmed but past their start time by more
/// than the configured `auto_release_minutes` and have never had a check-in.
async fn auto_release_no_shows(state: &SharedState) -> anyhow::Result<()> {
    // Read settings + bookings under a single short-lived read lock.
    let (enabled, threshold_mins, bookings) = {
        let guard = state.read().await;
        let enabled_str = guard
            .db
            .get_setting("auto_release_enabled")
            .await
            .unwrap_or(None)
            .unwrap_or_default();
        let enabled = enabled_str.parse::<bool>().unwrap_or(false);
        if !enabled {
            return Ok(());
        }
        let mins_str = guard
            .db
            .get_setting("auto_release_minutes")
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "30".to_string());
        let mins = mins_str.parse::<i64>().unwrap_or(30);
        let bookings = guard.db.list_bookings().await?;
        (enabled, mins, bookings)
    };

    if !enabled {
        return Ok(());
    }

    let now = Utc::now();
    let threshold = Duration::minutes(threshold_mins);

    let to_release: Vec<_> = bookings
        .into_iter()
        .filter(|b| {
            matches!(
                b.status,
                parkhub_common::BookingStatus::Active | parkhub_common::BookingStatus::Confirmed
            ) && b.check_in_time.is_none()
                && now > b.start_time + threshold
        })
        .collect();

    if to_release.is_empty() {
        return Ok(());
    }

    info!(
        "AutoRelease: releasing {} no-show booking(s)",
        to_release.len()
    );

    for mut booking in to_release {
        let slot_id = booking.slot_id.to_string();
        booking.status = parkhub_common::BookingStatus::NoShow;
        booking.updated_at = now;

        let guard = state.write().await;
        if let Err(e) = guard.db.save_booking(&booking).await {
            error!("AutoRelease: failed to save booking {}: {e}", booking.id);
            continue;
        }
        // Free the slot
        if let Err(e) = guard
            .db
            .update_slot_status(&slot_id, parkhub_common::SlotStatus::Available)
            .await
        {
            warn!(
                "AutoRelease: failed to free slot {slot_id} for booking {}: {e}",
                booking.id
            );
        }
        drop(guard);
        info!(
            "AutoRelease: booking {} marked NoShow, slot {slot_id} freed",
            booking.id
        );
    }

    Ok(())
}

/// For every active recurring booking, ensure single-booking instances exist for
/// the next 4 weeks.  Skips dates that already have a booking for the same slot.
async fn expand_recurring_bookings(state: &SharedState) -> anyhow::Result<()> {
    let (users, all_bookings) = {
        let guard = state.read().await;
        let users = guard.db.list_users().await?;
        let all_bookings = guard.db.list_bookings().await?;
        (users, all_bookings)
    };

    let now_date = Utc::now().date_naive();
    let horizon = now_date + Duration::weeks(4);
    let mut created = 0u32;

    for user in &users {
        let user_id_str = user.id.to_string();
        let recurring = {
            let guard = state.read().await;
            guard
                .db
                .list_recurring_bookings_by_user(&user_id_str)
                .await?
        };

        for rec in recurring.iter().filter(|r| r.active) {
            // Parse series start/end dates
            let Ok(series_start) = NaiveDate::parse_from_str(&rec.start_date, "%Y-%m-%d") else {
                warn!(
                    "ExpandRecurring: bad start_date '{}' on {}",
                    rec.start_date, rec.id
                );
                continue;
            };
            let series_end: Option<NaiveDate> = rec
                .end_date
                .as_deref()
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

            // Parse HH:MM slot times
            let Ok(slot_start) = NaiveTime::parse_from_str(&rec.start_time, "%H:%M") else {
                warn!(
                    "ExpandRecurring: bad start_time '{}' on {}",
                    rec.start_time, rec.id
                );
                continue;
            };
            let Ok(slot_end) = NaiveTime::parse_from_str(&rec.end_time, "%H:%M") else {
                warn!(
                    "ExpandRecurring: bad end_time '{}' on {}",
                    rec.end_time, rec.id
                );
                continue;
            };

            // days_of_week: 0 = Monday … 6 = Sunday (matches chrono Weekday::num_days_from_monday)
            let day_set: std::collections::HashSet<u8> = rec.days_of_week.iter().copied().collect();

            // Build set of existing booking dates for this user+slot to avoid duplicates
            let slot_id_opt = rec.slot_id;
            let existing_dates: std::collections::HashSet<NaiveDate> = all_bookings
                .iter()
                .filter(|b| {
                    b.user_id == rec.user_id
                        && slot_id_opt.is_none_or(|sid| b.slot_id == sid)
                        && !matches!(
                            b.status,
                            parkhub_common::BookingStatus::Cancelled
                                | parkhub_common::BookingStatus::Expired
                                | parkhub_common::BookingStatus::NoShow
                        )
                })
                .map(|b| b.start_time.date_naive())
                .collect();

            // Walk from max(series_start, today) to min(horizon, series_end)
            let walk_start = series_start.max(now_date);
            let walk_end = series_end.map_or(horizon, |e| e.min(horizon));

            let mut cursor = walk_start;
            while cursor <= walk_end {
                #[allow(clippy::cast_possible_truncation)] // weekday is always 0..6
                let dow = cursor.weekday().num_days_from_monday() as u8;
                if day_set.contains(&dow) && !existing_dates.contains(&cursor) {
                    // Create a new booking for this date (treat stored times as UTC)
                    let start_dt = cursor.and_time(slot_start).and_utc();
                    let end_dt = cursor.and_time(slot_end).and_utc();

                    // We need a vehicle — try to find the user's default vehicle.
                    // If none exists we skip silently (can't create a booking without one).
                    let vehicle = {
                        let guard = state.read().await;
                        guard
                            .db
                            .list_vehicles_by_user(&user_id_str)
                            .await
                            .unwrap_or_default()
                            .into_iter()
                            .next() // first available (prefer default if sorted)
                    };
                    let Some(vehicle) = vehicle else {
                        continue;
                    };

                    // Resolve slot_id: recurring bookings may not pin a slot
                    let slot_id = if let Some(sid) = rec.slot_id {
                        sid
                    } else {
                        // Pick the first available slot in the lot (best-effort)
                        let guard = state.read().await;
                        let slots = guard
                            .db
                            .list_slots_by_lot(&rec.lot_id.to_string())
                            .await
                            .unwrap_or_default();
                        let Some(s) = slots
                            .into_iter()
                            .find(|s| s.status == parkhub_common::SlotStatus::Available)
                        else {
                            // No available slot -- skip this date
                            cursor += Duration::days(1);
                            continue;
                        };
                        s.id
                    };

                    // Fetch slot + lot for metadata
                    let (slot_number, floor_name) = {
                        let guard = state.read().await;
                        let slot_opt = guard
                            .db
                            .get_parking_slot(&slot_id.to_string())
                            .await
                            .unwrap_or(None);
                        let lot_opt = guard
                            .db
                            .get_parking_lot(&rec.lot_id.to_string())
                            .await
                            .unwrap_or(None);
                        match slot_opt {
                            Some(s) => {
                                let fname = lot_opt
                                    .as_ref()
                                    .and_then(|lot| {
                                        lot.floors
                                            .iter()
                                            .find(|f| f.id == s.floor_id)
                                            .map(|f| f.name.clone())
                                    })
                                    .unwrap_or_else(|| "Level 1".to_string());
                                (s.slot_number, fname)
                            }
                            None => (0, "Level 1".to_string()),
                        }
                    };

                    let booking = parkhub_common::Booking {
                        id: Uuid::new_v4(),
                        user_id: rec.user_id,
                        lot_id: rec.lot_id,
                        slot_id,
                        slot_number,
                        floor_name,
                        vehicle,
                        start_time: start_dt,
                        end_time: end_dt,
                        status: parkhub_common::BookingStatus::Confirmed,
                        pricing: parkhub_common::BookingPricing {
                            base_price: 0.0,
                            discount: 0.0,
                            tax: 0.0,
                            total: 0.0,
                            currency: "EUR".to_string(),
                            payment_status: parkhub_common::PaymentStatus::Pending,
                            payment_method: None,
                        },
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        check_in_time: None,
                        check_out_time: None,
                        qr_code: None,
                        notes: Some(format!("Auto-expanded from recurring booking {}", rec.id)),
                        tenant_id: None,
                    };

                    let guard = state.write().await;
                    match guard.db.save_booking(&booking).await {
                        Ok(()) => {
                            created += 1;
                        }
                        Err(e) => {
                            error!(
                                "ExpandRecurring: failed to create booking for {} on {cursor}: {e}",
                                rec.id
                            );
                        }
                    }
                    drop(guard);
                }
                cursor += Duration::days(1);
            }
        }
    }

    if created > 0 {
        info!("ExpandRecurring: created {created} new booking instance(s)");
    }
    Ok(())
}

/// Delete cancelled, expired, or no-show bookings older than `retention_days`
/// (default 90).  Reads the `booking_retention_days` setting.
async fn purge_expired_bookings(state: &SharedState) -> anyhow::Result<()> {
    let (retention_days, bookings) = {
        let guard = state.read().await;
        let days_str = guard
            .db
            .get_setting("booking_retention_days")
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "90".to_string());
        let days = days_str.parse::<i64>().unwrap_or(90).max(1);
        let bookings = guard.db.list_bookings().await?;
        (days, bookings)
    };

    let cutoff = Utc::now() - Duration::days(retention_days);

    let to_purge: Vec<_> = bookings
        .into_iter()
        .filter(|b| {
            matches!(
                b.status,
                parkhub_common::BookingStatus::Cancelled
                    | parkhub_common::BookingStatus::Expired
                    | parkhub_common::BookingStatus::NoShow
            ) && b.updated_at < cutoff
        })
        .collect();

    if to_purge.is_empty() {
        return Ok(());
    }

    info!("PurgeExpired: deleting {} old booking(s)", to_purge.len());

    let mut deleted = 0u32;
    for booking in to_purge {
        let guard = state.write().await;
        match guard.db.delete_booking(&booking.id.to_string()).await {
            Ok(true) => deleted += 1,
            Ok(false) => {}
            Err(e) => error!("PurgeExpired: failed to delete booking {}: {e}", booking.id),
        }
        drop(guard);
    }

    info!("PurgeExpired: deleted {deleted} booking(s)");
    Ok(())
}

/// Compute and persist basic occupancy stats per lot into the settings store.
/// Key: `occupancy_stats_<lot_id>`, value: `<occupied>/<total>`.
async fn aggregate_occupancy_stats(state: &SharedState) -> anyhow::Result<()> {
    let (lots, bookings) = {
        let guard = state.read().await;
        let lots = guard.db.list_parking_lots().await?;
        let bookings = guard.db.list_bookings().await?;
        (lots, bookings)
    };

    let now = Utc::now();
    let active_statuses = [
        parkhub_common::BookingStatus::Active,
        parkhub_common::BookingStatus::Confirmed,
    ];

    let mut stats_written = 0u32;
    for lot in &lots {
        #[allow(clippy::cast_sign_loss)]
        let total = lot.total_slots.max(0) as u64;

        let occupied = bookings
            .iter()
            .filter(|b| {
                b.lot_id == lot.id
                    && active_statuses.contains(&b.status)
                    && b.start_time <= now
                    && b.end_time >= now
            })
            .count() as u64;

        let key = format!("occupancy_stats_{}", lot.id);
        let value = format!("{occupied}/{total}");

        let guard = state.write().await;
        if let Err(e) = guard.db.set_setting(&key, &value).await {
            error!(
                "AggregateOccupancy: failed to write stats for lot {}: {e}",
                lot.id
            );
        } else {
            stats_written += 1;
        }
        drop(guard);
    }

    if stats_written > 0 {
        info!("AggregateOccupancy: updated stats for {stats_written} lot(s)");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests (issue #112)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use crate::db::{Database, DatabaseConfig};

    /// Create a minimal test state backed by a tempdir.
    async fn job_test_state() -> (SharedState, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_config = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        let db = Database::open(&db_config).expect("open test db");
        let config = ServerConfig::default();
        let state = Arc::new(RwLock::new(AppState {
            config,
            db,
            mdns: None,
            scheduler: None,
            ws_events: crate::api::ws::EventBroadcaster::new(),
        }));
        (state, dir)
    }

    #[tokio::test]
    async fn auto_release_disabled_is_noop() {
        let (state, _dir) = job_test_state().await;
        // auto_release_enabled defaults to not set / false
        let result = auto_release_no_shows(&state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn purge_expired_empty_db_is_noop() {
        let (state, _dir) = job_test_state().await;
        let result = purge_expired_bookings(&state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn aggregate_occupancy_empty_db_is_noop() {
        let (state, _dir) = job_test_state().await;
        let result = aggregate_occupancy_stats(&state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn expand_recurring_empty_db_is_noop() {
        let (state, _dir) = job_test_state().await;
        let result = expand_recurring_bookings(&state).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn auto_release_marks_no_show_bookings() {
        let (state, _dir) = job_test_state().await;

        // Enable auto-release with a 0-minute threshold
        {
            let guard = state.read().await;
            guard
                .db
                .set_setting("auto_release_enabled", "true")
                .await
                .unwrap();
            guard
                .db
                .set_setting("auto_release_minutes", "0")
                .await
                .unwrap();
        }

        // Create a booking that started in the past with no check-in
        let lot_id = Uuid::new_v4();
        let slot_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let booking = parkhub_common::Booking {
            id: Uuid::new_v4(),
            user_id,
            lot_id,
            slot_id,
            slot_number: 1,
            floor_name: "Level 1".to_string(),
            vehicle: parkhub_common::Vehicle {
                id: Uuid::new_v4(),
                user_id,
                license_plate: "TEST-001".to_string(),
                make: None,
                model: None,
                color: None,
                vehicle_type: parkhub_common::VehicleType::Car,
                is_default: true,
                created_at: Utc::now(),
            },
            start_time: Utc::now() - Duration::hours(2),
            end_time: Utc::now() - Duration::hours(1),
            status: parkhub_common::BookingStatus::Confirmed,
            pricing: parkhub_common::BookingPricing {
                base_price: 0.0,
                discount: 0.0,
                tax: 0.0,
                total: 0.0,
                currency: "EUR".to_string(),
                payment_status: parkhub_common::PaymentStatus::Pending,
                payment_method: None,
            },
            created_at: Utc::now() - Duration::hours(3),
            updated_at: Utc::now() - Duration::hours(3),
            check_in_time: None,
            check_out_time: None,
            qr_code: None,
            notes: None,
            tenant_id: None,
        };

        {
            let guard = state.read().await;
            guard.db.save_booking(&booking).await.unwrap();
        }

        // Run auto-release
        auto_release_no_shows(&state).await.unwrap();

        // Verify the booking was marked as NoShow
        let guard = state.read().await;
        let updated = guard
            .db
            .get_booking(&booking.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, parkhub_common::BookingStatus::NoShow);
    }
}
