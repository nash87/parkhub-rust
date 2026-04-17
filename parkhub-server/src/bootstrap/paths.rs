//! Filesystem and network discovery helpers used during startup.
//!
//! - [`get_data_directory`] resolves the persistent data directory
//!   (portable exe-dir or OS-managed system dir).
//! - [`get_local_ip`] probes the outbound interface IP for logging
//!   and setup-wizard display.
//! - [`hash_password`] wraps Argon2 with a fresh salt, exposed as a
//!   `pub(crate)` symbol because a handful of tests + a setup-flow
//!   handler resolve it via `crate::hash_password`.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the application data directory
pub(crate) fn get_data_directory(portable_mode: Option<bool>) -> Result<PathBuf> {
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
pub(crate) fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}

/// Hash a password using Argon2
pub(crate) fn hash_password(password: &str) -> Result<String> {
    use argon2::{
        Argon2,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {e}"))?;

    Ok(hash.to_string())
}
