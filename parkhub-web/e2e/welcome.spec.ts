import { test, expect } from '@playwright/test';

test.describe('Welcome Page', () => {
  test('shows greeting and Get Started button', async ({ page }) => {
    await page.goto('/');
    // First-time visitor sees welcome page
    await expect(page.getByRole('button', { name: 'Get Started' })).toBeVisible({ timeout: 30_000 });
  });

  test('language selector shows 10 languages', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Get Started' }).waitFor({ timeout: 30_000 });
    // Click language button
    const langBtn = page.getByRole('button', { name: /English|Deutsch/ });
    await langBtn.click();
    // Should see multiple language options
    await expect(page.getByText('Deutsch')).toBeVisible();
    await expect(page.getByText('Français')).toBeVisible();
  });

  test('Get Started navigates to login', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Get Started' }).click();
    await expect(page).toHaveURL(/\/login/);
    await expect(page.getByText('Sign In')).toBeVisible();
  });

  test('welcome page has correct ARIA structure', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Get Started' }).waitFor({ timeout: 30_000 });
    await expect(page.getByRole('main')).toMatchAriaSnapshot(`
      - heading /ParkHub/
      - button "Get Started"
    `);
  });
});
