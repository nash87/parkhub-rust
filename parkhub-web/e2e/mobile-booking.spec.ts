import { test, expect, devices } from '@playwright/test';

test.use(devices['iPhone 13']);

async function login(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

test.describe('Mobile Booking Flow', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('book page loads on mobile without horizontal scroll', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');
    await expect(
      page.getByText(/Book a Spot|Select a Lot|Choose|Parking/i)
    ).toBeVisible({ timeout: 15_000 });
    const hasHScroll = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('mobile book page shows step indicator', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');
    // Step indicator has numbered steps like "1.", "2.", "3."
    await expect(page.getByText(/1\./)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/2\./)).toBeVisible();
    await expect(page.getByText(/3\./)).toBeVisible();
  });

  test('mobile book page shows nearby lots or empty state', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');
    // Either lot cards or a no-lots message
    const lotOrEmpty = page.getByText(/available|No .*(lots|parking)|Select a Lot|Choose/i);
    await expect(lotOrEmpty.first()).toBeVisible({ timeout: 30_000 });
  });

  test('mobile dashboard shows active bookings section without horizontal scroll', async ({ page }) => {
    await expect(page.getByText('Active Bookings')).toBeVisible({ timeout: 15_000 });
    const hasHScroll = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('quick book action is accessible on mobile dashboard', async ({ page }) => {
    await expect(page.getByText('Book a Spot')).toBeVisible({ timeout: 15_000 });
    // Tapping "Book a Spot" navigates to the booking flow
    await page.getByText('Book a Spot').click();
    await expect(page).toHaveURL(/\/book/, { timeout: 15_000 });
  });

  test('bookings list page loads on mobile without horizontal scroll', async ({ page }) => {
    await page.goto('/bookings');
    await page.waitForLoadState('networkidle');
    await expect(
      page.getByText('My Bookings').or(page.getByText(/Bookings/i).first())
    ).toBeVisible({ timeout: 15_000 });
    const hasHScroll = await page.evaluate(
      () => document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });
});
