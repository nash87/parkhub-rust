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

test.describe('Admin Roles', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  test('navigate to admin roles page', async ({ page }) => {
    await page.goto('/admin/roles');
    await page.waitForLoadState('networkidle');
    await expect(page.getByText('Roles & Permissions')).toBeVisible({ timeout: 15_000 });
  });

  test('roles page shows subtitle', async ({ page }) => {
    await page.goto('/admin/roles');
    await page.waitForLoadState('networkidle');
    await expect(page.getByText('Manage role-based access control')).toBeVisible({ timeout: 15_000 });
  });

  test('roles page shows create role button', async ({ page }) => {
    await page.goto('/admin/roles');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('button', { name: /Create Role/i })).toBeVisible({ timeout: 15_000 });
  });

  test('create role form opens on button click', async ({ page }) => {
    await page.goto('/admin/roles');
    await page.waitForLoadState('networkidle');
    await page.getByRole('button', { name: /Create Role/i }).click();
    // Name input should appear (placeholder: "Role name")
    await expect(page.getByPlaceholder('Role name')).toBeVisible({ timeout: 10_000 });
  });

  test('create role form shows permission checkboxes', async ({ page }) => {
    await page.goto('/admin/roles');
    await page.waitForLoadState('networkidle');
    await page.getByRole('button', { name: /Create Role/i }).click();
    // Permission labels from en.ts rbac.perm keys
    await expect(
      page.getByText('Manage Users').or(page.getByText('Manage Lots'))
    ).toBeVisible({ timeout: 10_000 });
  });

  test('roles page is accessible at /admin/roles', async ({ page }) => {
    await page.goto('/admin/roles');
    await expect(page).toHaveURL(/\/admin\/roles/);
  });

  test('roles page is reachable via admin tab navigation', async ({ page }) => {
    await page.goto('/admin');
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('link', { name: 'Roles' })).toBeVisible({ timeout: 15_000 });
    await page.getByRole('link', { name: 'Roles' }).click();
    await expect(page).toHaveURL(/\/admin\/roles/, { timeout: 15_000 });
  });
});
