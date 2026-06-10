//! `ParkHub` Server
//!
//! Database server with HTTP API and LAN autodiscovery.
//! Can run headless or with a configuration GUI.

// Hide console window on Windows when running with GUI
#![cfg_attr(all(feature = "gui", windows), windows_subsystem = "windows")]
// AppState read guards in cron job closures span the whole pass — db access
// goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod api;
#[allow(dead_code)]
mod audit;
mod bootstrap;
#[allow(dead_code)]
mod circuit_breaker;
mod config;
mod db;
mod demo;
mod discovery;
#[cfg(feature = "mod-email")]
mod email;
#[cfg(feature = "mod-email-templates")]
#[allow(dead_code)]
mod email_templates;
#[allow(dead_code)]
mod error;
#[allow(dead_code)]
mod health;
#[cfg(feature = "mod-jobs")]
mod jobs;
#[allow(dead_code)]
mod jwt;
#[cfg(feature = "mod-mcp")]
mod mcp;
#[allow(dead_code)]
mod metrics;
#[cfg(feature = "full")]
#[allow(dead_code)]
mod openapi;
#[allow(dead_code)]
mod rate_limit;
#[allow(dead_code)]
mod requests;
#[allow(dead_code)]
mod static_files;
#[allow(dead_code)]
mod tls;
pub mod utils;
#[allow(dead_code)]
mod validation;

#[cfg(all(test, feature = "full"))]
mod booking_tests;
#[cfg(all(test, feature = "full"))]
mod calendar_tests;
#[cfg(test)]
mod coverage_tests;
#[cfg(all(test, feature = "full"))]
mod integration_tests;
#[cfg(all(test, feature = "full"))]
mod mobile_tests;
#[cfg(all(test, feature = "full"))]
mod sse_events_tests;
#[cfg(all(test, feature = "full"))]
mod webhooks_v2_tests;

use bootstrap::cli::CliArgs;
use bootstrap::health::perform_health_check;
use bootstrap::paths::{get_data_directory, get_local_ip};
use bootstrap::revocation::build_revocation_store;
use bootstrap::seed::{UsernameStyle, generate_dummy_users, seed_demo_data};
use config::ServerConfig;
use db::{Database, DatabaseConfig};
use discovery::MdnsService;
use jwt::TokenRevocationList;

// Re-exports kept at the crate root so legacy call sites like
// `crate::hash_password`, `crate::create_admin_user`,
// `crate::create_sample_parking_lot`, and `crate::seed_demo_data`
// (sprinkled across tests and a handful of handlers) resolve without
// churn. The definitions live under `bootstrap::*`.
pub(crate) use bootstrap::paths::hash_password;
pub(crate) use bootstrap::seed::{create_admin_user, create_sample_parking_lot};

#[cfg(feature = "gui")]
use bootstrap::setup_wizard::{prompt_passphrase_gui, run_setup_wizard};
#[cfg(feature = "gui")]
use bootstrap::status_gui::run_status_gui;

#[cfg(feature = "gui")]
slint::include_modules!();

/// Application state shared across handlers
pub struct AppState {
    pub config: ServerConfig,
    pub db: Database,
    pub mdns: Option<MdnsService>,
    /// Holds the cron scheduler so it is not leaked via `mem::forget`.
    /// The scheduler runs background tasks (e.g. monthly credit refill).
    /// Dropping it will cancel scheduled jobs.
    pub scheduler: Option<tokio_cron_scheduler::JobScheduler>,
    /// Broadcast channel for WebSocket real-time events.
    pub ws_events: api::ws::EventBroadcaster,
    /// T-1946 — Broadcast channel for Server-Sent fleet events.
    /// Consumed by `/api/v1/events/fleet` subscribers; produced by the
    /// check-in / swap / EV-charging / guest-booking mutation handlers AFTER
    /// their DB commit.
    pub fleet_events: api::sse::FleetEventBroadcaster,
    /// JWT revocation store — backed by either an in-memory HashMap
    /// (single-replica default) or Redis (when the `redis-revocation`
    /// feature is enabled AND `PARKHUB_REDIS_URL` is set).
    ///
    /// Wired into every request via an axum `Extension` layer so the
    /// `AuthUser` extractor can consult it on token validation.
    pub revocation_store: Arc<TokenRevocationList>,
}

#[tokio::main]
#[allow(clippy::field_reassign_with_default, clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Parse CLI arguments first
    let cli = CliArgs::parse();

    if cli.help {
        CliArgs::print_help();
        return Ok(());
    }

    if cli.version {
        CliArgs::print_version();
        return Ok(());
    }

    // --health-check: probe the running server and exit 0 (healthy) or 1 (unhealthy/unreachable).
    // This is designed to be used as the Docker HEALTHCHECK CMD — it must be a bare binary call
    // so that it works inside distroless images that have no shell.
    if cli.health_check {
        let port = cli
            .port
            .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
            .unwrap_or(10000);
        std::process::exit(perform_health_check(port));
    }

    // Set DPI awareness before creating any windows (Windows-specific)
    #[cfg(all(feature = "gui", windows))]
    if !cli.headless {
        use windows_sys::Win32::UI::HiDpi::{
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
        };
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    // Force software renderer since fonts are embedded for it (EmbedForSoftwareRenderer in build.rs)
    // This ensures the app works on systems without GPU/OpenGL support
    #[cfg(feature = "gui")]
    if !cli.headless {
        // SAFETY: called before any threads are spawned (main function, GUI init)
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("SLINT_BACKEND", "winit-software");
        };
    }

    // Initialize logging based on debug flag
    let log_filter = if cli.debug {
        "debug,parkhub_server=trace"
    } else {
        "info,parkhub_server=debug"
    };

    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| log_filter.to_string()))
        .with_target(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();

    info!("Starting ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
    if cli.debug {
        info!("Debug mode enabled");
    }
    if cli.headless {
        info!("Headless mode enabled");
    }
    if cli.unattended {
        info!("Unattended mode enabled");
    }

    // Determine initial data directory (may change if setup wizard runs)
    #[allow(unused_mut)] // mut needed when gui feature is enabled
    let mut data_dir = if let Some(ref dir) = cli.data_dir {
        std::fs::create_dir_all(dir)?;
        dir.clone()
    } else {
        get_data_directory(None)?
    };
    info!("Data directory: {}", data_dir.display());

    // Load or create configuration
    let config_path = data_dir.join("config.toml");
    let mut config = if config_path.exists() {
        ServerConfig::load(&config_path)?
    } else if cli.unattended || cli.headless {
        // Unattended/headless mode - auto-configure with defaults
        info!("Auto-configuring with defaults (unattended mode)...");
        let mut config = ServerConfig::default();
        config.server_name = hostname::get().map_or_else(
            |_| "ParkHub Server".to_string(),
            |h| h.to_string_lossy().to_string(),
        );
        let admin_password = std::env::var("PARKHUB_ADMIN_PASSWORD").unwrap_or_else(|_| {
            use rand::RngExt;
            let generated: String = rand::rng()
                .sample_iter(&rand::distr::Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║  GENERATED ADMIN PASSWORD: {generated}  ║");
            println!("║  CHANGE THIS PASSWORD IMMEDIATELY AFTER FIRST LOGIN!    ║");
            println!("╚══════════════════════════════════════════════════════════╝");
            warn!("Using auto-generated admin password. Change it immediately after first login!");
            generated
        });
        config.admin_password_hash = hash_password(&admin_password)?;
        config.encryption_enabled = false; // Disable encryption for unattended setup
        config.enable_tls = false; // Disable TLS for easier initial setup
        config.generate_dummy_users = true;
        // Disable mDNS in headless/container mode. On Render (and any other
        // managed container host) the UDP 5353 socket confuses the proxy
        // port-detection, and LAN discovery is pointless without a LAN.
        // Users who actually want mDNS can enable it in their config.toml.
        config.enable_mdns = false;
        config.save(&config_path)?;
        info!("Default config saved. Admin user: admin");
        config
    } else {
        info!("No configuration found, running setup...");
        #[cfg(feature = "gui")]
        {
            let wizard_config = run_setup_wizard()?;
            // Update data directory based on portable mode choice
            data_dir = get_data_directory(Some(wizard_config.portable_mode))?;
            let new_config_path = data_dir.join("config.toml");
            wizard_config.save(&new_config_path)?;
            info!("Configuration saved to: {}", new_config_path.display());
            wizard_config
        }
        #[cfg(not(feature = "gui"))]
        {
            // Create default configuration in headless mode
            warn!("Running in headless mode - using default configuration");
            let mut config = ServerConfig::default();
            // Generate a random password or use env var
            let admin_password = std::env::var("PARKHUB_ADMIN_PASSWORD").unwrap_or_else(|_| {
                use rand::RngExt;
                let generated: String = rand::rng()
                    .sample_iter(&rand::distr::Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect();
                println!("╔══════════════════════════════════════════════════════════╗");
                println!("║  GENERATED ADMIN PASSWORD: {generated}  ║");
                println!("║  CHANGE THIS PASSWORD IMMEDIATELY AFTER FIRST LOGIN!    ║");
                println!("╚══════════════════════════════════════════════════════════╝");
                warn!(
                    "Using auto-generated admin password. Change it immediately after first login!"
                );
                generated
            });
            config.admin_password_hash = hash_password(&admin_password)?;
            // Use environment variable for encryption passphrase in headless mode
            config.encryption_passphrase = std::env::var("PARKHUB_DB_PASSPHRASE").ok();
            if config.encryption_enabled && config.encryption_passphrase.is_none() {
                anyhow::bail!(
                    "Database encryption is enabled but PARKHUB_DB_PASSPHRASE is not set.\n\
                     Set the PARKHUB_DB_PASSPHRASE environment variable to a strong, \
                     randomly generated passphrase before starting the server.\n\
                     Example: export PARKHUB_DB_PASSPHRASE=\"$(openssl rand -base64 32)\""
                );
            }
            config.save(&config_path)?;
            info!("Default config saved. Admin user: admin");
            config
        }
    };

    // Override port if specified on command line
    if let Some(port) = cli.port {
        config.port = port;
        info!("Port overridden from command line: {}", port);
    } else if let Ok(port_str) = std::env::var("PORT") {
        // Support the PORT environment variable used by Render and other PaaS platforms.
        if let Ok(port) = port_str.parse::<u16>() {
            config.port = port;
            info!("Port set from PORT environment variable: {}", port);
        }
    }

    // If encryption is enabled but no passphrase, try environment variable
    if config.encryption_enabled && config.encryption_passphrase.is_none() {
        config.encryption_passphrase = std::env::var("PARKHUB_DB_PASSPHRASE").ok();
        if config.encryption_passphrase.is_none() {
            #[cfg(feature = "gui")]
            {
                config.encryption_passphrase = Some(prompt_passphrase_gui()?);
            }
            #[cfg(not(feature = "gui"))]
            {
                anyhow::bail!(
                    "Database encryption enabled but no passphrase provided.\n\
                     Set PARKHUB_DB_PASSPHRASE environment variable."
                );
            }
        }
    }

    // Initialize database with encryption
    let db_config = DatabaseConfig {
        path: data_dir.clone(),
        encryption_enabled: config.encryption_enabled,
        passphrase: config.encryption_passphrase.clone(),
        create_if_missing: true,
    };
    let db = Database::open(&db_config).context("Failed to open database")?;
    info!(
        "Database opened: {} (encrypted: {})",
        data_dir.display(),
        db.is_encrypted()
    );

    // Create admin user if database is fresh
    if db.is_fresh().await? {
        info!("Creating admin user...");
        create_admin_user(&db, &config).await?;

        // Also create a sample parking lot
        create_sample_parking_lot(&db).await?;

        // Generate dummy users if requested during setup
        if config.generate_dummy_users {
            let style = match config.username_style {
                0 => UsernameStyle::FirstLastLetter,
                1 => UsernameStyle::FirstDotLast,
                2 => UsernameStyle::InitialLast,
                _ => UsernameStyle::FirstInitial,
            };
            generate_dummy_users(&db, style).await?;
        }

        // Enable credits system by default
        db.set_setting("credits_enabled", "true").await?;
        db.set_setting("credits_per_booking", "1").await?;
    }

    // Demo seeding: when SEED_DEMO_DATA=true or DEMO_MODE=true, seed 10 lots + 200 users
    // directly via DB functions (no shell scripts, no HTTP API calls).  Runs at most once
    // per database — skipped when parking lots already exist.
    {
        let want_seed = std::env::var("SEED_DEMO_DATA")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
            || std::env::var("DEMO_MODE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);
        if want_seed {
            let lot_count = db.list_parking_lots().await.map(|l| l.len()).unwrap_or(0);
            if lot_count < 2 {
                info!(
                    "Seeding demo data (SEED_DEMO_DATA/DEMO_MODE requested, {lot_count} lots found)..."
                );
                if let Err(e) = seed_demo_data(&db).await {
                    warn!("Demo seeding failed (non-fatal): {e}");
                }
            } else {
                info!("Demo data already present ({lot_count} lots) — skipping seed.");
            }
        }
    }

    // ── MCP stdio server (short-circuits normal HTTP startup) ─────────────────
    #[cfg(feature = "mod-mcp")]
    if cli.mcp {
        info!("Launching MCP server over stdio");
        mcp::run(db).await?;
        return Ok(());
    }

    // Start mDNS service for autodiscovery
    let mdns = if config.enable_mdns {
        match MdnsService::new(&config) {
            Ok(service) => {
                info!("mDNS autodiscovery enabled");
                Some(service)
            }
            Err(e) => {
                warn!("Failed to start mDNS: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Build the JWT revocation store — Redis when the `redis-revocation`
    // feature is enabled, in-memory otherwise. Must happen before AppState
    // so the backend is fixed for the lifetime of this process.
    let revocation_store = build_revocation_store().await;

    // Create application state
    let state = Arc::new(RwLock::new(AppState {
        config: config.clone(),
        db,
        mdns,
        scheduler: None,
        ws_events: api::ws::EventBroadcaster::new(),
        fleet_events: api::sse::FleetEventBroadcaster::new(),
        revocation_store: revocation_store.clone(),
    }));

    // Build the API router. `revocation_store` is passed alongside `state` so
    // `create_router` can install it as an axum `Extension` without having to
    // acquire the `AppState` lock synchronously.
    let (app, demo_state) = api::create_router(state.clone(), revocation_store);

    // Determine bind address
    let addr: SocketAddr = format!("0.0.0.0:{}", config.port).parse()?;
    info!("Server listening on {}", addr);
    info!(
        "Access URL: {}://{}:{}",
        if config.enable_tls { "https" } else { "http" },
        get_local_ip().unwrap_or_else(|| "localhost".to_string()),
        config.port
    );

    // Shared shutdown signal — when triggered, the HTTP server will drain
    // in-flight connections gracefully before exiting.
    let shutdown_tx = {
        let (tx, _) = tokio::sync::broadcast::channel::<()>(1);
        Arc::new(tx)
    };

    // Start server in background task
    let server_config = config.clone();
    let data_dir_for_server = data_dir.clone();
    let shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        if server_config.enable_tls {
            match tls::load_or_create_tls_config(&data_dir_for_server).await {
                Ok(tls_config) => {
                    info!("TLS enabled");
                    if let Err(e) = axum_server::bind_rustls(addr, tls_config)
                        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                        .await
                    {
                        tracing::error!("Server error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load TLS config: {}", e);
                }
            }
        } else {
            warn!("TLS disabled - connections are not encrypted!");
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    let mut shutdown_rx = shutdown_rx;
                    let shutdown_signal = async move {
                        let _ = shutdown_rx.recv().await;
                        info!("Graceful shutdown signal received — draining connections");
                    };
                    if let Err(e) = axum::serve(
                        listener,
                        app.into_make_service_with_connect_info::<SocketAddr>(),
                    )
                    .with_graceful_shutdown(shutdown_signal)
                    .await
                    {
                        tracing::error!("Server error: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to bind server: {}", e);
                }
            }
        }
    });

    // Start monthly credit refill cron job (1st of each month at 00:00)
    {
        use chrono::Datelike;
        use tokio_cron_scheduler::{Job, JobScheduler};

        let sched = JobScheduler::new().await?;
        let state_for_cron = state.clone();
        sched
            .add(Job::new_async("0 0 0 1 * *", move |_uuid, _lock| {
                let state = state_for_cron.clone();
                Box::pin(async move {
                    info!("Running monthly credit refill cron job...");
                    let now = chrono::Utc::now();

                    // Load users with a short-lived read lock, then release it
                    let user_ids_to_refill: Vec<uuid::Uuid> = {
                        let state_guard = state.read().await;
                        let users = match state_guard.db.list_users().await {
                            Ok(u) => u,
                            Err(e) => {
                                tracing::error!("Cron: failed to list users: {}", e);
                                return;
                            }
                        };
                        drop(state_guard);
                        users
                            .into_iter()
                            .filter(|user| {
                                user.is_active
                                    && user.role != parkhub_common::UserRole::Admin
                                    && user.role != parkhub_common::UserRole::SuperAdmin
                                    // Idempotency: skip users already refilled this month
                                    && !user
                                        .credits_last_refilled
                                        .is_some_and(|t| {
                                            t.month() == now.month() && t.year() == now.year()
                                        })
                            })
                            .map(|u| u.id)
                            .collect()
                    }; // read lock released here

                    let mut refilled = 0u32;
                    // Process in batches with short-lived write locks
                    for chunk in user_ids_to_refill.chunks(50) {
                        let state_guard = state.write().await;
                        for user_id in chunk {
                            let Some(mut user) = state_guard
                                .db
                                .get_user(&user_id.to_string())
                                .await
                                .ok()
                                .flatten()
                            else {
                                continue;
                            };
                            // Re-check idempotency under write lock
                            if user
                                .credits_last_refilled
                                .is_some_and(|t| t.month() == now.month() && t.year() == now.year())
                            {
                                continue;
                            }
                            let old_balance = user.credits_balance;
                            user.credits_balance = user.credits_monthly_quota;
                            user.credits_last_refilled = Some(now);
                            if matches!(state_guard.db.save_user(&user).await, Ok(())) {
                                let tx = parkhub_common::CreditTransaction {
                                    id: uuid::Uuid::new_v4(),
                                    user_id: user.id,
                                    booking_id: None,
                                    amount: user.credits_monthly_quota - old_balance,
                                    transaction_type:
                                        parkhub_common::CreditTransactionType::MonthlyRefill,
                                    description: Some("Automated monthly refill".to_string()),
                                    granted_by: None,
                                    created_at: now,
                                };
                                if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
                                    tracing::warn!(
                                        "Failed to save monthly refill transaction: {e}"
                                    );
                                }
                                refilled += 1;
                            }
                        }
                        drop(state_guard);
                    }
                    info!(
                        "Monthly credit refill complete: {} users refilled",
                        refilled
                    );
                })
            })?)
            .await?;
        // Booking reminder job — runs every 5 minutes, emails users whose
        // booking starts within the next 30–35 minutes and haven't been reminded.
        let state_for_reminder = state.clone();
        sched
            .add(Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
                let state = state_for_reminder.clone();
                Box::pin(async move {
                    let now = chrono::Utc::now();
                    let window_start = now + chrono::Duration::minutes(30);
                    let window_end = now + chrono::Duration::minutes(35);

                    let state_guard = state.read().await;
                    let bookings = match state_guard.db.list_bookings().await {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::warn!("Reminder cron: failed to list bookings: {}", e);
                            return;
                        }
                    };

                    let org_name = state_guard.config.organization_name.clone();

                    // Filter to bookings starting in the 30-35 minute window
                    let due: Vec<_> = bookings
                        .into_iter()
                        .filter(|b| {
                            matches!(
                                b.status,
                                parkhub_common::BookingStatus::Confirmed
                                    | parkhub_common::BookingStatus::Pending
                            ) && b.start_time >= window_start
                                && b.start_time < window_end
                        })
                        .collect();

                    for booking in due {
                        // Skip if already reminded (stored as setting key)
                        let reminder_key = format!("reminder_sent_{}", booking.id);
                        if state_guard
                            .db
                            .get_setting(&reminder_key)
                            .await
                            .ok()
                            .flatten()
                            .is_some()
                        {
                            continue;
                        }

                        let Some(user) = state_guard
                            .db
                            .get_user(&booking.user_id.to_string())
                            .await
                            .ok()
                            .flatten()
                        else {
                            continue;
                        };

                        let minutes_until = (booking.start_time - now).num_minutes().max(0);

                        #[cfg(feature = "mod-email")]
                        {
                            let email_html = crate::email::build_booking_reminder_email(
                                &user.name,
                                &booking.id.to_string(),
                                &booking.floor_name,
                                booking.slot_number,
                                &booking.start_time.format("%Y-%m-%d %H:%M").to_string(),
                                &booking.end_time.format("%Y-%m-%d %H:%M").to_string(),
                                minutes_until,
                                &org_name,
                            );
                            let subject =
                                format!("Parking reminder: your booking starts in {minutes_until} minutes — ParkHub");
                            if let Err(e) =
                                crate::email::send_email(&user.email, &subject, &email_html).await
                            {
                                tracing::warn!(
                                    "Failed to send booking reminder (booking {}): {}",
                                    booking.id,
                                    e
                                );
                            } else {
                                // Mark as reminded so we don't send again
                                if let Err(e) =
                                    state_guard.db.set_setting(&reminder_key, "1").await
                                {
                                    tracing::warn!(
                                        "Failed to mark reminder sent for booking {}: {}",
                                        booking.id,
                                        e
                                    );
                                }
                                tracing::info!(
                                    booking_id = %booking.id,
                                    user_id = %user.id,
                                    "Booking reminder sent"
                                );
                            }
                        }

                        #[cfg(not(feature = "mod-email"))]
                        {
                            let _ = (minutes_until, &org_name, &user);
                            tracing::debug!(
                                booking_id = %booking.id,
                                "Email module disabled — skipping booking reminder"
                            );
                        }
                    }
                })
            })?)
            .await?;

        sched.start().await?;
        info!("Credit refill scheduler started (runs 1st of each month at 00:00)");
        info!("Booking reminder scheduler started (runs every 5 minutes, 30-min window)");
        // Store scheduler in AppState so it is properly dropped on shutdown
        // instead of leaked via mem::forget.
        state.write().await.scheduler = Some(sched);
    }

    // Demo auto-reset scheduler (every 6 hours when DEMO_MODE=true)
    {
        let demo_enabled = {
            let ds = demo_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            ds.enabled
        };
        if demo_enabled {
            use tokio_cron_scheduler::{Job, JobScheduler};

            let sched = JobScheduler::new().await?;
            let state_for_demo = state.clone();
            let demo_for_cron = demo_state.clone();

            sched
                .add(Job::new_async("0 0 */6 * * *", move |_uuid, _lock| {
                    let state = state_for_demo.clone();
                    let demo = demo_for_cron.clone();
                    Box::pin(async move {
                        info!("Running scheduled demo data reset...");

                        // Mark reset in progress
                        if let Ok(mut ds) = demo.lock() {
                            ds.reset_in_progress = true;
                        }

                        // Clear all data
                        let state_guard = state.write().await;
                        if let Err(e) = state_guard.db.clear_all_data().await {
                            tracing::error!("Demo auto-reset: failed to clear data: {e}");
                            if let Ok(mut ds) = demo.lock() {
                                ds.reset_in_progress = false;
                            }
                            return;
                        }

                        // Re-create admin user and sample lot
                        if let Err(e) =
                            create_admin_user(&state_guard.db, &state_guard.config).await
                        {
                            tracing::error!("Demo auto-reset: failed to create admin: {e}");
                        }
                        if let Err(e) = create_sample_parking_lot(&state_guard.db).await {
                            tracing::error!("Demo auto-reset: failed to create sample lot: {e}");
                        }
                        // Re-enable credits
                        let _ = state_guard.db.set_setting("credits_enabled", "true").await;
                        let _ = state_guard.db.set_setting("credits_per_booking", "1").await;

                        // Re-seed demo data using the native Rust function — no shell or
                        // external script required (works in distroless containers).
                        if let Err(e) = seed_demo_data(&state_guard.db).await {
                            tracing::warn!("Demo auto-reset: seeding failed (non-fatal): {e}");
                        }
                        drop(state_guard);

                        // Mark reset complete and update timestamps
                        if let Ok(mut ds) = demo.lock() {
                            ds.reset();
                            ds.mark_reset_complete();
                        }

                        info!("Demo auto-reset complete");
                    })
                })?)
                .await?;
            sched.start().await?;
            info!(
                "Demo auto-reset scheduler started (runs every {}h)",
                demo::AUTO_RESET_INTERVAL_HOURS
            );
        }
    }

    // Periodic metrics gauge update (every 5 minutes) — lot occupancy, active bookings
    {
        use tokio_cron_scheduler::{Job, JobScheduler};

        let sched = JobScheduler::new().await?;
        let state_for_metrics = state.clone();
        sched
            .add(Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
                let state = state_for_metrics.clone();
                Box::pin(async move {
                    let state_guard = state.read().await;
                    // Active bookings count
                    let now = chrono::Utc::now();
                    if let Ok(bookings) = state_guard.db.list_bookings().await {
                        let active = bookings
                            .iter()
                            .filter(|b| {
                                b.status == parkhub_common::BookingStatus::Confirmed
                                    && b.start_time <= now
                                    && b.end_time >= now
                            })
                            .count();
                        metrics::record_active_bookings(active as u64);
                    }
                    // Registered users count
                    if let Ok(users) = state_guard.db.list_users().await {
                        metrics::record_registered_users(users.len() as u64);
                    }
                    // Lot occupancy
                    if let Ok(lots) = state_guard.db.list_parking_lots().await {
                        for lot in &lots {
                            #[allow(clippy::cast_sign_loss)] // values are clamped to >= 0
                            let total = lot.total_slots.max(0) as u64;
                            #[allow(clippy::cast_sign_loss)]
                            let occupied = (lot.total_slots - lot.available_slots).max(0) as u64;
                            metrics::record_lot_occupancy(
                                &lot.id.to_string(),
                                &lot.name,
                                total,
                                occupied,
                            );
                        }
                    }
                })
            })?)
            .await?;
        sched.start().await?;
        info!("Metrics gauge updater started (runs every 5 minutes)");
    }

    // Start background jobs (AutoRelease, ExpandRecurring, PurgeExpired, AggregateOccupancy)
    #[cfg(feature = "mod-jobs")]
    jobs::start_background_jobs(state.clone());

    // Show status GUI or wait for shutdown signal
    #[cfg(feature = "gui")]
    if cli.headless {
        // Headless mode requested via CLI
        info!("Server running in headless mode. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await?;
        info!("Shutting down...");
    } else {
        match run_status_gui(config, state, data_dir).await {
            Ok(()) => {}
            Err(e) => {
                tracing::error!("GUI error: {}", e);
                // Fall back to headless mode on GUI error
                info!("Falling back to headless mode due to GUI error");
                info!("Server running. Press Ctrl+C to stop.");
                tokio::signal::ctrl_c().await?;
                info!("Shutting down...");
            }
        }
    }

    #[cfg(not(feature = "gui"))]
    {
        // Headless mode - wait forever
        info!("Server running in headless mode. Press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await?;
        info!("Shutting down...");
    }

    // Trigger graceful shutdown — HTTP server will drain in-flight connections
    let _ = shutdown_tx.send(());
    info!("Graceful shutdown initiated, waiting for connections to drain...");
    // Give the server a moment to finish draining
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Ok(())
}
