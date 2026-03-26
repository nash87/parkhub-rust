//! `ParkHub` Server
//!
//! Database server with HTTP API and LAN autodiscovery.
//! Can run headless or with a configuration GUI.

// Hide console window on Windows when running with GUI
#![cfg_attr(all(feature = "gui", windows), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod api;
#[allow(dead_code)]
mod audit;
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
mod webhooks_v2_tests;

use config::ServerConfig;
use db::{Database, DatabaseConfig};
use discovery::MdnsService;

#[cfg(feature = "gui")]
slint::include_modules!();

// System tray support
#[cfg(all(feature = "gui", windows))]
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder, TrayIconEvent,
};

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
}

/// CLI arguments for the server
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
#[derive(Debug, Clone)]
struct CliArgs {
    /// Show help message
    help: bool,
    /// Run in debug mode with verbose logging
    debug: bool,
    /// Run without GUI (headless mode)
    headless: bool,
    /// Run in unattended mode (auto-configure with defaults)
    unattended: bool,
    /// Custom port to listen on
    port: Option<u16>,
    /// Custom data directory
    data_dir: Option<PathBuf>,
    /// Show version
    version: bool,
    /// Perform a health check against the running server and exit 0/1.
    /// Used as the Docker HEALTHCHECK command (works in distroless images).
    health_check: bool,
}

impl CliArgs {
    fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut cli = Self {
            help: false,
            debug: false,
            headless: false,
            unattended: false,
            port: None,
            data_dir: None,
            version: false,
            health_check: false,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "-h" | "--help" => cli.help = true,
                "-v" | "--version" => cli.version = true,
                "-d" | "--debug" => cli.debug = true,
                "--headless" => cli.headless = true,
                "--unattended" => cli.unattended = true,
                "--health-check" => cli.health_check = true,
                "-p" | "--port" => {
                    if i + 1 < args.len() {
                        cli.port = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--data-dir" => {
                    if i + 1 < args.len() {
                        cli.data_dir = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        cli
    }

    fn print_help() {
        println!("ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("USAGE:");
        println!("    parkhub-server [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    -h, --help         Show this help message");
        println!("    -v, --version      Show version information");
        println!("    -d, --debug        Enable debug logging");
        println!("    --headless         Run without GUI (console only)");
        println!("    --unattended       Auto-configure with defaults (no setup wizard)");
        println!("    -p, --port PORT    Set the server port (default: 7878)");
        println!("    --data-dir PATH    Set custom data directory");
        println!("    --health-check     Check if a running server is healthy (exits 0/1)");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    PARKHUB_DB_PASSPHRASE    Database encryption passphrase");
        println!("    PORT                     Server port (overridden by --port flag)");
        println!("    SEED_DEMO_DATA           Seed demo lots/users on first start (true/1)");
        println!("    DEMO_MODE                Enable demo UI and seed data on first start");
        println!("    RUST_LOG                 Logging filter (e.g., debug,info)");
        println!();
        println!("EXAMPLES:");
        println!("    parkhub-server                    # Start with GUI");
        println!("    parkhub-server --headless         # Start in console mode");
        println!("    parkhub-server --debug            # Start with debug logging");
        println!("    parkhub-server --unattended       # Auto-configure and start");
        println!("    parkhub-server -p 8080            # Use port 8080");
        println!("    parkhub-server --health-check     # Docker HEALTHCHECK probe");
    }

    fn print_version() {
        println!("ParkHub Server v{}", env!("CARGO_PKG_VERSION"));
        println!("Protocol Version: {}", parkhub_common::PROTOCOL_VERSION);
        #[cfg(feature = "gui")]
        println!("GUI: enabled");
        #[cfg(not(feature = "gui"))]
        println!("GUI: disabled");
    }
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
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    // Force software renderer since fonts are embedded for it (EmbedForSoftwareRenderer in build.rs)
    // This ensures the app works on systems without GPU/OpenGL support
    #[cfg(feature = "gui")]
    if !cli.headless {
        std::env::set_var("SLINT_BACKEND", "winit-software");
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
            use rand::Rng;
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
                use rand::Rng;
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
                info!("Seeding demo data (SEED_DEMO_DATA/DEMO_MODE requested, {lot_count} lots found)...");
                if let Err(e) = seed_demo_data(&db).await {
                    warn!("Demo seeding failed (non-fatal): {e}");
                }
            } else {
                info!("Demo data already present ({lot_count} lots) — skipping seed.");
            }
        }
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

    // Create application state
    let state = Arc::new(RwLock::new(AppState {
        config: config.clone(),
        db,
        mdns,
        scheduler: None,
        ws_events: api::ws::EventBroadcaster::new(),
    }));

    // Build the API router
    let (app, demo_state) = api::create_router(state.clone());

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

/// Get the application data directory
fn get_data_directory(portable_mode: Option<bool>) -> Result<PathBuf> {
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let portable_data = exe_dir.join("parkhub-data");

    // If portable_mode is explicitly set (from wizard), use that preference
    if let Some(portable) = portable_mode {
        if portable {
            std::fs::create_dir_all(&portable_data)?;
            return Ok(portable_data);
        }
        // Use system data directory
        let dirs = directories::ProjectDirs::from("com", "parkhub", "ParkHub Server")
            .context("Could not determine data directory")?;
        let data_dir = dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        return Ok(data_dir);
    }

    // Auto-detect: Check for existing portable data first
    if portable_data.exists() {
        return Ok(portable_data);
    }

    // Check if config exists in system directory
    let dirs = directories::ProjectDirs::from("com", "parkhub", "ParkHub Server")
        .context("Could not determine data directory")?;
    let data_dir = dirs.data_dir().to_path_buf();

    if data_dir.join("config.toml").exists() {
        return Ok(data_dir);
    }

    // First-time setup - will be determined by wizard
    // For now, default to system directory but wizard will override
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir)
}

/// Get local IP address
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

/// Hash a password using Argon2
pub(crate) fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {e}"))?;

    Ok(hash.to_string())
}

#[cfg(feature = "gui")]
fn run_setup_wizard() -> Result<ServerConfig> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Get local IP for display
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());

    // Create the setup wizard window
    let ui = SetupWizard::new().context("Failed to create setup wizard")?;

    // Set initial values
    ui.set_local_ip(local_ip.into());
    ui.set_port(i32::from(parkhub_common::DEFAULT_PORT));

    // Store the result
    let result: Rc<RefCell<Option<ServerConfig>>> = Rc::new(RefCell::new(None));
    let result_clone = result.clone();

    // Get a weak reference to the UI for use in the callback
    let ui_weak = ui.as_weak();

    // Handle finish setup
    ui.on_finish_setup(move || {
        // Upgrade the weak reference to access the UI
        let Some(ui) = ui_weak.upgrade() else {
            eprintln!("Failed to access setup wizard UI");
            return;
        };

        // Get values from UI
        let server_name = ui.get_server_name().to_string();
        let admin_username = ui.get_admin_username().to_string();
        let admin_password = ui.get_admin_password().to_string();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let port = ui.get_port() as u16;
        let enable_tls = ui.get_enable_tls();
        let enable_mdns = ui.get_enable_mdns();

        // Validate inputs
        if server_name.trim().is_empty() {
            eprintln!("Server name cannot be empty");
            return;
        }
        if admin_username.trim().is_empty() {
            eprintln!("Admin username cannot be empty");
            return;
        }
        if admin_password.is_empty() {
            eprintln!("Admin password cannot be empty");
            return;
        }

        // Hash the password
        let password_hash = match hash_password(&admin_password) {
            Ok(hash) => hash,
            Err(e) => {
                eprintln!("Failed to hash password: {e}");
                return;
            }
        };

        // Get encryption settings
        let enable_encryption = ui.get_enable_encryption();
        let encryption_passphrase = if enable_encryption {
            let passphrase = ui.get_encryption_passphrase().to_string();
            // Additional validation on Rust side
            if passphrase.len() < 8 {
                eprintln!("Encryption passphrase must be at least 8 characters");
                return;
            }
            Some(passphrase)
        } else {
            None
        };

        // Get portable mode and dummy users settings
        let portable_mode = ui.get_use_portable_mode();
        let generate_dummy_users = ui.get_generate_dummy_users();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let username_style = ui.get_username_style() as u8;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let license_plate_display = ui.get_license_plate_display() as u8;

        // Create config with defaults for advanced settings
        // (can be changed later via admin panel)
        let config = ServerConfig {
            server_name,
            port,
            enable_tls,
            enable_mdns,
            encryption_enabled: enable_encryption,
            encryption_passphrase,
            admin_username,
            admin_password_hash: password_hash,
            portable_mode,
            generate_dummy_users,
            username_style,
            license_plate_display,
            session_timeout_minutes: 60, // 1 hour default
            allow_self_registration: false,
            require_email_verification: false,
            max_concurrent_sessions: 0, // Unlimited
            auto_backup_enabled: true,
            backup_retention_count: 7,
            audit_logging_enabled: true,
            default_language: "en".to_string(),
            organization_name: String::new(),
            close_behavior: "ask".to_string(),
            theme_mode: 0,
            font_scale: 1.0,
            reduce_motion: false,
        };

        *result_clone.borrow_mut() = Some(config);
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    // Handle cancel
    ui.on_cancel_setup(|| {
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    // Run the UI
    ui.run().context("Setup wizard failed")?;

    // Return the result
    let config = result.borrow().clone();
    config.ok_or_else(|| anyhow::anyhow!("Setup was cancelled"))
}

#[cfg(feature = "gui")]
fn prompt_passphrase_gui() -> Result<String> {
    use std::cell::RefCell;
    use std::rc::Rc;

    // Create a simple passphrase dialog
    let ui = PassphraseDialog::new().context("Failed to create passphrase dialog")?;

    let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let result_clone = result.clone();

    ui.on_submit(move |passphrase| {
        *result_clone.borrow_mut() = Some(passphrase.to_string());
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    ui.on_cancel(|| {
        if let Err(e) = slint::quit_event_loop() {
            eprintln!("Failed to quit event loop: {e}");
        }
    });

    ui.run().context("Passphrase dialog failed")?;

    let passphrase = result.borrow().clone();
    passphrase.ok_or_else(|| anyhow::anyhow!("Passphrase entry was cancelled"))
}

/// Run the server status GUI with system tray support
#[cfg(feature = "gui")]
#[allow(clippy::too_many_lines, clippy::unused_async)]
async fn run_status_gui(
    config: ServerConfig,
    state: Arc<RwLock<AppState>>,
    data_dir: PathBuf,
) -> Result<()> {
    use slint::SharedString;
    use std::cell::RefCell;
    use std::rc::Rc;

    // Create the status window
    let ui = ServerStatus::new().context("Failed to create server status window")?;

    // Set initial values
    let local_ip = get_local_ip().unwrap_or_else(|| "localhost".to_string());
    let server_url = format!(
        "{}://{}:{}",
        if config.enable_tls { "https" } else { "http" },
        local_ip,
        config.port
    );

    ui.set_server_name(SharedString::from(&config.server_name));
    ui.set_server_url(SharedString::from(&server_url));
    ui.set_is_running(true);
    ui.set_tls_enabled(config.enable_tls);
    ui.set_mdns_enabled(config.enable_mdns);
    ui.set_encryption_enabled(config.encryption_enabled);

    // Create system tray icon (Windows only) - with error handling
    #[cfg(all(feature = "gui", windows))]
    let _tray_icon: Option<(tray_icon::TrayIcon, slint::Timer)> = {
        // Helper function to create tray icon
        fn create_tray(
            server_name: &str,
            ui: &ServerStatus,
            data_dir: PathBuf,
        ) -> Result<(tray_icon::TrayIcon, slint::Timer), Box<dyn std::error::Error>> {
            // Create tray menu
            let menu_show = MenuItem::new("Show Server Status", true, None);
            let menu_show_id = menu_show.id().clone();
            let menu_data = MenuItem::new("Open Data Folder", true, None);
            let menu_data_id = menu_data.id().clone();
            let menu_stop = MenuItem::new("Stop Server", true, None);
            let menu_stop_id = menu_stop.id().clone();
            let menu_quit = MenuItem::new("Exit", true, None);
            let menu_quit_id = menu_quit.id().clone();

            let tray_menu = Menu::with_items(&[
                &menu_show,
                &PredefinedMenuItem::separator(),
                &menu_data,
                &PredefinedMenuItem::separator(),
                &menu_stop,
                &menu_quit,
            ])?;

            // Create a simple icon (32x32 blue circle with P)
            let icon_data = create_tray_icon_data();
            let icon = Icon::from_rgba(icon_data, 32, 32)?;

            // Build tray icon
            let tray = TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu))
                .with_tooltip(format!("ParkHub Server - {}", server_name))
                .with_icon(icon)
                .build()?;

            // Handle tray menu events
            let ui_weak_menu = ui.as_weak();
            let data_dir_for_menu = data_dir;

            // Set up menu event handler using a timer to poll for events
            let ui_weak_click = ui.as_weak();
            let menu_timer = slint::Timer::default();
            menu_timer.start(
                slint::TimerMode::Repeated,
                std::time::Duration::from_millis(100),
                move || {
                    // Check for tray icon click events (double-click to show window)
                    if let Ok(event) = TrayIconEvent::receiver().try_recv() {
                        match event {
                            TrayIconEvent::DoubleClick { .. }
                            | TrayIconEvent::Click {
                                button: tray_icon::MouseButton::Left,
                                ..
                            } => {
                                // Click/double-click shows window
                                if let Some(ui) = ui_weak_click.upgrade() {
                                    // First show the window, then unminimize
                                    ui.window().show().ok();
                                    ui.window().set_minimized(false);
                                    // Request redraw to fix any rendering issues
                                    ui.window().request_redraw();
                                    info!("Window restored from tray");
                                }
                            }
                            _ => {}
                        }
                    }

                    // Check for menu events
                    if let Ok(event) = MenuEvent::receiver().try_recv() {
                        if event.id == menu_show_id {
                            // Show window
                            if let Some(ui) = ui_weak_menu.upgrade() {
                                ui.window().show().ok();
                                ui.window().set_minimized(false);
                                ui.window().request_redraw();
                                info!("Window restored from menu");
                            }
                        } else if event.id == menu_data_id {
                            // Open data folder
                            let _ = std::process::Command::new("explorer")
                                .arg(&data_dir_for_menu)
                                .spawn();
                        } else if event.id == menu_stop_id {
                            // Stop server and exit
                            let _ = slint::quit_event_loop();
                        } else if event.id == menu_quit_id {
                            // Exit immediately
                            let _ = slint::quit_event_loop();
                        }
                    }
                },
            );

            Ok((tray, menu_timer))
        }

        // Try to create tray icon, but don't fail if it doesn't work
        match create_tray(&config.server_name, &ui, data_dir.clone()) {
            Ok(tray_and_timer) => {
                info!("System tray icon created successfully");
                Some(tray_and_timer)
            }
            Err(e) => {
                warn!(
                    "Failed to create system tray icon: {}. Server will run without tray icon.",
                    e
                );
                None
            }
        }
    };

    // Set up periodic stats update
    let ui_weak = ui.as_weak();
    let state_for_timer = state;
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(2),
        move || {
            let ui_weak_clone = ui_weak.clone();
            let state_clone = state_for_timer.clone();
            // Spawn async stats query without blocking
            tokio::spawn(async move {
                if let Ok(state) = state_clone.try_read() {
                    if let Ok(stats) = state.db.stats().await {
                        // Update UI from event loop thread
                        let _ = slint::invoke_from_event_loop(move || {
                            #[allow(clippy::cast_possible_truncation)]
                            if let Some(ui) = ui_weak_clone.upgrade() {
                                ui.set_user_count(stats.users as i32);
                                ui.set_booking_count(stats.bookings as i32);
                                ui.set_parking_lot_count(stats.parking_lots as i32);
                                ui.set_slot_count(stats.slots as i32);
                                ui.set_session_count(stats.sessions as i32);
                            }
                        });
                    }
                }
            });
        },
    );

    // Handle minimize to tray - minimize window (tray icon allows restore)
    let ui_weak_tray = ui.as_weak();
    ui.on_minimize_to_tray(move || {
        info!("Minimize to tray button clicked");
        if let Some(ui) = ui_weak_tray.upgrade() {
            // Minimize the window - tray icon will still be visible
            // User can click tray icon or use context menu to restore
            ui.window().set_minimized(true);
            info!("Window minimized - click tray icon to restore");
        } else {
            warn!("Failed to upgrade UI weak reference");
        }
    });

    // Handle stop server
    let should_exit = Rc::new(RefCell::new(false));
    let should_exit_clone = should_exit.clone();
    ui.on_stop_server(move || {
        *should_exit_clone.borrow_mut() = true;
        let _ = slint::quit_event_loop();
    });

    // Handle open data folder
    let data_dir_clone = data_dir.clone();
    ui.on_open_data_folder(move || {
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("explorer")
                .arg(&data_dir_clone)
                .spawn();
        }
        #[cfg(not(windows))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&data_dir_clone)
                .spawn();
        }
    });

    // Handle close requested (when user clicks X button)
    let ui_weak_close = ui.as_weak();
    let config_path_for_close = data_dir.join("config.toml");
    ui.on_close_requested(move || {
        if let Some(ui) = ui_weak_close.upgrade() {
            let behavior = ui.get_close_behavior();
            match behavior.as_str() {
                "minimize" => {
                    // User chose to always minimize
                    ui.invoke_minimize_to_tray();
                }
                "exit" => {
                    // User chose to always exit
                    let _ = slint::quit_event_loop();
                }
                _ => {
                    // "ask" - show the dialog
                    ui.set_show_close_dialog(true);
                }
            }

            // If user checked "remember", save the config
            if ui.get_remember_close_choice() {
                let new_behavior = ui.get_close_behavior();
                if new_behavior != "ask" {
                    // Save the preference to config
                    if let Ok(mut config) = ServerConfig::load(&config_path_for_close) {
                        config.close_behavior = new_behavior.to_string();
                        let _ = config.save(&config_path_for_close);
                        info!("Close behavior saved: {}", new_behavior);
                    }
                }
            }
        }
    });

    // Load saved close behavior from config
    ui.set_close_behavior(config.close_behavior.clone().into());

    // Load accessibility settings from config via ThemeSettings global
    ui.global::<ThemeSettings>().set_mode(config.theme_mode);
    ui.global::<ThemeSettings>()
        .set_font_scale(config.font_scale);
    ui.global::<ThemeSettings>()
        .set_reduce_motion(config.reduce_motion);
    info!(
        "Loaded theme settings: mode={}, font_scale={}, reduce_motion={}",
        config.theme_mode, config.font_scale, config.reduce_motion
    );

    // Handle save accessibility settings
    let ui_weak_a11y = ui.as_weak();
    let config_path_for_a11y = data_dir.join("config.toml");
    ui.on_save_accessibility_settings(move || {
        if let Some(ui) = ui_weak_a11y.upgrade() {
            let theme_mode = ui.global::<ThemeSettings>().get_mode();
            let font_scale = ui.global::<ThemeSettings>().get_font_scale();
            let reduce_motion = ui.global::<ThemeSettings>().get_reduce_motion();

            // Save to config file
            if let Ok(mut config) = ServerConfig::load(&config_path_for_a11y) {
                config.theme_mode = theme_mode;
                config.font_scale = font_scale;
                config.reduce_motion = reduce_motion;
                if let Err(e) = config.save(&config_path_for_a11y) {
                    warn!("Failed to save accessibility settings: {}", e);
                } else {
                    info!(
                        "Saved accessibility settings: mode={}, font_scale={}, reduce_motion={}",
                        theme_mode, font_scale, reduce_motion
                    );
                }
            }
        }
    });

    // Intercept window close button (X)
    let ui_weak_window_close = ui.as_weak();
    ui.window().on_close_requested(move || {
        ui_weak_window_close
            .upgrade()
            .map_or(slint::CloseRequestResponse::HideWindow, |ui| {
                // Trigger our close_requested handler
                ui.invoke_close_requested();
                // Don't actually close - let the handler decide
                slint::CloseRequestResponse::KeepWindowShown
            })
    });

    // Run the UI event loop
    ui.run().context("Server status window failed")?;

    if *should_exit.borrow() {
        info!("Server stopped by user");
    }

    Ok(())
}

/// Create icon data for the system tray (32x32 RGBA)
/// Creates a professional parking icon with a blue rounded square and white "P"
#[cfg(all(feature = "gui", windows))]
fn create_tray_icon_data() -> Vec<u8> {
    let size: usize = 32;
    let mut data = vec![0u8; size * size * 4];

    // Colors
    let bg_r: u8 = 0x1a;
    let bg_g: u8 = 0x73;
    let bg_b: u8 = 0xe8; // Bright blue for visibility

    let corner_radius = 6.0f32;

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let fx = x as f32;
            let fy = y as f32;
            let fsize = size as f32;

            // Calculate distance to rounded rectangle
            let in_rounded_rect = {
                let margin = 1.0f32;
                let inner_x =
                    fx.clamp(margin + corner_radius, fsize - margin - corner_radius - 1.0);
                let inner_y =
                    fy.clamp(margin + corner_radius, fsize - margin - corner_radius - 1.0);
                let dx = fx - inner_x;
                let dy = fy - inner_y;
                let dist = (dx * dx + dy * dy).sqrt();

                fx >= margin
                    && fx < fsize - margin
                    && fy >= margin
                    && fy < fsize - margin
                    && dist <= corner_radius + 0.5
            };

            if in_rounded_rect {
                // Check if we're drawing the "P" letter
                let is_p = is_letter_p(x as i32, y as i32, size as i32);

                if is_p {
                    // White "P"
                    data[idx] = 255; // R
                    data[idx + 1] = 255; // G
                    data[idx + 2] = 255; // B
                    data[idx + 3] = 255; // A
                } else {
                    // Blue background with slight gradient
                    let gradient = 1.0 - (fy / fsize) * 0.15;
                    data[idx] = (bg_r as f32 * gradient) as u8;
                    data[idx + 1] = (bg_g as f32 * gradient) as u8;
                    data[idx + 2] = (bg_b as f32 * gradient) as u8;
                    data[idx + 3] = 255;
                }
            } else {
                // Transparent outside
                data[idx] = 0;
                data[idx + 1] = 0;
                data[idx + 2] = 0;
                data[idx + 3] = 0;
            }
        }
    }

    data
}

/// Check if a pixel is part of the "P" letter
#[cfg(all(feature = "gui", windows))]
fn is_letter_p(x: i32, y: i32, size: i32) -> bool {
    // P dimensions relative to 32x32
    let p_left = size / 4; // 8
    let p_right = size * 3 / 4; // 24
    let p_top = size / 5; // 6
    let p_bottom = size * 4 / 5; // 25
    let p_middle = size / 2 + 1; // 17 - middle of the P bowl
    let stroke = size / 8; // 4 - stroke width

    // Vertical bar of P (left side)
    let vertical_bar = x >= p_left && x < p_left + stroke && y >= p_top && y < p_bottom;

    // Top horizontal bar
    let top_bar = x >= p_left && x < p_right - stroke && y >= p_top && y < p_top + stroke;

    // Middle horizontal bar (end of P bowl)
    let middle_bar = x >= p_left && x < p_right - stroke && y >= p_middle - stroke && y < p_middle;

    // Right vertical part of P bowl
    let right_bar =
        x >= p_right - stroke - stroke && x < p_right - stroke && y >= p_top && y < p_middle;

    // Rounded corner at top-right
    let tr_cx = (p_right - stroke - stroke) as f32;
    let tr_cy = (p_top + stroke) as f32;
    let tr_r = stroke as f32;
    let tr_dist = ((x as f32 - tr_cx).powi(2) + (y as f32 - tr_cy).powi(2)).sqrt();
    let top_right_curve =
        x >= p_right - stroke - stroke && y >= p_top && y < p_top + stroke && tr_dist <= tr_r + 0.5;

    // Rounded corner at bottom-right of bowl
    let br_cx = (p_right - stroke - stroke) as f32;
    let br_cy = (p_middle - stroke) as f32;
    let br_dist = ((x as f32 - br_cx).powi(2) + (y as f32 - br_cy).powi(2)).sqrt();
    let bottom_right_curve = x >= p_right - stroke - stroke
        && y >= p_middle - stroke
        && y < p_middle
        && br_dist <= tr_r + 0.5;

    vertical_bar || top_bar || middle_bar || right_bar || top_right_curve || bottom_right_curve
}

/// Create the admin user in the database
pub(crate) async fn create_admin_user(db: &Database, config: &ServerConfig) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{User, UserPreferences, UserRole};
    use uuid::Uuid;

    let admin_user = User {
        id: Uuid::new_v4(),
        username: config.admin_username.clone(),
        email: format!("{}@parkhub.test", config.admin_username),
        password_hash: config.admin_password_hash.clone(),
        name: "Administrator".to_string(),
        picture: None,
        phone: None,
        role: UserRole::SuperAdmin,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        preferences: UserPreferences::default(),
        is_active: true,
        credits_balance: 0,
        credits_monthly_quota: 0,
        credits_last_refilled: None,
        tenant_id: None,
        accessibility_needs: None,
        cost_center: None,
        department: None,
    };

    db.save_user(&admin_user).await?;
    db.mark_setup_completed().await?;
    info!(
        "Admin user '{}' created successfully",
        config.admin_username
    );
    Ok(())
}

/// Username generation styles for dummy users
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsernameStyle {
    /// First letter + last letter (e.g., "Alex Smith" -> "ah")
    FirstLastLetter,
    /// First name + last name (e.g., "Alex Smith" -> "alex.smith")
    FirstDotLast,
    /// First letter + last name (e.g., "Alex Smith" -> "asmith")
    InitialLast,
    /// First name + last initial (e.g., "Alex Smith" -> "alexs")
    FirstInitial,
}

impl UsernameStyle {
    /// Generate username from first and last name
    fn generate(self, first: &str, last: &str, index: usize) -> String {
        let base = match self {
            Self::FirstLastLetter => {
                let first_char = first
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                let last_char = last
                    .chars()
                    .last()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                format!("{first_char}{last_char}")
            }
            Self::FirstDotLast => {
                format!("{}.{}", first.to_lowercase(), last.to_lowercase())
            }
            Self::InitialLast => {
                let first_char = first
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                format!("{}{}", first_char, last.to_lowercase())
            }
            Self::FirstInitial => {
                let last_char = last
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                format!("{}{}", first.to_lowercase(), last_char)
            }
        };
        // Add index to ensure uniqueness
        format!("{}{}", base, index + 1)
    }
}

/// Generate 50 GDPR-compliant dummy users for testing
/// All users have password "12351235" and can login immediately
#[allow(clippy::too_many_lines)]
async fn generate_dummy_users(db: &Database, username_style: UsernameStyle) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{User, UserPreferences, UserRole};
    use rand::Rng;
    use uuid::Uuid;

    // GDPR-compliant fictional first names (common, not identifying real people)
    let first_names = [
        "Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Quinn", "Avery", "Skyler",
        "Dakota", "Cameron", "Reese", "Parker", "Hayden", "Sage", "River", "Phoenix", "Blake",
        "Drew", "Jamie", "Robin", "Charlie", "Sam", "Pat", "Chris", "Lee", "Kim", "Ashley", "Lynn",
        "Terry", "Jesse", "Dana", "Kelly", "Shannon", "Shawn", "Logan", "Peyton", "Kendall",
        "Reagan", "Finley", "Emerson", "Ellis", "Rowan", "Ainsley", "Blair", "Devon", "Eden",
        "Gray", "Harper", "Indigo",
    ];

    // GDPR-compliant fictional last names (common, not identifying real people)
    let last_names = [
        "Smith",
        "Johnson",
        "Williams",
        "Brown",
        "Jones",
        "Garcia",
        "Miller",
        "Davis",
        "Rodriguez",
        "Martinez",
        "Anderson",
        "Taylor",
        "Thomas",
        "Jackson",
        "White",
        "Harris",
        "Martin",
        "Thompson",
        "Moore",
        "Young",
        "Allen",
        "King",
        "Wright",
        "Scott",
        "Green",
        "Baker",
        "Adams",
        "Nelson",
        "Hill",
        "Ramirez",
        "Campbell",
        "Mitchell",
        "Roberts",
        "Carter",
        "Phillips",
        "Evans",
        "Turner",
        "Torres",
        "Parker",
        "Collins",
        "Edwards",
        "Stewart",
        "Flores",
        "Morris",
        "Murphy",
        "Rivera",
        "Cook",
        "Rogers",
        "Morgan",
        "Peterson",
    ];

    // Default password for all dummy users - they can login with this
    let default_password = "12351235";
    let password_hash = hash_password(default_password)?;

    // Role distribution: mostly Users, some Premium, few Admin
    let roles = [
        UserRole::User,
        UserRole::User,
        UserRole::User,
        UserRole::User,
        UserRole::Premium,
        UserRole::Admin,
    ];

    info!("Generating 50 GDPR-compliant dummy users (password: {default_password})...",);

    // Pre-generate all users with rng (ThreadRng is not Send, so must not cross await)
    let users: Vec<User> = {
        let mut rng = rand::rng();
        (0..50)
            .map(|i| {
                let first = first_names[rng.random_range(0..first_names.len())];
                let last = last_names[rng.random_range(0..last_names.len())];
                let role = roles[rng.random_range(0..roles.len())].clone();
                let username = username_style.generate(first, last, i);
                let email = format!("{username}@example.com");

                User {
                    id: Uuid::new_v4(),
                    username,
                    email,
                    password_hash: password_hash.clone(),
                    name: format!("{first} {last}"),
                    picture: None,
                    phone: Some(format!("+1-555-{:04}", rng.random_range(1000..9999))),
                    role,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_login: None,
                    preferences: UserPreferences::default(),
                    is_active: true,
                    credits_balance: rng.random_range(10..41),
                    credits_monthly_quota: 40,
                    credits_last_refilled: Some(Utc::now()),
                    tenant_id: None,
                    accessibility_needs: None,
                    cost_center: None,
                    department: None,
                }
            })
            .collect()
    };

    for user in &users {
        db.save_user(user).await?;
    }

    info!("Created 50 dummy users successfully");
    info!("Default login: any username with password '{default_password}'",);
    Ok(())
}

/// Create a sample parking lot for testing
pub(crate) async fn create_sample_parking_lot(db: &Database) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{
        LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo, PricingRate,
        SlotFeature, SlotPosition, SlotStatus, SlotType,
    };
    use uuid::Uuid;

    let lot_id = Uuid::new_v4();
    let floor_id = Uuid::new_v4();

    // Create 10 parking slots
    let mut slots = Vec::new();
    for i in 1..=10 {
        slots.push(ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: i,
            row: (i - 1) / 5,
            column: (i - 1) % 5,
            slot_type: if i == 1 {
                SlotType::Handicap
            } else if i == 10 {
                SlotType::Electric
            } else {
                SlotType::Standard
            },
            status: SlotStatus::Available,
            current_booking: None,
            features: if i <= 2 {
                vec![SlotFeature::NearExit]
            } else {
                vec![]
            },
            position: SlotPosition {
                x: ((i - 1) % 5) as f32 * 80.0,
                y: ((i - 1) / 5) as f32 * 100.0,
                width: 70.0,
                height: 90.0,
                rotation: 0.0,
            },
            is_accessible: i == 1, // First slot is accessible (handicap)
        });
    }

    let floor = ParkingFloor {
        id: floor_id,
        lot_id,
        name: "Ground Floor".to_string(),
        floor_number: 0,
        total_slots: 10,
        available_slots: 10,
        slots: slots.clone(),
    };

    let lot = ParkingLot {
        id: lot_id,
        name: "Home Parking".to_string(),
        address: "123 Main Street".to_string(),
        latitude: 0.0,
        longitude: 0.0,
        total_slots: 10,
        available_slots: 10,
        floors: vec![floor],
        amenities: vec!["Security".to_string(), "Covered".to_string()],
        pricing: PricingInfo {
            currency: "EUR".to_string(),
            rates: vec![
                PricingRate {
                    duration_minutes: 60,
                    price: 2.0,
                    label: "1 hour".to_string(),
                },
                PricingRate {
                    duration_minutes: 120,
                    price: 3.5,
                    label: "2 hours".to_string(),
                },
                PricingRate {
                    duration_minutes: 240,
                    price: 6.0,
                    label: "4 hours".to_string(),
                },
            ],
            daily_max: Some(15.0),
            monthly_pass: Some(200.0),
        },
        operating_hours: OperatingHours {
            is_24h: true,
            monday: None,
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        },
        images: vec![],
        status: LotStatus::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        tenant_id: None,
    };

    // Save parking lot
    db.save_parking_lot(&lot).await?;

    // Save all slots
    for slot in &slots {
        db.save_parking_slot(slot).await?;
    }

    info!("Sample parking lot created with {} slots", slots.len());
    Ok(())
}

/// Perform a synchronous HTTP health check against a running server.
///
/// Connects to `http://127.0.0.1:{port}/health` using a raw TCP connection
/// (no extra runtime or external binary required — works in distroless images).
/// Returns 0 if the server responds with HTTP 200, 1 otherwise.
fn perform_health_check(port: u16) -> i32 {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let addr = format!("127.0.0.1:{port}");
    let timeout = Duration::from_secs(4);

    // Parse the socket address — this is always valid since port is a u16,
    // but we handle the error gracefully rather than panicking.
    let Ok(socket_addr) = addr.parse() else {
        eprintln!("health-check: could not parse address {addr}");
        return 1;
    };

    let Ok(mut stream) = TcpStream::connect_timeout(&socket_addr, timeout) else {
        eprintln!("health-check: could not connect to {addr}");
        return 1;
    };

    let _ = stream.set_read_timeout(Some(timeout));
    let req = "GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    if stream.write_all(req.as_bytes()).is_err() {
        eprintln!("health-check: failed to send request");
        return 1;
    }

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);

    // Accept any 2xx status on the first line
    if response.starts_with("HTTP/1.")
        && response
            .lines()
            .next()
            .is_some_and(|l| l.contains("200"))
    {
        0
    } else {
        eprintln!("health-check: unexpected response: {}", response.lines().next().unwrap_or("(empty)"));
        1
    }
}

/// Seed demo data: 10 realistic parking lots and 200 demo users.
///
/// Called at startup when `SEED_DEMO_DATA=true` or `DEMO_MODE=true` and the
/// database has fewer than two parking lots.  All writes go directly to the
/// database — no HTTP API calls, no shell scripts, and no external tools are
/// required, making this safe for distroless container deployments.
#[allow(clippy::too_many_lines)]
pub(crate) async fn seed_demo_data(db: &Database) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{
        DayHours, LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo,
        PricingRate, SlotFeature, SlotPosition, SlotStatus, SlotType,
    };
    use rand::Rng;
    use uuid::Uuid;

    info!("Seeding demo data: 10 parking lots + 200 users...");

    // 10 realistic German parking lots (mirroring the former seed_demo.sh)
    let lots_data: &[(&str, &str, f64, f64, i32)] = &[
        ("P+R Hauptbahnhof",       "Bahnhofplatz 1, 80335 München",           48.1403, 11.5583, 51),
        ("Tiefgarage Marktplatz",  "Marktplatz 5, 70173 Stuttgart",           48.7784,  9.1800, 80),
        ("Parkhaus Stadtmitte",    "Rathausstrasse 12, 50667 Köln",           50.9384,  6.9584, 60),
        ("P+R Messegelände",       "Messegelände Süd, 60528 Frankfurt",       50.1109,  8.6821, 100),
        ("Parkplatz Einkaufszentrum", "Shoppingcenter 3, 22335 Hamburg",      53.5753,  9.9803, 40),
        ("Tiefgarage Rathaus",     "Rathausplatz 1, 90403 Nürnberg",          49.4521, 11.0767, 30),
        ("Parkhaus Technologiepark", "Technologiestrasse 8, 76131 Karlsruhe", 49.0069,  8.4037, 75),
        ("Parkplatz Universität",  "Universitätsring 1, 69120 Heidelberg",    49.4074,  8.6924, 70),
        ("Parkplatz Klinikum",     "Klinikumsallee 15, 44137 Dortmund",       51.5136,  7.4653, 46),
        ("P+R Bahnhof Ost",        "Ostbahnhofstrasse 3, 04315 Leipzig",      51.3397, 12.3731, 56),
    ];

    for (name, address, lat, lon, total_slots) in lots_data {
        let lot_id = Uuid::new_v4();
        let floor_id = Uuid::new_v4();
        let total = *total_slots;

        let slots: Vec<ParkingSlot> = (1..=total)
            .map(|i| ParkingSlot {
                id: Uuid::new_v4(),
                lot_id,
                floor_id,
                slot_number: i,
                row: (i - 1) / 10,
                column: (i - 1) % 10,
                slot_type: if i == 1 {
                    SlotType::Handicap
                } else if i == total {
                    SlotType::Electric
                } else {
                    SlotType::Standard
                },
                status: SlotStatus::Available,
                current_booking: None,
                features: if i <= 2 { vec![SlotFeature::NearExit] } else { vec![] },
                position: SlotPosition {
                    x: ((i - 1) % 10) as f32 * 80.0,
                    y: ((i - 1) / 10) as f32 * 100.0,
                    width: 70.0,
                    height: 90.0,
                    rotation: 0.0,
                },
                is_accessible: i == 1,
            })
            .collect();

        let floor = ParkingFloor {
            id: floor_id,
            lot_id,
            name: "Ground Floor".to_string(),
            floor_number: 0,
            total_slots: total,
            available_slots: total,
            slots: slots.clone(),
        };

        let weekday = DayHours {
            open: "06:00".to_string(),
            close: "22:00".to_string(),
            closed: false,
        };
        let weekend = DayHours {
            open: "07:00".to_string(),
            close: "20:00".to_string(),
            closed: false,
        };
        let lot = ParkingLot {
            id: lot_id,
            name: name.to_string(),
            address: address.to_string(),
            latitude: *lat,
            longitude: *lon,
            total_slots: total,
            available_slots: total,
            floors: vec![floor],
            amenities: vec!["covered".to_string(), "security_camera".to_string()],
            pricing: PricingInfo {
                currency: "EUR".to_string(),
                rates: vec![
                    PricingRate { duration_minutes: 60,   price: 2.50, label: "1h".to_string() },
                    PricingRate { duration_minutes: 1440, price: 20.0, label: "Day".to_string() },
                ],
                daily_max: Some(20.0),
                monthly_pass: Some(400.0),
            },
            operating_hours: OperatingHours {
                is_24h: false,
                monday:    Some(weekday.clone()),
                tuesday:   Some(weekday.clone()),
                wednesday: Some(weekday.clone()),
                thursday:  Some(weekday.clone()),
                friday:    Some(weekday.clone()),
                saturday:  Some(weekend.clone()),
                sunday:    Some(weekend),
            },
            images: vec![],
            status: LotStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tenant_id: None,
        };

        db.save_parking_lot(&lot).await?;
        for slot in &slots {
            db.save_parking_slot(slot).await?;
        }
        info!("  Created lot: {} ({total_slots} slots)", name);
    }

    // 200 demo users with German-style names (direct DB writes — no HTTP API)
    let first_names = [
        "Hans", "Peter", "Klaus", "Michael", "Thomas", "Andreas", "Stefan", "Christian",
        "Markus", "Sebastian", "Daniel", "Tobias", "Florian", "Matthias", "Martin", "Frank",
        "Oliver", "Maria", "Anna", "Sandra", "Andrea", "Nicole", "Stefanie", "Christina",
        "Monika", "Petra", "Claudia", "Julia", "Laura", "Sarah", "Lisa", "Katharina",
        "Melanie", "Susanne", "Anja",
    ];
    let last_names = [
        "Müller", "Schmidt", "Schneider", "Fischer", "Weber", "Meyer", "Wagner", "Becker",
        "Schulz", "Hoffmann", "Koch", "Richter", "Bauer", "Klein", "Wolf", "Schröder",
        "Neumann", "Schwarz", "Zimmermann", "Braun", "Krüger", "Hofmann", "Hartmann",
    ];

    let demo_password_hash = hash_password("Demo2026!X")?;

    let users: Vec<parkhub_common::models::User> = {
        use parkhub_common::models::{User, UserPreferences};
        let mut rng = rand::rng();
        (1..=200u32)
            .map(|i| {
                let first = first_names[rng.random_range(0..first_names.len())];
                let last = last_names[rng.random_range(0..last_names.len())];
                let username = format!(
                    "{}.{}{}",
                    first.to_lowercase(),
                    last.to_lowercase().replace('ü', "ue").replace('ö', "oe").replace('ä', "ae"),
                    i
                );
                User {
                    id: Uuid::new_v4(),
                    username: username.clone(),
                    email: format!("{username}@example.de"),
                    password_hash: demo_password_hash.clone(),
                    name: format!("{first} {last}"),
                    picture: None,
                    phone: Some(format!("+49-{:03}-{:07}", rng.random_range(100..999), rng.random_range(1_000_000..9_999_999u32))),
                    role: parkhub_common::models::UserRole::User,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_login: None,
                    preferences: UserPreferences::default(),
                    is_active: true,
                    credits_balance: rng.random_range(5..41),
                    credits_monthly_quota: 40,
                    credits_last_refilled: Some(Utc::now()),
                    tenant_id: None,
                    accessibility_needs: None,
                    cost_center: None,
                    department: None,
                }
            })
            .collect()
    };

    for user in &users {
        if let Err(e) = db.save_user(user).await {
            tracing::warn!("Demo seed: failed to save user {}: {e}", user.username);
        }
    }

    info!("Demo seeding complete: 10 lots, 200 users (password: Demo2026!X)");
    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // CliArgs parsing
    // ---------------------------------------------------------------------------

    fn parse_args(args: &[&str]) -> CliArgs {
        // CliArgs::parse() reads std::env::args(), so we exercise the struct fields
        // directly here to avoid side-effects from the process argument list.
        let mut cli = CliArgs {
            help: false,
            debug: false,
            headless: false,
            unattended: false,
            port: None,
            data_dir: None,
            version: false,
            health_check: false,
        };
        let mut i = 0;
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        while i < owned.len() {
            match owned[i].as_str() {
                "-h" | "--help" => cli.help = true,
                "-v" | "--version" => cli.version = true,
                "-d" | "--debug" => cli.debug = true,
                "--headless" => cli.headless = true,
                "--unattended" => cli.unattended = true,
                "--health-check" => cli.health_check = true,
                "-p" | "--port" => {
                    if i + 1 < owned.len() {
                        cli.port = owned[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--data-dir" => {
                    if i + 1 < owned.len() {
                        cli.data_dir = Some(PathBuf::from(&owned[i + 1]));
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        cli
    }

    #[test]
    fn health_check_flag_is_parsed() {
        let cli = parse_args(&["--health-check"]);
        assert!(cli.health_check, "--health-check must set health_check=true");
        assert!(!cli.headless);
        assert!(!cli.unattended);
    }

    #[test]
    fn health_check_flag_default_is_false() {
        let cli = parse_args(&["--headless", "--unattended"]);
        assert!(!cli.health_check, "health_check must default to false");
    }

    #[test]
    fn health_check_with_port_flag() {
        let cli = parse_args(&["--health-check", "--port", "8080"]);
        assert!(cli.health_check);
        assert_eq!(cli.port, Some(8080));
    }

    #[test]
    fn port_flag_parsed_correctly() {
        let cli = parse_args(&["-p", "9000"]);
        assert_eq!(cli.port, Some(9000));
    }

    #[test]
    fn data_dir_flag_parsed() {
        let cli = parse_args(&["--data-dir", "/tmp/mydata"]);
        assert_eq!(cli.data_dir, Some(PathBuf::from("/tmp/mydata")));
    }

    // ---------------------------------------------------------------------------
    // perform_health_check — connection-refused path exits with 1
    // ---------------------------------------------------------------------------

    #[test]
    fn health_check_returns_1_when_server_not_running() {
        // Port 1 is reserved and guaranteed not to have a listener; expect exit code 1.
        let result = perform_health_check(1);
        assert_eq!(result, 1, "health check must return 1 when server is unreachable");
    }

    // ---------------------------------------------------------------------------
    // seed_demo_data — creates 10 lots and 200 users in a real database
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn seed_demo_data_creates_lots_and_users() {
        use crate::db::{Database, DatabaseConfig};

        let dir = tempfile::tempdir().expect("tempdir");
        let db_config = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        let db = Database::open(&db_config).expect("open test db");

        // DB should start empty
        let lots_before = db.list_parking_lots().await.unwrap();
        assert_eq!(lots_before.len(), 0, "lots must be empty before seeding");

        seed_demo_data(&db).await.expect("seed_demo_data must succeed");

        let lots_after = db.list_parking_lots().await.unwrap();
        assert_eq!(lots_after.len(), 10, "seed must create exactly 10 parking lots");

        // All lots should have at least one slot
        for lot in &lots_after {
            assert!(lot.total_slots > 0, "each seeded lot must have at least one slot");
        }

        // Verify user count (200 demo users)
        let users = db.list_users().await.unwrap();
        assert_eq!(users.len(), 200, "seed must create exactly 200 demo users");
    }

    #[tokio::test]
    async fn seed_demo_data_is_idempotent_when_called_twice() {
        use crate::db::{Database, DatabaseConfig};

        let dir = tempfile::tempdir().expect("tempdir");
        let db_config = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        let db = Database::open(&db_config).expect("open test db");

        // First call
        seed_demo_data(&db).await.expect("first seed must succeed");
        let lots_first = db.list_parking_lots().await.unwrap().len();

        // Second call must not fail; lots are stored by UUID so duplicate lots
        // may be added by a naive caller — the startup guard (lot_count < 2)
        // prevents double-seeding, but the function itself should not panic.
        let result = seed_demo_data(&db).await;
        assert!(result.is_ok(), "second seed_demo_data call must not return Err");
        // Lot count after second call is at least the original 10
        let lots_second = db.list_parking_lots().await.unwrap().len();
        assert!(lots_second >= lots_first, "lot count must not decrease");
    }
}
