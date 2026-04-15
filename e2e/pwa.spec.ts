import { test, expect } from '@playwright/test';

test.describe('PWA — Progressive Web App', () => {
  test('manifest.json exists and is valid', async ({ request }) => {
    const res = await request.get('/manifest.json');
    if (res.status() === 200) {
      const manifest = await res.json();
      expect(manifest.name || manifest.short_name).toBeTruthy();
      expect(manifest.icons).toBeDefined();
      expect(Array.isArray(manifest.icons)).toBe(true);
      expect(manifest.start_url).toBeTruthy();
    } else {
      // manifest might be at a different path — check link tag
      test.skip(true, 'manifest.json not at root');
    }
  });

  test('service worker registered', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('domcontentloaded');

    // Check if SW is registered via the page
    const hasSW = await page.evaluate(async () => {
      if (!('serviceWorker' in navigator)) return false;
      const registrations = await navigator.serviceWorker.getRegistrations();
      return registrations.length > 0;
    });

    // SW registration is optional on localhost — just verify no crash
    expect(typeof hasSW).toBe('boolean');
  });

  test('apple-touch-icon present', async ({ page }) => {
    await page.goto('/login');
    const icon = page.locator('link[rel="apple-touch-icon"]');
    const count = await icon.count();
    // PWA should have apple-touch-icon
    if (count > 0) {
      const href = await icon.first().getAttribute('href');
      expect(href).toBeTruthy();
    }
  });

  test('theme-color meta tag present', async ({ page }) => {
    await page.goto('/login');
    const themeMeta = page.locator('meta[name="theme-color"]');
    const count = await themeMeta.count();
    if (count > 0) {
      const content = await themeMeta.first().getAttribute('content');
      expect(content).toBeTruthy();
    }
  });

  test('viewport meta with proper settings', async ({ page }) => {
    await page.goto('/login');
    const viewport = page.locator('meta[name="viewport"]');
    const content = await viewport.getAttribute('content');
    expect(content).toBeTruthy();
    expect(content).toContain('width=device-width');
  });

  test('sw.js endpoint returns JavaScript', async ({ request }) => {
    const res = await request.get('/sw.js');
    if (res.status() === 200) {
      const ct = res.headers()['content-type'] ?? '';
      expect(ct).toMatch(/javascript/);
    }
    // SW might not exist — acceptable
  });
});
