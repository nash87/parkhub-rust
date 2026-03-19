import { describe, it, expect } from 'vitest';
import en from './locales/en';
import de from './locales/de';

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

describe('i18n translations', () => {
  const enPaths = collectPaths(en.translation);
  const dePaths = collectPaths(de.translation);
  const enKeys = enPaths.map(pathKey).sort();
  const deKeys = dePaths.map(pathKey).sort();

  it('English translations are loaded and non-empty', () => {
    expect(en).toBeDefined();
    expect(en.translation).toBeDefined();
    expect(enKeys.length).toBeGreaterThan(0);
  });

  it('German translations are loaded and non-empty', () => {
    expect(de).toBeDefined();
    expect(de.translation).toBeDefined();
    expect(deKeys.length).toBeGreaterThan(0);
  });

  it('key count matches between EN and DE', () => {
    expect(enKeys.length).toBe(deKeys.length);
  });

  it('all EN keys exist in DE', () => {
    const missing = enKeys.filter(k => !deKeys.includes(k));
    expect(missing).toEqual([]);
  });

  it('all DE keys exist in EN', () => {
    const extra = deKeys.filter(k => !enKeys.includes(k));
    expect(extra).toEqual([]);
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
    const empties = dePaths.filter(p => resolve(de.translation, p) === '');
    expect(empties.map(pathKey)).toEqual([]);
  });
});
