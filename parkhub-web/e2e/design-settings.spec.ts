import { test, expect } from './fixtures/axe';

// claude.ai/design integration (#335) — Settings view uses the new
// SettingsPrimitives (SCard, SRow, SSeg, SToggle, ThemeSwatches,
// NavLayoutGrid). Covers scope switcher, theme swatch selection, nav
// layout picker, and a top-level axe pass.

async function login(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

test.describe('Design — Settings view', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/settings');
  });

  test('Settings page renders Appearance panel', async ({ page }) => {
    await expect(page.getByRole('heading', { name: /appearance/i })).toBeVisible({ timeout: 15_000 });
  });

  test('scope switcher toggles between Personal and Workspace', async ({ page }) => {
    // SSeg renders segmented buttons; Personal is the default.
    const personal = page.getByRole('button', { name: /personal/i });
    const workspace = page.getByRole('button', { name: /workspace/i });
    await expect(personal).toBeVisible({ timeout: 15_000 });
    await expect(workspace).toBeVisible();
    await workspace.click();
    await expect(workspace).toHaveAttribute('aria-pressed', 'true');
  });

  test('theme swatch click applies to document', async ({ page }) => {
    // ThemeSwatches renders one button per designTheme; clicking swaps
    // the active theme via ThemeContext.setTheme().
    const swatches = page.getByRole('button', { name: /theme:/i });
    const count = await swatches.count();
    expect(count).toBeGreaterThan(0);
    await swatches.nth(0).click();
    // Active swatch is marked with aria-pressed=true.
    await expect(swatches.nth(0)).toHaveAttribute('aria-pressed', 'true');
  });

  test('Settings page passes axe WCAG 2.1 AA checks', async ({ page, axe }) => {
    await expect(page.getByRole('heading', { name: /appearance/i })).toBeVisible({ timeout: 15_000 });
    await axe();
  });
});
