import { describe, expect, it } from 'vitest';
import { NAV, SECTION_HEADINGS, byId, breadcrumbFor } from './nav';

describe('v5 NAV registry', () => {
  it('contains exactly 26 screens to match the design spec', () => {
    expect(NAV).toHaveLength(26);
  });

  it('numbers screens sequentially 01…26', () => {
    NAV.forEach((n, i) => {
      expect(n.n).toBe(String(i + 1).padStart(2, '0'));
    });
  });

  it('has no duplicate ids', () => {
    const ids = NAV.map((n) => n.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('has no duplicate numbers', () => {
    const ns = NAV.map((n) => n.n);
    expect(new Set(ns).size).toBe(ns.length);
  });

  it('every section heading has at least one nav item', () => {
    const sections = new Set(NAV.map((n) => n.section));
    for (const key of Object.keys(SECTION_HEADINGS) as Array<keyof typeof SECTION_HEADINGS>) {
      expect(sections.has(key)).toBe(true);
    }
  });

  it('byId resolves every registered item back to itself', () => {
    NAV.forEach((n) => {
      expect(byId.get(n.id)).toBe(n);
    });
  });

  it('breadcrumbFor combines section + label with /', () => {
    expect(breadcrumbFor('dashboard')).toBe('Grundlagen / Dashboard');
    expect(breadcrumbFor('analytics')).toBe('Admin / Analytics');
    expect(breadcrumbFor('ev')).toBe('Flotte / EV-Laden');
  });

  it('breadcrumbFor returns empty string for unknown id', () => {
    // @ts-expect-error intentional bad id
    expect(breadcrumbFor('not-a-real-screen')).toBe('');
  });
});
