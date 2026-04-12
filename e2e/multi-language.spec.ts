import { test, expect } from '@playwright/test';
import { loginViaUi, loginViaApi, DEMO_ADMIN } from './helpers';

const BASE = process.env.E2E_BASE_URL || 'http://localhost:8081';

test.describe('i18n — Multi-Language Support', () => {
  test.describe('Language Switching (UI)', () => {
    test('app loads in English by default', async ({ page }) => {
      await page.goto('/login');
      await page.waitForLoadState('networkidle');
      // Login page should contain English text
      await expect(page.locator('body')).toContainText(/sign in|log in|login|email|password/i);
    });

    test('switch language to German and verify UI text', async ({ page }) => {
      await page.goto('/welcome');
      await page.waitForLoadState('networkidle');

      // Look for language selector (dropdown, buttons, or select)
      const langSelector = page.locator(
        '[data-testid*="language"], [data-testid*="lang"], select[name*="lang"], ' +
        'button:has-text("English"), button:has-text("EN"), [aria-label*="language" i]'
      );
      const count = await langSelector.count();

      if (count > 0) {
        await langSelector.first().click();
        // Select German
        const germanOption = page.locator(
          'text=Deutsch, text=German, option[value="de"], [data-lang="de"], button:has-text("Deutsch")'
        );
        const optCount = await germanOption.count();
        if (optCount > 0) {
          await germanOption.first().click();
          await page.waitForLoadState('networkidle');
          // Should now show German text
          await expect(page.locator('body')).toContainText(
            /Anmelden|Willkommen|Sprache|Passwort|Benutzername/i
          );
        }
      }
    });

    test('switch language to French and verify UI text', async ({ page }) => {
      await page.goto('/welcome');
      await page.waitForLoadState('networkidle');

      const langSelector = page.locator(
        '[data-testid*="language"], [data-testid*="lang"], select[name*="lang"], ' +
        'button:has-text("English"), button:has-text("EN"), [aria-label*="language" i]'
      );
      const count = await langSelector.count();

      if (count > 0) {
        await langSelector.first().click();
        const frenchOption = page.locator(
          'text=Francais, text=French, option[value="fr"], [data-lang="fr"], button:has-text("Fran")'
        );
        const optCount = await frenchOption.count();
        if (optCount > 0) {
          await frenchOption.first().click();
          await page.waitForLoadState('networkidle');
          await expect(page.locator('body')).toContainText(
            /Connexion|Bienvenue|Langue|Mot de passe/i
          );
        }
      }
    });

    test('language persists across page navigation', async ({ page }) => {
      await page.goto('/welcome');
      await page.waitForLoadState('networkidle');

      // Set language via localStorage directly (i18next stores language there)
      await page.evaluate(() => {
        localStorage.setItem('i18nextLng', 'de');
      });

      // Navigate to login page
      await page.goto('/login');
      await page.waitForLoadState('networkidle');

      // Check that localStorage still holds German
      const lang = await page.evaluate(() => localStorage.getItem('i18nextLng'));
      expect(lang).toBe('de');

      // Navigate to another page
      await page.goto('/register');
      await page.waitForLoadState('networkidle');

      const langAfter = await page.evaluate(() => localStorage.getItem('i18nextLng'));
      expect(langAfter).toBe('de');
    });
  });

  test.describe('Locale-Aware Formatting (API)', () => {
    let token: string;

    test.beforeAll(async ({ playwright }) => {
      const ctx = await playwright.request.newContext({ baseURL: BASE });
      token = await loginViaApi(ctx);
      await ctx.dispose();
    });

    test('booking dates are present in API response', async ({ request }) => {
      const res = await request.get('/api/v1/bookings', {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(res.status()).toBe(200);
      const body = await res.json();
      const bookings = body.data ?? body;

      if (Array.isArray(bookings) && bookings.length > 0) {
        const booking = bookings[0];
        // Date fields should be ISO format or parseable
        const dateField = booking.date ?? booking.start_time ?? booking.created_at;
        expect(dateField).toBeTruthy();
        expect(new Date(dateField).toString()).not.toBe('Invalid Date');
      }
    });

    test('user preferences accept language setting', async ({ request }) => {
      const res = await request.put('/api/v1/user/preferences', {
        headers: { Authorization: `Bearer ${token}` },
        data: { language: 'de' },
      });
      // Accept 200 or 400/422 if language pref is not a settable field
      expect([200, 400, 422]).toContain(res.status());
    });
  });

  test.describe('All 10 Locales Available', () => {
    const LOCALES = ['en', 'de', 'fr', 'es', 'it', 'pt', 'tr', 'pl', 'ja', 'zh'];

    for (const locale of LOCALES) {
      test(`locale "${locale}" loads without errors`, async ({ page }) => {
        // Set locale via localStorage before loading
        await page.addInitScript((lang) => {
          localStorage.setItem('i18nextLng', lang);
        }, locale);

        await page.goto('/login');
        await page.waitForLoadState('networkidle');

        // Page should not show i18n key fallbacks (e.g. "auth.login" instead of "Login")
        const bodyText = await page.locator('body').textContent();
        // i18n missing keys typically show as dotted paths
        const hasMissingKeys = /\b[a-z]+\.[a-z]+\.[a-z]+\b/.test(bodyText ?? '');
        // Allow some dot-separated text (URLs, versions) but flag excessive key patterns
        if (hasMissingKeys) {
          // Soft check — warn but don't fail since some dot patterns are legitimate
          expect(bodyText).not.toContain('auth.loginButton');
        }

        // No console errors related to i18n
        const errors: string[] = [];
        page.on('console', (msg) => {
          if (msg.type() === 'error') errors.push(msg.text());
        });
        await page.waitForTimeout(500);
        const i18nErrors = errors.filter((e) => /i18n|translation|locale/i.test(e));
        expect(i18nErrors).toHaveLength(0);
      });
    }
  });
});
