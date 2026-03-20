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

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('shows greeting', async ({ page }) => {
    await expect(page.getByText(/Good (morning|afternoon|evening)/)).toBeVisible();
  });

  test('shows stat cards', async ({ page }) => {
    await expect(page.getByText('Active Bookings')).toBeVisible();
    await expect(page.getByText('Credits Left')).toBeVisible();
  });

  test('dashboard navigation has correct ARIA structure', async ({ page }) => {
    await expect(page.getByRole('navigation')).toMatchAriaSnapshot(`
      - link /Bookings/
      - link /Admin/
    `);
  });

  test('dashboard main content has stat sections', async ({ page }) => {
    await expect(page.getByRole('main')).toMatchAriaSnapshot(`
      - heading /Good (morning|afternoon|evening)/
      - text /Active Bookings/
      - text /Credits Left/
    `);
  });

  test('quick actions visible', async ({ page }) => {
    await expect(page.getByText('Book a Spot')).toBeVisible();
  });

  test('navigate to Bookings', async ({ page }) => {
    await page.getByRole('link', { name: 'Bookings' }).click();
    await expect(page.getByText('My Bookings')).toBeVisible({ timeout: 10_000 });
  });

  test('navigate to Admin', async ({ page }) => {
    await page.getByRole('link', { name: 'Admin' }).click();
    await expect(page.getByText(/Reports|Settings/)).toBeVisible({ timeout: 10_000 });
  });
});
