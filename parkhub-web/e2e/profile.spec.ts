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

test.describe('Profile Page', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/profile');
    await page.waitForLoadState('networkidle');
  });

  test('shows profile heading', async ({ page }) => {
    await expect(page.getByRole('heading', { name: /Profil|Profile/i })).toBeVisible({ timeout: 15_000 });
  });

  test('displays user name and email', async ({ page }) => {
    // The demo admin user info should be rendered
    await expect(page.getByText(/admin@parkhub\.test|Admin/)).toBeVisible({ timeout: 15_000 });
  });

  test('shows user initials avatar', async ({ page }) => {
    // Initials are rendered in a div
    const initials = page.locator('.text-xl.font-bold').first();
    await expect(initials).toBeVisible({ timeout: 15_000 });
  });

  test('has edit button', async ({ page }) => {
    const editBtn = page.getByRole('button', { name: /Edit|Bearbeiten/i });
    await expect(editBtn).toBeVisible({ timeout: 15_000 });
  });

  test('edit mode shows name and email inputs', async ({ page }) => {
    const editBtn = page.getByRole('button', { name: /Edit|Bearbeiten/i });
    await editBtn.click();

    // Edit form should show name and email fields
    await expect(page.getByLabel(/Name/i)).toBeVisible({ timeout: 10_000 });
    await expect(page.getByLabel(/E-Mail|Email/i)).toBeVisible();
  });

  test('shows password change section', async ({ page }) => {
    await expect(page.getByText(/Passwort ändern|Change Password/i)).toBeVisible({ timeout: 15_000 });
  });

  test('password section expands on click', async ({ page }) => {
    const pwSection = page.getByText(/Passwort ändern|Change Password/i);
    await pwSection.click();

    // Should show password fields
    await expect(page.getByLabel(/Aktuelles Passwort|Current Password/i)).toBeVisible({ timeout: 10_000 });
    await expect(page.getByLabel(/Neues Passwort|New Password/i)).toBeVisible();
  });

  test('shows GDPR section', async ({ page }) => {
    await expect(page.getByText('DSGVO / GDPR')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/Daten exportieren|Data Export/i)).toBeVisible();
    await expect(page.getByText(/Konto löschen|Delete Account/i)).toBeVisible();
  });

  test('shows stats cards', async ({ page }) => {
    // Profile stats section
    await expect(page.getByText(/Buchungen|Bookings/i).first()).toBeVisible({ timeout: 15_000 });
  });
});
