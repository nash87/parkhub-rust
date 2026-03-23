// ParkHub — Load Test
// Sustained load: ramp to 50 VUs over 5 minutes
// Run: k6 run tests/load/load.js

import http from "k6/http";
import { check, sleep } from "k6";
import { BASE_URL, CREDENTIALS, HEADERS, THRESHOLDS, login } from "./config.js";

export const options = {
  stages: [
    { duration: "1m", target: 10 },
    { duration: "2m", target: 50 },
    { duration: "1m", target: 50 },
    { duration: "1m", target: 0 },
  ],
  thresholds: THRESHOLDS,
};

export function setup() {
  const authHeaders = login(http, CREDENTIALS.user);
  return { authHeaders };
}

export default function (data) {
  // 1. Login
  const loginRes = http.post(
    `${BASE_URL}/api/v1/auth/login`,
    JSON.stringify({
      email: CREDENTIALS.user.email,
      password: CREDENTIALS.user.password,
    }),
    { headers: HEADERS }
  );
  check(loginRes, { "login ok": (r) => r.status === 200 });

  // 2. List lots
  const lots = http.get(`${BASE_URL}/api/v1/lots`, {
    headers: data.authHeaders,
  });
  check(lots, { "lots ok": (r) => r.status === 200 });

  let lotId = null;
  if (lots.status === 200) {
    const body = lots.json();
    const items = Array.isArray(body) ? body : body.data || body.lots || [];
    if (items.length > 0) {
      lotId = items[0].id;
    }
  }

  // 3. Create booking
  if (lotId) {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const dateStr = tomorrow.toISOString().split("T")[0];

    const bookRes = http.post(
      `${BASE_URL}/api/v1/bookings`,
      JSON.stringify({ lot_id: lotId, date: dateStr }),
      { headers: data.authHeaders }
    );
    check(bookRes, {
      "booking created": (r) => r.status === 200 || r.status === 201,
    });

    // 4. Cancel booking
    if (bookRes.status === 200 || bookRes.status === 201) {
      const booking = bookRes.json();
      const bookingId = booking.id || booking.booking_id;
      if (bookingId) {
        const cancelRes = http.del(
          `${BASE_URL}/api/v1/bookings/${bookingId}`,
          null,
          { headers: data.authHeaders }
        );
        check(cancelRes, {
          "booking cancelled": (r) => r.status === 200 || r.status === 204,
        });
      }
    }
  }

  sleep(0.5);
}
