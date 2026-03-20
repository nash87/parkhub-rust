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

test.describe('Dark Mode Toggle (Extended)', () => {
  test('dashboard theme toggle switches to dark mode', async ({ page }) => {
    await login(page);

    // Find the theme toggle button in the sidebar
    const themeToggle = page.getByRole('button', { name: /dark mode|light mode|theme/i });
    await expect(themeToggle).toBeVisible({ timeout: 15_000 });

    // Check initial state
    const initialIsDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));

    // Click to toggle
    await themeToggle.click();

    // State should have changed
    const afterToggle = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(afterToggle).not.toBe(initialIsDark);
  });

  test('dark mode persists via localStorage', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('parkhub_welcome_seen', '1');
      localStorage.setItem('parkhub_theme', 'dark');
    });
    await page.goto('/login');
    await expect(page.getByText('Sign In')).toBeVisible({ timeout: 15_000 });

    const isDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(isDark).toBe(true);

    // Toggle to light
    await page.evaluate(() => localStorage.setItem('parkhub_theme', 'light'));
    await page.reload();
    await expect(page.getByText('Sign In')).toBeVisible({ timeout: 15_000 });

    const isLight = await page.evaluate(() => !document.documentElement.classList.contains('dark'));
    expect(isLight).toBe(true);
  });

  test('dark mode changes background color on dashboard', async ({ page }) => {
    // Start in light mode
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('parkhub_welcome_seen', '1');
      localStorage.setItem('parkhub_theme', 'light');
    });
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
    await page.click('#demo-autofill');
    await page.click('#login-submit');
    await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });

    const lightBg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);

    // Switch to dark
    const themeToggle = page.getByRole('button', { name: /dark mode|light mode|theme/i });
    await themeToggle.click();

    const darkBg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);

    // Colors should differ
    expect(lightBg).not.toBe(darkBg);
  });

  test('dark mode applies to register page', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => {
      localStorage.setItem('parkhub_welcome_seen', '1');
      localStorage.setItem('parkhub_theme', 'dark');
    });
    await page.goto('/register');
    await expect(page.locator('#reg-name')).toBeVisible({ timeout: 15_000 });

    const isDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    expect(isDark).toBe(true);

    // Background should not be white
    const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    expect(bg).not.toBe('rgb(255, 255, 255)');
  });
});
