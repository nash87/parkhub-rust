//! Slint-based server status window plus Windows system-tray glue.
//!
//! Only compiled when the `gui` feature is on. The tray-icon helpers at
//! the bottom (`create_tray_icon_data`, `is_letter_p`) are additionally
//! gated on `target_os = "windows"` because `tray_icon` is
//! Windows-only in this build.

#![cfg(feature = "gui")]

use anyhow::{Context, Result};
use slint::ComponentHandle;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::AppState;
use crate::config::ServerConfig;
use crate::{ServerStatus, ThemeSettings};

use super::paths::get_local_ip;

// System tray support (Windows only)
#[cfg(all(feature = "gui", windows))]
use tray_icon::{
    Icon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

/// Run the server status GUI with system tray support
#[allow(clippy::too_many_lines, clippy::unused_async)]
pub(crate) async fn run_status_gui(
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
                if let Ok(state) = state_clone.try_read()
                    && let Ok(stats) = state.db.stats().await
                {
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
