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

test.describe('Command Palette', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Ctrl+K opens command palette', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible({ timeout: 10_000 });
  });

  test('command palette has search input', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByTestId('command-palette-input');
    await expect(input).toBeVisible({ timeout: 10_000 });
    await expect(input).toBeFocused();
  });

  test('command palette shows action items', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await expect(page.getByTestId('command-action-book-a-spot')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId('command-action-my-bookings')).toBeVisible();
    await expect(page.getByTestId('command-action-profile')).toBeVisible();
  });

  test('typing filters actions', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByTestId('command-palette-input');
    await input.fill('book');
    // "Book a Spot" and "My Bookings" should still show
    await expect(page.getByTestId('command-action-book-a-spot')).toBeVisible({ timeout: 5_000 });
    // "Profile" should not match "book"
    await expect(page.getByTestId('command-action-profile')).not.toBeVisible();
  });

  test('typing with no match shows "No results"', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByTestId('command-palette-input');
    await input.fill('xyznonexistent');
    await expect(page.getByText('No results')).toBeVisible({ timeout: 5_000 });
  });

  test('Escape closes command palette', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible({ timeout: 10_000 });
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog', { name: 'Command palette' })).not.toBeVisible();
  });

  test('clicking backdrop closes command palette', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible({ timeout: 10_000 });
    await page.getByTestId('command-palette-backdrop').click();
    await expect(page.getByRole('dialog', { name: 'Command palette' })).not.toBeVisible();
  });

  test('selecting action navigates to page', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await page.getByTestId('command-action-profile').click();
    await expect(page).toHaveURL(/\/profile/);
  });

  test('keyboard navigation with Enter selects action', async ({ page }) => {
    await page.keyboard.press('Control+k');
    const input = page.getByTestId('command-palette-input');
    await input.fill('Profile');
    // Press Enter to select the first (and only) match
    await page.keyboard.press('Enter');
    await expect(page).toHaveURL(/\/profile/);
  });

  test('shows keyboard hints in footer', async ({ page }) => {
    await page.keyboard.press('Control+k');
    await expect(page.getByText('navigate')).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText('select')).toBeVisible();
    await expect(page.getByText('close')).toBeVisible();
  });
});
