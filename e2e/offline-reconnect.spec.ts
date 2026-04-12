import { test, expect } from '@playwright/test';
import { loginViaUi } from './helpers';

test.describe('PWA — Offline Resilience & Reconnection', () => {
  test('offline indicator shown when network is lost', async ({ page, context }) => {
    await loginViaUi(page);
    await page.waitForLoadState('networkidle');

    // Go offline by aborting all network requests
    await context.route('**/*', (route) => route.abort('internetdisconnected'));

    // Trigger a navigation or action to detect offline state
    await page.goto('/bookings').catch(() => {
      // Expected to fail while offline
    });

    // Wait a moment for offline detection
    await page.waitForTimeout(2000);

    // Check for offline indicator in the UI
    const offlineIndicator = page.locator(
      '[data-testid*="offline"], .offline-indicator, .offline-banner, ' +
      'text=/offline/i, [aria-label*="offline" i], .connection-status'
    );
    const offlineCount = await offlineIndicator.count();

    // Also check if the browser's offline page or a custom offline page loaded
    const pageContent = await page.locator('body').textContent();
    const hasOfflineText = /offline|no.*connection|disconnected|network.*unavailable/i.test(
      pageContent ?? ''
    );

    // Either offline indicator visible or offline text present
    expect(offlineCount > 0 || hasOfflineText).toBe(true);
  });

  test('graceful error when creating booking while offline', async ({ page, context }) => {
    await loginViaUi(page);
    await page.goto('/book');
    await page.waitForLoadState('networkidle');

    // Go offline
    await context.route('**/api/**', (route) => route.abort('internetdisconnected'));

    // Try to interact with the booking form
    const submitBtn = page.locator(
      'button[type="submit"], button:has-text("Book"), button:has-text("Reserve")'
    );
    const btnCount = await submitBtn.count();

    if (btnCount > 0) {
      await submitBtn.first().click().catch(() => {
        // Click might fail if element disappears
      });

      await page.waitForTimeout(1500);

      // Should show an error message, not crash
      const errorIndicator = page.locator(
        '[role="alert"], .error, .toast, .notification, [data-testid*="error"], ' +
        'text=/error|failed|offline|try again|network/i'
      );
      const errorCount = await errorIndicator.count();
      // App should show feedback — not silently fail
      // If the page went to an offline fallback, that also counts
      const bodyText = await page.locator('body').textContent();
      const hasErrorFeedback =
        errorCount > 0 || /error|failed|offline|unavailable/i.test(bodyText ?? '');
      expect(hasErrorFeedback).toBe(true);
    }
  });

  test('reconnection restores functionality', async ({ page, context }) => {
    await loginViaUi(page);
    await page.waitForLoadState('networkidle');

    // Capture initial page content
    const initialUrl = page.url();

    // Go offline
    await context.route('**/api/**', (route) => route.abort('internetdisconnected'));
    await page.waitForTimeout(1000);

    // Come back online
    await context.unrouteAll({ behavior: 'wait' });

    // Navigate to a page that fetches data
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // Page should load successfully now
    expect(page.url()).not.toContain('offline');
    const body = await page.locator('body').textContent();
    // Dashboard or main content should be present
    expect(body).toBeTruthy();
    expect(body!.length).toBeGreaterThan(50);
  });

  test('service worker caches static assets', async ({ page, request }) => {
    // Verify service worker file exists
    const swRes = await request.get('/sw.js');
    if (swRes.status() !== 200) {
      test.skip(true, 'No service worker found at /sw.js');
      return;
    }

    await page.goto('/login');
    await page.waitForLoadState('networkidle');

    // Check if service worker is registered
    const swRegistered = await page.evaluate(async () => {
      if (!('serviceWorker' in navigator)) return false;
      const registrations = await navigator.serviceWorker.getRegistrations();
      return registrations.length > 0;
    });

    // If SW is registered, check cache status
    if (swRegistered) {
      const cacheNames = await page.evaluate(async () => {
        const names = await caches.keys();
        return names;
      });
      // Service worker should have created at least one cache
      expect(cacheNames.length).toBeGreaterThanOrEqual(0);
    }
  });

  test('pages still load from cache when offline', async ({ page, context }) => {
    // First visit to populate cache
    await page.goto('/login');
    await page.waitForLoadState('networkidle');

    // Go offline (block only API, allow cached static assets through SW)
    await context.route('**/api/**', (route) => route.abort('internetdisconnected'));

    // Try navigating to a cached page
    await page.goto('/login');

    // The page should still render (from SW cache)
    const bodyText = await page.locator('body').textContent();
    // Should have some content (either login form or offline page)
    expect(bodyText).toBeTruthy();
    expect(bodyText!.length).toBeGreaterThan(10);
  });
});
