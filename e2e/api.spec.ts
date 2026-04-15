import { test, expect } from '@playwright/test';
import {
  DEMO_ADMIN,
  loginViaApi,
  PUBLIC_API_ENDPOINTS,
  PROTECTED_API_ENDPOINTS,
  ADMIN_API_ENDPOINTS,
} from './helpers';

test.describe('API — Public Endpoints', () => {
  for (const endpoint of PUBLIC_API_ENDPOINTS) {
    test(`GET ${endpoint} → 200`, async ({ request }) => {
      const start = Date.now();
      const res = await request.get(endpoint);
      const elapsed = Date.now() - start;

      expect(res.status()).toBe(200);
      expect(elapsed).toBeLessThan(2000);
    });
  }
});

test.describe('API — Auth Flow', () => {
  test('POST /api/v1/auth/login with valid creds → 200 + token', async ({ request }) => {
    const res = await request.post('/api/v1/auth/login', { data: DEMO_ADMIN });
    expect(res.status()).toBe(200);

    const body = await res.json();
    const token = body.data?.tokens?.access_token ?? body.data?.token ?? body.token;
    expect(token).toBeTruthy();
  });

  test('POST /api/v1/auth/login with bad creds → 401', async ({ request }) => {
    const res = await request.post('/api/v1/auth/login', {
      data: { username: 'wrong@test.com', password: 'wrong' },
    });
    expect(res.status()).toBe(401);
  });

  test('POST /api/v1/auth/login returns application/json', async ({ request }) => {
    const res = await request.post('/api/v1/auth/login', { data: DEMO_ADMIN });
    const ct = res.headers()['content-type'] ?? '';
    expect(ct).toContain('application/json');
  });
});

test.describe('API — Protected Endpoints (no auth → 401)', () => {
  for (const endpoint of PROTECTED_API_ENDPOINTS) {
    test(`GET ${endpoint} without auth → 401`, async ({ request }) => {
      const res = await request.get(endpoint);
      expect(res.status()).toBe(401);
    });
  }
});

test.describe('API — Protected Endpoints (authenticated)', () => {
  let token: string;

  test.beforeAll(async ({ playwright }) => {
    const ctx = await playwright.request.newContext({
      baseURL: process.env.E2E_BASE_URL || 'http://localhost:8081',
    });
    token = await loginViaApi(ctx);
    await ctx.dispose();
  });

  for (const endpoint of PROTECTED_API_ENDPOINTS) {
    test(`GET ${endpoint} (auth) → 200 + JSON`, async ({ request }) => {
      const start = Date.now();
      const res = await request.get(endpoint, {
        headers: { Authorization: `Bearer ${token}` },
      });
      const elapsed = Date.now() - start;

      expect(res.status()).toBe(200);
      expect(elapsed).toBeLessThan(2000);

      const ct = res.headers()['content-type'] ?? '';
      expect(ct).toContain('application/json');
    });
  }
});

test.describe('API — Admin Endpoints (authenticated)', () => {
  let token: string;

  test.beforeAll(async ({ playwright }) => {
    const ctx = await playwright.request.newContext({
      baseURL: process.env.E2E_BASE_URL || 'http://localhost:8081',
    });
    token = await loginViaApi(ctx);
    await ctx.dispose();
  });

  for (const endpoint of ADMIN_API_ENDPOINTS) {
    test(`GET ${endpoint} (admin) → 200 + JSON`, async ({ request }) => {
      const start = Date.now();
      const res = await request.get(endpoint, {
        headers: { Authorization: `Bearer ${token}` },
      });
      const elapsed = Date.now() - start;

      expect(res.status()).toBe(200);
      expect(elapsed).toBeLessThan(2000);

      const ct = res.headers()['content-type'] ?? '';
      expect(ct).toContain('application/json');
    });
  }
});

test.describe('API — Content Types', () => {
  test('health endpoint returns JSON', async ({ request }) => {
    // Rust exposes /health, PHP exposes /api/v1/health — try both.
    let res = await request.get('/api/v1/health/live');
    if (res.status() === 404) res = await request.get('/health');
    const ct = res.headers()['content-type'] ?? '';
    expect(ct).toContain('application/json');
  });

  test('modules endpoint returns JSON array', async ({ request }) => {
    const res = await request.get('/api/v1/modules');
    const body = await res.json();
    // Both backends wrap the module list differently: Rust returns
    // {modules: {...}}, PHP returns {data: {modules: {...}}}, older
    // Rust builds returned a bare array. Flatten all shapes to the
    // catalog itself (object of name→bool or array of names).
    let data = Array.isArray(body) ? body : body.data ?? body;
    if (data && typeof data === 'object' && !Array.isArray(data) && 'modules' in data) {
      data = (data as Record<string, unknown>).modules;
    }
    const count = Array.isArray(data)
      ? data.length
      : data && typeof data === 'object'
        ? Object.keys(data as Record<string, unknown>).length
        : 0;
    expect(count).toBeGreaterThan(0);
  });
});
