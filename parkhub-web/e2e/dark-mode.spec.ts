import { test, expect } from '@playwright/test';

test.describe('Dark Mode', () => {
  test('welcome page toggles dark mode', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Get Started' }).waitFor({ timeout: 30_000 });

    // Click theme toggle
    const toggle = page.getByRole('button', { name: /theme|dark|light/i });
    await toggle.click();

    // html should have dark class
    const isDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(isDark).toBe(true);

    // Background should be dark
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    // Dark backgrounds have low RGB values
    expect(bg).not.toBe('rgb(255, 255, 255)');
  });

  test('login page renders correctly in dark mode', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('parkhub_welcome_seen', '1');
      localStorage.setItem('parkhub_theme', 'dark');
    });
    await page.goto('/login');
    await expect(page.getByText('Sign In')).toBeVisible({ timeout: 15_000 });

    // Check dark class is applied
    const isDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(isDark).toBe(true);
  });
});
