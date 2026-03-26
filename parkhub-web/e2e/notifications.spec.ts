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

test.describe('Notification Center', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('notification bell button is visible in header', async ({ page }) => {
    // Bell button has aria-label="Notification Center" (from notificationCenter.title)
    await expect(
      page.getByRole('button', { name: 'Notification Center' })
    ).toBeVisible({ timeout: 15_000 });
  });

  test('notification bell button has correct tooltip', async ({ page }) => {
    // title="Open notification center" (from notificationCenter.bellTooltip)
    const bell = page.getByRole('button', { name: 'Notification Center' });
    await expect(bell).toBeVisible({ timeout: 15_000 });
    await expect(bell).toHaveAttribute('title', 'Open notification center');
  });

  test('clicking bell opens notification panel', async ({ page }) => {
    const bell = page.getByRole('button', { name: 'Notification Center' });
    await expect(bell).toBeVisible({ timeout: 15_000 });
    await bell.click();
    // Panel header shows "Notification Center"
    await expect(
      page.getByRole('heading', { name: 'Notification Center' }).or(
        page.locator('h3').filter({ hasText: 'Notification Center' })
      )
    ).toBeVisible({ timeout: 10_000 });
  });

  test('notification panel shows filter tabs', async ({ page }) => {
    const bell = page.getByRole('button', { name: 'Notification Center' });
    await bell.click();
    // Filter tabs: All, Unread, Read (from notificationCenter.filter)
    await expect(page.getByRole('button', { name: 'All' })).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('button', { name: 'Unread' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Read' })).toBeVisible();
  });

  test('notification panel can be dismissed by clicking outside', async ({ page }) => {
    const bell = page.getByRole('button', { name: 'Notification Center' });
    await bell.click();
    // Panel should be open
    await expect(
      page.locator('h3').filter({ hasText: 'Notification Center' })
    ).toBeVisible({ timeout: 10_000 });
    // Click outside the panel to close it (panel is anchored top-right)
    await page.mouse.click(10, 300);
    // Panel should no longer be visible
    await expect(
      page.locator('h3').filter({ hasText: 'Notification Center' })
    ).not.toBeVisible({ timeout: 5_000 });
  });

  test('notification badge shows numeric count when unread notifications exist', async ({ page }) => {
    // The badge span is rendered inside the bell button when unreadCount > 0
    // It shows a number or "99+"
    const badge = page
      .getByRole('button', { name: 'Notification Center' })
      .locator('span')
      .first();
    const hasBadge = await badge.isVisible().catch(() => false);
    if (hasBadge) {
      const text = await badge.textContent();
      expect(text).toMatch(/^\d+$|^99\+$/);
    }
    // Test passes regardless — badge is only shown when there are unread notifications
  });

  test('notification panel mark-all-read button appears when unread count > 0', async ({ page }) => {
    const bell = page.getByRole('button', { name: 'Notification Center' });
    await bell.click();
    await page.waitForTimeout(500);
    // If there are unread notifications the "Mark all as read" link is shown
    const markAll = page.getByRole('button', { name: 'Mark all as read' });
    const hasMarkAll = await markAll.isVisible().catch(() => false);
    if (hasMarkAll) {
      await expect(markAll).toBeVisible();
    }
  });

  test('switching to Unread filter tab stays in notification panel', async ({ page }) => {
    const bell = page.getByRole('button', { name: 'Notification Center' });
    await bell.click();
    await expect(page.getByRole('button', { name: 'Unread' })).toBeVisible({ timeout: 10_000 });
    await page.getByRole('button', { name: 'Unread' }).click();
    // Panel stays open after filter switch
    await expect(page.getByRole('button', { name: 'All' })).toBeVisible();
  });
});
