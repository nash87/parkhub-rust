//! API Client Module for ParkHub Parking System
//!
//! Handles all HTTP communication with the backend server.
//! Supports both online and offline modes with local caching.

#![allow(dead_code)]

pub mod client;
pub mod endpoints;
pub mod error;
pub mod models;
