//! Simulation profiles defining the scale and behavior of each test scenario.

/// Simulation profile configuration.
#[derive(Debug, Clone)]
pub struct SimProfile {
    /// Human-readable name
    pub name: &'static str,
    /// Number of parking lots
    pub lots: usize,
    /// Slots per lot
    pub slots_per_lot: usize,
    /// Total users to create
    pub users: usize,
    /// Target bookings per simulated day
    pub bookings_per_day: usize,
    /// Number of simulated days
    pub days: usize,
    /// Enable recurring booking generation (~5% of users)
    pub enable_recurring: bool,
    /// Enable cancellations (~15% cancel rate)
    pub enable_cancellations: bool,
    /// Enable waitlist entries (~3% of booking attempts)
    pub enable_waitlist: bool,
    /// Enable intentional conflict attempts (~2% of bookings)
    pub enable_conflicts: bool,
    /// Enable peak-hour surge (8-9am, 5-6pm)
    pub enable_peak_hours: bool,
}

/// Small office: 1 lot, 200 slots, 500 users, 50 bookings/day, 30 days.
pub const SMALL: SimProfile = SimProfile {
    name: "small",
    lots: 1,
    slots_per_lot: 200,
    users: 500,
    bookings_per_day: 50,
    days: 30,
    enable_recurring: true,
    enable_cancellations: true,
    enable_waitlist: true,
    enable_conflicts: true,
    enable_peak_hours: true,
};

/// University campus: 3 lots, 267 slots each, 2000 users, 200 bookings/day.
pub const CAMPUS: SimProfile = SimProfile {
    name: "campus",
    lots: 3,
    slots_per_lot: 267,
    users: 2000,
    bookings_per_day: 200,
    days: 30,
    enable_recurring: true,
    enable_cancellations: true,
    enable_waitlist: true,
    enable_conflicts: true,
    enable_peak_hours: true,
};

/// Enterprise campus: 5 lots, 400 slots each, 5000 users, 500 bookings/day.
pub const ENTERPRISE: SimProfile = SimProfile {
    name: "enterprise",
    lots: 5,
    slots_per_lot: 400,
    users: 5000,
    bookings_per_day: 500,
    days: 30,
    enable_recurring: true,
    enable_cancellations: true,
    enable_waitlist: true,
    enable_conflicts: true,
    enable_peak_hours: true,
};

impl SimProfile {
    /// Total slots across all lots.
    pub fn total_slots(&self) -> usize {
        self.lots * self.slots_per_lot
    }

    /// Expected total bookings over the simulation.
    pub fn expected_total_bookings(&self) -> usize {
        self.bookings_per_day * self.days
    }

    /// Expected cancellation count (~15%).
    pub fn expected_cancellations(&self) -> usize {
        (self.expected_total_bookings() as f64 * 0.15) as usize
    }

    /// Expected recurring users (~5%).
    #[allow(dead_code)]
    pub fn expected_recurring_users(&self) -> usize {
        (self.users as f64 * 0.05) as usize
    }

    /// Expected waitlist entries (~3% of booking attempts).
    #[allow(dead_code)]
    pub fn expected_waitlist_entries(&self) -> usize {
        (self.expected_total_bookings() as f64 * 0.03) as usize
    }

    /// Expected conflict attempts (~2%).
    #[allow(dead_code)]
    pub fn expected_conflict_attempts(&self) -> usize {
        (self.expected_total_bookings() as f64 * 0.02) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_profile_numbers() {
        assert_eq!(SMALL.total_slots(), 200);
        assert_eq!(SMALL.expected_total_bookings(), 1500);
        assert_eq!(SMALL.expected_cancellations(), 225);
    }

    #[test]
    fn campus_profile_numbers() {
        assert_eq!(CAMPUS.total_slots(), 801);
        assert_eq!(CAMPUS.expected_total_bookings(), 6000);
    }

    #[test]
    fn enterprise_profile_numbers() {
        assert_eq!(ENTERPRISE.total_slots(), 2000);
        assert_eq!(ENTERPRISE.expected_total_bookings(), 15000);
    }
}
