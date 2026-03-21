//! Booking handlers: create, list, get, cancel, quick-book, guest booking,
//! invoice generation, and calendar events.
//!
//! TODO: Move these handlers from mod.rs into this module:
//! - `list_bookings`
//! - `create_booking`
//! - `get_booking`
//! - `cancel_booking`
//! - `get_booking_invoice`
//! - `quick_book`
//! - `create_guest_booking`
//! - `calendar_events`

#[cfg(test)]
mod tests {
    use parkhub_common::{
        Booking, BookingPricing, BookingStatus, GuestBooking, PaymentStatus, Vehicle, VehicleType,
    };
    use uuid::Uuid;

    fn make_vehicle() -> Vehicle {
        Vehicle {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            license_plate: "AB-CD-1234".to_string(),
            make: Some("BMW".to_string()),
            model: Some("X5".to_string()),
            color: Some("Black".to_string()),
            vehicle_type: VehicleType::Car,
            is_default: true,
            created_at: chrono::Utc::now(),
        }
    }

    fn make_pricing() -> BookingPricing {
        BookingPricing {
            base_price: 5.0,
            discount: 0.0,
            tax: 0.5,
            total: 5.5,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        }
    }

    // ── BookingStatus serde ──────────────────────────────────────────────────

    #[test]
    fn test_booking_status_serde_all_variants() {
        let cases = [
            (BookingStatus::Pending, "\"pending\""),
            (BookingStatus::Confirmed, "\"confirmed\""),
            (BookingStatus::Active, "\"active\""),
            (BookingStatus::Completed, "\"completed\""),
            (BookingStatus::Cancelled, "\"cancelled\""),
            (BookingStatus::Expired, "\"expired\""),
            (BookingStatus::NoShow, "\"no_show\""),
        ];
        for (variant, expected_json) in &cases {
            let serialized = serde_json::to_string(variant).unwrap();
            assert_eq!(&serialized, expected_json, "Variant {:?} failed", variant);
            let deserialized: BookingStatus = serde_json::from_str(expected_json).unwrap();
            assert_eq!(&deserialized, variant);
        }
    }

    #[test]
    fn test_booking_status_unknown_fails() {
        let result: Result<BookingStatus, _> = serde_json::from_str(r#""unknown_status""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_booking_status_default_is_pending() {
        let status = BookingStatus::default();
        assert_eq!(status, BookingStatus::Pending);
    }

    // ── PaymentStatus serde ──────────────────────────────────────────────────

    #[test]
    fn test_payment_status_serde() {
        let pending: PaymentStatus = serde_json::from_str(r#""pending""#).unwrap();
        assert_eq!(pending, PaymentStatus::Pending);

        let paid: PaymentStatus = serde_json::from_str(r#""paid""#).unwrap();
        assert_eq!(paid, PaymentStatus::Paid);

        let refunded: PaymentStatus = serde_json::from_str(r#""refunded""#).unwrap();
        assert_eq!(refunded, PaymentStatus::Refunded);
    }

    #[test]
    fn test_payment_status_default_is_pending() {
        assert_eq!(PaymentStatus::default(), PaymentStatus::Pending);
    }

    // ── BookingPricing serde ─────────────────────────────────────────────────

    #[test]
    fn test_booking_pricing_serde_roundtrip() {
        let pricing = make_pricing();
        let json = serde_json::to_string(&pricing).unwrap();
        let back: BookingPricing = serde_json::from_str(&json).unwrap();
        assert!((back.base_price - 5.0).abs() < 1e-9);
        assert!((back.total - 5.5).abs() < 1e-9);
        assert_eq!(back.currency, "EUR");
        assert!(back.payment_method.is_none());
    }

    #[test]
    fn test_booking_pricing_zero_discount() {
        let json = serde_json::json!({
            "base_price": 10.0,
            "discount": 0.0,
            "tax": 1.0,
            "total": 11.0,
            "currency": "USD",
            "payment_status": "pending",
            "payment_method": null
        });
        let pricing: BookingPricing = serde_json::from_value(json).unwrap();
        assert_eq!(pricing.discount, 0.0);
        assert_eq!(pricing.total, 11.0);
    }

    #[test]
    fn test_booking_pricing_with_payment_method() {
        let json = serde_json::json!({
            "base_price": 8.0,
            "discount": 1.0,
            "tax": 0.7,
            "total": 7.7,
            "currency": "EUR",
            "payment_status": "paid",
            "payment_method": "credit_card"
        });
        let pricing: BookingPricing = serde_json::from_value(json).unwrap();
        assert_eq!(pricing.payment_method.as_deref(), Some("credit_card"));
        assert_eq!(pricing.payment_status, PaymentStatus::Paid);
    }

    // ── Booking full model serde ─────────────────────────────────────────────

    #[test]
    fn test_booking_serde_roundtrip() {
        let now = chrono::Utc::now();
        let booking = Booking {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            slot_number: 42,
            floor_name: "Ground Floor".to_string(),
            vehicle: make_vehicle(),
            start_time: now,
            end_time: now + chrono::Duration::hours(2),
            status: BookingStatus::Confirmed,
            pricing: make_pricing(),
            created_at: now,
            updated_at: now,
            check_in_time: None,
            check_out_time: None,
            qr_code: Some("QR_DATA".to_string()),
            notes: None,
        };

        let json = serde_json::to_string(&booking).unwrap();
        let back: Booking = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, booking.id);
        assert_eq!(back.slot_number, 42);
        assert_eq!(back.status, BookingStatus::Confirmed);
        assert!(back.check_in_time.is_none());
        assert!(back.notes.is_none());
        assert_eq!(back.qr_code.as_deref(), Some("QR_DATA"));
    }

    #[test]
    fn test_booking_with_check_in_out_times() {
        let now = chrono::Utc::now();
        let booking = Booking {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            slot_number: 1,
            floor_name: "Level 1".to_string(),
            vehicle: make_vehicle(),
            start_time: now,
            end_time: now + chrono::Duration::hours(1),
            status: BookingStatus::Completed,
            pricing: make_pricing(),
            created_at: now,
            updated_at: now,
            check_in_time: Some(now + chrono::Duration::minutes(5)),
            check_out_time: Some(now + chrono::Duration::minutes(65)),
            qr_code: None,
            notes: Some("late arrival".to_string()),
        };

        let json = serde_json::to_string(&booking).unwrap();
        let back: Booking = serde_json::from_str(&json).unwrap();
        assert!(back.check_in_time.is_some());
        assert!(back.check_out_time.is_some());
        assert_eq!(back.notes.as_deref(), Some("late arrival"));
        assert_eq!(back.status, BookingStatus::Completed);
    }

    // ── GuestBooking serde ───────────────────────────────────────────────────

    #[test]
    fn test_guest_booking_serde_roundtrip() {
        let now = chrono::Utc::now();
        let guest = GuestBooking {
            id: Uuid::new_v4(),
            created_by: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            guest_name: "Max Muster".to_string(),
            guest_email: Some("max@example.com".to_string()),
            vehicle_plate: None,
            start_time: now,
            end_time: now + chrono::Duration::hours(3),
            guest_code: "ABCD1234".to_string(),
            status: BookingStatus::Confirmed,
            created_at: now,
        };

        let json = serde_json::to_string(&guest).unwrap();
        let back: GuestBooking = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, guest.id);
        assert_eq!(back.guest_name, "Max Muster");
        assert_eq!(back.guest_code, "ABCD1234");
        assert_eq!(back.guest_email.as_deref(), Some("max@example.com"));
        assert!(back.vehicle_plate.is_none());
        assert_eq!(back.status, BookingStatus::Confirmed);
    }

    #[test]
    fn test_guest_booking_no_email() {
        let now = chrono::Utc::now();
        let guest = GuestBooking {
            id: Uuid::new_v4(),
            created_by: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            guest_name: "Anonymous".to_string(),
            guest_email: None,
            vehicle_plate: Some("MUC-AB-123".to_string()),
            start_time: now,
            end_time: now + chrono::Duration::hours(1),
            guest_code: "ZZZZZZZZ".to_string(),
            status: BookingStatus::Pending,
            created_at: now,
        };

        let json = serde_json::to_string(&guest).unwrap();
        let back: GuestBooking = serde_json::from_str(&json).unwrap();
        assert!(back.guest_email.is_none());
        assert_eq!(back.vehicle_plate.as_deref(), Some("MUC-AB-123"));
        assert_eq!(back.status, BookingStatus::Pending);
    }
}
