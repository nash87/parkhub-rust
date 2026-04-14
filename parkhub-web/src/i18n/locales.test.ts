import { describe, it, expect } from 'vitest';
import en from './locales/en';
import de from './locales/de';
import es from './locales/es';
import fr from './locales/fr';
import itLocale from './locales/it';
import ja from './locales/ja';
import pl from './locales/pl';
import pt from './locales/pt';
import tr from './locales/tr';
import zh from './locales/zh';

const ALL_LOCALES: Record<string, { translation: Record<string, unknown> }> = {
  en,
  de,
  es,
  fr,
  it: itLocale,
  ja,
  pl,
  pt,
  tr,
  zh,
};

const REQUIRED_SECTIONS = [
  'nav',
  'auth',
  'bookings',
  'guestBooking',
  'swap',
  'dashboard',
  'credits',
  'vehicles',
  'absences',
  'admin',
  'common',
  'welcome',
  'notifications',
  'profile',
  'features',
  'useCase',
  'translations',
  'app',
];

function getTopLevelKeys(locale: { translation: Record<string, unknown> }): string[] {
  return Object.keys(locale.translation);
}

// Recursively collect all leaf string values
function collectValues(obj: unknown, path = ''): { path: string; value: string }[] {
  const results: { path: string; value: string }[] = [];
  if (typeof obj === 'string') {
    results.push({ path, value: obj });
  } else if (typeof obj === 'object' && obj !== null && !Array.isArray(obj)) {
    for (const [key, val] of Object.entries(obj as Record<string, unknown>)) {
      results.push(...collectValues(val, path ? `${path}.${key}` : key));
    }
  }
  return results;
}

describe('i18n locales', () => {
  const enKeys = getTopLevelKeys(en);

  it('all 10 locale files are importable', () => {
    expect(Object.keys(ALL_LOCALES)).toHaveLength(10);
    for (const [code, locale] of Object.entries(ALL_LOCALES)) {
      expect(locale.translation, `${code} should have translation key`).toBeDefined();
    }
  });

  describe.each(Object.entries(ALL_LOCALES))('%s locale', (code, locale) => {
    it('has the same top-level keys as en.ts', () => {
      const localeKeys = getTopLevelKeys(locale);
      const missing = enKeys.filter(k => !localeKeys.includes(k));
      const extra = localeKeys.filter(k => !enKeys.includes(k));

      expect(missing, `${code} is missing keys: ${missing.join(', ')}`).toEqual([]);
      expect(extra, `${code} has extra keys: ${extra.join(', ')}`).toEqual([]);
    });

    it('has no empty string values', () => {
      const values = collectValues(locale.translation);
      const empties = values.filter(v => v.value === '');
      expect(
        empties.map(e => e.path),
        `${code} has empty strings at: ${empties.map(e => e.path).join(', ')}`,
      ).toEqual([]);
    });

    it('contains all required sections', () => {
      const topKeys = getTopLevelKeys(locale);
      for (const section of REQUIRED_SECTIONS) {
        expect(topKeys, `${code} missing required section: ${section}`).toContain(section);
      }
    });

    it('translation key is an object with content', () => {
      expect(typeof locale.translation).toBe('object');
      expect(Object.keys(locale.translation).length).toBeGreaterThan(0);
    });
  });

  it('en locale has the most content (reference locale)', () => {
    const enValues = collectValues(en.translation);
    expect(enValues.length).toBeGreaterThan(100);
  });
});
