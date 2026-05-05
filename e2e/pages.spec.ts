import { test, expect, type Page } from '@playwright/test';
import { gotoAppPage, loginBrowserViaApi, PUBLIC_ROUTES, PROTECTED_ROUTES, ADMIN_ROUTES } from './helpers';

const UNPROFESSIONAL_ROUTE_COPY = /\bAI-powered\b|\bAI-generated\b|\bKI-powered\b|lorem ipsum|Migration in Arbeit/i;

async function expectKnownRouteShell(page: Page) {
  const body = page.locator('body');
  await expect(body).not.toContainText(/\b404\b|not found|nicht gefunden/i);
  await expect(body).not.toContainText(UNPROFESSIONAL_ROUTE_COPY);
  await expect(page.locator('.blur-3xl')).toHaveCount(0);
}

test.describe('Pages — Public Routes', () => {
  for (const route of PUBLIC_ROUTES) {
    test(`${route} loads without errors`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      // Page should render some content
      await expect(page.locator('body')).not.toBeEmpty();
      await expectKnownRouteShell(page);
    });
  }

  test('/login shows a login form', async ({ page }) => {
    await gotoAppPage(page, '/login');
    await expect(page.getByRole('button', { name: /sign in|log in|login/i })).toBeVisible();
  });

  test('/register shows a registration form', async ({ page }) => {
    await gotoAppPage(page, '/register');
    await expect(page.getByRole('button', { name: /sign up|register|create/i })).toBeVisible();
  });

  test('/forgot-password shows password reset form', async ({ page }) => {
    await gotoAppPage(page, '/forgot-password');
    await expect(page.getByLabel(/email/i)).toBeVisible();
  });
});

test.describe('Pages — Protected Routes (after login)', () => {
  test.beforeEach(async ({ page }) => {
    await loginBrowserViaApi(page);
  });

  for (const route of PROTECTED_ROUTES) {
    test(`${route} loads after auth`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      await expect(page.locator('body')).not.toBeEmpty();
      await expectKnownRouteShell(page);
    });
  }

  test('dashboard shows stats or content', async ({ page }) => {
    await gotoAppPage(page, '/');
    // Dashboard should have some visible element
    await expect(page.locator('main, [data-testid], h1, h2')).not.toHaveCount(0);
  });

  test('/profile page has settings section', async ({ page }) => {
    await gotoAppPage(page, '/profile');
    await expect(page.locator('body')).toContainText(/profile|settings|theme|account/i);
  });
});

test.describe('Pages — Admin Routes (after admin login)', () => {
  test.beforeEach(async ({ page }) => {
    await loginBrowserViaApi(page);
  });

  for (const route of ADMIN_ROUTES) {
    test(`${route} loads for admin`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      await expect(page.locator('body')).not.toBeEmpty();
      await expectKnownRouteShell(page);
      await expect(page.locator('body')).not.toContainText(/returned an object instead of string/i);
    });
  }

  test('admin modules, chargers, and updates settle without broken UI chrome', async ({ page }) => {
    for (const route of ['/admin/modules', '/admin/chargers', '/admin/updates']) {
      await gotoAppPage(page, route);
      await expect(page.locator('body')).not.toContainText(/returned an object instead of string/i);
      await expect(page.locator('body')).not.toContainText(/key '[^']+' returned/i);
      await expect(page.locator('h1, h2').first()).toBeVisible();
    }

    await gotoAppPage(page, '/admin/chargers');
    await expect(page.getByText(/Total Chargers/i).first()).toBeVisible();
    await expect(page.locator('body')).not.toContainText(/Could not load charger statistics/i);

    await gotoAppPage(page, '/admin/updates');
    await expect(page.getByTestId('current-version')).not.toHaveText('—');
  });
});

test.describe('Pages — Redirects', () => {
  test('/ without auth redirects to /login or /welcome', async ({ page }) => {
    await gotoAppPage(page, '/');
    // AuthProvider shows LoadingSplash on mount until /api/v1/users/me
    // resolves, so URL rewrites to /login or /welcome only AFTER the
    // unauth check returns. Wait for the redirect before reading URL.
    await page.waitForURL(/\/(login|welcome)/, { timeout: 10_000, waitUntil: 'commit' });
  });

  test('unknown route shows 404 page', async ({ page }) => {
    await gotoAppPage(page, '/this-route-does-not-exist');
    await expect(page.locator('body')).toContainText(/not found|404/i);
  });
});
