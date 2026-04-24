//! First-run seeding: bootstrap admin, dummy users, sample parking lot,
//! and the full demo-mode fixture (10 realistic lots + 200 users).
//!
//! Every write goes directly to [`crate::db::Database`] so the seed path
//! works in distroless container builds without shelling out to an
//! external script.

use anyhow::Result;
use tracing::info;

use crate::config::ServerConfig;
use crate::db::Database;

use super::paths::hash_password;

/// Create the admin user in the database
pub(crate) async fn create_admin_user(db: &Database, config: &ServerConfig) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{User, UserPreferences, UserRole};
    use uuid::Uuid;

    let admin_user = User {
        id: Uuid::new_v4(),
        username: config.admin_username.clone(),
        email: format!("{}@parkhub.test", config.admin_username),
        password_hash: config.admin_password_hash.clone(),
        name: "Administrator".to_string(),
        picture: None,
        phone: None,
        role: UserRole::SuperAdmin,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
        preferences: UserPreferences::default(),
        is_active: true,
        // Give the admin a real monthly credits allowance so the dashboard
        // KPI row shows something useful on first login instead of a row
        // of zeros — the seeded demo users get rand(5..41), this mirrors
        // the generous end of that range for the principal account.
        credits_balance: 40,
        credits_monthly_quota: 40,
        credits_last_refilled: Some(Utc::now()),
        // SAFETY(T-1731): bootstrap SuperAdmin created from CLI config at
        // first launch — platform admin, intentionally tenant-less.
        tenant_id: None,
        accessibility_needs: None,
        cost_center: None,
        department: Some("IT".to_string()),
        settings: None,
    };

    db.save_user(&admin_user).await?;
    db.mark_setup_completed().await?;
    info!(
        "Admin user '{}' created successfully",
        config.admin_username
    );
    Ok(())
}

/// Username generation styles for dummy users
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsernameStyle {
    /// First letter + last letter (e.g., "Alex Smith" -> "ah")
    FirstLastLetter,
    /// First name + last name (e.g., "Alex Smith" -> "alex.smith")
    FirstDotLast,
    /// First letter + last name (e.g., "Alex Smith" -> "asmith")
    InitialLast,
    /// First name + last initial (e.g., "Alex Smith" -> "alexs")
    FirstInitial,
}

impl UsernameStyle {
    /// Generate username from first and last name
    fn generate(self, first: &str, last: &str, index: usize) -> String {
        let base = match self {
            Self::FirstLastLetter => {
                let first_char = first
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                let last_char = last
                    .chars()
                    .last()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                format!("{first_char}{last_char}")
            }
            Self::FirstDotLast => {
                format!("{}.{}", first.to_lowercase(), last.to_lowercase())
            }
            Self::InitialLast => {
                let first_char = first
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                format!("{}{}", first_char, last.to_lowercase())
            }
            Self::FirstInitial => {
                let last_char = last
                    .chars()
                    .next()
                    .unwrap_or('x')
                    .to_lowercase()
                    .next()
                    .unwrap();
                format!("{}{}", first.to_lowercase(), last_char)
            }
        };
        // Add index to ensure uniqueness
        format!("{}{}", base, index + 1)
    }
}

/// Generate 50 GDPR-compliant dummy users for testing.
#[allow(clippy::too_many_lines)]
pub(crate) async fn generate_dummy_users(
    db: &Database,
    username_style: UsernameStyle,
) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{User, UserPreferences, UserRole};
    use rand::RngExt;
    use uuid::Uuid;

    // GDPR-compliant fictional first names (common, not identifying real people)
    let first_names = [
        "Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Quinn", "Avery", "Skyler",
        "Dakota", "Cameron", "Reese", "Parker", "Hayden", "Sage", "River", "Phoenix", "Blake",
        "Drew", "Jamie", "Robin", "Charlie", "Sam", "Pat", "Chris", "Lee", "Kim", "Ashley", "Lynn",
        "Terry", "Jesse", "Dana", "Kelly", "Shannon", "Shawn", "Logan", "Peyton", "Kendall",
        "Reagan", "Finley", "Emerson", "Ellis", "Rowan", "Ainsley", "Blair", "Devon", "Eden",
        "Gray", "Harper", "Indigo",
    ];

    // GDPR-compliant fictional last names (common, not identifying real people)
    let last_names = [
        "Smith",
        "Johnson",
        "Williams",
        "Brown",
        "Jones",
        "Garcia",
        "Miller",
        "Davis",
        "Rodriguez",
        "Martinez",
        "Anderson",
        "Taylor",
        "Thomas",
        "Jackson",
        "White",
        "Harris",
        "Martin",
        "Thompson",
        "Moore",
        "Young",
        "Allen",
        "King",
        "Wright",
        "Scott",
        "Green",
        "Baker",
        "Adams",
        "Nelson",
        "Hill",
        "Ramirez",
        "Campbell",
        "Mitchell",
        "Roberts",
        "Carter",
        "Phillips",
        "Evans",
        "Turner",
        "Torres",
        "Parker",
        "Collins",
        "Edwards",
        "Stewart",
        "Flores",
        "Morris",
        "Murphy",
        "Rivera",
        "Cook",
        "Rogers",
        "Morgan",
        "Peterson",
    ];

    let default_password = seed_password("PARKHUB_DUMMY_USERS_PASSWORD", "dummy-users");
    let password_hash = hash_password(&default_password)?;

    // Role distribution: mostly Users, some Premium, few Admin
    let roles = [
        UserRole::User,
        UserRole::User,
        UserRole::User,
        UserRole::User,
        UserRole::Premium,
        UserRole::Admin,
    ];

    info!(
        "Generating 50 GDPR-compliant dummy users (password source: PARKHUB_DUMMY_USERS_PASSWORD or generated fallback)..."
    );

    // Pre-generate all users with rng (ThreadRng is not Send, so must not cross await)
    let users: Vec<User> = {
        let mut rng = rand::rng();
        (0..50)
            .map(|i| {
                let first = first_names[rng.random_range(0..first_names.len())];
                let last = last_names[rng.random_range(0..last_names.len())];
                let role = roles[rng.random_range(0..roles.len())].clone();
                let username = username_style.generate(first, last, i);
                let email = format!("{username}@example.com");

                User {
                    id: Uuid::new_v4(),
                    username,
                    email,
                    password_hash: password_hash.clone(),
                    name: format!("{first} {last}"),
                    picture: None,
                    phone: Some(format!("+1-555-{:04}", rng.random_range(1000..9999))),
                    role,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_login: None,
                    preferences: UserPreferences::default(),
                    is_active: true,
                    credits_balance: rng.random_range(10..41),
                    credits_monthly_quota: 40,
                    credits_last_refilled: Some(Utc::now()),
                    // SAFETY(T-1731): dummy seed users, platform-wide; kept
                    // tenant-less until an operator assigns them a tenant.
                    tenant_id: None,
                    accessibility_needs: None,
                    cost_center: None,
                    department: None,
                    settings: None,
                }
            })
            .collect()
    };

    for user in &users {
        db.save_user(user).await?;
    }

    info!("Created 50 dummy users successfully");
    info!("Default login: any username with password '{default_password}'",);
    Ok(())
}

/// Create a sample parking lot for testing
pub(crate) async fn create_sample_parking_lot(db: &Database) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{
        LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo, PricingRate,
        SlotFeature, SlotPosition, SlotStatus, SlotType,
    };
    use uuid::Uuid;

    let lot_id = Uuid::new_v4();
    let floor_id = Uuid::new_v4();

    // Create 10 parking slots
    let mut slots = Vec::new();
    for i in 1..=10 {
        slots.push(ParkingSlot {
            id: Uuid::new_v4(),
            lot_id,
            floor_id,
            slot_number: i,
            row: (i - 1) / 5,
            column: (i - 1) % 5,
            slot_type: if i == 1 {
                SlotType::Handicap
            } else if i == 10 {
                SlotType::Electric
            } else {
                SlotType::Standard
            },
            status: SlotStatus::Available,
            current_booking: None,
            features: if i <= 2 {
                vec![SlotFeature::NearExit]
            } else {
                vec![]
            },
            position: SlotPosition {
                x: ((i - 1) % 5) as f32 * 80.0,
                y: ((i - 1) / 5) as f32 * 100.0,
                width: 70.0,
                height: 90.0,
                rotation: 0.0,
            },
            is_accessible: i == 1, // First slot is accessible (handicap)
        });
    }

    let floor = ParkingFloor {
        id: floor_id,
        lot_id,
        name: "Ground Floor".to_string(),
        floor_number: 0,
        total_slots: 10,
        available_slots: 10,
        slots: slots.clone(),
    };

    let lot = ParkingLot {
        id: lot_id,
        name: "Home Parking".to_string(),
        address: "123 Main Street".to_string(),
        latitude: 0.0,
        longitude: 0.0,
        total_slots: 10,
        available_slots: 10,
        floors: vec![floor],
        amenities: vec!["Security".to_string(), "Covered".to_string()],
        pricing: PricingInfo {
            currency: "EUR".to_string(),
            rates: vec![
                PricingRate {
                    duration_minutes: 60,
                    price: 2.0,
                    label: "1 hour".to_string(),
                },
                PricingRate {
                    duration_minutes: 120,
                    price: 3.5,
                    label: "2 hours".to_string(),
                },
                PricingRate {
                    duration_minutes: 240,
                    price: 6.0,
                    label: "4 hours".to_string(),
                },
            ],
            daily_max: Some(15.0),
            monthly_pass: Some(200.0),
        },
        operating_hours: OperatingHours {
            is_24h: true,
            monday: None,
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        },
        images: vec![],
        status: LotStatus::Open,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        // SAFETY(T-1731): sample seed lot created by `create_sample_parking_lot`
        // at bootstrap; platform-owned until a tenant claims it.
        tenant_id: None,
    };

    // Save parking lot
    db.save_parking_lot(&lot).await?;

    // Save all slots
    for slot in &slots {
        db.save_parking_slot(slot).await?;
    }

    info!("Sample parking lot created with {} slots", slots.len());
    Ok(())
}

/// Seed demo data: 10 realistic parking lots and 200 demo users.
///
/// Called at startup when `SEED_DEMO_DATA=true` or `DEMO_MODE=true` and the
/// database has fewer than two parking lots.  All writes go directly to the
/// database — no HTTP API calls, no shell scripts, and no external tools are
/// required, making this safe for distroless container deployments.
#[allow(clippy::too_many_lines)]
pub(crate) async fn seed_demo_data(db: &Database) -> Result<()> {
    use chrono::Utc;
    use parkhub_common::models::{
        DayHours, LotStatus, OperatingHours, ParkingFloor, ParkingLot, ParkingSlot, PricingInfo,
        PricingRate, SlotFeature, SlotPosition, SlotStatus, SlotType,
    };
    use rand::RngExt;
    use uuid::Uuid;

    info!("Seeding demo data: 10 parking lots + 200 users...");

    // 10 realistic German parking lots (mirroring the former seed_demo.sh)
    let lots_data: &[(&str, &str, f64, f64, i32)] = &[
        (
            "P+R Hauptbahnhof",
            "Bahnhofplatz 1, 80335 München",
            48.1403,
            11.5583,
            51,
        ),
        (
            "Tiefgarage Marktplatz",
            "Marktplatz 5, 70173 Stuttgart",
            48.7784,
            9.1800,
            80,
        ),
        (
            "Parkhaus Stadtmitte",
            "Rathausstrasse 12, 50667 Köln",
            50.9384,
            6.9584,
            60,
        ),
        (
            "P+R Messegelände",
            "Messegelände Süd, 60528 Frankfurt",
            50.1109,
            8.6821,
            100,
        ),
        (
            "Parkplatz Einkaufszentrum",
            "Shoppingcenter 3, 22335 Hamburg",
            53.5753,
            9.9803,
            40,
        ),
        (
            "Tiefgarage Rathaus",
            "Rathausplatz 1, 90403 Nürnberg",
            49.4521,
            11.0767,
            30,
        ),
        (
            "Parkhaus Technologiepark",
            "Technologiestrasse 8, 76131 Karlsruhe",
            49.0069,
            8.4037,
            75,
        ),
        (
            "Parkplatz Universität",
            "Universitätsring 1, 69120 Heidelberg",
            49.4074,
            8.6924,
            70,
        ),
        (
            "Parkplatz Klinikum",
            "Klinikumsallee 15, 44137 Dortmund",
            51.5136,
            7.4653,
            46,
        ),
        (
            "P+R Bahnhof Ost",
            "Ostbahnhofstrasse 3, 04315 Leipzig",
            51.3397,
            12.3731,
            56,
        ),
    ];

    for (name, address, lat, lon, total_slots) in lots_data {
        let lot_id = Uuid::new_v4();
        let floor_id = Uuid::new_v4();
        let total = *total_slots;

        let slots: Vec<ParkingSlot> = (1..=total)
            .map(|i| ParkingSlot {
                id: Uuid::new_v4(),
                lot_id,
                floor_id,
                slot_number: i,
                row: (i - 1) / 10,
                column: (i - 1) % 10,
                slot_type: if i == 1 {
                    SlotType::Handicap
                } else if i == total {
                    SlotType::Electric
                } else {
                    SlotType::Standard
                },
                status: SlotStatus::Available,
                current_booking: None,
                features: if i <= 2 {
                    vec![SlotFeature::NearExit]
                } else {
                    vec![]
                },
                position: SlotPosition {
                    x: ((i - 1) % 10) as f32 * 80.0,
                    y: ((i - 1) / 10) as f32 * 100.0,
                    width: 70.0,
                    height: 90.0,
                    rotation: 0.0,
                },
                is_accessible: i == 1,
            })
            .collect();

        let floor = ParkingFloor {
            id: floor_id,
            lot_id,
            name: "Ground Floor".to_string(),
            floor_number: 0,
            total_slots: total,
            available_slots: total,
            slots: slots.clone(),
        };

        let weekday = DayHours {
            open: "06:00".to_string(),
            close: "22:00".to_string(),
            closed: false,
        };
        let weekend = DayHours {
            open: "07:00".to_string(),
            close: "20:00".to_string(),
            closed: false,
        };
        let lot = ParkingLot {
            id: lot_id,
            name: (*name).to_string(),
            address: (*address).to_string(),
            latitude: *lat,
            longitude: *lon,
            total_slots: total,
            available_slots: total,
            floors: vec![floor],
            amenities: vec!["covered".to_string(), "security_camera".to_string()],
            pricing: PricingInfo {
                currency: "EUR".to_string(),
                rates: vec![
                    PricingRate {
                        duration_minutes: 60,
                        price: 2.50,
                        label: "1h".to_string(),
                    },
                    PricingRate {
                        duration_minutes: 1440,
                        price: 20.0,
                        label: "Day".to_string(),
                    },
                ],
                daily_max: Some(20.0),
                monthly_pass: Some(400.0),
            },
            operating_hours: OperatingHours {
                is_24h: false,
                monday: Some(weekday.clone()),
                tuesday: Some(weekday.clone()),
                wednesday: Some(weekday.clone()),
                thursday: Some(weekday.clone()),
                friday: Some(weekday.clone()),
                saturday: Some(weekend.clone()),
                sunday: Some(weekend),
            },
            images: vec![],
            status: LotStatus::Open,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            // SAFETY(T-1731): demo seed lot (10-lot fixture), platform-owned.
            tenant_id: None,
        };

        db.save_parking_lot(&lot).await?;
        for slot in &slots {
            db.save_parking_slot(slot).await?;
        }
        info!("  Created lot: {} ({total_slots} slots)", name);
    }

    // 200 demo users with German-style names (direct DB writes — no HTTP API)
    let first_names = [
        "Hans",
        "Peter",
        "Klaus",
        "Michael",
        "Thomas",
        "Andreas",
        "Stefan",
        "Christian",
        "Markus",
        "Sebastian",
        "Daniel",
        "Tobias",
        "Florian",
        "Matthias",
        "Martin",
        "Frank",
        "Oliver",
        "Maria",
        "Anna",
        "Sandra",
        "Andrea",
        "Nicole",
        "Stefanie",
        "Christina",
        "Monika",
        "Petra",
        "Claudia",
        "Julia",
        "Laura",
        "Sarah",
        "Lisa",
        "Katharina",
        "Melanie",
        "Susanne",
        "Anja",
    ];
    let last_names = [
        "Müller",
        "Schmidt",
        "Schneider",
        "Fischer",
        "Weber",
        "Meyer",
        "Wagner",
        "Becker",
        "Schulz",
        "Hoffmann",
        "Koch",
        "Richter",
        "Bauer",
        "Klein",
        "Wolf",
        "Schröder",
        "Neumann",
        "Schwarz",
        "Zimmermann",
        "Braun",
        "Krüger",
        "Hofmann",
        "Hartmann",
    ];

    let demo_password = seed_password("PARKHUB_DEMO_USERS_PASSWORD", "demo-users");
    let demo_password_hash = hash_password(&demo_password)?;

    let users: Vec<parkhub_common::models::User> = {
        use parkhub_common::models::{User, UserPreferences};
        let mut rng = rand::rng();
        (1..=200u32)
            .map(|i| {
                let first = first_names[rng.random_range(0..first_names.len())];
                let last = last_names[rng.random_range(0..last_names.len())];
                let username = format!(
                    "{}.{}{}",
                    first.to_lowercase(),
                    last.to_lowercase()
                        .replace('ü', "ue")
                        .replace('ö', "oe")
                        .replace('ä', "ae"),
                    i
                );
                User {
                    id: Uuid::new_v4(),
                    username: username.clone(),
                    email: format!("{username}@example.de"),
                    password_hash: demo_password_hash.clone(),
                    name: format!("{first} {last}"),
                    picture: None,
                    phone: Some(format!(
                        "+49-{:03}-{:07}",
                        rng.random_range(100..999),
                        rng.random_range(1_000_000..9_999_999u32)
                    )),
                    role: parkhub_common::models::UserRole::User,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_login: None,
                    preferences: UserPreferences::default(),
                    is_active: true,
                    credits_balance: rng.random_range(5..41),
                    credits_monthly_quota: 40,
                    credits_last_refilled: Some(Utc::now()),
                    // SAFETY(T-1731): demo seed users (200 German-style fixture
                    // accounts), platform-wide and intentionally tenant-less.
                    tenant_id: None,
                    accessibility_needs: None,
                    cost_center: None,
                    department: None,
                    settings: None,
                }
            })
            .collect()
    };

    for user in &users {
        if let Err(e) = db.save_user(user).await {
            tracing::warn!("Demo seed: failed to save user {}: {e}", user.username);
        }
    }

    info!(
        "Demo seeding complete: 10 lots, 200 users (password source: PARKHUB_DEMO_USERS_PASSWORD or generated fallback)"
    );
    Ok(())
}

fn seed_password(env_name: &str, label: &str) -> String {
    use rand::distr::{Alphanumeric, SampleString};

    std::env::var(env_name).unwrap_or_else(|_| {
        let generated = Alphanumeric.sample_string(&mut rand::rng(), 20);
        tracing::warn!("{label}: generated a temporary password because {env_name} was not set");
        generated
    })
}
