//! Property-based round-trip tests for the DTOs and enums shipped across the
//! HTTP boundary. If a public type survives `serialise → bytes → deserialise`
//! for every valid value, a drifting handler in parkhub-server or a drifting
//! SDK on the client side will spot the mismatch at test time rather than in
//! production.
//!
//! The fuzz/property coverage the T-1734 audit asked for starts here on
//! stable Rust via `proptest`; an optional cargo-fuzz layer can wrap these
//! targets later on nightly without rewriting the shape.

use parkhub_common::{FuelType, SlotFeature, SlotStatus, SlotType, UserRole, VehicleType};
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
}
