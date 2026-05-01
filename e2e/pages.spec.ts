import { test, expect } from '@playwright/test';
import { gotoAppPage, loginBrowserViaApi, PUBLIC_ROUTES, PROTECTED_ROUTES, ADMIN_ROUTES } from './helpers';

test.describe('Pages — Public Routes', () => {
  for (const route of PUBLIC_ROUTES) {
    test(`${route} loads without errors`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      // Page should render some content
      await expect(page.locator('body')).not.toBeEmpty();
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
    });
  }
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
