import { test, expect, devices } from '@playwright/test';

test.use(devices['iPhone 13']);

test.describe('Mobile', () => {
  test('welcome page fits viewport', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Get Started' }).waitFor({ timeout: 30_000 });

    // No horizontal scroll
    const hasHScroll = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(hasHScroll).toBe(false);
  });

  test('login page usable on mobile', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await expect(page.getByText('Sign In')).toBeVisible({ timeout: 15_000 });

    // Form fields are visible
    await expect(page.locator('#username')).toBeVisible();
    await expect(page.locator('#password')).toBeVisible();

    // No horizontal scroll
    const hasHScroll = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(hasHScroll).toBe(false);
  });
});
