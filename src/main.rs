//! Securanido Parking Desktop Application
//!
//! A Windows desktop application for parking lot booking with mock backend.

// CRITICAL: Prevents console window from appearing on Windows
#![windows_subsystem = "windows"]

use anyhow::{Context, Result};
use slint::{Color, ModelRc, VecModel};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

mod api;
mod auth;
mod config;
mod database;
mod layout_storage;
mod mock_api;

use config::{AppConfig, DevUserConfig};
use layout_storage::{
    ElementType as StorageElementType, LayoutElement as StorageLayoutElement, LayoutStorage,
    ParkingLayout,
};
use mock_api::MockParkingApi;

slint::include_modules!();

// ============================================================================
// Application State
// ============================================================================

struct AppState {
    config: AppConfig,
    dev_users: Vec<DevUserConfig>,
    mock_api: MockParkingApi,
    current_user: Option<UserSession>,
    server_mode: String,
    layout_storage: LayoutStorage,
    current_layout: Option<ParkingLayout>,
    layout_elements: Vec<StorageLayoutElement>,
    next_slot_number: i32,
}

#[derive(Debug, Clone)]
struct UserSession {
    id: String,
    email: String,
    name: String,
    role: String,
}

impl AppState {
    fn get_base_url(&self) -> &str {
        if self.server_mode == "local" {
            &self.config.server.local_url
        } else {
            &self.config.server.production_url
        }
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Set DPI awareness before creating any windows (Windows-specific)
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    // Force software renderer for compatibility (avoids OpenGL driver issues)
    std::env::set_var("SLINT_BACKEND", "winit-software");

    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Starting Securanido Parking Desktop v0.1.0");

    // Clean up old screenshots on startup
    {
        let screenshots_dir =
            std::path::PathBuf::from(r"C:\dev\securanido-parking-desktop\screenshots");
        if screenshots_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&screenshots_dir) {
                let mut deleted_count = 0;
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with("screenshot_")
                            && name.ends_with(".png")
                            && std::fs::remove_file(entry.path()).is_ok()
                        {
                            deleted_count += 1;
                        }
                    }
                }
                if deleted_count > 0 {
                    info!("Cleaned up {} old screenshots", deleted_count);
                }
            }
        }
    }

    // Load configuration
    let config = config::load_config().context("Failed to load config")?;
    let dev_users = config::load_dev_users().context("Failed to load dev users")?;

    info!("Loaded {} dev users", dev_users.len());
    info!("Server mode: {}", config.server.active);

    // Initialize mock API
    let mock_api = MockParkingApi::new();

    // Initialize layout storage
    let layout_storage = LayoutStorage::new().context("Failed to initialize layout storage")?;

    // Initialize state
    let state = Arc::new(RwLock::new(AppState {
        server_mode: config.server.active.clone(),
        config,
        dev_users: dev_users.clone(),
        mock_api,
        current_user: None,
        layout_storage,
        current_layout: None,
        layout_elements: Vec::new(),
        next_slot_number: 1,
    }));

    // Create UI
    let app = MainWindow::new()?;
    let app_weak = app.as_weak();

    // ========================================================================
    // Center window on screen (Windows-specific)
    // ========================================================================
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
        use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

        let mut work_area: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work_area as *mut _ as *mut _, 0);
        }

        // Get system DPI (96 is 100%, 144 is 150%, 192 is 200%)
        let dpi = unsafe { GetDpiForSystem() } as f32;
        let scale_factor = dpi / 96.0;

        // Work area is in physical pixels, convert to logical
        let phys_width = (work_area.right - work_area.left) as f32;
        let phys_height = (work_area.bottom - work_area.top) as f32;
        let screen_logical_width = phys_width / scale_factor;
        let screen_logical_height = phys_height / scale_factor;
        let screen_logical_x = work_area.left as f32 / scale_factor;
        let screen_logical_y = work_area.top as f32 / scale_factor;

        // Window size (matching preferred size in main.slint)
        let window_width = 450.0f32;
        let window_height = 800.0f32;

        // Adjust window height if it's too tall for the screen
        let actual_window_height = if window_height > screen_logical_height {
            screen_logical_height - 40.0 // Leave some margin
        } else {
            window_height
        };

        // Calculate centered position
        let center_x = screen_logical_x + (screen_logical_width - window_width) / 2.0;
        let mut center_y = screen_logical_y + (screen_logical_height - actual_window_height) / 2.0;

        // Ensure title bar is always visible (Y >= 0)
        if center_y < 0.0 {
            center_y = 0.0;
        }

        info!(
            "Screen work area: {}x{} physical, {}x{} logical (DPI: {}, scale: {})",
            phys_width, phys_height, screen_logical_width, screen_logical_height, dpi, scale_factor
        );
        info!(
            "Window positioned at ({}, {}) with size {}x{}",
            center_x, center_y, window_width, actual_window_height
        );

        // Set window size and position (centered)
        app.window()
            .set_size(slint::LogicalSize::new(window_width, actual_window_height));
        app.window()
            .set_position(slint::LogicalPosition::new(center_x, center_y));
    }

    // Set initial UI state
    {
        let s = state.read().await;
        app.set_dev_mode_enabled(s.config.development.enabled);
        app.set_server_mode(s.server_mode.clone().into());
        app.set_server_url(s.get_base_url().into());

        // Set dev users
        let dev_users_model: Vec<DevUser> = dev_users
            .iter()
            .map(|u| {
                let color_u32 = u32::from_str_radix(&u.color[1..], 16).unwrap_or(0xFF6366F1);
                let initial = u.name.chars().next().unwrap_or('?').to_string();
                DevUser {
                    id: u.id.clone().into(),
                    email: u.email.clone().into(),
                    name: u.name.clone().into(),
                    initial: initial.into(),
                    role: u.role.clone().into(),
                    color: Color::from_argb_u8(
                        255,
                        ((color_u32 >> 16) & 0xFF) as u8,
                        ((color_u32 >> 8) & 0xFF) as u8,
                        (color_u32 & 0xFF) as u8,
                    ),
                }
            })
            .collect();
        app.set_dev_users(ModelRc::new(VecModel::from(dev_users_model)));

        // Set duration options
        let duration_options = vec![
            DurationOption {
                minutes: 15,
                label: "15m".into(),
            },
            DurationOption {
                minutes: 30,
                label: "30m".into(),
            },
            DurationOption {
                minutes: 60,
                label: "1h".into(),
            },
            DurationOption {
                minutes: 120,
                label: "2h".into(),
            },
            DurationOption {
                minutes: 240,
                label: "4h".into(),
            },
            DurationOption {
                minutes: 480,
                label: "8h".into(),
            },
        ];
        app.set_duration_options(ModelRc::new(VecModel::from(duration_options)));
    }

    // ========================================================================
    // Setup Callbacks
    // ========================================================================

    // Google Login (mock - just show message)
    {
        let app_weak = app_weak.clone();
        app.on_google_login(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_login_error(
                    "Google OAuth not available in mock mode. Use dev login.".into(),
                );
            }
        });
    }

    // Dev Login
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_dev_login(move |user_id| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let user_id = user_id.to_string();

            let _ = slint::spawn_local(async move {
                info!("Dev login attempt for user: {}", user_id);

                let s = state.read().await;

                // Find the dev user
                if let Some(dev_user) = s.dev_users.iter().find(|u| u.id == user_id) {
                    let session = UserSession {
                        id: dev_user.id.clone(),
                        email: dev_user.email.clone(),
                        name: dev_user.name.clone(),
                        role: dev_user.role.clone(),
                    };

                    drop(s);

                    // Store session
                    {
                        let mut s = state.write().await;
                        s.current_user = Some(session.clone());
                    }

                    if let Some(app) = app_weak.upgrade() {
                        let initial = session.name.chars().next().unwrap_or('?').to_string();
                        app.set_current_user(CurrentUser {
                            id: session.id.into(),
                            email: session.email.into(),
                            name: session.name.into(),
                            initial: initial.into(),
                            picture: "".into(),
                            role: session.role.into(),
                        });
                        app.set_is_authenticated(true);
                        app.set_current_view(AppView::Parking);
                        app.set_login_error("".into());

                        // Load parking data
                        load_parking_data(&state, &app_weak).await;
                    }

                    info!("Dev login successful for: {}", user_id);
                } else if let Some(app) = app_weak.upgrade() {
                    app.set_login_error("User not found".into());
                }
            });
        });
    }

    // Toggle Server Mode
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_toggle_server_mode(move || {
            let state = state.clone();
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                let mut s = state.write().await;

                if s.server_mode == "local" {
                    s.server_mode = "production".to_string();
                } else {
                    s.server_mode = "local".to_string();
                }

                let new_url = s.get_base_url().to_string();
                info!("Switched to {} server: {}", s.server_mode, new_url);

                if let Some(app) = app_weak.upgrade() {
                    app.set_server_mode(s.server_mode.clone().into());
                    app.set_server_url(new_url.into());
                }
            });
        });
    }

    // Logout
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_logout(move || {
            let state = state.clone();
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                {
                    let mut s = state.write().await;
                    s.current_user = None;
                }

                if let Some(app) = app_weak.upgrade() {
                    app.set_is_authenticated(false);
                    app.set_current_view(AppView::Login);
                    app.set_current_user(CurrentUser::default());
                    app.set_selected_slot_number(-1);
                    app.set_show_booking_panel(false);
                }

                info!("User logged out");
            });
        });
    }

    // Slot Tapped
    {
        let _state = state.clone();
        let app_weak = app_weak.clone();
        app.on_slot_tapped(move |slot_number| {
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                info!("Slot {} tapped", slot_number);

                // Calculate estimated cost
                if let Some(app) = app_weak.upgrade() {
                    let duration = app.get_selected_duration();
                    let cost = calculate_cost(duration);
                    app.set_estimated_cost(format!("{:.2} EUR", cost).into());
                }
            });
        });
    }

    // Book Slot
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_book_slot(move |slot_number, duration_minutes, license_plate| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let license_plate = license_plate.to_string();

            let _ = slint::spawn_local(async move {
                info!(
                    "Booking slot {} for {} minutes",
                    slot_number, duration_minutes
                );

                if let Some(app) = app_weak.upgrade() {
                    app.set_is_booking(true);
                }

                // Simulate API delay
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Create booking in mock API
                {
                    let mut s = state.write().await;
                    let user_id = s.current_user.as_ref().map(|u| u.id.clone());
                    if let Some(user_id) = user_id {
                        let booking_id = s.mock_api.create_booking(
                            slot_number,
                            duration_minutes,
                            license_plate.clone(),
                            user_id,
                        );
                        info!("Created booking: {}", booking_id);
                    }
                }

                if let Some(app) = app_weak.upgrade() {
                    app.set_is_booking(false);
                    app.set_show_booking_panel(false);
                    app.set_selected_slot_number(-1);
                    app.set_license_plate("".into());
                }

                // Reload parking data
                load_parking_data(&state, &app_weak).await;
            });
        });
    }

    // Cancel Booking
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_cancel_booking(move |booking_id| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let booking_id = booking_id.to_string();

            let _ = slint::spawn_local(async move {
                info!("Cancelling booking: {}", booking_id);

                {
                    let mut s = state.write().await;
                    s.mock_api.cancel_booking(&booking_id);
                }

                // Reload parking data
                load_parking_data(&state, &app_weak).await;
            });
        });
    }

    // Refresh Parking
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_refresh_parking(move || {
            let state = state.clone();
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                info!("Refreshing parking data");
                load_parking_data(&state, &app_weak).await;
            });
        });
    }

    // Parking Tab Changed
    {
        let _app_weak = app_weak.clone();
        app.on_parking_tab_changed(move |tab_index| {
            info!("Switched to parking tab: {}", tab_index);
        });
    }

    // Open Layout Editor
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_open_layout_editor(move || {
            let state = state.clone();
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                info!("Opening layout editor");

                // Load saved layouts list
                if let Some(app) = app_weak.upgrade() {
                    let s = state.read().await;
                    if let Ok(layouts) = s.layout_storage.list_layouts() {
                        let ui_layouts: Vec<SavedLayout> = layouts
                            .iter()
                            .map(|l| SavedLayout {
                                id: l.id.clone().into(),
                                name: l.name.clone().into(),
                                created: l.created.clone().into(),
                                elements_count: l.elements_count,
                                thumbnail: "".into(),
                            })
                            .collect();
                        app.set_saved_layouts(ModelRc::new(VecModel::from(ui_layouts)));
                    }
                    drop(s);

                    app.set_current_view(AppView::LayoutEditor);
                }
            });
        });
    }

    // Editor: Add Element
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_add_element(move |elem_type, x, y| {
            let state = state.clone();
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                info!("Adding element {:?} at ({}, {})", elem_type, x, y);

                let (_width, _height, slot_num) = {
                    let mut s = state.write().await;
                    let is_slot = matches!(
                        elem_type,
                        ElementType::ParkingSlot
                            | ElementType::Handicap
                            | ElementType::Electric
                            | ElementType::Motorcycle
                    );

                    let slot_num = if is_slot {
                        let n = s.next_slot_number;
                        s.next_slot_number += 1;
                        n
                    } else {
                        0
                    };

                    // Default sizes based on element type
                    let (w, h) = match elem_type {
                        ElementType::ParkingSlot
                        | ElementType::Handicap
                        | ElementType::Electric
                        | ElementType::Motorcycle => (80.0, 120.0),
                        ElementType::Wall => (20.0, 100.0),
                        ElementType::Pillar => (20.0, 20.0),
                        ElementType::Entry | ElementType::Exit => (60.0, 40.0),
                        ElementType::Lane => (200.0, 60.0),
                        ElementType::Arrow => (40.0, 40.0),
                    };

                    let element = StorageLayoutElement {
                        id: Uuid::new_v4().to_string(),
                        element_type: convert_element_type(elem_type),
                        x,
                        y,
                        width: w,
                        height: h,
                        rotation: 0.0,
                        slot_number: slot_num,
                        color: get_element_color(elem_type),
                    };

                    s.layout_elements.push(element);
                    (w, h, s.next_slot_number)
                };

                // Update UI
                update_layout_elements_ui(&state, &app_weak).await;

                if let Some(app) = app_weak.upgrade() {
                    app.set_editor_next_slot_number(slot_num);
                }
            });
        });
    }

    // Editor: Select Element
    {
        let app_weak = app_weak.clone();
        app.on_editor_select_element(move |id| {
            if let Some(app) = app_weak.upgrade() {
                app.set_editor_selected_element_id(id);
            }
        });
    }

    // Editor: Move Element
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_move_element(move |id, dx, dy| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let id = id.to_string();

            let _ = slint::spawn_local(async move {
                {
                    let mut s = state.write().await;
                    if let Some(elem) = s.layout_elements.iter_mut().find(|e| e.id == id) {
                        elem.x += dx;
                        elem.y += dy;
                    }
                }
                update_layout_elements_ui(&state, &app_weak).await;
            });
        });
    }

    // Editor: Rotate Element
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_rotate_element(move |id| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let id = id.to_string();

            let _ = slint::spawn_local(async move {
                {
                    let mut s = state.write().await;
                    if let Some(elem) = s.layout_elements.iter_mut().find(|e| e.id == id) {
                        elem.rotation = (elem.rotation + 90.0) % 360.0;
                        // Swap width and height for rotation
                        std::mem::swap(&mut elem.width, &mut elem.height);
                    }
                }
                update_layout_elements_ui(&state, &app_weak).await;
            });
        });
    }

    // Editor: Delete Element
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_delete_element(move |id| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let id = id.to_string();

            let _ = slint::spawn_local(async move {
                {
                    let mut s = state.write().await;
                    s.layout_elements.retain(|e| e.id != id);
                }
                update_layout_elements_ui(&state, &app_weak).await;

                if let Some(app) = app_weak.upgrade() {
                    app.set_editor_selected_element_id("".into());
                }
            });
        });
    }

    // Editor: Save Layout
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_save_layout(move |name| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let name = name.to_string();

            let _ = slint::spawn_local(async move {
                info!("Saving layout: {}", name);

                {
                    let s = state.read().await;
                    let mut layout = ParkingLayout::new(name.clone());
                    layout.elements = s.layout_elements.clone();

                    if let Err(e) = s.layout_storage.save_layout(&layout) {
                        warn!("Failed to save layout: {}", e);
                    } else {
                        info!("Layout saved successfully");
                    }
                }

                // Refresh saved layouts list
                refresh_saved_layouts(&state, &app_weak).await;
            });
        });
    }

    // Editor: Load Layout
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_load_layout(move |id| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let id = id.to_string();

            let _ = slint::spawn_local(async move {
                info!("Loading layout: {}", id);

                {
                    let mut s = state.write().await;
                    match s.layout_storage.load_layout(&id) {
                        Ok(layout) => {
                            s.layout_elements = layout.elements.clone();
                            s.current_layout = Some(layout.clone());

                            // Find max slot number
                            let max_slot = s
                                .layout_elements
                                .iter()
                                .map(|e| e.slot_number)
                                .max()
                                .unwrap_or(0);
                            s.next_slot_number = max_slot + 1;

                            if let Some(app) = app_weak.upgrade() {
                                app.set_editor_layout_name(layout.name.into());
                                app.set_editor_next_slot_number(s.next_slot_number);
                            }

                            info!("Layout loaded with {} elements", s.layout_elements.len());
                        }
                        Err(e) => {
                            warn!("Failed to load layout: {}", e);
                        }
                    }
                }

                update_layout_elements_ui(&state, &app_weak).await;
            });
        });
    }

    // Editor: Delete Layout
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_delete_layout(move |id| {
            let state = state.clone();
            let app_weak = app_weak.clone();
            let id = id.to_string();

            let _ = slint::spawn_local(async move {
                info!("Deleting layout: {}", id);

                {
                    let s = state.read().await;
                    if let Err(e) = s.layout_storage.delete_layout(&id) {
                        warn!("Failed to delete layout: {}", e);
                    }
                }

                refresh_saved_layouts(&state, &app_weak).await;
            });
        });
    }

    // Editor: Clear Canvas
    {
        let state = state.clone();
        let app_weak = app_weak.clone();
        app.on_editor_clear_canvas(move || {
            let state = state.clone();
            let app_weak = app_weak.clone();

            let _ = slint::spawn_local(async move {
                info!("Clearing canvas");

                {
                    let mut s = state.write().await;
                    s.layout_elements.clear();
                    s.next_slot_number = 1;
                }

                update_layout_elements_ui(&state, &app_weak).await;

                if let Some(app) = app_weak.upgrade() {
                    app.set_editor_next_slot_number(1);
                    app.set_editor_selected_element_id("".into());
                }
            });
        });
    }

    // Editor: Toggle Grid
    {
        let app_weak = app_weak.clone();
        app.on_editor_toggle_grid(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_editor_show_grid(!app.get_editor_show_grid());
            }
        });
    }

    // Editor: Zoom In
    {
        let app_weak = app_weak.clone();
        app.on_editor_zoom_in(move || {
            if let Some(app) = app_weak.upgrade() {
                let current = app.get_editor_zoom();
                if current < 2.0 {
                    app.set_editor_zoom(current + 0.25);
                }
            }
        });
    }

    // Editor: Zoom Out
    {
        let app_weak = app_weak.clone();
        app.on_editor_zoom_out(move || {
            if let Some(app) = app_weak.upgrade() {
                let current = app.get_editor_zoom();
                if current > 0.5 {
                    app.set_editor_zoom(current - 0.25);
                }
            }
        });
    }

    // Editor: Back
    {
        let app_weak = app_weak.clone();
        app.on_editor_back(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_current_view(AppView::Parking);
            }
        });
    }

    // ========================================================================
    // Window Control Callbacks (for custom title bar)
    // ========================================================================

    // Minimize Window
    {
        let app_weak = app_weak.clone();
        app.on_minimize_window(move || {
            if let Some(app) = app_weak.upgrade() {
                app.window().set_minimized(true);
            }
        });
    }

    // Maximize Window
    {
        let app_weak = app_weak.clone();
        app.on_maximize_window(move || {
            if let Some(app) = app_weak.upgrade() {
                let window = app.window();
                window.set_maximized(!window.is_maximized());
            }
        });
    }

    // Close Window
    app.on_close_window(move || {
        info!("Window close requested");
        slint::quit_event_loop().ok();
    });

    // Take Screenshot - saves to dev folder with incrementing numbers
    {
        let app_weak = app_weak.clone();
        app.on_take_screenshot(move || {
            let app_weak = app_weak.clone();

            #[cfg(windows)]
            {
                use std::path::PathBuf;

                // Get the dev folder path (same as app source)
                let screenshots_dir =
                    PathBuf::from(r"C:\dev\securanido-parking-desktop\screenshots");

                // Create directory if it doesn't exist
                if let Err(e) = std::fs::create_dir_all(&screenshots_dir) {
                    warn!("Failed to create screenshots directory: {}", e);
                    return;
                }

                // Find next screenshot number
                let mut max_num = 0;
                if let Ok(entries) = std::fs::read_dir(&screenshots_dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with("screenshot_") && name.ends_with(".png") {
                                if let Ok(num) = name
                                    .trim_start_matches("screenshot_")
                                    .trim_end_matches(".png")
                                    .parse::<i32>()
                                {
                                    max_num = max_num.max(num);
                                }
                            }
                        }
                    }
                }

                let screenshot_num = max_num + 1;
                let screenshot_path =
                    screenshots_dir.join(format!("screenshot_{:03}.png", screenshot_num));

                // Capture the primary screen
                match screenshots::Screen::all() {
                    Ok(screens) => {
                        if let Some(screen) = screens.first() {
                            match screen.capture() {
                                Ok(image) => {
                                    if let Err(e) = image.save(&screenshot_path) {
                                        warn!("Failed to save screenshot: {}", e);
                                    } else {
                                        info!("Screenshot saved: {:?}", screenshot_path);

                                        // Show notification
                                        if let Some(app) = app_weak.upgrade() {
                                            let display_path = format!(
                                                "screenshots/screenshot_{:03}.png",
                                                screenshot_num
                                            );
                                            app.set_screenshot_path(display_path.into());
                                            app.set_show_screenshot_notification(true);

                                            // Auto-hide notification after 4 seconds
                                            let app_weak_timer = app.as_weak();
                                            slint::Timer::single_shot(
                                                std::time::Duration::from_secs(4),
                                                move || {
                                                    if let Some(app) = app_weak_timer.upgrade() {
                                                        app.set_show_screenshot_notification(false);
                                                    }
                                                },
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to capture screenshot: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get screens: {}", e);
                    }
                }
            }
        });
    }

    // Start Window Drag
    // Uses Windows API to initiate a native window move operation
    {
        let app_weak = app_weak.clone();
        app.on_start_window_drag(move || {
            #[cfg(windows)]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    SendMessageW, HTCAPTION, WM_NCLBUTTONDOWN,
                };

                if let Some(app) = app_weak.upgrade() {
                    let slint_handle = app.window().window_handle();
                    if let Ok(rwh) = slint_handle.window_handle() {
                        if let RawWindowHandle::Win32(win32_handle) = rwh.as_raw() {
                            let hwnd = win32_handle.hwnd.get() as *mut std::ffi::c_void;
                            unsafe {
                                ReleaseCapture();
                                SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, 0);
                            }
                        }
                    }
                }
            }
        });
    }

    // ========================================================================
    // Show Window
    // ========================================================================

    // Show the window
    app.window().show()?;

    // Run the application
    info!("Application ready, starting UI...");
    app.run()?;

    info!("Application closed");
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn load_parking_data(state: &Arc<RwLock<AppState>>, app_weak: &slint::Weak<MainWindow>) {
    let s = state.read().await;

    let current_user_id = s.current_user.as_ref().map(|u| u.id.clone());

    // Get slots from mock API
    let slots_data = s.mock_api.get_slots();
    let bookings_data = s
        .mock_api
        .get_user_bookings(current_user_id.as_deref().unwrap_or(""));

    drop(s);

    if let Some(app) = app_weak.upgrade() {
        // Convert to UI model
        let slots: Vec<ParkingSlotData> = slots_data
            .iter()
            .map(|slot| {
                let status = if !slot.is_active {
                    SlotStatus::Disabled
                } else if let Some(ref booking) = slot.current_booking {
                    if current_user_id
                        .as_ref()
                        .map(|id| id == &booking.user_id)
                        .unwrap_or(false)
                    {
                        SlotStatus::MyBooking
                    } else {
                        SlotStatus::Occupied
                    }
                } else {
                    SlotStatus::Available
                };

                ParkingSlotData {
                    id: slot.id.clone().into(),
                    slot_number: slot.slot_number,
                    row: slot.row,
                    col: slot.col,
                    status,
                    license_plate: slot
                        .current_booking
                        .as_ref()
                        .map(|b| b.license_plate.clone())
                        .unwrap_or_default()
                        .into(),
                    end_time: slot
                        .current_booking
                        .as_ref()
                        .map(|b| b.end_time.clone())
                        .unwrap_or_default()
                        .into(),
                    booked_by: slot
                        .current_booking
                        .as_ref()
                        .map(|b| b.user_id.clone())
                        .unwrap_or_default()
                        .into(),
                }
            })
            .collect();

        let available_count = slots
            .iter()
            .filter(|s| matches!(s.status, SlotStatus::Available))
            .count() as i32;

        app.set_slots(ModelRc::new(VecModel::from(slots)));
        app.set_available_slots(available_count);
        app.set_total_slots(10);

        // Convert bookings
        let my_bookings: Vec<BookingData> = bookings_data
            .iter()
            .map(|b| BookingData {
                id: b.id.clone().into(),
                slot_number: b.slot_number,
                start_time: b.start_time.clone().into(),
                end_time: b.end_time.clone().into(),
                license_plate: b.license_plate.clone().into(),
                status: b.status.clone().into(),
            })
            .collect();

        app.set_my_bookings(ModelRc::new(VecModel::from(my_bookings)));
    }
}

fn calculate_cost(duration_minutes: i32) -> f64 {
    // Simple pricing: 2.50 EUR per hour
    let hours = duration_minutes as f64 / 60.0;
    (hours * 2.50 * 100.0).round() / 100.0
}

// ============================================================================
// Layout Editor Helper Functions
// ============================================================================

/// Convert Slint ElementType to storage ElementType
fn convert_element_type(elem_type: ElementType) -> StorageElementType {
    match elem_type {
        ElementType::ParkingSlot => StorageElementType::ParkingSlot,
        ElementType::Wall => StorageElementType::Wall,
        ElementType::Pillar => StorageElementType::Pillar,
        ElementType::Entry => StorageElementType::Entry,
        ElementType::Exit => StorageElementType::Exit,
        ElementType::Handicap => StorageElementType::Handicap,
        ElementType::Electric => StorageElementType::Electric,
        ElementType::Motorcycle => StorageElementType::Motorcycle,
        ElementType::Lane => StorageElementType::Lane,
        ElementType::Arrow => StorageElementType::Arrow,
    }
}

/// Convert storage ElementType to Slint ElementType
fn convert_element_type_to_slint(elem_type: &StorageElementType) -> ElementType {
    match elem_type {
        StorageElementType::ParkingSlot => ElementType::ParkingSlot,
        StorageElementType::Wall => ElementType::Wall,
        StorageElementType::Pillar => ElementType::Pillar,
        StorageElementType::Entry => ElementType::Entry,
        StorageElementType::Exit => ElementType::Exit,
        StorageElementType::Handicap => ElementType::Handicap,
        StorageElementType::Electric => ElementType::Electric,
        StorageElementType::Motorcycle => ElementType::Motorcycle,
        StorageElementType::Lane => ElementType::Lane,
        StorageElementType::Arrow => ElementType::Arrow,
    }
}

/// Get default color for element type
fn get_element_color(elem_type: ElementType) -> String {
    match elem_type {
        ElementType::ParkingSlot => "#6366f1".to_string(),
        ElementType::Handicap => "#3b82f6".to_string(),
        ElementType::Electric => "#22c55e".to_string(),
        ElementType::Motorcycle => "#a855f7".to_string(),
        ElementType::Wall => "#6b7280".to_string(),
        ElementType::Pillar => "#374151".to_string(),
        ElementType::Entry => "#22c55e".to_string(),
        ElementType::Exit => "#ef4444".to_string(),
        ElementType::Lane => "#64748b".to_string(),
        ElementType::Arrow => "#94a3b8".to_string(),
    }
}

/// Parse hex color to Slint Color
fn parse_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    let value = u32::from_str_radix(hex, 16).unwrap_or(0xFF6366F1);
    Color::from_argb_u8(
        255,
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    )
}

/// Update the layout elements in the UI
async fn update_layout_elements_ui(
    state: &Arc<RwLock<AppState>>,
    app_weak: &slint::Weak<MainWindow>,
) {
    let s = state.read().await;

    if let Some(app) = app_weak.upgrade() {
        let ui_elements: Vec<LayoutElement> = s
            .layout_elements
            .iter()
            .map(|e| LayoutElement {
                id: e.id.clone().into(),
                element_type: convert_element_type_to_slint(&e.element_type),
                x: e.x,
                y: e.y,
                width: e.width,
                height: e.height,
                rotation: e.rotation,
                slot_number: e.slot_number,
                color: parse_color(&e.color),
            })
            .collect();

        app.set_layout_elements(ModelRc::new(VecModel::from(ui_elements)));
    }
}

/// Refresh the saved layouts list in the UI
async fn refresh_saved_layouts(state: &Arc<RwLock<AppState>>, app_weak: &slint::Weak<MainWindow>) {
    let s = state.read().await;

    if let Some(app) = app_weak.upgrade() {
        if let Ok(layouts) = s.layout_storage.list_layouts() {
            let ui_layouts: Vec<SavedLayout> = layouts
                .iter()
                .map(|l| SavedLayout {
                    id: l.id.clone().into(),
                    name: l.name.clone().into(),
                    created: l.created.clone().into(),
                    elements_count: l.elements_count,
                    thumbnail: "".into(),
                })
                .collect();
            app.set_saved_layouts(ModelRc::new(VecModel::from(ui_layouts)));
        }
    }
}
