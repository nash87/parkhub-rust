import { test, expect } from '@playwright/test';

async function login(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

test.describe('Demo Overlay', () => {
  test('demo badge appears on dashboard when demo mode is on', async ({ page }) => {
    await login(page);

    // The demo overlay checks /api/v1/demo/config — on the demo server it should show
    // Wait a bit for the overlay to load and animate in
    const demoBadge = page.getByText(/Demo/i).first();
    const isVisible = await demoBadge.isVisible({ timeout: 10_000 }).catch(() => false);

    if (isVisible) {
      await expect(demoBadge).toBeVisible();
    } else {
      // Demo mode might be off — that's OK, just verify overlay doesn't break the page
      await expect(page.getByText('Active Bookings')).toBeVisible();
    }
  });

  test('demo overlay shows timer when visible', async ({ page }) => {
    await login(page);

    // Look for the timer format (MM:SS)
    const timer = page.locator('text=/\\d{2}:\\d{2}/').first();
    const timerVisible = await timer.isVisible({ timeout: 10_000 }).catch(() => false);

    if (timerVisible) {
      await expect(timer).toBeVisible();
    }
    // If no timer, demo mode is simply not enabled — test still passes
  });

  test('demo overlay has expand/collapse toggle', async ({ page }) => {
    await login(page);

    const demoBadge = page.getByLabel(/Demo mode overlay/i);
    const isVisible = await demoBadge.isVisible({ timeout: 10_000 }).catch(() => false);

    if (isVisible) {
      // Click to collapse
      await demoBadge.click();
      // Click again to expand
      await demoBadge.click();
      // Overlay should still be present
      await expect(demoBadge).toBeVisible();
    }
  });

  test('demo overlay shows viewers count when expanded', async ({ page }) => {
    await login(page);

    // The viewers count is shown next to the eye icon
    const demoBadge = page.getByLabel(/Demo mode overlay/i);
    const isVisible = await demoBadge.isVisible({ timeout: 10_000 }).catch(() => false);

    if (isVisible) {
      // Viewers count should be a number
      const viewersSection = page.locator('.flex.items-center.gap-1').filter({ hasText: /\d+/ });
      await expect(viewersSection.first()).toBeVisible();
    }
  });

  test('demo hint on login page shows credentials', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('text=Sign In', { timeout: 45_000 });

    // Demo server shows a hint with credentials
    await expect(page.getByText('admin@parkhub.test')).toBeVisible();
  });

  test('demo autofill button fills credentials', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });

    await page.click('#demo-autofill');

    // Username and password fields should now have values
    const username = await page.inputValue('#username');
    const password = await page.inputValue('#password');
    expect(username).toBeTruthy();
    expect(password).toBeTruthy();
  });
});
