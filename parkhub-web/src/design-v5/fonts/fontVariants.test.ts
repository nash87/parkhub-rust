import { describe, expect, it, beforeEach, vi } from 'vitest';
import {
  FONT_VARIANTS,
  V5_FONT_LABELS,
  __resetFontLoaderForTests,
  applyFontVariant,
} from './fontVariants';

describe('font variants', () => {
  beforeEach(() => {
    document.documentElement.style.removeProperty('--v5-font-family');
    __resetFontLoaderForTests();
  });

  it('exposes a descriptor for every catalog variant', () => {
    expect(Object.keys(FONT_VARIANTS).sort()).toEqual([
      'atkinson',
      'dmmono',
      'inter',
      'plex',
      'system',
    ]);
  });

  it('exposes a German-friendly label for every variant', () => {
    for (const v of Object.keys(FONT_VARIANTS) as Array<keyof typeof FONT_VARIANTS>) {
      expect(V5_FONT_LABELS[v]).toBeTruthy();
    }
  });

  it('inter / dmmono / system have no lazy loader', () => {
    expect(FONT_VARIANTS.inter.load).toBeUndefined();
    expect(FONT_VARIANTS.dmmono.load).toBeUndefined();
    expect(FONT_VARIANTS.system.load).toBeUndefined();
  });

  it('plex and atkinson are lazy-loaded', () => {
    expect(typeof FONT_VARIANTS.plex.load).toBe('function');
    expect(typeof FONT_VARIANTS.atkinson.load).toBe('function');
  });

  it('applyFontVariant sets the CSS variable for an inline variant', async () => {
    await applyFontVariant('system');
    expect(document.documentElement.style.getPropertyValue('--v5-font-family'))
      .toContain('system-ui');
  });

  it('applyFontVariant invokes lazy loader exactly once per variant', async () => {
    const spy = vi.fn().mockResolvedValue(undefined);
    const original = FONT_VARIANTS.plex.load;
    FONT_VARIANTS.plex.load = spy;
    try {
      await applyFontVariant('plex');
      await applyFontVariant('plex');
      expect(spy).toHaveBeenCalledTimes(1);
    } finally {
      FONT_VARIANTS.plex.load = original;
    }
  });

  it('applyFontVariant degrades silently on lazy-load failure', async () => {
    const original = FONT_VARIANTS.atkinson.load;
    FONT_VARIANTS.atkinson.load = () => Promise.reject(new Error('offline'));
    try {
      await expect(applyFontVariant('atkinson')).resolves.toBeUndefined();
      // Variable still set even though network failed.
      expect(document.documentElement.style.getPropertyValue('--v5-font-family'))
        .toContain('Atkinson');
    } finally {
      FONT_VARIANTS.atkinson.load = original;
    }
  });
});
