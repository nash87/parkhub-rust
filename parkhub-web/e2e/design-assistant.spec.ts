import { test, expect } from './fixtures/axe';

// claude.ai/design integration (#335) — Assistant is a ⌘. side-panel with a
// local scripted helper (no external LLM). Covers the open/close paths, the
// canned-reply surface, and an axe-core pass on the panel content.

async function login(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

test.describe('Design — Assistant (⌘.)', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('⌘. opens the assistant panel', async ({ page }) => {
    await page.keyboard.press('Control+.');
    const panel = page.getByRole('dialog', { name: /assistant/i });
    await expect(panel).toBeVisible({ timeout: 10_000 });
  });

  test('Escape closes the assistant panel', async ({ page }) => {
    await page.keyboard.press('Control+.');
    const panel = page.getByRole('dialog', { name: /assistant/i });
    await expect(panel).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(panel).toBeHidden();
  });

  test('assistant replies to "create booking" intent', async ({ page }) => {
    await page.keyboard.press('Control+.');
    const panel = page.getByRole('dialog', { name: /assistant/i });
    await expect(panel).toBeVisible();
    // Type a scripted-intent query and verify the local helper responds in-panel.
    const input = panel.getByRole('textbox').first();
    await input.fill('how do I create a booking');
    await input.press('Enter');
    // Local scripted helper responds near-instantly with a canned reply.
    await expect(panel).toContainText(/book/i, { timeout: 5_000 });
  });

  test('assistant panel passes axe WCAG 2.1 AA checks', async ({ page, axe }) => {
    await page.keyboard.press('Control+.');
    await expect(page.getByRole('dialog', { name: /assistant/i }))
      .toBeVisible({ timeout: 10_000 });
    await axe({ include: '[role="dialog"]' });
  });
});
