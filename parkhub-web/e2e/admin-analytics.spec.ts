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

test.describe('Admin Analytics', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  test('navigate to admin analytics page', async ({ page }) => {
    await page.goto('/admin/analytics');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('heading', { name: 'Analytics' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Comprehensive parking analytics and trends')).toBeVisible();
  });

  test('analytics page shows time range buttons', async ({ page }) => {
    await page.goto('/admin/analytics');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('button', { name: '7d' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: '30d' })).toBeVisible();
    await expect(page.getByRole('button', { name: '90d' })).toBeVisible();
    await expect(page.getByRole('button', { name: '1y' })).toBeVisible();
  });

  test('analytics page has CSV export button', async ({ page }) => {
    await page.goto('/admin/analytics');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('button', { name: /CSV/i })).toBeVisible({ timeout: 15_000 });
  });

  test('analytics stat cards appear after loading', async ({ page }) => {
    await page.goto('/admin/analytics');
    await page.waitForLoadState('networkidle');
    // Stat cards or loading skeletons should be visible
    const statsContent = page
      .getByText(/Total Bookings|Total Revenue|Avg Duration|Active Users/)
      .first();
    await expect(statsContent).toBeVisible({ timeout: 30_000 });
  });

  test('clicking a time range button keeps it visible', async ({ page }) => {
    await page.goto('/admin/analytics');
    await page.waitForLoadState('networkidle');
    const btn7d = page.getByRole('button', { name: '7d' });
    await expect(btn7d).toBeVisible({ timeout: 15_000 });
    await btn7d.click();
    await expect(btn7d).toBeVisible();
  });

  test('analytics page is accessible at /admin/analytics', async ({ page }) => {
    await page.goto('/admin/analytics');
    await expect(page).toHaveURL(/\/admin\/analytics/);
  });

  test('analytics page is reachable via admin tab navigation', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('link', { name: 'Analytics' })).toBeVisible({ timeout: 15_000 });
    await page.getByRole('link', { name: 'Analytics' }).click();
    await expect(page).toHaveURL(/\/admin\/analytics/, { timeout: 15_000 });
  });
});
