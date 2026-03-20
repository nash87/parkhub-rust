import { test, expect } from '@playwright/test';

// Test with a narrow mobile viewport (375px like iPhone SE/13 mini)
test.use({ viewport: { width: 375, height: 812 } });

test.describe('Responsive (375px Mobile)', () => {
  test('welcome page fits mobile viewport', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Get Started' }).waitFor({ timeout: 30_000 });

    const hasHScroll = await page.evaluate(() =>
      document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('login page usable on 375px mobile', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await expect(page.getByText('Sign In')).toBeVisible({ timeout: 15_000 });

    await expect(page.locator('#username')).toBeVisible();
    await expect(page.locator('#password')).toBeVisible();
    await expect(page.locator('#login-submit')).toBeVisible();

    const hasHScroll = await page.evaluate(() =>
      document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('register page fits mobile viewport', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/register');
    await expect(page.locator('#reg-name')).toBeVisible({ timeout: 15_000 });

    const hasHScroll = await page.evaluate(() =>
      document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('dashboard shows mobile header with hamburger menu', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
    await page.click('#demo-autofill');
    await page.click('#login-submit');
    await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });

    // On mobile, the sidebar is hidden and a hamburger menu button appears
    const menuButton = page.getByLabel('Open navigation menu');
    await expect(menuButton).toBeVisible();

    // Desktop sidebar should not be visible
    const desktopSidebar = page.locator('aside.hidden.lg\\:flex');
    await expect(desktopSidebar).not.toBeVisible();
  });

  test('mobile hamburger menu opens sidebar overlay', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
    await page.click('#demo-autofill');
    await page.click('#login-submit');
    await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });

    const menuButton = page.getByLabel('Open navigation menu');
    await menuButton.click();

    // Mobile sidebar overlay should appear
    const mobileSidebar = page.getByRole('dialog', { name: 'Navigation menu' });
    await expect(mobileSidebar).toBeVisible({ timeout: 10_000 });

    // Close button should be available
    const closeButton = page.getByLabel('Close navigation menu');
    await expect(closeButton).toBeVisible();
  });

  test('mobile sidebar navigation works', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
    await page.click('#demo-autofill');
    await page.click('#login-submit');
    await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });

    const menuButton = page.getByLabel('Open navigation menu');
    await menuButton.click();

    // Navigate via mobile sidebar
    const profileLink = page.getByRole('dialog', { name: 'Navigation menu' }).getByText(/Profile|Profil/i);
    await profileLink.click();

    await expect(page).toHaveURL(/\/profile/);
    // Sidebar should close after navigation
    await expect(page.getByRole('dialog', { name: 'Navigation menu' })).not.toBeVisible();
  });

  test('dashboard has no horizontal scroll on mobile', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
    await page.click('#demo-autofill');
    await page.click('#login-submit');
    await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });

    const hasHScroll = await page.evaluate(() =>
      document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('404 page fits mobile viewport', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/nonexistent');
    await expect(page.getByText('404')).toBeVisible({ timeout: 30_000 });

    const hasHScroll = await page.evaluate(() =>
      document.documentElement.scrollWidth > document.documentElement.clientWidth
    );
    expect(hasHScroll).toBe(false);
  });

  test('mobile theme toggle is accessible', async ({ page }) => {
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
    await page.goto('/login');
    await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
    await page.click('#demo-autofill');
    await page.click('#login-submit');
    await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });

    // Mobile header has its own theme toggle
    const themeButton = page.getByLabel(/light mode|dark mode/i);
    await expect(themeButton).toBeVisible();
  });
});
