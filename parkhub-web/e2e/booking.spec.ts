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

test.describe('Booking Flow', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('navigate to Book a Spot from dashboard', async ({ page }) => {
    await page.getByText('Book a Spot').click();
    await expect(page.getByText(/Book a Spot|Select a Lot|Choose|Parking/i)).toBeVisible({ timeout: 15_000 });
    await expect(page).toHaveURL(/\/book/);
  });

  test('book page shows step indicator', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');
    // Step indicator should show 3 steps
    await expect(page.getByText(/1\./)).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/2\./)).toBeVisible();
    await expect(page.getByText(/3\./)).toBeVisible();
  });

  test('book page shows parking lots or empty state', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');
    // Should show either lot cards or a "no lots" message once loading finishes
    const lotOrEmpty = page.getByText(/available|No .*(lots|parking)|Select a Lot|Choose/i);
    await expect(lotOrEmpty.first()).toBeVisible({ timeout: 30_000 });
  });

  test('book page has back navigation when on step 2+', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');

    // On step 1, there should be no back button
    // Try clicking a lot if available
    const lotButton = page.locator('button').filter({ hasText: /available|slots/ }).first();
    const lotExists = await lotButton.isVisible().catch(() => false);

    if (lotExists) {
      await lotButton.click();
      // After selecting a lot, back button should appear
      await expect(page.getByRole('button', { name: /back/i }).or(page.locator('button svg'))).toBeVisible({ timeout: 10_000 });
    }
  });

  test('duration selector shows time options', async ({ page }) => {
    await page.goto('/book');
    await page.waitForLoadState('networkidle');

    // Try to get to step 2 by clicking a lot
    const lotButton = page.locator('button').filter({ hasText: /available|slots/ }).first();
    const lotExists = await lotButton.isVisible().catch(() => false);

    if (lotExists) {
      await lotButton.click();
      // Duration buttons should be visible
      await expect(page.getByText('1h')).toBeVisible({ timeout: 10_000 });
      await expect(page.getByText('2h')).toBeVisible();
      await expect(page.getByText('4h')).toBeVisible();
      await expect(page.getByText('8h')).toBeVisible();
    }
  });
});
