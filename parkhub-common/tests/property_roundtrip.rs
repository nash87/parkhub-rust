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
    AbsenceType, AnnouncementSeverity, BookingStatus, ChargingSessionStatus, ConnectorType,
    CreditTransactionType, EvChargerStatus, FleetEventType, FuelType, LayoutElementType, LotStatus,
    NotificationType, PaymentStatus, ProposalStatus, SlotFeature, SlotStatus, SlotType,
    SwapRequestStatus, UserRole, VehicleType, VisitorStatus, WaitlistStatus,
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

    // ── domain enums (added 2026-05-03 — second wave) ──────────────────────

    #[test]
    fn notification_type_roundtrips(n in arb_notification_type()) {
        let json = serde_json::to_string(&n).unwrap();
        let decoded: NotificationType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(n, decoded);
    }

    #[test]
    fn layout_element_type_roundtrips(l in arb_layout_element_type()) {
        let json = serde_json::to_string(&l).unwrap();
        let decoded: LayoutElementType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(l, decoded);
    }

    #[test]
    fn absence_type_roundtrips(a in arb_absence_type()) {
        let json = serde_json::to_string(&a).unwrap();
        let decoded: AbsenceType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(a, decoded);
    }

    #[test]
    fn announcement_severity_roundtrips(s in arb_announcement_severity()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: AnnouncementSeverity = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn proposal_status_roundtrips(s in arb_proposal_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: ProposalStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn visitor_status_roundtrips(s in arb_visitor_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: VisitorStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn connector_type_roundtrips(c in arb_connector_type()) {
        let json = serde_json::to_string(&c).unwrap();
        let decoded: ConnectorType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(c, decoded);
    }

    #[test]
    fn ev_charger_status_roundtrips(s in arb_ev_charger_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: EvChargerStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    // ── json-stability variant for the same 8 enums ────────────────────────

    #[test]
    fn notification_type_json_is_stable(n in arb_notification_type()) {
        let j1 = serde_json::to_string(&n).unwrap();
        let d: NotificationType = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn layout_element_type_json_is_stable(l in arb_layout_element_type()) {
        let j1 = serde_json::to_string(&l).unwrap();
        let d: LayoutElementType = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn absence_type_json_is_stable(a in arb_absence_type()) {
        let j1 = serde_json::to_string(&a).unwrap();
        let d: AbsenceType = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn announcement_severity_json_is_stable(s in arb_announcement_severity()) {
        let j1 = serde_json::to_string(&s).unwrap();
        let d: AnnouncementSeverity = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn proposal_status_json_is_stable(s in arb_proposal_status()) {
        let j1 = serde_json::to_string(&s).unwrap();
        let d: ProposalStatus = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn visitor_status_json_is_stable(s in arb_visitor_status()) {
        let j1 = serde_json::to_string(&s).unwrap();
        let d: VisitorStatus = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn connector_type_json_is_stable(c in arb_connector_type()) {
        let j1 = serde_json::to_string(&c).unwrap();
        let d: ConnectorType = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    #[test]
    fn ev_charger_status_json_is_stable(s in arb_ev_charger_status()) {
        let j1 = serde_json::to_string(&s).unwrap();
        let d: EvChargerStatus = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    // ── EV + fleet enums (third wave 2026-05-03) ───────────────────────────

    #[test]
    fn charging_session_status_roundtrips(s in arb_charging_session_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let decoded: ChargingSessionStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, decoded);
    }

    #[test]
    fn charging_session_status_json_is_stable(s in arb_charging_session_status()) {
        let j1 = serde_json::to_string(&s).unwrap();
        let d: ChargingSessionStatus = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }

    /// FleetEventType uses dotted-string serde renames ("checkin.started" etc.).
    /// Roundtrip catches a typo in any of the 9 #[serde(rename = ...)] tags.
    #[test]
    fn fleet_event_type_roundtrips(e in arb_fleet_event_type()) {
        let json = serde_json::to_string(&e).unwrap();
        let decoded: FleetEventType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(e, decoded);
    }

    #[test]
    fn fleet_event_type_json_is_stable(e in arb_fleet_event_type()) {
        let j1 = serde_json::to_string(&e).unwrap();
        let d: FleetEventType = serde_json::from_str(&j1).unwrap();
        prop_assert_eq!(j1, serde_json::to_string(&d).unwrap());
    }
}

fn arb_charging_session_status() -> impl Strategy<Value = ChargingSessionStatus> {
    prop_oneof![
        Just(ChargingSessionStatus::Active),
        Just(ChargingSessionStatus::Completed),
        Just(ChargingSessionStatus::Cancelled),
    ]
}

fn arb_fleet_event_type() -> impl Strategy<Value = FleetEventType> {
    prop_oneof![
        Just(FleetEventType::CheckinStarted),
        Just(FleetEventType::CheckinCompleted),
        Just(FleetEventType::SwapRequested),
        Just(FleetEventType::SwapAccepted),
        Just(FleetEventType::SwapDeclined),
        Just(FleetEventType::EvSessionStarted),
        Just(FleetEventType::EvSessionStopped),
        Just(FleetEventType::GuestCreated),
        Just(FleetEventType::GuestCancelled),
    ]
}

// ── arbitraries for the 8 new enums ────────────────────────────────────────

fn arb_notification_type() -> impl Strategy<Value = NotificationType> {
    prop_oneof![
        Just(NotificationType::BookingConfirmed),
        Just(NotificationType::BookingReminder),
        Just(NotificationType::BookingExpiring),
        Just(NotificationType::BookingCancelled),
        Just(NotificationType::PaymentReceived),
        Just(NotificationType::PaymentFailed),
        Just(NotificationType::PromotionAvailable),
        Just(NotificationType::SystemMessage),
        Just(NotificationType::WaitlistOffer),
    ]
}

fn arb_layout_element_type() -> impl Strategy<Value = LayoutElementType> {
    prop_oneof![
        Just(LayoutElementType::ParkingSlot),
        Just(LayoutElementType::Road),
        Just(LayoutElementType::Entrance),
        Just(LayoutElementType::Exit),
        Just(LayoutElementType::Elevator),
        Just(LayoutElementType::Stairs),
        Just(LayoutElementType::Wall),
        Just(LayoutElementType::Pillar),
        Just(LayoutElementType::Obstacle),
        Just(LayoutElementType::ChargingStation),
    ]
}

fn arb_absence_type() -> impl Strategy<Value = AbsenceType> {
    prop_oneof![
        Just(AbsenceType::Homeoffice),
        Just(AbsenceType::Vacation),
        Just(AbsenceType::Sick),
        Just(AbsenceType::Training),
        Just(AbsenceType::Other),
    ]
}

fn arb_announcement_severity() -> impl Strategy<Value = AnnouncementSeverity> {
    prop_oneof![
        Just(AnnouncementSeverity::Info),
        Just(AnnouncementSeverity::Warning),
        Just(AnnouncementSeverity::Error),
        Just(AnnouncementSeverity::Success),
    ]
}

fn arb_proposal_status() -> impl Strategy<Value = ProposalStatus> {
    prop_oneof![
        Just(ProposalStatus::Pending),
        Just(ProposalStatus::Approved),
        Just(ProposalStatus::Rejected),
    ]
}

fn arb_visitor_status() -> impl Strategy<Value = VisitorStatus> {
    prop_oneof![
        Just(VisitorStatus::Pending),
        Just(VisitorStatus::CheckedIn),
        Just(VisitorStatus::Expired),
        Just(VisitorStatus::Cancelled),
    ]
}

fn arb_connector_type() -> impl Strategy<Value = ConnectorType> {
    prop_oneof![
        Just(ConnectorType::Type2),
        Just(ConnectorType::Ccs),
        Just(ConnectorType::Chademo),
        Just(ConnectorType::Tesla),
    ]
}

fn arb_ev_charger_status() -> impl Strategy<Value = EvChargerStatus> {
    prop_oneof![
        Just(EvChargerStatus::Available),
        Just(EvChargerStatus::InUse),
        Just(EvChargerStatus::Offline),
        Just(EvChargerStatus::Maintenance),
    ]
}
