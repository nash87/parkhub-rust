import { test, expect } from '@playwright/test';

test.describe('404 Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  });

  test('navigating to nonexistent route shows 404', async ({ page }) => {
    await page.goto('/nonexistent-page-xyz');
    await expect(page.getByText('404')).toBeVisible({ timeout: 30_000 });
  });

  test('404 page shows "Page not found" heading', async ({ page }) => {
    await page.goto('/this-route-does-not-exist');
    await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible({ timeout: 30_000 });
  });

  test('404 page shows descriptive message', async ({ page }) => {
    await page.goto('/some-random-path');
    await expect(page.getByText(/doesn't exist|has been moved/)).toBeVisible({ timeout: 30_000 });
  });

  test('404 page has ParkHub branding', async ({ page }) => {
    await page.goto('/nonexistent');
    await expect(page.getByText('404')).toBeVisible({ timeout: 30_000 });
    // ParkHub car icon area should be present
    await expect(page.locator('.bg-primary-600')).toBeVisible();
  });

  test('404 page has "Back to Dashboard" link', async ({ page }) => {
    await page.goto('/nonexistent');
    const backLink = page.getByRole('link', { name: /Back to Dashboard/i });
    await expect(backLink).toBeVisible({ timeout: 30_000 });
  });

  test('"Back to Dashboard" navigates to home', async ({ page }) => {
    await page.goto('/nonexistent');
    const backLink = page.getByRole('link', { name: /Back to Dashboard/i });
    await expect(backLink).toBeVisible({ timeout: 30_000 });
    await backLink.click();
    // Should redirect to login (since not authenticated) or dashboard
    await expect(page).toHaveURL(/\/(login)?$/);
  });

  test('deeply nested nonexistent route also shows 404', async ({ page }) => {
    await page.goto('/a/b/c/d/e');
    await expect(page.getByText('404')).toBeVisible({ timeout: 30_000 });
  });
});
