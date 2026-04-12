//! Realistic data generation for simulation.
//!
//! Generates user names, emails, vehicle plates, and booking patterns
//! with workday-heavy traffic and peak-hour surges.

use rand::Rng;

/// Realistic German first names.
const FIRST_NAMES: &[&str] = &[
    "Anna",
    "Ben",
    "Clara",
    "David",
    "Elena",
    "Felix",
    "Greta",
    "Hans",
    "Ines",
    "Jan",
    "Katharina",
    "Lukas",
    "Marie",
    "Niklas",
    "Olivia",
    "Paul",
    "Quirin",
    "Rosa",
    "Stefan",
    "Tanja",
    "Uwe",
    "Vera",
    "Wolfgang",
    "Xenia",
    "Yannick",
    "Zoe",
    "Alexander",
    "Bianca",
    "Christian",
    "Diana",
    "Erik",
    "Franziska",
    "Georg",
    "Helena",
    "Igor",
    "Julia",
    "Klaus",
    "Laura",
    "Markus",
    "Nina",
    "Otto",
    "Petra",
    "Robert",
    "Sabine",
    "Thomas",
    "Ursula",
    "Viktor",
    "Waltraud",
    "Maximilian",
    "Sophie",
];

/// Realistic German last names.
const LAST_NAMES: &[&str] = &[
    "Mueller",
    "Schmidt",
    "Schneider",
    "Fischer",
    "Weber",
    "Meyer",
    "Wagner",
    "Becker",
    "Schulz",
    "Hoffmann",
    "Schaefer",
    "Koch",
    "Bauer",
    "Richter",
    "Klein",
    "Wolf",
    "Schroeder",
    "Neumann",
    "Schwarz",
    "Zimmermann",
    "Braun",
    "Krueger",
    "Hofmann",
    "Hartmann",
    "Lange",
    "Schmitt",
    "Werner",
    "Schmitz",
    "Krause",
    "Meier",
    "Lehmann",
    "Schmid",
    "Schulze",
    "Maier",
    "Koehler",
    "Herrmann",
    "Koenig",
    "Walter",
    "Mayer",
    "Huber",
    "Kaiser",
    "Fuchs",
    "Peters",
    "Lang",
    "Scholz",
    "Moeller",
    "Weiland",
    "Jung",
    "Gross",
    "Friedrich",
];

/// German city codes for license plates.
const CITY_CODES: &[&str] = &[
    "M", "B", "HH", "K", "F", "S", "D", "HB", "H", "N", "DO", "E", "DD", "L", "KA", "WI", "MA",
    "A", "KI", "LU", "FR", "MS", "OL", "DA", "GI", "MR", "OF", "AC", "BN", "SB",
];

/// Generate a random user name.
pub fn random_username(idx: usize) -> String {
    let mut rng = rand::rng();
    let first = FIRST_NAMES[rng.random_range(0..FIRST_NAMES.len())];
    let last = LAST_NAMES[rng.random_range(0..LAST_NAMES.len())];
    format!("{}_{}{}", first.to_lowercase(), last.to_lowercase(), idx)
}

/// Generate a random email.
pub fn random_email(idx: usize) -> String {
    let mut rng = rand::rng();
    let first = FIRST_NAMES[rng.random_range(0..FIRST_NAMES.len())];
    let last = LAST_NAMES[rng.random_range(0..LAST_NAMES.len())];
    format!(
        "{}.{}{}@example.com",
        first.to_lowercase(),
        last.to_lowercase(),
        idx
    )
}

/// Generate a random full name.
pub fn _random_name() -> String {
    let mut rng = rand::rng();
    let first = FIRST_NAMES[rng.random_range(0..FIRST_NAMES.len())];
    let last = LAST_NAMES[rng.random_range(0..LAST_NAMES.len())];
    format!("{first} {last}")
}

/// Generate a random German license plate.
pub fn random_license_plate() -> String {
    let mut rng = rand::rng();
    let city = CITY_CODES[rng.random_range(0..CITY_CODES.len())];
    let letter1 = (b'A' + rng.random_range(0..26u8)) as char;
    let letter2 = (b'A' + rng.random_range(0..26u8)) as char;
    let number: u16 = rng.random_range(1..9999);
    format!("{city}-{letter1}{letter2} {number}")
}

/// Pick a booking start hour based on peak-hour distribution.
///
/// Peak hours (8-9, 17-18) get ~3x the weight of off-peak hours.
/// Weekdays have heavier traffic than weekends.
pub fn booking_start_hour(is_weekday: bool, enable_peak: bool) -> u32 {
    let mut rng = rand::rng();

    if !is_weekday {
        // Weekend: mostly 10-16 with light spread
        return rng.random_range(9..18);
    }

    if !enable_peak {
        return rng.random_range(7..20);
    }

    // Weighted distribution for weekdays with peak hours
    let r: f64 = rng.random_range(0.0..1.0);
    if r < 0.25 {
        // 25% chance: morning peak 7-9
        rng.random_range(7..10)
    } else if r < 0.40 {
        // 15% chance: late morning 9-12
        rng.random_range(9..12)
    } else if r < 0.55 {
        // 15% chance: lunch 12-14
        rng.random_range(12..14)
    } else if r < 0.70 {
        // 15% chance: afternoon 14-17
        rng.random_range(14..17)
    } else if r < 0.90 {
        // 20% chance: evening peak 17-19
        rng.random_range(17..19)
    } else {
        // 10% chance: off-peak early/late
        rng.random_range(6..21)
    }
}

/// Pick a booking duration in minutes (common patterns).
pub fn booking_duration_minutes() -> i32 {
    let mut rng = rand::rng();
    let r: f64 = rng.random_range(0.0..1.0);

    if r < 0.10 {
        60 // 10%: 1 hour
    } else if r < 0.30 {
        120 // 20%: 2 hours
    } else if r < 0.55 {
        240 // 25%: 4 hours (half day)
    } else if r < 0.80 {
        480 // 25%: 8 hours (full day)
    } else if r < 0.90 {
        360 // 10%: 6 hours
    } else {
        rng.random_range(30..600) // 10%: random 30min to 10h
    }
}

/// Decide whether a booking should be cancelled (15% rate).
pub fn should_cancel() -> bool {
    let mut rng = rand::rng();
    rng.random_range(0.0..1.0) < 0.15
}

/// Decide whether this user gets recurring bookings (5%).
pub fn is_recurring_user() -> bool {
    let mut rng = rand::rng();
    rng.random_range(0.0..1.0) < 0.05
}

/// Decide whether this booking attempt should be a waitlist entry (3%).
pub fn should_waitlist() -> bool {
    let mut rng = rand::rng();
    rng.random_range(0.0..1.0) < 0.03
}

/// Decide whether this booking is an intentional conflict (2%).
pub fn is_conflict_attempt() -> bool {
    let mut rng = rand::rng();
    rng.random_range(0.0..1.0) < 0.02
}

/// Decide if today is a weekday (Mon-Fri).
pub fn is_weekday(day_in_month: usize) -> bool {
    // Simple simulation: days 1-5 = weekdays, 6-7 = weekend, repeat
    let day_of_week = day_in_month % 7;
    day_of_week < 5
}

/// Traffic multiplier based on day type.
/// Weekdays get 100% traffic, weekends get ~30%.
pub fn day_traffic_multiplier(day_in_month: usize) -> f64 {
    if is_weekday(day_in_month) { 1.0 } else { 0.3 }
}

/// Pick a random lot index.
pub fn random_lot_index(num_lots: usize) -> usize {
    let mut rng = rand::rng();
    rng.random_range(0..num_lots)
}

/// Pick a random slot index.
pub fn random_slot_index(num_slots: usize) -> usize {
    let mut rng = rand::rng();
    rng.random_range(0..num_slots)
}

/// Pick a random user index.
pub fn random_user_index(num_users: usize) -> usize {
    let mut rng = rand::rng();
    rng.random_range(0..num_users)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_is_valid() {
        let name = random_username(42);
        assert!(name.len() > 3);
        assert!(name.contains('_'));
    }

    #[test]
    fn email_has_at_sign() {
        let email = random_email(7);
        assert!(email.contains('@'));
        assert!(email.ends_with("@example.com"));
    }

    #[test]
    fn license_plate_has_dash() {
        let plate = random_license_plate();
        assert!(plate.contains('-'));
    }

    #[test]
    fn booking_hour_within_range() {
        for _ in 0..100 {
            let hour = booking_start_hour(true, true);
            assert!(hour < 24);
        }
    }

    #[test]
    fn duration_is_positive() {
        for _ in 0..100 {
            let d = booking_duration_minutes();
            assert!(d > 0);
        }
    }

    #[test]
    fn weekday_detection() {
        assert!(is_weekday(0)); // day 0 = weekday
        assert!(is_weekday(4)); // day 4 = weekday (Fri)
        assert!(!is_weekday(5)); // day 5 = weekend (Sat)
        assert!(!is_weekday(6)); // day 6 = weekend (Sun)
        assert!(is_weekday(7)); // day 7 = weekday (Mon again)
    }
}
