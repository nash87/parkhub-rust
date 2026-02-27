#!/usr/bin/env python3
"""
ParkHub Demo Seed Script
========================
Populates the ParkHub Rust/Axum API with:
  - 10 German parking lots (each with floors and slots)
  - 200 demo users with vehicles
  - ~3500 bookings spread over the next 30 days

Usage:
    python scripts/seed_demo.py [--base-url URL] [--admin-password PW] [--dry-run]

Prerequisites:
    - ParkHub server running (default: http://localhost:7878)
    - Admin account already initialised (default: admin / ParkHub2026!)
    - Python 3.8+, no external dependencies
"""

import argparse
import json
import math
import random
import sys
import uuid
from datetime import datetime, timedelta, timezone
from typing import Any, Optional
from urllib import request, error
from urllib.parse import urlencode

# ─── Configuration ────────────────────────────────────────────────────────────
BASE_URL       = "http://localhost:7878"
ADMIN_USER     = "admin"
ADMIN_PASSWORD = "ParkHub2026!"
ADMIN_EMAIL    = "admin@parkhub-demo.de"
ADMIN_NAME     = "Administrator"

# ─── German parking lots ──────────────────────────────────────────────────────
LOTS = [
    {"name": "P+R Hauptbahnhof",         "address": "Bahnhofplatz 1, 80335 München",            "lat": 48.1403, "lon": 11.5583, "floors": 3, "slots_per_floor": 17},
    {"name": "Tiefgarage Marktplatz",    "address": "Marktplatz 5, 70173 Stuttgart",             "lat": 48.7784, "lon":  9.1800, "floors": 2, "slots_per_floor": 40},
    {"name": "Parkhaus Stadtmitte",      "address": "Rathausstraße 12, 50667 Köln",              "lat": 50.9384, "lon":  6.9584, "floors": 3, "slots_per_floor": 20},
    {"name": "P+R Messegelände",         "address": "Messegelände Süd, 60528 Frankfurt am Main", "lat": 50.1109, "lon":  8.6821, "floors": 2, "slots_per_floor": 50},
    {"name": "Parkplatz Einkaufszentrum","address": "Shoppingcenter 3, 22335 Hamburg",           "lat": 53.5753, "lon":  9.9803, "floors": 2, "slots_per_floor": 20},
    {"name": "Tiefgarage Rathaus",       "address": "Rathausplatz 1, 90403 Nürnberg",            "lat": 49.4521, "lon": 11.0767, "floors": 1, "slots_per_floor": 30},
    {"name": "Parkhaus Technologiepark", "address": "Technologiestraße 8, 76131 Karlsruhe",      "lat": 49.0069, "lon":  8.4037, "floors": 3, "slots_per_floor": 25},
    {"name": "Parkplatz Universität",    "address": "Universitätsring 1, 69120 Heidelberg",      "lat": 49.4074, "lon":  8.6924, "floors": 2, "slots_per_floor": 35},
    {"name": "Parkplatz Klinikum",       "address": "Klinikumsallee 15, 44137 Dortmund",         "lat": 51.5136, "lon":  7.4653, "floors": 2, "slots_per_floor": 23},
    {"name": "P+R Bahnhof Ost",          "address": "Ostbahnhofstraße 3, 04315 Leipzig",         "lat": 51.3397, "lon": 12.3731, "floors": 2, "slots_per_floor": 28},
]

FLOOR_NAMES = {1: "Erdgeschoss", 2: "1. Obergeschoss", 3: "2. Obergeschoss",
               4: "3. Obergeschoss", -1: "Untergeschoss 1", -2: "Untergeschoss 2"}
HOURLY_RATES = [2.00, 2.50, 3.00, 1.50, 1.80]

# ─── German names ─────────────────────────────────────────────────────────────
FIRST_NAMES = [
    "Hans", "Peter", "Klaus", "Michael", "Thomas", "Andreas", "Stefan", "Christian",
    "Markus", "Sebastian", "Daniel", "Tobias", "Florian", "Matthias", "Martin",
    "Frank", "Juergen", "Uwe", "Carsten", "Oliver", "Maria", "Anna", "Sandra",
    "Andrea", "Nicole", "Stefanie", "Christina", "Monika", "Petra", "Claudia",
    "Katja", "Sabine", "Julia", "Laura", "Sarah", "Lisa", "Katharina", "Melanie",
    "Susanne", "Anja", "Bernd", "Wolfgang", "Rainer", "Dieter", "Helmut",
    "Gerhard", "Manfred", "Werner", "Karl", "Heike", "Renate", "Ursula",
    "Brigitte", "Ingrid", "Elke", "Gabi", "Birgit", "Karin", "Silke", "Patrick",
]
LAST_NAMES = [
    "Mueller", "Schmidt", "Schneider", "Fischer", "Weber", "Meyer", "Wagner",
    "Becker", "Schulz", "Hoffmann", "Koch", "Richter", "Bauer", "Klein", "Wolf",
    "Schroeder", "Neumann", "Schwarz", "Zimmermann", "Braun", "Krueger", "Hofmann",
    "Hartmann", "Lang", "Schmitt", "Winter", "Berger", "Weiss", "Lange", "Schmitz",
    "Kraus", "Mayer", "Huber", "Lehmann", "Koehler", "Herrmann", "Koenig",
    "Walter", "Fuchs", "Kaiser", "Peters", "Jung", "Hahn", "Scholz", "Roth",
]
PLATE_PREFIXES = ["M", "HH", "B", "K", "F", "S", "N", "DO", "E", "L",
                  "HD", "KA", "MA", "A", "R", "BO", "WUE", "OB", "WI"]
CAR_DATA = [
    ("Volkswagen", ["Golf", "Passat", "Tiguan", "Polo"]),
    ("BMW",        ["3er", "5er", "X5", "1er", "X3"]),
    ("Mercedes",   ["C-Klasse", "E-Klasse", "A-Klasse", "GLC"]),
    ("Audi",       ["A4", "A6", "Q5", "A3", "Q3"]),
    ("Opel",       ["Astra", "Corsa", "Insignia", "Mokka"]),
    ("Ford",       ["Focus", "Fiesta", "Kuga", "Puma"]),
    ("Skoda",      ["Octavia", "Superb", "Fabia", "Karoq"]),
    ("Toyota",     ["Corolla", "Yaris", "RAV4", "C-HR"]),
]
COLORS = ["Schwarz", "Weiss", "Silber", "Grau", "Blau", "Rot", "Gruen", "Braun"]


# ─── HTTP helpers ─────────────────────────────────────────────────────────────

class Client:
    def __init__(self, base_url: str, token: Optional[str] = None):
        self.base_url = base_url.rstrip("/")
        self.token = token

    def post(self, path: str, body: dict) -> dict:
        return self._request("POST", path, body)

    def get(self, path: str) -> dict:
        return self._request("GET", path)

    def _request(self, method: str, path: str, body: Optional[dict] = None) -> dict:
        url  = self.base_url + path
        data = json.dumps(body).encode("utf-8") if body else None
        headers = {"Content-Type": "application/json", "Accept": "application/json"}
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"
        req = request.Request(url, data=data, headers=headers, method=method)
        try:
            with request.urlopen(req, timeout=30) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except error.HTTPError as e:
            body_text = e.read().decode("utf-8", errors="replace")
            raise RuntimeError(f"HTTP {e.code} on {method} {path}: {body_text}") from e


# ─── Model builders ───────────────────────────────────────────────────────────

def make_slot(lot_id: str, floor_id: str, slot_num: int, row: int, col: int) -> dict:
    return {
        "id":              str(uuid.uuid4()),
        "lot_id":          lot_id,
        "floor_id":        floor_id,
        "slot_number":     slot_num,
        "row":             row,
        "column":          col,
        "slot_type":       "standard",
        "status":          "available",
        "current_booking": None,
        "features":        [],
        "position":        {"x": float(col) * 3.0, "y": float(row) * 5.5,
                            "width": 2.5, "height": 5.0, "rotation": 0.0},
    }


def make_floor(lot_id: str, floor_index: int, slot_count: int) -> dict:
    floor_id   = str(uuid.uuid4())
    floor_num  = floor_index + 1
    floor_name = FLOOR_NAMES.get(floor_num, f"Ebene {floor_num}")
    cols       = max(5, math.ceil(math.sqrt(slot_count)))

    slots = [
        make_slot(lot_id, floor_id, i + 1,
                  row=i // cols, col=i % cols)
        for i in range(slot_count)
    ]

    return {
        "id":              floor_id,
        "lot_id":          lot_id,
        "name":            floor_name,
        "floor_number":    floor_num,
        "total_slots":     slot_count,
        "available_slots": slot_count,
        "slots":           slots,
    }


def make_lot(lot_def: dict) -> dict:
    lot_id      = str(uuid.uuid4())
    floors_n    = lot_def["floors"]
    spf         = lot_def["slots_per_floor"]
    total_slots = floors_n * spf
    hourly      = random.choice(HOURLY_RATES)

    floors = [make_floor(lot_id, i, spf) for i in range(floors_n)]

    pricing = {
        "currency": "EUR",
        "rates": [
            {"duration_minutes": 60,   "price": hourly,         "label": "1 Stunde"},
            {"duration_minutes": 120,  "price": hourly * 2,     "label": "2 Stunden"},
            {"duration_minutes": 240,  "price": hourly * 3.5,   "label": "4 Stunden"},
            {"duration_minutes": 1440, "price": hourly * 8,     "label": "Tagesticket"},
        ],
        "daily_max":     round(hourly * 8, 2),
        "monthly_pass":  round(hourly * 8 * 20, 2),
    }

    day_hours = {"open": "06:00", "close": "22:00"}
    operating_hours = {
        "is_24h":    False,
        "monday":    day_hours, "tuesday":  day_hours, "wednesday": day_hours,
        "thursday":  day_hours, "friday":   day_hours,
        "saturday":  {"open": "07:00", "close": "20:00"},
        "sunday":    {"open": "08:00", "close": "18:00"},
    }

    now = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    return {
        "id":               lot_id,
        "name":             lot_def["name"],
        "address":          lot_def["address"],
        "latitude":         lot_def["lat"],
        "longitude":        lot_def["lon"],
        "total_slots":      total_slots,
        "available_slots":  total_slots,
        "floors":           floors,
        "amenities":        ["covered", "security_camera", "well_lit"],
        "pricing":          pricing,
        "operating_hours":  operating_hours,
        "images":           [],
        "status":           "open",
        "created_at":       now,
        "updated_at":       now,
    }


def generate_plate(used: set) -> str:
    for _ in range(50):
        prefix  = random.choice(PLATE_PREFIXES)
        letters = "".join(random.choice("ABCDEFGHJKLMNPRSTUVWXYZ") for _ in range(2))
        digits  = random.randint(100, 9999)
        plate   = f"{prefix}-{letters} {digits}"
        if plate not in used:
            used.add(plate)
            return plate
    return f"M-XX {random.randint(1000, 9999)}"


# ─── Seed routines ────────────────────────────────────────────────────────────

def ensure_admin(client: Client) -> str:
    """Login as admin or register if not yet created. Returns access token."""
    print("  → Logging in as admin...")
    try:
        resp = client.post("/api/v1/auth/login", {"username": ADMIN_USER, "password": ADMIN_PASSWORD})
        token = resp["data"]["tokens"]["access_token"]
        print(f"  → Admin login OK (token: {token[:16]}...)")
        return token
    except RuntimeError as e:
        if "401" in str(e) or "404" in str(e) or "not found" in str(e).lower():
            # Try to register admin
            print("  → Admin not found, registering...")
            resp = client.post("/api/v1/auth/register", {
                "username": ADMIN_USER,
                "email":    ADMIN_EMAIL,
                "password": ADMIN_PASSWORD,
                "name":     ADMIN_NAME,
            })
            token = resp["data"]["tokens"]["access_token"]
            print(f"  → Admin registered OK")
            return token
        raise


def seed_lots(client: Client) -> list[dict]:
    """Create 10 parking lots. Returns list of lot metadata with flat slot IDs."""
    print(f"\n  Seeding {len(LOTS)} parking lots...")
    results = []
    for lot_def in LOTS:
        lot_payload = make_lot(lot_def)
        try:
            resp = client.post("/api/v1/lots", lot_payload)
            saved = resp.get("data", lot_payload)
        except RuntimeError as e:
            print(f"  ⚠ Failed to create lot '{lot_def['name']}': {e}")
            saved = lot_payload  # use locally generated data

        # Flatten slot IDs from all floors for booking generation
        slot_ids = []
        for floor in lot_payload["floors"]:
            slot_ids.extend(s["id"] for s in floor["slots"])

        results.append({
            "id":       lot_payload["id"],
            "name":     lot_def["name"],
            "slot_ids": slot_ids,
        })
        print(f"  ✓ {lot_def['name']} ({len(slot_ids)} slots)")

    return results


def seed_users(client: Client) -> list[dict]:
    """Register 198 demo users with vehicles. Returns list of {id, token, plate}."""
    print("\n  Seeding 198 demo users...")
    users  = []
    used_names: set[str] = set()
    used_plates: set[str] = set()

    for i in range(198):
        first = random.choice(FIRST_NAMES)
        last  = random.choice(LAST_NAMES)
        base  = f"{first.lower()}.{last.lower()}"
        username = base
        attempt  = 1
        while username in used_names:
            username = f"{base}{attempt}"
            attempt += 1
        used_names.add(username)

        plate = generate_plate(used_plates)
        car   = random.choice(CAR_DATA)

        try:
            resp = client.post("/api/v1/auth/register", {
                "username": username,
                "email":    f"{username}@example.de",
                "password": "Demo2026!X",
                "name":     f"{first} {last}",
            })
            user_token = resp["data"]["tokens"]["access_token"]
            user_id    = resp["data"]["user"]["id"]

            # Create primary vehicle
            user_client = Client(BASE_URL, user_token)
            now = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
            user_client.post("/api/v1/vehicles", {
                "id":            str(uuid.uuid4()),
                "user_id":       user_id,
                "license_plate": plate,
                "make":          car[0],
                "model":         random.choice(car[1]),
                "color":         random.choice(COLORS),
                "vehicle_type":  "car",
                "is_default":    True,
                "created_at":    now,
            })

            users.append({"id": user_id, "token": user_token, "plate": plate})

        except RuntimeError as e:
            print(f"  ⚠ User {username} failed: {e}")
            continue

        if (i + 1) % 20 == 0:
            print(f"  ✓ {i + 1}/198 users created")

    print(f"  ✓ {len(users)} users created")
    return users


def seed_bookings(lot_data: list[dict], users: list[dict]) -> None:
    """Create ~3500 bookings over the next 30 days using per-user tokens."""
    print("\n  Seeding ~3500 bookings (next 30 days)...")
    now       = datetime.now(timezone.utc)
    total     = 0
    errors    = 0

    for day_offset in range(30):
        day        = now + timedelta(days=day_offset)
        dow        = day.weekday()  # 0=Mon, 6=Sun
        is_weekend = dow >= 5
        target     = random.randint(40, 70) if is_weekend else random.randint(120, 165)

        for _ in range(target):
            user = random.choice(users)
            lot  = random.choice(lot_data)
            slot_id = random.choice(lot["slot_ids"])

            # Booking window
            if is_weekend:
                start_hour = random.randint(9, 15)
            else:
                start_hour = random.choice([7, 7, 8, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17])

            start_min  = random.choice([0, 15, 30, 45])
            duration   = random.choice([30, 60, 90, 120, 180, 240, 300, 360, 480])

            start_time = day.replace(hour=start_hour, minute=start_min, second=0, microsecond=0)
            # Bookings must be in the future (server validates this)
            if start_time <= now:
                start_time = now + timedelta(minutes=random.randint(5, 60))

            user_client = Client(BASE_URL, user["token"])
            try:
                user_client.post("/api/v1/bookings", {
                    "lot_id":          lot["id"],
                    "slot_id":         slot_id,
                    "start_time":      start_time.isoformat().replace("+00:00", "Z"),
                    "duration_minutes": duration,
                    "license_plate":   user["plate"],
                    "notes":           None,
                })
                total += 1
            except RuntimeError:
                errors += 1
                continue

        if (day_offset + 1) % 5 == 0:
            print(f"  ✓ Day {day_offset + 1}/30 done — {total} bookings so far ({errors} errors)")

    print(f"  ✓ {total} bookings created ({errors} slot conflicts / errors skipped)")


# ─── Entry point ──────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="ParkHub demo data seeder")
    parser.add_argument("--base-url",        default=BASE_URL,       help="API base URL")
    parser.add_argument("--admin-password",  default=ADMIN_PASSWORD, help="Admin password")
    parser.add_argument("--dry-run",         action="store_true",    help="Build payloads but don't call API")
    args = parser.parse_args()

    global BASE_URL, ADMIN_PASSWORD
    BASE_URL       = args.base_url
    ADMIN_PASSWORD = args.admin_password

    if args.dry_run:
        print("DRY RUN — building one lot payload and printing:")
        print(json.dumps(make_lot(LOTS[0]), indent=2)[:2000])
        return

    print("🏁 ParkHub Rust — Production Demo Seeder")
    print(f"   Target: {BASE_URL}")

    client = Client(BASE_URL)

    # 1. Admin login
    admin_token = ensure_admin(client)
    admin_client = Client(BASE_URL, admin_token)

    # 2. Lots
    lot_data = seed_lots(admin_client)

    # 3. Users + vehicles
    users = seed_users(client)  # registers use public endpoint
    if not users:
        print("❌ No users created — aborting booking seeding")
        sys.exit(1)

    # 4. Bookings
    seed_bookings(lot_data, users)

    print("\n✅ Seed complete!")
    print(f"   Parking lots : {len(lot_data)}")
    print(f"   Users        : {len(users)}")
    print(f"   Credentials  : admin / {ADMIN_PASSWORD} | any user / Demo2026!X")
    print(f"   Dashboard    : {BASE_URL}")


if __name__ == "__main__":
    main()
