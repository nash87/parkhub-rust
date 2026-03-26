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

test.describe('Bookings List', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('navigate to bookings page from dashboard', async ({ page }) => {
    await page.getByRole('link', { name: 'Bookings' }).click();
    await expect(page.getByText('My Bookings')).toBeVisible({ timeout: 15_000 });
    await expect(page).toHaveURL(/\/bookings/);
  });

  test('bookings page shows status filter tabs', async ({ page }) => {
    await page.goto('/bookings');
    await page.waitForLoadState('networkidle');
    // Status filter options
    await expect(
      page.getByRole('button', { name: /all/i }).or(page.getByText(/all/i).first())
    ).toBeVisible({ timeout: 15_000 });
    await expect(
      page.getByRole('button', { name: /active/i }).or(page.getByText(/active/i).first())
    ).toBeVisible();
  });

  test('bookings page shows search input', async ({ page }) => {
    await page.goto('/bookings');
    await page.waitForLoadState('networkidle');
    await expect(page.getByPlaceholder('Search lot...')).toBeVisible({ timeout: 15_000 });
  });

  test('bookings page shows active bookings section or empty state', async ({ page }) => {
    await page.goto('/bookings');
    await page.waitForLoadState('networkidle');
    // Either bookings are listed or an empty state is shown
    const content = page.getByText(
      /Active|Upcoming|Past|No bookings|no active|no upcoming/i
    ).first();
    await expect(content).toBeVisible({ timeout: 30_000 });
  });

  test('bookings page is accessible at /bookings', async ({ page }) => {
    await page.goto('/bookings');
    await expect(page).toHaveURL(/\/bookings/);
  });
});
