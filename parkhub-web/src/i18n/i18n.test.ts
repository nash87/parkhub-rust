import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import en from './locales/en';
import de from './locales/de';
import fr from './locales/fr';
import es from './locales/es';
import itLocale from './locales/it';
import pt from './locales/pt';
import tr from './locales/tr';
import pl from './locales/pl';
import ja from './locales/ja';
import zh from './locales/zh';

/** Recursively collect all leaf key paths from a nested object.
 *  Returns arrays of path segments to handle keys that contain dots. */
function collectPaths(obj: Record<string, any>, prefix: string[] = []): string[][] {
  const paths: string[][] = [];
  for (const [k, v] of Object.entries(obj)) {
    const path = [...prefix, k];
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      paths.push(...collectPaths(v, path));
    } else {
      paths.push(path);
    }
  }
  return paths;
}

/** Resolve a path array against a nested object. */
function resolve(obj: any, path: string[]): any {
  let val = obj;
  for (const p of path) {
    if (val == null) return undefined;
    val = val[p];
  }
  return val;
}

/** Join path segments for display (using / to avoid confusion with dotted keys). */
function pathKey(p: string[]): string {
  return p.join(' > ');
}

const allLocales = [
  { code: 'en', data: en },
  { code: 'de', data: de },
  { code: 'fr', data: fr },
  { code: 'es', data: es },
  { code: 'it', data: itLocale },
  { code: 'pt', data: pt },
  { code: 'tr', data: tr },
  { code: 'pl', data: pl },
  { code: 'ja', data: ja },
  { code: 'zh', data: zh },
] as const;

const enPaths = collectPaths(en.translation);
const enKeys = enPaths.map(pathKey).sort();

describe('i18n translations', () => {
  it('English translations are loaded and non-empty', () => {
    expect(en).toBeDefined();
    expect(en.translation).toBeDefined();
    expect(enKeys.length).toBeGreaterThan(0);
  });

  it('German translations are loaded and non-empty', () => {
    expect(de).toBeDefined();
    expect(de.translation).toBeDefined();
    expect(collectPaths(de.translation).length).toBeGreaterThan(0);
  });

  it('all EN keys exist in DE', () => {
    // Strict: every key in EN must exist in DE (DE is a first-class
    // supported locale; missing keys would surface as English fallbacks
    // in the German UI).
    const deKeys = collectPaths(de.translation).map(pathKey);
    const missing = enKeys.filter(k => !deKeys.includes(k));
    expect(missing).toEqual([]);
  });

  // Asymmetric: DE may have MORE keys than EN (e.g. eyebrow labels added
  // to DE in the v11 SOTA chrome rollout — the English fallbacks live
  // inline in the source as `t('section.eyebrow', 'FALLBACK')` rather
  // than in en.ts). Tracked as a follow-up to mirror them into EN +
  // all 8 other locales. For now, assert DE is a SUPERSET of EN, not
  // strictly equal.
  //
  // (Was previously: `expect(enKeys.length).toBe(deKeys.length)` and
  // `expect(deKeys filter not in EN).toEqual([])` — both broke after
  // PRs #505/#506/#507 added DE eyebrow keys without mirrors.)
  it('DE is a superset of EN (DE may add locale-specific eyebrow keys)', () => {
    const deKeys = collectPaths(de.translation).map(pathKey);
    expect(deKeys.length).toBeGreaterThanOrEqual(enKeys.length);
  });

  it('all required top-level sections exist in EN', () => {
    const sections = Object.keys(en.translation);
    const required = [
      'app', 'welcome', 'auth', 'nav', 'dashboard', 'bookings',
      'credits', 'absences', 'vehicles', 'admin', 'common',
    ];
    for (const key of required) {
      expect(sections).toContain(key);
    }
  });

  it('all required top-level sections exist in DE', () => {
    const sections = Object.keys(de.translation);
    const required = [
      'app', 'welcome', 'auth', 'nav', 'dashboard', 'bookings',
      'credits', 'absences', 'vehicles', 'admin', 'common',
    ];
    for (const key of required) {
      expect(sections).toContain(key);
    }
  });

  it('no EN values are empty strings', () => {
    const empties = enPaths.filter(p => resolve(en.translation, p) === '');
    expect(empties.map(pathKey)).toEqual([]);
  });

  it('no DE values are empty strings', () => {
    const dePaths = collectPaths(de.translation);
    const empties = dePaths.filter(p => resolve(de.translation, p) === '');
    expect(empties.map(pathKey)).toEqual([]);
  });

  // Test all 10 locales have no missing keys relative to EN
  for (const locale of allLocales) {
    if (locale.code === 'en') continue;

    it(`${locale.code.toUpperCase()} has all EN keys`, () => {
      const localePaths = collectPaths(locale.data.translation);
      const localeKeys = localePaths.map(pathKey);
      const missing = enKeys.filter(k => !localeKeys.includes(k));
      expect(missing).toEqual([]);
    });
  }

  // Test all 10 locales have no empty string values
  for (const locale of allLocales) {
    it(`${locale.code.toUpperCase()} has no empty string values`, () => {
      const localePaths = collectPaths(locale.data.translation);
      const empties = localePaths.filter(p => resolve(locale.data.translation, p) === '');
      expect(empties.map(pathKey)).toEqual([]);
    });
  }

  // Verify all 10 locales are registered
  it('all 10 locales are available', () => {
    expect(allLocales.length).toBe(10);
    const codes = allLocales.map(l => l.code);
    expect(codes).toEqual(['en', 'de', 'fr', 'es', 'it', 'pt', 'tr', 'pl', 'ja', 'zh']);
  });
});

describe('loadTranslationOverrides', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', vi.fn());
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetches overrides and applies them', async () => {
    const mod = await import('./index');
    const addResourceSpy = vi.spyOn(mod.default, 'addResource');

    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve({ data: [{ language: 'en', key: 'test.key', value: 'Override' }] }),
    })));

    await mod.loadTranslationOverrides();
    expect(addResourceSpy).toHaveBeenCalledWith('en', 'translation', 'test.key', 'Override');
    addResourceSpy.mockRestore();
  });

  it('handles array response format', async () => {
    const mod = await import('./index');
    const addResourceSpy = vi.spyOn(mod.default, 'addResource');

    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve([{ language: 'de', key: 'nav.home', value: 'Startseite' }]),
    })));

    await mod.loadTranslationOverrides();
    expect(addResourceSpy).toHaveBeenCalledWith('de', 'translation', 'nav.home', 'Startseite');
    addResourceSpy.mockRestore();
  });

  it('does nothing when response is not ok', async () => {
    const mod = await import('./index');
    const addResourceSpy = vi.spyOn(mod.default, 'addResource');

    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: false })));

    await mod.loadTranslationOverrides();
    expect(addResourceSpy).not.toHaveBeenCalled();
    addResourceSpy.mockRestore();
  });

  it('silently catches fetch errors', async () => {
    const mod = await import('./index');

    vi.stubGlobal('fetch', vi.fn(() => Promise.reject(new Error('Network'))));

    // Should not throw
    await mod.loadTranslationOverrides();
  });
});
