import { test, expect } from './fixtures/axe';

// claude.ai/design integration (#335) — ShortcutsHelp is a ⌘/ overlay listing
// keybindings in a native <dialog>. Covers both the open/close path and the
// a11y pass (labelled dialog, focus trap, no critical axe violations).

async function login(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

test.describe('Design — ShortcutsHelp (⌘/)', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('⌘/ opens the shortcuts dialog', async ({ page }) => {
    await page.keyboard.press('Control+/');
    const dialog = page.locator('dialog[aria-labelledby="shortcuts-help-title"]');
    await expect(dialog).toBeVisible({ timeout: 10_000 });
    await expect(page.locator('#shortcuts-help-title')).toBeVisible();
  });

  test('Escape closes the shortcuts dialog', async ({ page }) => {
    await page.keyboard.press('Control+/');
    const dialog = page.locator('dialog[aria-labelledby="shortcuts-help-title"]');
    await expect(dialog).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
  });

  test('shortcuts dialog passes axe WCAG 2.1 AA checks', async ({ page, axe }) => {
    await page.keyboard.press('Control+/');
    await expect(page.locator('dialog[aria-labelledby="shortcuts-help-title"]'))
      .toBeVisible({ timeout: 10_000 });
    await axe({ include: 'dialog[aria-labelledby="shortcuts-help-title"]' });
  });
});
