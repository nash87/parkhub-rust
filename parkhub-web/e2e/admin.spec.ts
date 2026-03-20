import { test, expect } from '@playwright/test';

async function loginAsAdmin(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

test.describe('Admin Panel', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  test('admin link visible in navigation', async ({ page }) => {
    await expect(page.getByRole('link', { name: /Admin/i })).toBeVisible();
  });

  test('navigate to admin dashboard', async ({ page }) => {
    await page.getByRole('link', { name: /Admin/i }).click();
    await expect(page.getByRole('heading', { name: 'Admin' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Manage your ParkHub instance')).toBeVisible();
  });

  test('admin has tab navigation', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');

    await expect(page.getByRole('link', { name: 'Overview' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('link', { name: 'Settings' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Users' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Lots' })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Reports' })).toBeVisible();
  });

  test('admin reports page shows stats', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');

    // Default admin page shows reports with stats
    await expect(page.getByText('Reports').first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Total Users')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Total Lots')).toBeVisible();
    await expect(page.getByText('Total Bookings')).toBeVisible();
    await expect(page.getByText('Active Bookings')).toBeVisible();
  });

  test('admin reports shows overview section', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');

    await expect(page.getByText('Overview').first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Utilization Rate')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Avg. Bookings per User')).toBeVisible();
  });

  test('navigate to admin users page', async ({ page }) => {
    await page.goto('/admin/users');
    await page.waitForLoadState('networkidle');

    await expect(page.getByRole('heading', { name: /Users/i }).or(page.getByText('Users').first())).toBeVisible({ timeout: 15_000 });
    // Should see user table headers
    await expect(page.getByText('Role').first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Credits').first()).toBeVisible();
    await expect(page.getByText('Status').first()).toBeVisible();
  });

  test('admin users page has search input', async ({ page }) => {
    await page.goto('/admin/users');
    await page.waitForLoadState('networkidle');

    const searchInput = page.getByPlaceholder('Search users...');
    await expect(searchInput).toBeVisible({ timeout: 15_000 });
  });

  test('admin users search filters list', async ({ page }) => {
    await page.goto('/admin/users');
    await page.waitForLoadState('networkidle');

    const searchInput = page.getByPlaceholder('Search users...');
    await expect(searchInput).toBeVisible({ timeout: 15_000 });
    await searchInput.fill('admin');
    // Should still show at least the admin user
    await expect(page.getByText('admin').first()).toBeVisible({ timeout: 10_000 });
  });

  test('navigate to admin reports tab', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');

    await page.getByRole('link', { name: 'Reports' }).click();
    await expect(page).toHaveURL(/\/admin\/reports/);
    await expect(page.getByText('Bookings This Week')).toBeVisible({ timeout: 15_000 });
  });
});
