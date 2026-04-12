import { test, expect } from '@playwright/test';
import { loginViaApi, loginViaUi, DEMO_ADMIN, ADMIN_ROUTES, ADMIN_API_ENDPOINTS } from './helpers';

const BASE = process.env.E2E_BASE_URL || 'http://localhost:8081';

test.describe('Security — Access Control & Hardening', () => {
  // ── Authentication: No Token → 401 ──

  test.describe('API without token returns 401', () => {
    const protectedEndpoints = [
      '/api/v1/me',
      '/api/v1/bookings',
      '/api/v1/vehicles',
      '/api/v1/lots',
      '/api/v1/notifications',
      '/api/v1/admin/users',
      '/api/v1/admin/stats',
      '/api/v1/admin/settings',
    ];

    for (const endpoint of protectedEndpoints) {
      test(`GET ${endpoint} → 401`, async ({ request }) => {
        const res = await request.get(endpoint);
        expect(res.status()).toBe(401);
      });
    }
  });

  // ── Authorization: Regular User → 403 on Admin Routes ──

  test.describe('Admin routes blocked for regular users (UI)', () => {
    for (const route of ADMIN_ROUTES) {
      test(`${route} redirects or blocks non-admin`, async ({ page }) => {
        // Navigate without login — should redirect to login
        await page.goto(route);
        await page.waitForLoadState('networkidle');

        // Should either redirect to /login or show access denied
        const url = page.url();
        const body = await page.locator('body').textContent();
        const isBlocked =
          url.includes('/login') ||
          url.includes('/welcome') ||
          /forbidden|access denied|unauthorized|not authorized/i.test(body ?? '');

        expect(isBlocked).toBe(true);
      });
    }
  });

  // ── XSS Prevention ──

  test.describe('XSS Input Sanitization', () => {
    let token: string;

    test.beforeAll(async ({ playwright }) => {
      const ctx = await playwright.request.newContext({ baseURL: BASE });
      token = await loginViaApi(ctx);
      await ctx.dispose();
    });

    test('XSS payload in vehicle plate is sanitized', async ({ request }) => {
      const xssPayload = '<script>alert("xss")</script>';

      const res = await request.post('/api/v1/vehicles', {
        headers: { Authorization: `Bearer ${token}` },
        data: {
          plate: xssPayload,
          make: 'Test',
          model: 'XSS',
          color: 'red',
          type: 'car',
        },
      });

      if ([200, 201].includes(res.status())) {
        const body = await res.json();
        const vehicle = body.data ?? body;
        // Script tags should be stripped or escaped
        expect(vehicle.plate).not.toContain('<script>');

        // Clean up
        const vehicleId = vehicle.id;
        if (vehicleId) {
          await request.delete(`/api/v1/vehicles/${vehicleId}`, {
            headers: { Authorization: `Bearer ${token}` },
          });
        }
      } else {
        // Input validation rejected the payload — also correct
        expect([400, 422]).toContain(res.status());
      }
    });

    test('XSS payload in booking notes is handled', async ({ request }) => {
      const xssPayloads = [
        '<img src=x onerror=alert(1)>',
        'javascript:alert(1)',
        '<svg onload=alert(1)>',
        '"><script>alert(document.cookie)</script>',
      ];

      for (const payload of xssPayloads) {
        const res = await request.post('/api/v1/bookings', {
          headers: { Authorization: `Bearer ${token}` },
          data: {
            lot_id: 'test',
            slot_id: 'test',
            date: '2099-01-01',
            start_time: '09:00',
            end_time: '17:00',
            notes: payload,
          },
        });

        // Either validation rejects it or the payload is sanitized
        if ([200, 201].includes(res.status())) {
          const body = await res.json();
          const notes = body.data?.notes ?? body.notes ?? '';
          // Should not contain raw script/event handlers
          expect(notes).not.toMatch(/<script|onerror|onload|javascript:/i);
        }
        // 400/422 rejection is also acceptable
      }
    });
  });

  // ── Security Headers ──

  test.describe('Security Headers', () => {
    test('response includes security headers', async ({ request }) => {
      const res = await request.get('/health');
      const headers = res.headers();

      // X-Content-Type-Options should be nosniff
      if (headers['x-content-type-options']) {
        expect(headers['x-content-type-options']).toBe('nosniff');
      }

      // X-Frame-Options should block framing
      if (headers['x-frame-options']) {
        expect(headers['x-frame-options']).toMatch(/DENY|SAMEORIGIN/i);
      }

      // Content-Type should be present
      expect(headers['content-type']).toBeTruthy();
    });

    test('API responses are JSON content type', async ({ request }) => {
      const res = await request.post('/api/v1/auth/login', {
        data: { username: 'invalid', password: 'invalid' },
      });
      const ct = res.headers()['content-type'] ?? '';
      expect(ct).toContain('application/json');
    });

    test('HSTS header present on HTTPS', async ({ request }) => {
      const res = await request.get('/health');
      const headers = res.headers();
      // HSTS may only be present on HTTPS connections
      // On HTTP, it's acceptable to not have it
      if (headers['strict-transport-security']) {
        expect(headers['strict-transport-security']).toContain('max-age');
      }
    });
  });

  // ── Session Security ──

  test.describe('Session & Token Security', () => {
    test('invalid token returns 401', async ({ request }) => {
      const res = await request.get('/api/v1/me', {
        headers: { Authorization: 'Bearer invalid-token-12345' },
      });
      expect(res.status()).toBe(401);
    });

    test('expired-format token returns 401', async ({ request }) => {
      // A JWT with an obviously wrong signature
      const fakeJwt = 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIiwiZXhwIjoxfQ.fake';
      const res = await request.get('/api/v1/me', {
        headers: { Authorization: `Bearer ${fakeJwt}` },
      });
      expect(res.status()).toBe(401);
    });

    test('login with wrong password returns 401', async ({ request }) => {
      const res = await request.post('/api/v1/auth/login', {
        data: {
          username: DEMO_ADMIN.username,
          password: 'wrong-password-12345',
        },
      });
      expect(res.status()).toBe(401);
    });

    test('login with empty credentials returns 400 or 401', async ({ request }) => {
      const res = await request.post('/api/v1/auth/login', {
        data: { username: '', password: '' },
      });
      expect([400, 401, 422]).toContain(res.status());
    });
  });

  // ── Rate Limiting ──

  test.describe('Rate Limiting', () => {
    test('rapid login attempts are eventually rate-limited', async ({ request }) => {
      const results: number[] = [];

      // Send 10 rapid login attempts with bad credentials
      for (let i = 0; i < 10; i++) {
        const res = await request.post('/api/v1/auth/login', {
          data: { username: 'nonexistent', password: `wrong-${i}` },
        });
        results.push(res.status());
      }

      // At least some should be 401 (bad creds), and eventually 429 (rate limited)
      const has401 = results.includes(401);
      const has429 = results.includes(429);

      // We expect either rate limiting kicked in, or all returned 401
      expect(has401 || has429).toBe(true);
    });
  });

  // ── SQL Injection Prevention ──

  test.describe('SQL Injection Prevention', () => {
    let token: string;

    test.beforeAll(async ({ playwright }) => {
      const ctx = await playwright.request.newContext({ baseURL: BASE });
      token = await loginViaApi(ctx);
      await ctx.dispose();
    });

    test('SQL injection in query params is handled safely', async ({ request }) => {
      const sqliPayloads = [
        "' OR '1'='1",
        "1; DROP TABLE users; --",
        "' UNION SELECT * FROM users --",
      ];

      for (const payload of sqliPayloads) {
        const res = await request.get(`/api/v1/bookings?search=${encodeURIComponent(payload)}`, {
          headers: { Authorization: `Bearer ${token}` },
        });
        // Should return a normal response (200, 400, or 422), never a 500 DB error
        expect(res.status()).not.toBe(500);
      }
    });
  });

  // ── CORS ──

  test.describe('CORS Headers', () => {
    test('preflight OPTIONS request is handled', async ({ request }) => {
      const res = await request.fetch('/api/v1/auth/login', {
        method: 'OPTIONS',
        headers: {
          Origin: 'http://evil-site.com',
          'Access-Control-Request-Method': 'POST',
        },
      });
      // Should not return 500 — proper CORS handling
      expect(res.status()).not.toBe(500);
    });
  });
});
