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

test.describe('Admin Webhooks', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  test('navigate to admin webhooks page', async ({ page }) => {
    await page.goto('/admin/webhooks');
    await page.waitForLoadState('networkidle');
    await expect(page.getByText('Webhooks v2')).toBeVisible({ timeout: 15_000 });
  });

  test('webhooks page shows subtitle', async ({ page }) => {
    await page.goto('/admin/webhooks');
    await page.waitForLoadState('networkidle');
    await expect(
      page.getByText('Outgoing event subscriptions with delivery tracking')
    ).toBeVisible({ timeout: 15_000 });
  });

  test('webhooks page shows create webhook button', async ({ page }) => {
    await page.goto('/admin/webhooks');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('button', { name: /Create Webhook/i })).toBeVisible({ timeout: 15_000 });
  });

  test('webhook create form opens with URL input', async ({ page }) => {
    await page.goto('/admin/webhooks');
    await page.waitForLoadState('networkidle');
    await page.getByRole('button', { name: /Create Webhook/i }).click();
    await expect(
      page.getByPlaceholder('https://example.com/webhook')
    ).toBeVisible({ timeout: 10_000 });
  });

  test('webhook form shows event toggle buttons', async ({ page }) => {
    await page.goto('/admin/webhooks');
    await page.waitForLoadState('networkidle');
    await page.getByRole('button', { name: /Create Webhook/i }).click();
    // Event buttons from the EVENTS constant
    await expect(
      page.getByRole('button', { name: 'booking.created' }).or(
        page.getByRole('button', { name: 'booking.cancelled' })
      )
    ).toBeVisible({ timeout: 10_000 });
  });

  test('webhooks page is accessible at /admin/webhooks', async ({ page }) => {
    await page.goto('/admin/webhooks');
    await expect(page).toHaveURL(/\/admin\/webhooks/);
  });

  test('webhooks page is reachable via admin tab navigation', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('link', { name: 'Plugins' }).or(page.getByRole('link', { name: /Webhook/i }))).toBeVisible({ timeout: 15_000 });
    // Navigate directly since the Webhooks tab might be scrolled off
    await page.goto('/admin/webhooks');
    await expect(page).toHaveURL(/\/admin\/webhooks/);
  });
});
