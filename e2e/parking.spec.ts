import { test, expect } from '@playwright/test';
import { loginViaUi, loginViaApi } from './helpers';

test.describe('Parking — Booking Flow', () => {
  test('login with demo credentials', async ({ page }) => {
    await loginViaUi(page);
    // Should land on dashboard
    expect(page.url()).not.toContain('/login');
  });

  test('navigate to booking flow', async ({ page }) => {
    await loginViaUi(page);
    await page.goto('/book');
    await page.waitForLoadState('networkidle');
    // Booking page should show lots or booking form
    await expect(page.locator('body')).toContainText(/book|lot|park|slot|reserve/i);
  });

  test('view bookings list', async ({ page }) => {
    await loginViaUi(page);
    await page.goto('/bookings');
    await page.waitForLoadState('networkidle');
    // Should show booking list (possibly empty)
    await expect(page.locator('body')).toContainText(/booking|reservation|no.*booking/i);
  });

  test('view dashboard KPIs', async ({ page }) => {
    await loginViaUi(page);
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    // Dashboard should show statistics or KPI cards
    await expect(page.locator('main, [data-testid]')).not.toHaveCount(0);
  });
});

test.describe('Parking — API Booking Lifecycle', () => {
  let token: string;

  test.beforeAll(async ({ playwright }) => {
    const ctx = await playwright.request.newContext({
      baseURL: process.env.E2E_BASE_URL || 'http://localhost:8081',
    });
    token = await loginViaApi(ctx);
    await ctx.dispose();
  });

  test('list lots', async ({ request }) => {
    const res = await request.get('/api/v1/lots', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    const lots = body.data ?? body;
    expect(Array.isArray(lots)).toBe(true);
  });

  test('list bookings', async ({ request }) => {
    const res = await request.get('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    const bookings = body.data ?? body;
    expect(Array.isArray(bookings)).toBe(true);
  });

  test('get user stats', async ({ request }) => {
    const res = await request.get('/api/v1/user/stats', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
  });
});

test.describe('Parking — Admin', () => {
  let token: string;

  test.beforeAll(async ({ playwright }) => {
    const ctx = await playwright.request.newContext({
      baseURL: process.env.E2E_BASE_URL || 'http://localhost:8081',
    });
    token = await loginViaApi(ctx);
    await ctx.dispose();
  });

  test('admin stats accessible', async ({ request }) => {
    const res = await request.get('/api/v1/admin/stats', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
  });

  test('admin users list accessible', async ({ request }) => {
    const res = await request.get('/api/v1/admin/users', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    const users = body.data ?? body;
    expect(Array.isArray(users)).toBe(true);
  });

  test('admin audit log accessible', async ({ request }) => {
    const res = await request.get('/api/v1/admin/audit-log', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
  });

  test('admin reports page loads', async ({ page }) => {
    await loginViaUi(page);
    await page.goto('/admin/reports');
    await page.waitForLoadState('networkidle');
    await expect(page.locator('body')).not.toBeEmpty();
  });
});
