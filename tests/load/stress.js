// ParkHub — Stress Test
// Push limits: ramp to 100 VUs over 10 minutes, hit all major endpoints
// Run: k6 run tests/load/stress.js

import http from "k6/http";
import { check, sleep } from "k6";
import { Counter } from "k6/metrics";
import { BASE_URL, CREDENTIALS, HEADERS, login } from "./config.js";

const errorCount = new Counter("errors");

export const options = {
  stages: [
    { duration: "2m", target: 25 },
    { duration: "3m", target: 50 },
    { duration: "3m", target: 100 },
    { duration: "1m", target: 100 },
    { duration: "1m", target: 0 },
  ],
  thresholds: {
    http_req_duration: ["p(95)<1000", "p(99)<3000"],
    http_req_failed: ["rate<0.05"],
    errors: ["count<100"],
  },
};

export function setup() {
  const userHeaders = login(http, CREDENTIALS.user);
  const adminHeaders = login(http, CREDENTIALS.admin);
  return { userHeaders, adminHeaders };
}

function hitEndpoint(url, headers, name) {
  const res = http.get(url, { headers, tags: { name } });
  const ok = check(res, { [`${name} ok`]: (r) => r.status === 200 });
  if (!ok) errorCount.add(1);
  return res;
}

export default function (data) {
  const endpoints = [
    // Public
    { url: `${BASE_URL}/health`, headers: HEADERS, name: "health" },

    // User endpoints
    { url: `${BASE_URL}/api/v1/bookings`, headers: data.userHeaders, name: "bookings" },
    { url: `${BASE_URL}/api/v1/lots`, headers: data.userHeaders, name: "lots" },
    { url: `${BASE_URL}/api/v1/me`, headers: data.userHeaders, name: "profile" },
    { url: `${BASE_URL}/api/v1/vehicles`, headers: data.userHeaders, name: "vehicles" },
    { url: `${BASE_URL}/api/v1/notifications`, headers: data.userHeaders, name: "notifications" },
    { url: `${BASE_URL}/api/v1/credits`, headers: data.userHeaders, name: "credits" },
    { url: `${BASE_URL}/api/v1/favorites`, headers: data.userHeaders, name: "favorites" },
    { url: `${BASE_URL}/api/v1/absences`, headers: data.userHeaders, name: "absences" },
    { url: `${BASE_URL}/api/v1/bookings/history`, headers: data.userHeaders, name: "history" },

    // Admin endpoints
    { url: `${BASE_URL}/api/v1/admin/users`, headers: data.adminHeaders, name: "admin-users" },
    { url: `${BASE_URL}/api/v1/admin/lots`, headers: data.adminHeaders, name: "admin-lots" },
    { url: `${BASE_URL}/api/v1/admin/bookings`, headers: data.adminHeaders, name: "admin-bookings" },
    { url: `${BASE_URL}/api/v1/admin/analytics`, headers: data.adminHeaders, name: "admin-analytics" },
  ];

  // Hit 3-5 random endpoints per iteration
  const count = 3 + Math.floor(Math.random() * 3);
  for (let i = 0; i < count; i++) {
    const ep = endpoints[Math.floor(Math.random() * endpoints.length)];
    hitEndpoint(ep.url, ep.headers, ep.name);
    sleep(0.1);
  }

  sleep(0.3);
}
