import { test, expect } from '@playwright/test';

test.describe('Login Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('text=Sign In', { timeout: 45_000 });
  });

  test('shows demo hint with credentials', async ({ page }) => {
    await expect(page.getByText('admin@parkhub.test')).toBeVisible();
  });

  test('login form has correct ARIA structure', async ({ page }) => {
    await expect(page.getByRole('main')).toMatchAriaSnapshot(`
      - heading /ParkHub/ [level=1]
      - textbox "Username"
      - textbox "Password"
      - button /Sign [Ii]n/
    `);
  });

  test('login with demo autofill', async ({ page }) => {
    // Click the demo autofill button to fill credentials
    const autofill = page.locator('#demo-autofill');
    await autofill.click();
    // Submit
    await page.click('#login-submit');
    // Wait for dashboard
    await expect(page.getByText(/Active Bookings|Good/)).toBeVisible({ timeout: 30_000 });
  });

  test('login with wrong password shows error', async ({ page }) => {
    await page.fill('#username', 'admin@parkhub.test');
    await page.fill('#password', 'wrongpassword');
    await page.click('#login-submit');
    await expect(page.getByText(/Invalid|failed/i)).toBeVisible({ timeout: 15_000 });
  });

  test('shows version badge', async ({ page }) => {
    await expect(page.getByText(/ParkHub v\d+\.\d+/)).toBeVisible();
  });
});
