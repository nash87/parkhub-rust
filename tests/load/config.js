// ParkHub k6 shared configuration
// Override via environment: K6_BASE_URL, K6_USERNAME, K6_PASSWORD

export const BASE_URL = __ENV.K6_BASE_URL || "http://localhost:8080";

export const CREDENTIALS = {
  admin: {
    email: __ENV.K6_ADMIN_EMAIL || "admin@parkhub.test",
    password: __ENV.K6_ADMIN_PASSWORD || "admin123",
  },
  user: {
    email: __ENV.K6_USER_EMAIL || "user@parkhub.test",
    password: __ENV.K6_USER_PASSWORD || "user123",
  },
};

export const THRESHOLDS = {
  http_req_duration: ["p(95)<500", "p(99)<1500"],
  http_req_failed: ["rate<0.01"],
  http_reqs: ["rate>10"],
};

export const HEADERS = {
  "Content-Type": "application/json",
  Accept: "application/json",
};

/**
 * Login and return auth headers with token.
 */
export function login(http, credentials) {
  const res = http.post(
    `${BASE_URL}/api/v1/auth/login`,
    JSON.stringify({
      email: credentials.email,
      password: credentials.password,
    }),
    { headers: HEADERS }
  );

  if (res.status !== 200) {
    console.error(`Login failed: ${res.status} ${res.body}`);
    return HEADERS;
  }

  const body = res.json();
  const token = body.token || body.access_token || "";

  return {
    ...HEADERS,
    Authorization: `Bearer ${token}`,
  };
}
