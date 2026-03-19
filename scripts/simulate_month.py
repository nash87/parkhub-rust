#!/usr/bin/env python3
"""
ParkHub 1-Month Booking Simulation
====================================
Generates realistic usage data against a pre-seeded ParkHub instance.
Assumes seed_demo.py has already run (10 lots, ~200 users, vehicles).

Simulation model (30 days):
  - Workdays (Mon-Thu): 60-80% of users book a slot (8am-6pm)
  - Fridays:            40-50% capacity (WFH effect)
  - Weekends:           5-10% capacity (residential use)
  - 10% of users absent on any given day
  - 5-10% of bookings get cancelled
  - 2-3% swap requests per day
  - 2-3 guest bookings per day
  - ~15% of active users have recurring weekly bookings
  - 1 admin announcement per week
  - 1 credit refill (monthly)
  - 2-3 user role changes over the month

Usage:
    python scripts/simulate_month.py [--base-url URL] [--admin-password PW] [--dry-run]

Prerequisites:
    - ParkHub server running (default: http://localhost:7878)
    - Demo data seeded (run seed_demo.py first)
    - Python 3.8+, no external dependencies
"""

import argparse
import json
import random
import sys
import time
import uuid
from collections import defaultdict
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, List, Optional, Tuple
from urllib import error, request

# ─── Defaults ────────────────────────────────────────────────────────────────
BASE_URL = "http://localhost:7878"
ADMIN_USER = "admin"
ADMIN_PASSWORD = "demo"
SIMULATION_DAYS = 30
RATE_LIMIT_DELAY = 0.05  # 50ms between API calls

# ─── Announcement templates ──────────────────────────────────────────────────
ANNOUNCEMENT_TEMPLATES = [
    {
        "title": "Wartungsarbeiten Tiefgarage",
        "message": "Am kommenden Wochenende finden Wartungsarbeiten in der Tiefgarage statt. "
                   "Bitte nutzen Sie alternative Parkplätze.",
        "severity": "warning",
    },
    {
        "title": "Neue Ladestationen verfügbar",
        "message": "Wir haben 10 neue E-Ladestationen im Parkhaus Stadtmitte installiert. "
                   "Buchen Sie jetzt Ihren Platz mit Lademöglichkeit!",
        "severity": "info",
    },
    {
        "title": "Preisanpassung ab nächstem Monat",
        "message": "Aufgrund gestiegener Betriebskosten passen wir die Stundentarife "
                   "leicht an. Details finden Sie in Ihrem Profil.",
        "severity": "info",
    },
    {
        "title": "Sicherheitshinweis",
        "message": "Bitte verriegeln Sie Ihr Fahrzeug und lassen Sie keine Wertsachen sichtbar. "
                   "In den letzten Wochen gab es vereinzelte Vorfälle.",
        "severity": "critical",
    },
    {
        "title": "Frohe Feiertage!",
        "message": "Das ParkHub-Team wünscht Ihnen erholsame Feiertage. "
                   "Unsere Parkplätze sind auch über die Feiertage geöffnet.",
        "severity": "info",
    },
]

GUEST_NAMES = [
    "Max Mustermann", "Erika Musterfrau", "Johann Testmann", "Sabine Probe",
    "Otto Beispiel", "Helga Versuch", "Fritz Temporär", "Gisela Besucher",
    "Karl Gast", "Petra Kurzzeitparker", "Heinz Lieferant", "Inge Kundin",
]


# ─── HTTP Client ─────────────────────────────────────────────────────────────

class Client:
    """Minimal HTTP client using only stdlib. Handles auth and rate limiting."""

    def __init__(self, base_url: str, token: Optional[str] = None, dry_run: bool = False):
        self.base_url = base_url.rstrip("/")
        self.token = token
        self.dry_run = dry_run
        self.request_count = 0
        self.error_count = 0
        self._rate_limit_delay = RATE_LIMIT_DELAY

    def post(self, path: str, body: dict) -> Optional[dict]:
        return self._request("POST", path, body)

    def get(self, path: str) -> Optional[dict]:
        return self._request("GET", path)

    def patch(self, path: str, body: dict) -> Optional[dict]:
        return self._request("PATCH", path, body)

    def delete(self, path: str) -> Optional[dict]:
        return self._request("DELETE", path)

    def put(self, path: str, body: dict) -> Optional[dict]:
        return self._request("PUT", path, body)

    def _request(self, method: str, path: str, body: Optional[dict] = None) -> Optional[dict]:
        if self.dry_run:
            self.request_count += 1
            return {"success": True, "data": {}}

        url = self.base_url + path
        data = json.dumps(body, default=str).encode("utf-8") if body else None
        headers = {"Content-Type": "application/json", "Accept": "application/json"}
        if self.token:
            headers["Authorization"] = f"Bearer {self.token}"

        req = request.Request(url, data=data, headers=headers, method=method)

        # Rate limiting
        time.sleep(self._rate_limit_delay)
        self.request_count += 1

        for attempt in range(3):
            try:
                with request.urlopen(req, timeout=30) as resp:
                    return json.loads(resp.read().decode("utf-8"))
            except error.HTTPError as e:
                body_text = e.read().decode("utf-8", errors="replace")
                if e.code == 429:
                    # Rate limited — back off
                    wait = 2 ** (attempt + 1)
                    print(f"  [rate-limited] waiting {wait}s...")
                    time.sleep(wait)
                    continue
                elif e.code in (409, 422):
                    # Conflict or validation error — skip gracefully
                    self.error_count += 1
                    return None
                else:
                    self.error_count += 1
                    print(f"  [HTTP {e.code}] {method} {path}: {body_text[:200]}")
                    return None
            except Exception as e:
                self.error_count += 1
                if attempt < 2:
                    time.sleep(1)
                    continue
                print(f"  [error] {method} {path}: {e}")
                return None
        return None


# ─── Simulation Engine ───────────────────────────────────────────────────────

class MonthSimulation:
    """Orchestrates a 30-day realistic booking simulation."""

    def __init__(self, client: Client, dry_run: bool = False, days: int = SIMULATION_DAYS):
        self.client = client
        self.dry_run = dry_run
        self.days = days

        # Fetched from API
        self.admin_token: str = ""
        self.admin_user_id: str = ""
        self.users: List[dict] = []
        self.user_tokens: Dict[str, str] = {}  # user_id -> token
        self.lots: List[dict] = []
        self.lot_slots: Dict[str, List[dict]] = {}  # lot_id -> [slots]

        # Simulation state
        self.recurring_users: set = set()       # user IDs with recurring bookings
        self.absent_users_today: set = set()    # user IDs absent on current day
        self.booking_ids: List[Tuple[str, str]] = []  # (booking_id, user_id)

        # Stats
        self.stats = {
            "total_bookings": 0,
            "bookings_cancelled": 0,
            "swap_requests": 0,
            "guest_bookings": 0,
            "recurring_bookings_created": 0,
            "announcements": 0,
            "role_changes": 0,
            "credit_refills": 0,
            "peak_occupancy_day": "",
            "peak_occupancy_count": 0,
            "daily_bookings": defaultdict(int),
            "total_revenue_eur": 0.0,
            "api_requests": 0,
            "api_errors": 0,
        }

    # ── Setup ────────────────────────────────────────────────────────────────

    def login_admin(self) -> bool:
        """Login as admin and store token."""
        print("[1/6] Logging in as admin...")
        resp = self.client.post("/api/v1/auth/login", {
            "username": ADMIN_USER,
            "password": self.client._admin_password,
        })
        if not resp or not resp.get("data"):
            print("  FATAL: Admin login failed")
            return False

        data = resp["data"]
        self.admin_token = data["tokens"]["access_token"]
        self.admin_user_id = data["user"]["id"]
        self.client.token = self.admin_token
        print(f"  Admin logged in: {data['user']['username']} ({self.admin_user_id})")
        return True

    def fetch_users(self) -> bool:
        """Fetch all users from admin endpoint."""
        print("[2/6] Fetching users...")
        resp = self.client.get("/api/v1/admin/users")
        if not resp or not resp.get("data"):
            print("  FATAL: Could not fetch users")
            return False

        self.users = [u for u in resp["data"] if u.get("role") != "superadmin"]
        regular_users = [u for u in self.users if u.get("role") == "user"]
        print(f"  Found {len(self.users)} users ({len(regular_users)} regular)")

        # Select ~15% as recurring bookers
        recurring_count = max(1, int(len(regular_users) * 0.15))
        self.recurring_users = set(
            u["id"] for u in random.sample(regular_users, min(recurring_count, len(regular_users)))
        )
        print(f"  {len(self.recurring_users)} users flagged as recurring bookers")
        return True

    def fetch_lots(self) -> bool:
        """Fetch all lots and their slots."""
        print("[3/6] Fetching lots and slots...")
        resp = self.client.get("/api/v1/lots")
        if not resp or not resp.get("data"):
            print("  FATAL: Could not fetch lots")
            return False

        self.lots = resp["data"]
        total_slots = 0
        for lot in self.lots:
            lot_id = lot["id"]
            slots_resp = self.client.get(f"/api/v1/lots/{lot_id}/slots")
            if slots_resp and slots_resp.get("data"):
                self.lot_slots[lot_id] = slots_resp["data"]
                total_slots += len(slots_resp["data"])
            else:
                self.lot_slots[lot_id] = []

        print(f"  Found {len(self.lots)} lots with {total_slots} total slots")
        return True

    def login_user(self, username: str) -> Optional[str]:
        """Login as a specific user and return their token. Uses a shared password."""
        # Demo users are seeded with password "Test1234!"
        resp = self.client.post("/api/v1/auth/login", {
            "username": username,
            "password": "Test1234!",
        })
        if resp and resp.get("data"):
            return resp["data"]["tokens"]["access_token"]
        return None

    def login_batch_users(self):
        """Login a subset of users to get tokens for booking as them."""
        print("[4/6] Logging in user batch...")
        regular = [u for u in self.users if u.get("role") == "user"]
        # Login a representative sample (max 50 to avoid flooding)
        sample_size = min(50, len(regular))
        sample = random.sample(regular, sample_size)

        logged_in = 0
        for user in sample:
            token = self.login_user(user["username"])
            if token:
                self.user_tokens[user["id"]] = token
                logged_in += 1

        print(f"  Logged in {logged_in}/{sample_size} users")

        # For users we couldn't login, we'll use admin token (quick-book)
        if logged_in == 0:
            print("  WARNING: No user logins succeeded, using admin quick-book only")

    # ── Day Simulation ───────────────────────────────────────────────────────

    def simulate_day(self, day_offset: int, sim_date: datetime):
        """Simulate a single day of activity."""
        day_name = sim_date.strftime("%A")
        day_str = sim_date.strftime("%Y-%m-%d")
        weekday = sim_date.weekday()  # 0=Mon, 6=Sun

        # Determine capacity target
        if weekday == 4:  # Friday
            capacity_pct = random.uniform(0.40, 0.50)
            day_type = "Friday"
        elif weekday >= 5:  # Weekend
            capacity_pct = random.uniform(0.05, 0.10)
            day_type = "Weekend"
        else:  # Mon-Thu
            capacity_pct = random.uniform(0.60, 0.80)
            day_type = "Workday"

        # Calculate how many users should book today
        regular_users = [u for u in self.users if u.get("role") == "user"]
        target_bookings = int(len(regular_users) * capacity_pct)

        # Remove 10% as absent
        self.absent_users_today = set(
            u["id"] for u in random.sample(regular_users, max(1, int(len(regular_users) * 0.10)))
        )
        available_users = [u for u in regular_users if u["id"] not in self.absent_users_today]
        target_bookings = min(target_bookings, len(available_users))

        # Select users to book today
        booking_users = random.sample(available_users, min(target_bookings, len(available_users)))

        day_bookings = 0
        day_revenue = 0.0

        for user in booking_users:
            lot = random.choice(self.lots)
            lot_id = lot["id"]
            slots = self.lot_slots.get(lot_id, [])
            available = [s for s in slots if s.get("status") == "available"]

            if not available:
                # All slots taken in this lot, try another
                for fallback_lot in random.sample(self.lots, min(3, len(self.lots))):
                    fallback_slots = self.lot_slots.get(fallback_lot["id"], [])
                    available = [s for s in fallback_slots if s.get("status") == "available"]
                    if available:
                        lot = fallback_lot
                        lot_id = lot["id"]
                        slots = fallback_slots
                        break

            if not available:
                continue

            slot = random.choice(available)

            # Generate realistic booking times
            if weekday >= 5:
                # Weekend: more varied hours
                start_hour = random.randint(8, 18)
                duration = random.choice([60, 120, 180, 240])
            else:
                # Workday: 8am-6pm core hours
                start_hour = random.choice([7, 8, 8, 8, 9, 9, 10])
                duration = random.choice([480, 540, 600, 420, 360])  # 6-10 hours

            start_time = sim_date.replace(
                hour=start_hour,
                minute=random.choice([0, 15, 30]),
                second=0, microsecond=0,
                tzinfo=timezone.utc,
            )
            # Ensure start_time is in the future for API validation
            now = datetime.now(timezone.utc)
            if start_time <= now:
                start_time = now + timedelta(minutes=2)

            # Use quick-book (admin) or create-booking (user)
            user_token = self.user_tokens.get(user["id"])

            if user_token:
                # Book as user
                old_token = self.client.token
                self.client.token = user_token
                resp = self.client.post("/api/v1/bookings", {
                    "lot_id": lot_id,
                    "slot_id": slot["id"],
                    "start_time": start_time.isoformat(),
                    "duration_minutes": duration,
                    "vehicle_id": str(uuid.UUID(int=0)),
                    "license_plate": "",
                })
                self.client.token = old_token
            else:
                # Use admin quick-book
                self.client.token = self.admin_token
                resp = self.client.post("/api/v1/bookings/quick", {
                    "lot_id": lot_id,
                    "booking_type": "full_day" if duration >= 420 else "half_day_am",
                })

            if resp and resp.get("data"):
                booking_data = resp["data"]
                booking_id = booking_data.get("id", str(uuid.uuid4()))
                self.booking_ids.append((booking_id, user["id"]))
                day_bookings += 1

                # Calculate revenue
                pricing = booking_data.get("pricing", {})
                revenue = pricing.get("total", 0.0)
                if isinstance(revenue, (int, float)):
                    day_revenue += revenue

                # Mark slot as reserved in our local cache
                slot["status"] = "reserved"

        self.stats["total_bookings"] += day_bookings
        self.stats["daily_bookings"][day_str] = day_bookings
        self.stats["total_revenue_eur"] += day_revenue

        if day_bookings > self.stats["peak_occupancy_count"]:
            self.stats["peak_occupancy_count"] = day_bookings
            self.stats["peak_occupancy_day"] = day_str

        # ── Cancellations (5-10% of today's bookings) ────────────────────────
        cancel_count = max(0, int(day_bookings * random.uniform(0.05, 0.10)))
        recent_bookings = self.booking_ids[-day_bookings:] if day_bookings > 0 else []
        cancellable = random.sample(recent_bookings, min(cancel_count, len(recent_bookings)))

        for booking_id, user_id in cancellable:
            token = self.user_tokens.get(user_id, self.admin_token)
            old_token = self.client.token
            self.client.token = token
            resp = self.client.delete(f"/api/v1/bookings/{booking_id}")
            self.client.token = old_token
            if resp is not None:
                self.stats["bookings_cancelled"] += 1

        # ── Swap requests (2-3% of today's bookings) ────────────────────────
        if day_bookings >= 4:
            swap_count = max(0, int(day_bookings * random.uniform(0.02, 0.03)))
            swap_count = max(swap_count, 1) if day_bookings > 10 else swap_count

            for _ in range(swap_count):
                if len(recent_bookings) < 2:
                    break
                pair = random.sample(recent_bookings, 2)
                requester_id = pair[0][0]
                target_id = pair[1][0]
                requester_user = pair[0][1]

                token = self.user_tokens.get(requester_user, self.admin_token)
                old_token = self.client.token
                self.client.token = token
                resp = self.client.post(f"/api/v1/bookings/{requester_id}/swap-request", {
                    "target_booking_id": target_id,
                    "message": random.choice([
                        "Können wir tauschen? Mein Platz ist näher am Aufzug.",
                        "Tausch gewünscht — brauche Ladeplatz.",
                        "Wäre ein Tausch möglich? Danke!",
                        None,
                    ]),
                })
                self.client.token = old_token
                if resp and resp.get("data"):
                    self.stats["swap_requests"] += 1

        # ── Guest bookings (2-3 per day) ─────────────────────────────────────
        guest_count = random.randint(2, 3) if weekday < 5 else random.randint(0, 1)
        self.client.token = self.admin_token
        for _ in range(guest_count):
            lot = random.choice(self.lots)
            lot_id = lot["id"]
            slots = self.lot_slots.get(lot_id, [])
            available = [s for s in slots if s.get("status") == "available"]
            if not available:
                continue
            slot = random.choice(available)

            g_start = sim_date.replace(
                hour=random.randint(9, 15), minute=0, second=0, microsecond=0,
                tzinfo=timezone.utc,
            )
            if g_start <= datetime.now(timezone.utc):
                g_start = datetime.now(timezone.utc) + timedelta(minutes=5)
            g_end = g_start + timedelta(hours=random.randint(1, 4))

            resp = self.client.post("/api/v1/bookings/guest", {
                "lot_id": lot_id,
                "slot_id": slot["id"],
                "start_time": g_start.isoformat(),
                "end_time": g_end.isoformat(),
                "guest_name": random.choice(GUEST_NAMES),
                "guest_email": f"gast{random.randint(100,999)}@example.de",
            })
            if resp and resp.get("data"):
                self.stats["guest_bookings"] += 1

        # ── Reset slot cache for next day (simplified: mark all available) ───
        for lot_id, slots in self.lot_slots.items():
            for slot in slots:
                slot["status"] = "available"

        return day_bookings

    # ── Recurring bookings ───────────────────────────────────────────────────

    def create_recurring_bookings(self, start_date: datetime):
        """Create recurring weekly bookings for ~15% of users."""
        print("[5/6] Creating recurring bookings...")
        self.client.token = self.admin_token
        created = 0

        for user_id in self.recurring_users:
            token = self.user_tokens.get(user_id)
            if not token:
                continue

            lot = random.choice(self.lots)
            lot_id = lot["id"]
            slots = self.lot_slots.get(lot_id, [])
            slot = random.choice(slots) if slots else None

            # Pick 2-3 weekdays
            days = sorted(random.sample([1, 2, 3, 4, 5], random.randint(2, 3)))
            end_date = start_date + timedelta(days=30)

            old_token = self.client.token
            self.client.token = token
            resp = self.client.post("/api/v1/recurring-bookings", {
                "lot_id": lot_id,
                "slot_id": slot["id"] if slot else None,
                "days_of_week": days,
                "start_date": start_date.strftime("%Y-%m-%d"),
                "end_date": end_date.strftime("%Y-%m-%d"),
                "start_time": random.choice(["07:30", "08:00", "08:30", "09:00"]),
                "end_time": random.choice(["16:30", "17:00", "17:30", "18:00"]),
                "vehicle_plate": None,
            })
            self.client.token = old_token

            if resp and resp.get("data"):
                created += 1

        self.stats["recurring_bookings_created"] = created
        print(f"  Created {created} recurring bookings")

    # ── Admin actions ────────────────────────────────────────────────────────

    def admin_weekly_announcement(self, week_num: int, sim_date: datetime):
        """Post one announcement per week."""
        self.client.token = self.admin_token
        template = ANNOUNCEMENT_TEMPLATES[week_num % len(ANNOUNCEMENT_TEMPLATES)]
        expires = sim_date + timedelta(days=7)

        resp = self.client.post("/api/v1/admin/announcements", {
            "title": template["title"],
            "message": template["message"],
            "severity": template["severity"],
            "active": True,
            "expires_at": expires.isoformat(),
        })
        if resp and resp.get("data"):
            self.stats["announcements"] += 1

    def admin_monthly_credit_refill(self):
        """Trigger monthly credit refill for all users."""
        self.client.token = self.admin_token
        resp = self.client.post("/api/v1/admin/credits/refill-all", {})
        if resp and resp.get("data"):
            self.stats["credit_refills"] += 1
            refilled = resp["data"].get("users_refilled", 0)
            print(f"  Credits refilled for {refilled} users")

    def admin_role_changes(self):
        """Promote 2-3 users to admin role."""
        self.client.token = self.admin_token
        regular_users = [u for u in self.users if u.get("role") == "user"]
        if len(regular_users) < 3:
            return

        promote_count = random.randint(2, 3)
        targets = random.sample(regular_users, promote_count)

        for user in targets:
            resp = self.client.patch(f"/api/v1/admin/users/{user['id']}/role", {
                "role": "admin",
            })
            if resp and resp.get("data"):
                self.stats["role_changes"] += 1
                print(f"  Promoted {user.get('username', user['id'])} to admin")

    # ── Main Loop ────────────────────────────────────────────────────────────

    def run(self):
        """Execute the full 30-day simulation."""
        start_time = time.monotonic()

        # Setup
        if not self.login_admin():
            return False
        if not self.fetch_users():
            return False
        if not self.fetch_lots():
            return False
        if not self.dry_run:
            self.login_batch_users()

        # Determine simulation start date (tomorrow)
        sim_start = datetime.now(timezone.utc).replace(
            hour=0, minute=0, second=0, microsecond=0
        ) + timedelta(days=1)

        # Create recurring bookings at the start
        self.create_recurring_bookings(sim_start)

        # Simulate each day
        print(f"\n[6/6] Simulating {self.days} days starting {sim_start.strftime('%Y-%m-%d')}...")
        print(f"{'Day':>4}  {'Date':>12}  {'Type':>9}  {'Bookings':>8}  {'Cancelled':>9}  {'Running Total':>13}")
        print("-" * 72)

        for day_offset in range(self.days):
            sim_date = sim_start + timedelta(days=day_offset)
            weekday = sim_date.weekday()

            # Weekly announcement (every Monday)
            if weekday == 0:
                week_num = day_offset // 7
                self.admin_weekly_announcement(week_num, sim_date)

            # Monthly credit refill (day 1 of simulation)
            if day_offset == 0:
                self.admin_monthly_credit_refill()

            # Role changes (mid-month)
            if day_offset == 15:
                self.admin_role_changes()

            # Simulate the day
            prev_cancelled = self.stats["bookings_cancelled"]
            day_bookings = self.simulate_day(day_offset, sim_date)
            day_cancelled = self.stats["bookings_cancelled"] - prev_cancelled

            day_type = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][weekday]
            print(
                f"{day_offset + 1:>4}  {sim_date.strftime('%Y-%m-%d'):>12}  "
                f"{day_type:>9}  {day_bookings:>8}  {day_cancelled:>9}  "
                f"{self.stats['total_bookings']:>13}"
            )

        elapsed = time.monotonic() - start_time

        # Final stats
        self.stats["api_requests"] = self.client.request_count
        self.stats["api_errors"] = self.client.error_count

        return self.print_report(elapsed)

    def print_report(self, elapsed: float) -> bool:
        """Print final statistics report."""
        s = self.stats
        total = s["total_bookings"]
        cancelled = s["bookings_cancelled"]
        cancel_rate = (cancelled / total * 100) if total > 0 else 0

        # Find top 5 busiest days
        sorted_days = sorted(s["daily_bookings"].items(), key=lambda x: x[1], reverse=True)
        top_days = sorted_days[:5]

        print("\n" + "=" * 72)
        print("  PARKHUB 1-MONTH SIMULATION REPORT")
        print("=" * 72)
        print()
        print(f"  Duration:              {elapsed:.1f}s")
        print(f"  API Requests:          {s['api_requests']:,}")
        print(f"  API Errors:            {s['api_errors']:,}")
        print()
        print("  ── Bookings ──────────────────────────────────────")
        print(f"  Total Created:         {total:,}")
        print(f"  Cancelled:             {cancelled:,}  ({cancel_rate:.1f}%)")
        print(f"  Net Active:            {total - cancelled:,}")
        print(f"  Guest Bookings:        {s['guest_bookings']:,}")
        print(f"  Recurring Created:     {s['recurring_bookings_created']:,}")
        print(f"  Swap Requests:         {s['swap_requests']:,}")
        print()
        print("  ── Occupancy ─────────────────────────────────────")
        print(f"  Peak Day:              {s['peak_occupancy_day']}  ({s['peak_occupancy_count']} bookings)")
        avg = total / self.days if self.days > 0 else 0
        print(f"  Avg Bookings/Day:      {avg:.1f}")
        print()
        print("  Top 5 Busiest Days:")
        for day, count in top_days:
            print(f"    {day}:  {count} bookings")
        print()
        print("  ── Revenue ───────────────────────────────────────")
        print(f"  Total Revenue:         EUR {s['total_revenue_eur']:,.2f}")
        avg_rev = s['total_revenue_eur'] / total if total > 0 else 0
        print(f"  Avg per Booking:       EUR {avg_rev:.2f}")
        print()
        print("  ── Admin Actions ─────────────────────────────────")
        print(f"  Announcements:         {s['announcements']}")
        print(f"  Credit Refills:        {s['credit_refills']}")
        print(f"  Role Changes:          {s['role_changes']}")
        print()
        print("=" * 72)

        # Return as JSON for programmatic use
        return True


# ─── Dry-Run Estimator ───────────────────────────────────────────────────────

def estimate_dry_run(users_count: int, lots_count: int, total_slots: int, days: int = 30):
    """When --dry-run, estimate what would happen without API calls."""
    print("\n" + "=" * 72)
    print("  PARKHUB SIMULATION ESTIMATE (DRY RUN)")
    print("=" * 72)

    # Scale workdays/fridays/weekends proportionally
    workdays = int(days * 22 / 30)
    fridays = int(days * 4 / 30)
    weekends = int(days * 8 / 30)

    # Bookings estimate
    wd_avg = users_count * 0.70  # 60-80% midpoint
    fri_avg = users_count * 0.45  # 40-50% midpoint
    we_avg = users_count * 0.075  # 5-10% midpoint

    total_bookings = int(
        (workdays - fridays) * wd_avg * 0.90 +  # -10% absent
        fridays * fri_avg * 0.90 +
        weekends * we_avg * 0.90
    )

    cancelled = int(total_bookings * 0.075)  # 7.5% midpoint
    swap_requests = int(total_bookings * 0.025 * 30)  # 2.5% per day
    guest_bookings = int(2.5 * workdays + 0.5 * weekends)

    # Revenue estimate (EUR 2.50 avg hourly * 8h avg)
    avg_booking_value = 2.50 * 8 * 1.19  # incl. VAT
    revenue = total_bookings * avg_booking_value

    # API call estimate
    setup_calls = 1 + 1 + 1 + lots_count + 50  # login + users + lots + slot lists + user logins
    booking_calls = total_bookings  # create_booking
    cancel_calls = cancelled
    swap_calls = swap_requests
    guest_calls = guest_bookings
    recurring_calls = int(users_count * 0.15)
    admin_calls = 4 + 1 + 3  # announcements + refill + role changes
    total_api = setup_calls + booking_calls + cancel_calls + swap_calls + guest_calls + recurring_calls + admin_calls

    print()
    print(f"  Users:                 {users_count}")
    print(f"  Lots:                  {lots_count}")
    print(f"  Total Slots:           {total_slots}")
    print()
    print(f"  Est. Total Bookings:   ~{total_bookings:,}")
    print(f"  Est. Cancellations:    ~{cancelled:,}  (~7.5%)")
    print(f"  Est. Swap Requests:    ~{swap_requests:,}")
    print(f"  Est. Guest Bookings:   ~{guest_bookings:,}")
    print(f"  Est. Recurring:        ~{recurring_calls:,}")
    print()
    print(f"  Est. Revenue:          ~EUR {revenue:,.2f}")
    print(f"  Est. API Calls:        ~{total_api:,}")
    print(f"  Est. Duration:         ~{total_api * RATE_LIMIT_DELAY / 60:.0f} min (at {RATE_LIMIT_DELAY*1000:.0f}ms delay)")
    print()
    print("=" * 72)


# ─── Main ────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="ParkHub 1-month booking simulation",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--base-url", default=BASE_URL,
        help=f"ParkHub API base URL (default: {BASE_URL})",
    )
    parser.add_argument(
        "--admin-password", default=ADMIN_PASSWORD,
        help="Admin password (default: ParkHub2026!)",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Print estimated stats without making any API calls",
    )
    parser.add_argument(
        "--days", type=int, default=SIMULATION_DAYS,
        help=f"Number of days to simulate (default: {SIMULATION_DAYS})",
    )
    args = parser.parse_args()

    sim_days = args.days

    print("=" * 72)
    print("  ParkHub 1-Month Booking Simulation")
    print(f"  Target: {args.base_url}")
    print(f"  Days:   {sim_days}")
    print(f"  Mode:   {'DRY RUN' if args.dry_run else 'LIVE'}")
    print("=" * 72)
    print()

    client = Client(args.base_url, dry_run=args.dry_run)
    client._admin_password = args.admin_password

    if args.dry_run:
        # Quick fetch to get counts, then estimate
        print("Fetching current state for estimation...")

        # Try to login and get counts
        resp = client.post("/api/v1/auth/login", {
            "username": ADMIN_USER,
            "password": args.admin_password,
        })
        if resp and resp.get("data"):
            client.token = resp["data"]["tokens"]["access_token"]
            users_resp = client.get("/api/v1/admin/users")
            lots_resp = client.get("/api/v1/lots")

            users_count = len(users_resp.get("data", [])) if users_resp else 200
            lots_data = lots_resp.get("data", []) if lots_resp else []
            lots_count = len(lots_data) if lots_data else 10

            total_slots = 0
            for lot in lots_data:
                for floor in lot.get("floors", []):
                    total_slots += floor.get("total_slots", 0)
            if total_slots == 0:
                total_slots = 600  # estimate

            estimate_dry_run(users_count, lots_count, total_slots, sim_days)
        else:
            # Can't connect — use defaults from seed_demo
            print("  Could not connect to API, using seed_demo defaults")
            estimate_dry_run(200, 10, 600, sim_days)

        return 0

    sim = MonthSimulation(client, dry_run=False, days=sim_days)
    success = sim.run()
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
