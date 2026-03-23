// ParkHub — Smoke Test
// Quick sanity check: 1 VU, 30s
// Run: k6 run tests/load/smoke.js

import http from "k6/http";
import { check, sleep } from "k6";
import { BASE_URL, CREDENTIALS, HEADERS, login } from "./config.js";

export const options = {
  vus: 1,
  duration: "30s",
  thresholds: {
    http_req_duration: ["p(95)<300"],
    http_req_failed: ["rate<0.01"],
  },
};

export function setup() {
  const authHeaders = login(http, CREDENTIALS.user);
  return { authHeaders };
}

export default function (data) {
  // Health check
  const health = http.get(`${BASE_URL}/health`);
  check(health, {
    "health returns 200": (r) => r.status === 200,
  });

  // Login flow
  const loginRes = http.post(
    `${BASE_URL}/api/v1/auth/login`,
    JSON.stringify({
      email: CREDENTIALS.user.email,
      password: CREDENTIALS.user.password,
    }),
    { headers: HEADERS }
  );
  check(loginRes, {
    "login returns 200": (r) => r.status === 200,
    "login has token": (r) => {
      const body = r.json();
      return body.token || body.access_token;
    },
  });

  // List bookings
  const bookings = http.get(`${BASE_URL}/api/v1/bookings`, {
    headers: data.authHeaders,
  });
  check(bookings, {
    "bookings returns 200": (r) => r.status === 200,
  });

  sleep(1);
}
