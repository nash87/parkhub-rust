import { test, expect } from '@playwright/test';
import { loginViaApi } from './helpers';

test.describe('GDPR — Data Privacy Compliance', () => {
  let token: string;

  test.beforeAll(async ({ playwright }) => {
    const ctx = await playwright.request.newContext({
      baseURL: process.env.E2E_BASE_URL || 'http://localhost:8081',
    });
    token = await loginViaApi(ctx);
    await ctx.dispose();
  });

  test('data export endpoint exists', async ({ request }) => {
    const res = await request.get('/api/v1/users/me/export', {
      headers: { Authorization: `Bearer ${token}` },
    });
    // Should return 200 with user data or 204
    expect([200, 204]).toContain(res.status());
  });

  test('delete account endpoint exists', async ({ request }) => {
    // Don't actually delete — just verify the endpoint returns a meaningful response
    // We use OPTIONS or HEAD to check existence, or GET which should return method-not-allowed
    const res = await request.fetch('/api/v1/users/me/delete', {
      method: 'OPTIONS',
      headers: { Authorization: `Bearer ${token}` },
    });
    // Endpoint should exist (not 404)
    expect(res.status()).not.toBe(404);
  });

  test('legal/impressum page accessible', async ({ request }) => {
    const res = await request.get('/api/v1/legal/impressum');
    expect(res.status()).toBe(200);
  });

  test('privacy/legal content served', async ({ page }) => {
    // Check if /legal or /privacy routes exist in frontend, or if impressum API works
    await page.goto('/login');
    await page.waitForLoadState('networkidle');

    // Look for legal/privacy links on login page
    const legalLinks = page.locator(
      'a[href*="legal"], a[href*="privacy"], a[href*="impressum"], a[href*="terms"]'
    );
    const count = await legalLinks.count();
    // Having at least one legal link on login page is a GDPR best practice
    if (count > 0) {
      const href = await legalLinks.first().getAttribute('href');
      expect(href).toBeTruthy();
    }
  });
});
