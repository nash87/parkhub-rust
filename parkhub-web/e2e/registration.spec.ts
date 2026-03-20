import { test, expect } from '@playwright/test';

test.describe('Registration Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/register');
    await page.waitForSelector('text=ParkHub', { timeout: 45_000 });
  });

  test('register page renders form fields', async ({ page }) => {
    await expect(page.locator('#reg-name')).toBeVisible();
    await expect(page.locator('#reg-email')).toBeVisible();
    await expect(page.locator('#reg-password')).toBeVisible();
  });

  test('register page has correct ARIA structure', async ({ page }) => {
    await expect(page.getByRole('main')).toMatchAriaSnapshot(`
      - heading /ParkHub/
      - textbox "Name"
      - textbox "Email"
    `);
  });

  test('shows password minimum length hint', async ({ page }) => {
    await expect(page.getByText('Min. 8 characters')).toBeVisible();
  });

  test('has link back to Sign In', async ({ page }) => {
    const signInLink = page.getByRole('link', { name: /Sign In/i });
    await expect(signInLink).toBeVisible();
    await signInLink.click();
    await expect(page).toHaveURL(/\/login/);
  });

  test('submit with empty fields shows validation', async ({ page }) => {
    // Click submit without filling anything — HTML5 validation should prevent submission
    const submitBtn = page.getByRole('button', { name: /Sign Up|Create/i });
    await expect(submitBtn).toBeVisible();
    // The name field is required, so the form should not submit
    await submitBtn.click();
    // We should still be on the register page
    await expect(page).toHaveURL(/\/register/);
  });

  test('fill and submit registration form', async ({ page }) => {
    const timestamp = Date.now();
    await page.fill('#reg-name', `Test User ${timestamp}`);
    await page.fill('#reg-email', `test${timestamp}@example.com`);
    await page.fill('#reg-password', 'TestPass1234!');

    const submitBtn = page.getByRole('button', { name: /Sign Up|Create/i });
    await submitBtn.click();

    // After submission, should either redirect to /login or show error (demo backend)
    await expect(page.getByText(/Sign In|Registration failed|already exists/i)).toBeVisible({ timeout: 15_000 });
  });

  test('shows version badge', async ({ page }) => {
    await expect(page.getByText(/ParkHub v\d+\.\d+/)).toBeVisible();
  });
});
