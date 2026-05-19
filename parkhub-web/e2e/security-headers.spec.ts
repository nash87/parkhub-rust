/**
 * E2E spec: Security isolation headers (CSP / COOP / CORP / SameSite).
 *
 * Parity target: parkhub-php `e2e/security-flows.spec.ts` "Security Headers"
 * section (added ~2026-05-16).  The PHP app delivers headers via Laravel
 * middleware; parkhub-rust delivers them via `security_headers_middleware` in
 * `parkhub-server/src/api/system.rs` (applied globally to every response).
 *
 * Run mode: CI-only.  Do NOT run locally (Chromium under memory pressure).
 * The Playwright config resolves baseURL from E2E_BASE_URL → BASE_URL →
 * https://parkhub-rust-demo.onrender.com (see parkhub-web/playwright.config.ts).
 */

import { test, expect, type Page } from '@playwright/test';

// ── helpers ────────────────────────────────────────────────────────────────

/** Log in as the seeded demo admin via the UI `#demo-autofill` shortcut. */
async function loginAsAdmin(page: Page) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

// ── suite ──────────────────────────────────────────────────────────────────

test.describe('Security Headers (CSP / COOP / CORP / SameSite)', () => {

  // ── 1. HTML document routes ────────────────────────────────────────────

  test.describe('GET / — document isolation headers', () => {
    test('Content-Security-Policy: default-src self', async ({ request }) => {
      const res = await request.get('/');
      const csp = res.headers()['content-security-policy'] ?? '';
      expect(csp).toBeTruthy();
      expect(csp).toContain("default-src 'self'");
    });

    test('Content-Security-Policy: frame-ancestors none', async ({ request }) => {
      const res = await request.get('/');
      const csp = res.headers()['content-security-policy'] ?? '';
      expect(csp).toContain("frame-ancestors 'none'");
    });

    test('Content-Security-Policy: object-src none', async ({ request }) => {
      const res = await request.get('/');
      const csp = res.headers()['content-security-policy'] ?? '';
      expect(csp).toContain("object-src 'none'");
    });

    test('Cross-Origin-Opener-Policy: same-origin', async ({ request }) => {
      const res = await request.get('/');
      expect(res.headers()['cross-origin-opener-policy']).toBe('same-origin');
    });

    test('Cross-Origin-Resource-Policy: same-origin', async ({ request }) => {
      const res = await request.get('/');
      expect(res.headers()['cross-origin-resource-policy']).toBe('same-origin');
    });

    test('X-Content-Type-Options: nosniff', async ({ request }) => {
      const res = await request.get('/');
      expect(res.headers()['x-content-type-options']).toBe('nosniff');
    });

    test('X-Frame-Options: DENY', async ({ request }) => {
      const res = await request.get('/');
      expect(res.headers()['x-frame-options']).toBe('DENY');
    });

    test('Referrer-Policy: strict-origin-when-cross-origin', async ({ request }) => {
      const res = await request.get('/');
      expect(res.headers()['referrer-policy']).toBe('strict-origin-when-cross-origin');
    });
  });

  // ── 2. Auth-gated route + SameSite cookie ─────────────────────────────

  test.describe('GET /admin — auth-gated route headers + SameSite cookie', () => {
    test('isolation headers present on /admin', async ({ request }) => {
      // Hit /admin unauthenticated — headers must still be set regardless of
      // redirect/401 response.
      const res = await request.get('/admin', { maxRedirects: 0 });
      const h = res.headers();
      const csp = h['content-security-policy'] ?? '';
      expect(csp).toBeTruthy();
      expect(csp).toContain("default-src 'self'");
      expect(h['cross-origin-opener-policy']).toBe('same-origin');
      expect(h['cross-origin-resource-policy']).toBe('same-origin');
    });

    test('auth cookie has SameSite=Lax after login', async ({ page, context }) => {
      await loginAsAdmin(page);
      const cookies = await context.cookies();
      const authCookie = cookies.find(
        (c) => c.name === 'parkhub_token' || c.name === 'ph_session' || c.name.startsWith('parkhub'),
      );
      // If no session cookie exists the backend is purely JWT-in-header —
      // that is also acceptable (no SameSite risk).  Only fail if the cookie
      // exists but is misconfigured.
      if (authCookie) {
        // Playwright reports sameSite as 'Lax', 'Strict', or 'None'.
        expect(['Lax', 'Strict']).toContain(authCookie.sameSite);
      }
    });

    test('isolation headers present on /admin after login', async ({ page, request }) => {
      await loginAsAdmin(page);
      const res = await request.get('/admin');
      const csp = res.headers()['content-security-policy'] ?? '';
      expect(csp).toBeTruthy();
      expect(csp).toContain("default-src 'self'");
      expect(res.headers()['cross-origin-opener-policy']).toBe('same-origin');
      expect(res.headers()['cross-origin-resource-policy']).toBe('same-origin');
    });
  });

  // ── 3. API responses ───────────────────────────────────────────────────

  test.describe('API — isolation headers on JSON endpoints', () => {
    test('GET /api/v1/health includes COOP and CORP', async ({ request }) => {
      const res = await request.get('/api/v1/health');
      expect(res.headers()['cross-origin-opener-policy']).toBe('same-origin');
      expect(res.headers()['cross-origin-resource-policy']).toBe('same-origin');
    });

    test('GET /api/v1/health CSP default-src self', async ({ request }) => {
      const res = await request.get('/api/v1/health');
      const csp = res.headers()['content-security-policy'] ?? '';
      expect(csp).toContain("default-src 'self'");
    });
  });

  // ── 4. Static assets — isolation must not break loading ───────────────

  test.describe('Static assets — isolation headers present', () => {
    test('favicon asset served with CORP: same-origin', async ({ request }) => {
      // Try canonical Astro output paths.  A 404 means the asset was renamed
      // or deleted — skip rather than fail the security-header assertion.
      const candidates = ['/favicon.svg', '/favicon.ico', '/favicon.png'];
      let checked = false;
      for (const path of candidates) {
        const res = await request.get(path);
        if (res.status() === 200) {
          // Asset must carry CORP so it isn't embeddable cross-origin.
          expect(res.headers()['cross-origin-resource-policy']).toBe('same-origin');
          // X-Content-Type-Options guards MIME confusion attacks on assets.
          expect(res.headers()['x-content-type-options']).toBe('nosniff');
          checked = true;
          break;
        }
      }
      if (!checked) {
        test.skip(true, 'No favicon asset found at canonical paths — skipping CORP asset check');
      }
    });

    test('_astro JS bundle served with CORP: same-origin', async ({ request }) => {
      // Discover any _astro/*.js via the document source rather than
      // hardcoding a content-hashed filename.
      const docRes = await request.get('/');
      const body = await docRes.text();
      const match = body.match(/\/_astro\/[A-Za-z0-9._-]+\.js/);
      if (!match) {
        test.skip(true, 'No _astro/*.js asset found in document — skipping bundle CORP check');
        return;
      }
      const res = await request.get(match[0]);
      if (res.status() !== 200) {
        test.skip(true, `Asset ${match[0]} returned ${res.status()} — skipping`);
        return;
      }
      expect(res.headers()['cross-origin-resource-policy']).toBe('same-origin');
      expect(res.headers()['x-content-type-options']).toBe('nosniff');
    });
  });
});
