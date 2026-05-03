//! Property-based round-trip tests for the DTOs and enums shipped across the
//! HTTP boundary. If a public type survives `serialise → bytes → deserialise`
//! for every valid value, a drifting handler in parkhub-server or a drifting
//! SDK on the client side will spot the mismatch at test time rather than in
//! production.
//!
//! The fuzz/property coverage the T-1734 audit asked for starts here on
//! stable Rust via `proptest`; an optional cargo-fuzz layer can wrap these
//! targets later on nightly without rewriting the shape.

use parkhub_common::{
    BookingStatus, CreditTransactionType, FuelType, LotStatus, PaymentStatus, SlotFeature,
    SlotStatus, SlotType, SwapRequestStatus, UserRole, VehicleType, WaitlistStatus,
};
use proptest::prelude::*;

// ── strategies ─────────────────────────────────────────────────────────────

fn arb_vehicle_type() -> impl Strategy<Value = VehicleType> {
    prop_oneof![
        Just(VehicleType::Car),
        Just(VehicleType::Suv),
        Just(VehicleType::Motorcycle),
        Just(VehicleType::Bicycle),
        Just(VehicleType::Truck),
        Just(VehicleType::Van),
        Just(VehicleType::Electric),
    ]
}

fn arb_fuel_type() -> impl Strategy<Value = FuelType> {
    prop_oneof![
        Just(FuelType::Unknown),
        Just(FuelType::Gasoline),
        Just(FuelType::Diesel),
        Just(FuelType::Hybrid),
        Just(FuelType::PluginHybrid),
        Just(FuelType::Electric),
        Just(FuelType::Hydrogen),
    ]
}

fn arb_booking_status() -> impl Strategy<Value = BookingStatus> {
    prop_oneof![
        Just(BookingStatus::Pending),
        Just(BookingStatus::Confirmed),
        Just(BookingStatus::Active),
        Just(BookingStatus::Completed),
        Just(BookingStatus::Cancelled),
        Just(BookingStatus::Expired),
        Just(BookingStatus::NoShow),
    ]
}

fn arb_payment_status() -> impl Strategy<Value = PaymentStatus> {
    prop_oneof![
        Just(PaymentStatus::Pending),
        Just(PaymentStatus::Paid),
        Just(PaymentStatus::Failed),
        Just(PaymentStatus::Refunded),
        Just(PaymentStatus::PartialRefund),
    ]
}

fn arb_waitlist_status() -> impl Strategy<Value = WaitlistStatus> {
    prop_oneof![
        Just(WaitlistStatus::Waiting),
        Just(WaitlistStatus::Offered),
        Just(WaitlistStatus::Accepted),
        Just(WaitlistStatus::Declined),
        Just(WaitlistStatus::Expired),
    ]
}

fn arb_lot_status() -> impl Strategy<Value = LotStatus> {
    prop_oneof![
        Just(LotStatus::Open),
        Just(LotStatus::Closed),
        Just(LotStatus::Full),
        Just(LotStatus::Maintenance),
    ]
}

fn arb_swap_request_status() -> impl Strategy<Value = SwapRequestStatus> {
    prop_oneof![
        Just(SwapRequestStatus::Pending),
        Just(SwapRequestStatus::Accepted),
        Just(SwapRequestStatus::Declined),
        Just(SwapRequestStatus::Cancelled),
    ]
}

fn arb_credit_transaction_type() -> impl Strategy<Value = CreditTransactionType> {
    prop_oneof![
        Just(CreditTransactionType::Grant),
        Just(CreditTransactionType::Deduction),
        Just(CreditTransactionType::Refund),
        Just(CreditTransactionType::MonthlyRefill),
        Just(CreditTransactionType::Adjustment),
    ]
}

fn arb_user_role() -> impl Strategy<Value = UserRole> {
    prop_oneof![
        Just(UserRole::User),
        Just(UserRole::Premium),
        Just(UserRole::Admin),
        Just(UserRole::SuperAdmin),
    ]
}

fn arb_slot_type() -> impl Strategy<Value = SlotType> {
    prop_oneof![
        Just(SlotType::Standard),
        Just(SlotType::Compact),
        Just(SlotType::Large),
        Just(SlotType::Handicap),
        Just(SlotType::Electric),
        Just(SlotType::Motorcycle),
        Just(SlotType::Reserved),
        Just(SlotType::Vip),
    ]
}

fn arb_slot_status() -> impl Strategy<Value = SlotStatus> {
    prop_oneof![
        Just(SlotStatus::Available),
        Just(SlotStatus::Occupied),
        Just(SlotStatus::Reserved),
        Just(SlotStatus::Maintenance),
        Just(SlotStatus::Disabled),
    ]
}

fn arb_slot_feature() -> impl Strategy<Value = SlotFeature> {
    prop_oneof![
        Just(SlotFeature::NearExit),
        Just(SlotFeature::NearElevator),
        Just(SlotFeature::NearStairs),
        Just(SlotFeature::Covered),
        Just(SlotFeature::SecurityCamera),
        Just(SlotFeature::WellLit),
        Just(SlotFeature::WideLane),
        Just(SlotFeature::ChargingStation),
    ]
}

// ── tests ──────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn vehicle_type_roundtrips(v in arb_vehicle_type()) {
        let json = serde_json::to_string(&v).unwrap();
        let decoded: VehicleType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(v, decoded);
    }

    #[test]
    fn fuel_type_roundtrips(f in arb_fuel_type()) {
        let json = serde_json::to_string(&f).unwrap();
        let decoded: FuelType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(f, decoded);
    }

    #[test]
    fn user_role_roundtrips(r in arb_user_role()) {
        let json = serde_json::to_string(&r).unwrap();
        let decoded: UserRole = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(r, decoded);
    }

    #[test]
    fn slot_type_roundtrips(s in arb_slot_type()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: SlotType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn slot_status_roundtrips(s in arb_slot_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: SlotStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn slot_feature_roundtrips(f in arb_slot_feature()) {
        let json = serde_json::to_string(&f).unwrap();
        let decoded: SlotFeature = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(f, decoded);
    }

    /// Any valid JSON that deserialises into a FuelType must serialise back
    /// into a string we accept — no partial parsers, no asymmetric aliases.
    #[test]
    fn fuel_type_json_is_stable_through_roundtrip(f in arb_fuel_type()) {
        let json1 = serde_json::to_string(&f).unwrap();
        let decoded: FuelType = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }

    // ── lifecycle / state-machine enums (added 2026-05-03) ─────────────────

    #[test]
    fn booking_status_roundtrips(s in arb_booking_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: BookingStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn booking_status_json_is_stable(s in arb_booking_status()) {
        let json1 = serde_json::to_string(&s).unwrap();
        let decoded: BookingStatus = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }

    #[test]
    fn payment_status_roundtrips(s in arb_payment_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: PaymentStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn payment_status_json_is_stable(s in arb_payment_status()) {
        let json1 = serde_json::to_string(&s).unwrap();
        let decoded: PaymentStatus = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }

    #[test]
    fn waitlist_status_roundtrips(s in arb_waitlist_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: WaitlistStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn waitlist_status_json_is_stable(s in arb_waitlist_status()) {
        let json1 = serde_json::to_string(&s).unwrap();
        let decoded: WaitlistStatus = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }

    #[test]
    fn lot_status_roundtrips(s in arb_lot_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: LotStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn lot_status_json_is_stable(s in arb_lot_status()) {
        let json1 = serde_json::to_string(&s).unwrap();
        let decoded: LotStatus = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }

    #[test]
    fn swap_request_status_roundtrips(s in arb_swap_request_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: SwapRequestStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn swap_request_status_json_is_stable(s in arb_swap_request_status()) {
        let json1 = serde_json::to_string(&s).unwrap();
        let decoded: SwapRequestStatus = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }

    #[test]
    fn credit_transaction_type_roundtrips(t in arb_credit_transaction_type()) {
        let json = serde_json::to_string(&t).unwrap();
        let decoded: CreditTransactionType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(t, decoded);
    }

    #[test]
    fn credit_transaction_type_json_is_stable(t in arb_credit_transaction_type()) {
        let json1 = serde_json::to_string(&t).unwrap();
        let decoded: CreditTransactionType = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&decoded).unwrap();
        prop_assert_eq!(json1, json2);
    }
}
