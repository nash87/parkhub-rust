//! Mock Parking API
//!
//! Simulates backend API responses for development and testing.
//! This will be replaced with real API calls when backend is ready.

#![allow(dead_code)]

use chrono::{Duration, Local};
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

/// Mock slot data
#[derive(Debug, Clone)]
pub struct MockSlot {
    pub id: String,
    pub slot_number: i32,
    pub row: i32,
    pub col: i32,
    pub is_active: bool,
    pub current_booking: Option<MockBookingInfo>,
}

/// Booking info attached to a slot
#[derive(Debug, Clone)]
pub struct MockBookingInfo {
    pub booking_id: String,
    pub user_id: String,
    pub license_plate: String,
    pub start_time: String,
    pub end_time: String,
}

/// Full booking data
#[derive(Debug, Clone)]
pub struct MockBooking {
    pub id: String,
    pub slot_number: i32,
    pub user_id: String,
    pub license_plate: String,
    pub start_time: String,
    pub end_time: String,
    pub status: String,
}

/// Mock Parking API client
pub struct MockParkingApi {
    slots: Mutex<Vec<MockSlot>>,
    bookings: Mutex<HashMap<String, MockBooking>>,
}

impl MockParkingApi {
    /// Create a new mock API with default parking lot (10 slots)
    pub fn new() -> Self {
        let mut slots = Vec::new();

        // Create 10 slots: 5 on top row, 5 on bottom row
        for i in 1..=10 {
            let row = if i <= 5 { 0 } else { 1 };
            let col = if i <= 5 { i - 1 } else { i - 6 };

            slots.push(MockSlot {
                id: Uuid::new_v4().to_string(),
                slot_number: i,
                row,
                col,
                is_active: true,
                current_booking: None,
            });
        }

        // Add some sample bookings for demonstration
        let now = Local::now();

        // Slot 2 is occupied by someone else
        if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == 2) {
            slot.current_booking = Some(MockBookingInfo {
                booking_id: Uuid::new_v4().to_string(),
                user_id: "other-user".to_string(),
                license_plate: "AB-CD-123".to_string(),
                start_time: now.format("%H:%M").to_string(),
                end_time: (now + Duration::hours(2)).format("%H:%M").to_string(),
            });
        }

        // Slot 7 is occupied by someone else
        if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == 7) {
            slot.current_booking = Some(MockBookingInfo {
                booking_id: Uuid::new_v4().to_string(),
                user_id: "another-user".to_string(),
                license_plate: "XY-ZZ-999".to_string(),
                start_time: now.format("%H:%M").to_string(),
                end_time: (now + Duration::hours(4)).format("%H:%M").to_string(),
            });
        }

        // Slot 10 is disabled (maintenance)
        if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == 10) {
            slot.is_active = false;
        }

        Self {
            slots: Mutex::new(slots),
            bookings: Mutex::new(HashMap::new()),
        }
    }

    /// Get all slots with their current status
    pub fn get_slots(&self) -> Vec<MockSlot> {
        self.slots.lock().unwrap().clone()
    }

    /// Get bookings for a specific user
    pub fn get_user_bookings(&self, user_id: &str) -> Vec<MockBooking> {
        self.bookings
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.user_id == user_id && b.status == "active")
            .cloned()
            .collect()
    }

    /// Create a new booking
    pub fn create_booking(
        &mut self,
        slot_number: i32,
        duration_minutes: i32,
        license_plate: String,
        user_id: String,
    ) -> String {
        let now = Local::now();
        let end_time = now + Duration::minutes(duration_minutes as i64);

        let booking_id = Uuid::new_v4().to_string();

        let booking = MockBooking {
            id: booking_id.clone(),
            slot_number,
            user_id: user_id.clone(),
            license_plate: license_plate.clone(),
            start_time: now.format("%H:%M").to_string(),
            end_time: end_time.format("%H:%M").to_string(),
            status: "active".to_string(),
        };

        // Update slot
        {
            let mut slots = self.slots.lock().unwrap();
            if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == slot_number) {
                slot.current_booking = Some(MockBookingInfo {
                    booking_id: booking_id.clone(),
                    user_id,
                    license_plate,
                    start_time: now.format("%H:%M").to_string(),
                    end_time: end_time.format("%H:%M").to_string(),
                });
            }
        }

        // Store booking
        self.bookings
            .lock()
            .unwrap()
            .insert(booking_id.clone(), booking);

        booking_id
    }

    /// Cancel a booking
    pub fn cancel_booking(&mut self, booking_id: &str) {
        let mut bookings = self.bookings.lock().unwrap();

        if let Some(booking) = bookings.get_mut(booking_id) {
            booking.status = "cancelled".to_string();

            // Free up the slot
            let slot_number = booking.slot_number;
            drop(bookings);

            let mut slots = self.slots.lock().unwrap();
            if let Some(slot) = slots.iter_mut().find(|s| s.slot_number == slot_number) {
                if slot.current_booking.as_ref().map(|b| &b.booking_id)
                    == Some(&booking_id.to_string())
                {
                    slot.current_booking = None;
                }
            }
        }
    }

    /// Check slot availability for a time range (future use)
    pub fn is_slot_available(&self, slot_number: i32) -> bool {
        self.slots
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.slot_number == slot_number)
            .map(|s| s.is_active && s.current_booking.is_none())
            .unwrap_or(false)
    }
}

impl Default for MockParkingApi {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HEADLESS UNIT TESTS - State-of-the-art 2026 Rust Testing
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that a new MockParkingApi initializes with correct default state
    #[test]
    fn test_new_api_has_ten_slots() {
        let api = MockParkingApi::new();
        let slots = api.get_slots();
        assert_eq!(slots.len(), 10, "Should have 10 parking slots");
    }

    /// Test that initial slot numbers are 1-10
    #[test]
    fn test_slot_numbers_are_sequential() {
        let api = MockParkingApi::new();
        let slots = api.get_slots();

        let slot_numbers: Vec<i32> = slots.iter().map(|s| s.slot_number).collect();
        let expected: Vec<i32> = (1..=10).collect();

        assert_eq!(slot_numbers, expected, "Slot numbers should be 1-10");
    }

    /// Test that slots are arranged in 2 rows of 5
    #[test]
    fn test_slot_layout_is_two_rows() {
        let api = MockParkingApi::new();
        let slots = api.get_slots();

        // First 5 slots should be in row 0
        for i in 0..5 {
            assert_eq!(slots[i].row, 0, "Slots 1-5 should be in row 0");
            assert_eq!(slots[i].col, i as i32, "Column should match position");
        }

        // Last 5 slots should be in row 1
        for i in 5..10 {
            assert_eq!(slots[i].row, 1, "Slots 6-10 should be in row 1");
            assert_eq!(slots[i].col, (i - 5) as i32, "Column should match position");
        }
    }

    /// Test initial bookings: slots 2 and 7 should be occupied
    #[test]
    fn test_initial_bookings() {
        let api = MockParkingApi::new();
        let slots = api.get_slots();

        // Slot 2 should have a booking
        let slot2 = slots.iter().find(|s| s.slot_number == 2).unwrap();
        assert!(slot2.current_booking.is_some(), "Slot 2 should be booked");

        // Slot 7 should have a booking
        let slot7 = slots.iter().find(|s| s.slot_number == 7).unwrap();
        assert!(slot7.current_booking.is_some(), "Slot 7 should be booked");

        // Slot 1 should be free
        let slot1 = slots.iter().find(|s| s.slot_number == 1).unwrap();
        assert!(slot1.current_booking.is_none(), "Slot 1 should be free");
    }

    /// Test slot 10 is disabled (maintenance)
    #[test]
    fn test_slot_10_is_disabled() {
        let api = MockParkingApi::new();
        let slots = api.get_slots();

        let slot10 = slots.iter().find(|s| s.slot_number == 10).unwrap();
        assert!(!slot10.is_active, "Slot 10 should be disabled");
    }

    /// Test slot availability check
    #[test]
    fn test_slot_availability() {
        let api = MockParkingApi::new();

        // Slot 1 should be available
        assert!(api.is_slot_available(1), "Slot 1 should be available");

        // Slot 2 should not be available (booked)
        assert!(!api.is_slot_available(2), "Slot 2 should not be available");

        // Slot 10 should not be available (disabled)
        assert!(
            !api.is_slot_available(10),
            "Slot 10 should not be available"
        );

        // Non-existent slot should not be available
        assert!(
            !api.is_slot_available(99),
            "Non-existent slot should not be available"
        );
    }

    /// Test creating a new booking
    #[test]
    fn test_create_booking() {
        let mut api = MockParkingApi::new();

        // Book slot 1
        let booking_id =
            api.create_booking(1, 60, "XX-YY-123".to_string(), "test-user".to_string());

        // Booking ID should be returned
        assert!(!booking_id.is_empty(), "Booking ID should be returned");

        // Slot 1 should now be occupied
        assert!(
            !api.is_slot_available(1),
            "Slot 1 should be occupied after booking"
        );

        // Check slot has correct booking info
        let slots = api.get_slots();
        let slot1 = slots.iter().find(|s| s.slot_number == 1).unwrap();
        let booking = slot1.current_booking.as_ref().unwrap();

        assert_eq!(booking.booking_id, booking_id);
        assert_eq!(booking.user_id, "test-user");
        assert_eq!(booking.license_plate, "XX-YY-123");
    }

    /// Test getting user bookings
    #[test]
    fn test_get_user_bookings() {
        let mut api = MockParkingApi::new();

        // Initially no bookings for test user
        let bookings = api.get_user_bookings("test-user");
        assert!(bookings.is_empty(), "No bookings initially");

        // Create a booking
        api.create_booking(1, 60, "AA-BB-111".to_string(), "test-user".to_string());

        // Now there should be 1 booking
        let bookings = api.get_user_bookings("test-user");
        assert_eq!(bookings.len(), 1, "Should have 1 booking");
        assert_eq!(bookings[0].license_plate, "AA-BB-111");
    }

    /// Test cancelling a booking
    #[test]
    fn test_cancel_booking() {
        let mut api = MockParkingApi::new();

        // Create a booking
        let booking_id =
            api.create_booking(3, 120, "ZZ-XX-999".to_string(), "test-user".to_string());

        // Verify slot is booked
        assert!(!api.is_slot_available(3), "Slot 3 should be booked");

        // Cancel the booking
        api.cancel_booking(&booking_id);

        // Slot should be available again
        assert!(
            api.is_slot_available(3),
            "Slot 3 should be available after cancellation"
        );

        // User should have no active bookings
        let bookings = api.get_user_bookings("test-user");
        assert!(bookings.is_empty(), "No active bookings after cancellation");
    }

    /// Test multiple bookings by same user
    #[test]
    fn test_multiple_bookings() {
        let mut api = MockParkingApi::new();

        // Create multiple bookings
        api.create_booking(1, 60, "A".to_string(), "user1".to_string());
        api.create_booking(3, 60, "B".to_string(), "user1".to_string());
        api.create_booking(4, 60, "C".to_string(), "user2".to_string());

        // User1 should have 2 bookings
        let user1_bookings = api.get_user_bookings("user1");
        assert_eq!(user1_bookings.len(), 2, "User1 should have 2 bookings");

        // User2 should have 1 booking
        let user2_bookings = api.get_user_bookings("user2");
        assert_eq!(user2_bookings.len(), 1, "User2 should have 1 booking");
    }

    /// Test that booking has correct time format
    #[test]
    fn test_booking_time_format() {
        let mut api = MockParkingApi::new();

        let booking_id = api.create_booking(5, 120, "T".to_string(), "user".to_string());
        let bookings = api.get_user_bookings("user");

        assert_eq!(bookings.len(), 1);

        // Time should be in HH:MM format
        let time_regex = regex::Regex::new(r"^\d{2}:\d{2}$").unwrap();
        assert!(
            time_regex.is_match(&bookings[0].start_time),
            "Start time should be HH:MM format"
        );
        assert!(
            time_regex.is_match(&bookings[0].end_time),
            "End time should be HH:MM format"
        );
    }

    /// Test Default trait implementation
    #[test]
    fn test_default_trait() {
        let api = MockParkingApi::default();
        let slots = api.get_slots();
        assert_eq!(slots.len(), 10, "Default should create API with 10 slots");
    }
}
