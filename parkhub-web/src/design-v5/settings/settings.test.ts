import { describe, expect, it, beforeEach } from 'vitest';
import {
  DEFAULT_SETTINGS,
  SETTINGS_VERSION,
  STORAGE_KEY,
  migrate,
  readStoredSettings,
  writeStoredSettings,
} from './settings';

describe('v5 settings schema', () => {
  it('default settings are at the current version', () => {
    expect(DEFAULT_SETTINGS.version).toBe(SETTINGS_VERSION);
  });

  it('default appearance mode is marble_light', () => {
    expect(DEFAULT_SETTINGS.appearance.mode).toBe('marble_light');
  });

  it('default sidebar variant is marble (current shipped UI)', () => {
    expect(DEFAULT_SETTINGS.appearance.sidebar).toBe('marble');
  });

  it('default density is comfortable', () => {
    expect(DEFAULT_SETTINGS.appearance.density).toBe('comfortable');
  });

  it('default font is inter', () => {
    expect(DEFAULT_SETTINGS.appearance.font).toBe('inter');
  });

  it('exposes 11 feature toggles', () => {
    expect(Object.keys(DEFAULT_SETTINGS.features)).toHaveLength(11);
  });
});

describe('migrate()', () => {
  it('returns defaults for non-object input', () => {
    expect(migrate(null)).toEqual(DEFAULT_SETTINGS);
    expect(migrate(undefined)).toEqual(DEFAULT_SETTINGS);
    expect(migrate('garbage')).toEqual(DEFAULT_SETTINGS);
    expect(migrate(42)).toEqual(DEFAULT_SETTINGS);
    expect(migrate([])).toEqual(DEFAULT_SETTINGS);
  });

  it('upgrades a partial v1 input to a fully-shaped object', () => {
    const partial = { appearance: { mode: 'void' } };
    const out = migrate(partial);
    expect(out.appearance.mode).toBe('void');
    expect(out.appearance.sidebar).toBe('marble'); // default kicks in
    expect(out.features.smartSuggestions).toBe(true);
  });

  it('drops unknown enum values and falls back to defaults', () => {
    const malicious = { appearance: { mode: 'haxxor', sidebar: 'evil' } };
    const out = migrate(malicious);
    expect(out.appearance.mode).toBe('marble_light');
    expect(out.appearance.sidebar).toBe('marble');
  });

  it('coerces non-boolean feature values to defaults', () => {
    const bad = { features: { smartSuggestions: 'yes', voiceCommands: 1 } };
    const out = migrate(bad);
    expect(out.features.smartSuggestions).toBe(true);
    expect(out.features.voiceCommands).toBe(false);
  });

  it('clamps font scale to known steps', () => {
    expect(migrate({ appearance: { fontScale: 0.875 } }).appearance.fontScale).toBe(0.875);
    expect(migrate({ appearance: { fontScale: 1.25 } }).appearance.fontScale).toBe(1.25);
    expect(migrate({ appearance: { fontScale: 99 } }).appearance.fontScale).toBe(1.0);
    expect(migrate({ appearance: { fontScale: '1.0' } }).appearance.fontScale).toBe(1.0);
  });

  it('always stamps the current version on output', () => {
    expect(migrate({ version: 0 }).version).toBe(SETTINGS_VERSION);
    expect(migrate({ version: 999 }).version).toBe(SETTINGS_VERSION);
  });
});

describe('storage round-trip', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('reads defaults when storage is empty', () => {
    expect(readStoredSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it('reads defaults when storage value is malformed JSON', () => {
    window.localStorage.setItem(STORAGE_KEY, '{not json');
    expect(readStoredSettings()).toEqual(DEFAULT_SETTINGS);
  });

  it('persists and restores a custom configuration', () => {
    const next = {
      ...DEFAULT_SETTINGS,
      appearance: { ...DEFAULT_SETTINGS.appearance, mode: 'void' as const, sidebar: 'columns' as const },
      features: { ...DEFAULT_SETTINGS.features, voiceCommands: true },
    };
    writeStoredSettings(next);
    const restored = readStoredSettings();
    expect(restored.appearance.mode).toBe('void');
    expect(restored.appearance.sidebar).toBe('columns');
    expect(restored.features.voiceCommands).toBe(true);
  });

  it('write swallows errors when localStorage throws', () => {
    const original = window.localStorage.setItem;
    window.localStorage.setItem = () => { throw new Error('quota'); };
    expect(() => writeStoredSettings(DEFAULT_SETTINGS)).not.toThrow();
    window.localStorage.setItem = original;
  });
});
